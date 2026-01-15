//! Email indexer using Tantivy
//!
//! Provides full-text search indexing for email messages.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::path::Path;
use std::sync::Arc;
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    doc,
    query::{BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{
        Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, FAST, STORED, STRING,
    },
    tokenizer::{LowerCaser, RemoveLongFilter, SimpleTokenizer, TextAnalyzer},
    Index, IndexReader, IndexWriter, IndexSettings, ReloadPolicy, TantivyDocument, Term,
};
use tokio::sync::RwLock;

use super::types::{SearchQuery, SearchResult, SearchResults};

/// Schema fields for email documents
pub struct EmailFields {
    pub message_id: Field,
    pub owner_email: Field,
    pub folder: Field,
    pub from: Field,
    pub to: Field,
    pub subject: Field,
    pub body: Field,
    pub date_timestamp: Field,
}

/// Email indexer for full-text search
pub struct EmailIndexer {
    index: Index,
    reader: IndexReader,
    writer: Arc<RwLock<IndexWriter>>,
    fields: EmailFields,
    query_parser: QueryParser,
}

impl EmailIndexer {
    /// Create a new indexer at the given path
    pub fn new(index_path: &Path) -> Result<Self> {
        // Create directory if it doesn't exist
        std::fs::create_dir_all(index_path)?;

        // Build schema
        let (schema, fields) = Self::build_schema();

        // Open or create index
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path)?
        } else {
            let dir = MmapDirectory::open(index_path)?;
            Index::create(dir, schema.clone(), IndexSettings::default())?
        };

        // Register custom tokenizer for better search
        let text_analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(100))
            .filter(LowerCaser)
            .build();
        index.tokenizers().register("email_tokenizer", text_analyzer);

        // Create reader with automatic reload
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        // Create writer with 50MB buffer
        let writer = index.writer(50_000_000)?;

        // Create query parser searching across subject and body
        let query_parser = QueryParser::for_index(&index, vec![fields.subject, fields.body, fields.from, fields.to]);

        Ok(Self {
            index,
            reader,
            writer: Arc::new(RwLock::new(writer)),
            fields,
            query_parser,
        })
    }

    /// Build the Tantivy schema
    fn build_schema() -> (Schema, EmailFields) {
        let mut schema_builder = Schema::builder();

        // Text field options for searchable content
        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer("email_tokenizer")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();

        // Fields
        let message_id = schema_builder.add_text_field("message_id", STRING | STORED);
        let owner_email = schema_builder.add_text_field("owner_email", STRING | STORED);
        let folder = schema_builder.add_text_field("folder", STRING | STORED);
        let from = schema_builder.add_text_field("from", text_options.clone());
        let to = schema_builder.add_text_field("to", text_options.clone());
        let subject = schema_builder.add_text_field("subject", text_options.clone());
        let body = schema_builder.add_text_field("body", text_options);
        let date_timestamp = schema_builder.add_i64_field("date_timestamp", FAST | STORED);

        let schema = schema_builder.build();

        let fields = EmailFields {
            message_id,
            owner_email,
            folder,
            from,
            to,
            subject,
            body,
            date_timestamp,
        };

        (schema, fields)
    }

    /// Index a single email
    pub async fn index_email(
        &self,
        message_id: &str,
        owner_email: &str,
        folder: &str,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
        date: DateTime<Utc>,
    ) -> Result<()> {
        // First remove any existing document with this message_id
        self.remove_email(message_id).await?;

        // Create document
        let mut writer = self.writer.write().await;
        writer.add_document(doc!(
            self.fields.message_id => message_id,
            self.fields.owner_email => owner_email,
            self.fields.folder => folder,
            self.fields.from => from,
            self.fields.to => to,
            self.fields.subject => subject,
            self.fields.body => body,
            self.fields.date_timestamp => date.timestamp(),
        ))?;

        Ok(())
    }

    /// Remove an email from index
    pub async fn remove_email(&self, message_id: &str) -> Result<()> {
        let mut writer = self.writer.write().await;
        let term = Term::from_field_text(self.fields.message_id, message_id);
        writer.delete_term(term);
        Ok(())
    }

    /// Commit pending changes
    pub async fn commit(&self) -> Result<()> {
        let mut writer = self.writer.write().await;
        writer.commit()?;
        Ok(())
    }

    /// Search emails for a user
    pub async fn search(&self, owner_email: &str, query: SearchQuery) -> Result<SearchResults> {
        let start_time = std::time::Instant::now();

        let searcher = self.reader.searcher();
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        // Build the query
        let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        // Owner email must match
        let owner_term = Term::from_field_text(self.fields.owner_email, owner_email);
        subqueries.push((Occur::Must, Box::new(TermQuery::new(owner_term, IndexRecordOption::Basic))));

        // Folder filter if specified
        if let Some(folder) = &query.folder {
            let folder_term = Term::from_field_text(self.fields.folder, folder);
            subqueries.push((Occur::Must, Box::new(TermQuery::new(folder_term, IndexRecordOption::Basic))));
        }

        // Parse text query
        if !query.query.is_empty() {
            let parsed_query = self.query_parser.parse_query(&query.query)
                .map_err(|e| anyhow!("Query parse error: {}", e))?;
            subqueries.push((Occur::Must, Box::new(parsed_query)));
        }

        let combined_query = BooleanQuery::new(subqueries);

        // Execute search
        let top_docs = searcher.search(&combined_query, &TopDocs::with_limit(limit + offset))?;
        let total = top_docs.len();

        // Convert results
        let mut results = Vec::new();
        for (score, doc_address) in top_docs.into_iter().skip(offset).take(limit) {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let message_id = retrieved_doc
                .get_first(self.fields.message_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let subject = retrieved_doc
                .get_first(self.fields.subject)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let from = retrieved_doc
                .get_first(self.fields.from)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let folder = retrieved_doc
                .get_first(self.fields.folder)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let body = retrieved_doc
                .get_first(self.fields.body)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let date_timestamp = retrieved_doc
                .get_first(self.fields.date_timestamp)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let date = DateTime::from_timestamp(date_timestamp, 0)
                .unwrap_or_else(|| Utc::now());

            // Create snippet from body
            let snippet = Self::create_snippet(&body, &query.query, 150);

            results.push(SearchResult {
                message_id,
                subject,
                from,
                date,
                folder,
                snippet,
                score,
            });
        }

        let query_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(SearchResults {
            results,
            total,
            query_time_ms,
        })
    }

    /// Create a search snippet with highlighted terms
    fn create_snippet(body: &str, query: &str, max_len: usize) -> String {
        let body_lower = body.to_lowercase();
        let query_terms: Vec<&str> = query.split_whitespace().collect();

        // Find the first occurrence of any query term
        let mut best_pos = 0;
        for term in &query_terms {
            if let Some(pos) = body_lower.find(&term.to_lowercase()) {
                best_pos = pos;
                break;
            }
        }

        // Calculate snippet boundaries
        let start = if best_pos > 50 {
            body[..best_pos].rfind(' ').map(|p| p + 1).unwrap_or(best_pos.saturating_sub(50))
        } else {
            0
        };

        let end = std::cmp::min(start + max_len, body.len());
        let end = body[..end].rfind(' ').unwrap_or(end);

        let mut snippet = String::new();
        if start > 0 {
            snippet.push_str("...");
        }
        snippet.push_str(body[start..end].trim());
        if end < body.len() {
            snippet.push_str("...");
        }

        snippet
    }

    /// Get document count
    pub fn document_count(&self) -> u64 {
        let searcher = self.reader.searcher();
        searcher.num_docs()
    }

    /// Get index size in bytes (approximate)
    pub fn index_size_bytes(&self) -> u64 {
        // Return approximate size based on document count
        self.document_count() * 1024 // Rough estimate: 1KB per document
    }

    /// Re-index all emails for a user from their mailbox
    pub async fn reindex_mailbox(&self, mailbox_path: &Path, owner_email: &str) -> Result<u64> {
        let mut indexed = 0u64;

        // Walk through mailbox directories
        if !mailbox_path.exists() {
            return Ok(0);
        }

        for folder_entry in std::fs::read_dir(mailbox_path)? {
            let folder_entry = folder_entry?;
            let folder_path = folder_entry.path();

            if !folder_path.is_dir() {
                continue;
            }

            let folder_name = folder_entry.file_name().to_string_lossy().to_string();

            // Look for email files (typically in cur/ or new/ subdirectories)
            for subdir in &["cur", "new", "."] {
                let mail_dir = folder_path.join(subdir);
                if !mail_dir.exists() || !mail_dir.is_dir() {
                    continue;
                }

                for mail_entry in std::fs::read_dir(&mail_dir)? {
                    let mail_entry = mail_entry?;
                    let mail_path = mail_entry.path();

                    if !mail_path.is_file() {
                        continue;
                    }

                    // Try to parse and index the email
                    if let Ok(content) = std::fs::read(&mail_path) {
                        if let Some(parsed) = mail_parser::MessageParser::default().parse(&content) {
                            let message_id = mail_path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();

                            let from = parsed.from()
                                .and_then(|f| f.first())
                                .map(|a| a.address().map(|s| s.to_string()).unwrap_or_default())
                                .unwrap_or_default();

                            let to = parsed.to()
                                .and_then(|t| t.first())
                                .map(|a| a.address().map(|s| s.to_string()).unwrap_or_default())
                                .unwrap_or_default();

                            let subject = parsed.subject().unwrap_or("").to_string();

                            let body = parsed.body_text(0)
                                .map(|b| b.to_string())
                                .unwrap_or_default();

                            let date = parsed.date()
                                .map(|d| DateTime::from_timestamp(d.to_timestamp(), 0).unwrap_or_else(|| Utc::now()))
                                .unwrap_or_else(|| Utc::now());

                            if let Err(e) = self.index_email(
                                &message_id,
                                owner_email,
                                &folder_name,
                                &from,
                                &to,
                                &subject,
                                &body,
                                date,
                            ).await {
                                tracing::warn!("Failed to index email {}: {}", message_id, e);
                            } else {
                                indexed += 1;
                            }
                        }
                    }
                }
            }
        }

        // Commit all changes
        self.commit().await?;

        Ok(indexed)
    }
}

//! Search manager
//!
//! Provides a high-level interface for email search operations.

use anyhow::Result;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::indexer::EmailIndexer;
use super::types::*;

/// Search manager configuration
pub struct SearchConfig {
    /// Path to the search index directory
    pub index_path: PathBuf,
    /// Path to the mailbox root directory
    pub mailbox_path: PathBuf,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from("/var/lib/mail-rs/search-index"),
            mailbox_path: PathBuf::from("/var/mail"),
        }
    }
}

/// Search manager
pub struct SearchManager {
    indexer: Arc<RwLock<Option<EmailIndexer>>>,
    config: SearchConfig,
    is_indexing: Arc<AtomicBool>,
    last_indexed_at: Arc<RwLock<Option<chrono::DateTime<Utc>>>>,
}

impl SearchManager {
    /// Create a new search manager with default configuration
    pub fn new() -> Self {
        Self::with_config(SearchConfig::default())
    }

    /// Create a new search manager with custom configuration
    pub fn with_config(config: SearchConfig) -> Self {
        Self {
            indexer: Arc::new(RwLock::new(None)),
            config,
            is_indexing: Arc::new(AtomicBool::new(false)),
            last_indexed_at: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the search index
    pub async fn init(&self) -> Result<()> {
        let indexer = EmailIndexer::new(&self.config.index_path)?;
        let mut guard = self.indexer.write().await;
        *guard = Some(indexer);
        Ok(())
    }

    /// Search emails for a user
    pub async fn search(&self, email: &str, query: SearchQuery) -> Result<SearchResults> {
        let guard = self.indexer.read().await;
        if let Some(indexer) = guard.as_ref() {
            indexer.search(email, query).await
        } else {
            Ok(SearchResults {
                results: vec![],
                total: 0,
                query_time_ms: 0,
            })
        }
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
        date: chrono::DateTime<Utc>,
    ) -> Result<()> {
        let guard = self.indexer.read().await;
        if let Some(indexer) = guard.as_ref() {
            indexer.index_email(message_id, owner_email, folder, from, to, subject, body, date).await?;
            indexer.commit().await?;
        }
        Ok(())
    }

    /// Remove an email from the index
    pub async fn remove_email(&self, message_id: &str) -> Result<()> {
        let guard = self.indexer.read().await;
        if let Some(indexer) = guard.as_ref() {
            indexer.remove_email(message_id).await?;
            indexer.commit().await?;
        }
        Ok(())
    }

    /// Re-index all emails for a user
    pub async fn reindex_user(&self, email: &str) -> Result<u64> {
        if self.is_indexing.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Indexing already in progress"));
        }

        self.is_indexing.store(true, Ordering::SeqCst);

        let result = async {
            let guard = self.indexer.read().await;
            if let Some(indexer) = guard.as_ref() {
                // Construct mailbox path for user
                let local_part = email.split('@').next().unwrap_or(email);
                let mailbox_path = self.config.mailbox_path.join(local_part);

                let count = indexer.reindex_mailbox(&mailbox_path, email).await?;

                let mut last_indexed = self.last_indexed_at.write().await;
                *last_indexed = Some(Utc::now());

                Ok(count)
            } else {
                Ok(0)
            }
        }.await;

        self.is_indexing.store(false, Ordering::SeqCst);

        result
    }

    /// Re-index all users
    pub async fn reindex_all(&self) -> Result<u64> {
        if self.is_indexing.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Indexing already in progress"));
        }

        self.is_indexing.store(true, Ordering::SeqCst);

        let result = async {
            let guard = self.indexer.read().await;
            if let Some(indexer) = guard.as_ref() {
                let mut total_indexed = 0u64;

                // List all user directories in mailbox path
                if self.config.mailbox_path.exists() {
                    for entry in std::fs::read_dir(&self.config.mailbox_path)? {
                        let entry = entry?;
                        let path = entry.path();

                        if path.is_dir() {
                            let username = entry.file_name().to_string_lossy().to_string();
                            // Skip system directories
                            if username.starts_with('.') {
                                continue;
                            }

                            let email = format!("{}@localhost", username);
                            match indexer.reindex_mailbox(&path, &email).await {
                                Ok(count) => {
                                    total_indexed += count;
                                    tracing::info!("Indexed {} emails for {}", count, email);
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to index mailbox for {}: {}", email, e);
                                }
                            }
                        }
                    }
                }

                let mut last_indexed = self.last_indexed_at.write().await;
                *last_indexed = Some(Utc::now());

                Ok(total_indexed)
            } else {
                Ok(0)
            }
        }.await;

        self.is_indexing.store(false, Ordering::SeqCst);

        result
    }

    /// Get index status
    pub async fn get_status(&self) -> Result<IndexStatus> {
        let guard = self.indexer.read().await;
        let (document_count, index_size_bytes) = if let Some(indexer) = guard.as_ref() {
            (indexer.document_count(), indexer.index_size_bytes())
        } else {
            (0, 0)
        };

        let last_indexed = self.last_indexed_at.read().await;

        Ok(IndexStatus {
            document_count,
            index_size_bytes,
            last_indexed_at: *last_indexed,
            is_indexing: self.is_indexing.load(Ordering::SeqCst),
        })
    }

    /// Clear all indexed data
    pub async fn clear_index(&self) -> Result<()> {
        // Drop the current indexer
        {
            let mut guard = self.indexer.write().await;
            *guard = None;
        }

        // Remove index directory
        if self.config.index_path.exists() {
            std::fs::remove_dir_all(&self.config.index_path)?;
        }

        // Recreate the indexer
        self.init().await?;

        Ok(())
    }
}

impl Default for SearchManager {
    fn default() -> Self {
        Self::new()
    }
}

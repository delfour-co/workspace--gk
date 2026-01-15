//! Import/Export manager
//!
//! Provides management of import and export operations.

use anyhow::{anyhow, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use super::mbox::{MboxReader, MboxWriter, count_messages};
use super::types::*;

/// Import/Export manager
pub struct ImportExportManager {
    /// Active export jobs
    export_jobs: Arc<RwLock<HashMap<String, ExportJob>>>,
    /// Active import jobs
    import_jobs: Arc<RwLock<HashMap<String, ImportJob>>>,
    /// Statistics
    stats: Arc<RwLock<ImportExportStats>>,
    /// Base path for exports
    export_path: PathBuf,
    /// Maildir root path
    maildir_root: PathBuf,
}

impl ImportExportManager {
    /// Create a new import/export manager
    pub fn new(export_path: PathBuf, maildir_root: PathBuf) -> Self {
        Self {
            export_jobs: Arc::new(RwLock::new(HashMap::new())),
            import_jobs: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ImportExportStats::default())),
            export_path,
            maildir_root,
        }
    }

    /// Initialize the export directory
    pub async fn init(&self) -> Result<()> {
        if !self.export_path.exists() {
            fs::create_dir_all(&self.export_path)?;
        }
        Ok(())
    }

    /// Start an export job
    pub async fn start_export(&self, request: ExportRequest) -> Result<ExportJob> {
        let job_id = Uuid::new_v4().to_string();

        // Get maildir path for user
        let user_maildir = self.maildir_root.join(&request.email);
        if !user_maildir.exists() {
            return Err(anyhow!("Maildir not found for user: {}", request.email));
        }

        // Count messages to export
        let total_messages = self.count_user_messages(&user_maildir, &request.folders)?;

        let job = ExportJob {
            id: job_id.clone(),
            email: request.email.clone(),
            format: request.format,
            status: OperationStatus::Pending,
            progress: 0,
            total_messages,
            exported_messages: 0,
            output_path: None,
            file_size: None,
            error: None,
            created_at: Utc::now(),
            completed_at: None,
        };

        {
            let mut jobs = self.export_jobs.write().await;
            jobs.insert(job_id.clone(), job.clone());
        }

        // Start export in background
        let manager = self.clone_state();
        let req = request.clone();
        tokio::spawn(async move {
            let _ = manager.run_export(&job_id, req).await;
        });

        Ok(job)
    }

    /// Clone state for spawned tasks
    fn clone_state(&self) -> ImportExportManager {
        ImportExportManager {
            export_jobs: self.export_jobs.clone(),
            import_jobs: self.import_jobs.clone(),
            stats: self.stats.clone(),
            export_path: self.export_path.clone(),
            maildir_root: self.maildir_root.clone(),
        }
    }

    /// Run the actual export
    async fn run_export(&self, job_id: &str, request: ExportRequest) -> Result<()> {
        // Update status to running
        self.update_export_status(job_id, OperationStatus::Running, None).await;

        let user_maildir = self.maildir_root.join(&request.email);
        let output_filename = format!(
            "{}_{}.{}",
            request.email.replace('@', "_"),
            Utc::now().format("%Y%m%d_%H%M%S"),
            match request.format {
                ExportFormat::Mbox => "mbox",
                ExportFormat::Eml => "eml",
                ExportFormat::EmlZip => "zip",
            }
        );
        let output_path = self.export_path.join(&output_filename);

        let result = match request.format {
            ExportFormat::Mbox => {
                self.export_mbox(job_id, &user_maildir, &output_path, &request).await
            }
            ExportFormat::Eml | ExportFormat::EmlZip => {
                self.export_eml(job_id, &user_maildir, &output_path, &request).await
            }
        };

        match result {
            Ok(exported) => {
                let file_size = fs::metadata(&output_path).ok().map(|m| m.len());

                {
                    let mut jobs = self.export_jobs.write().await;
                    if let Some(job) = jobs.get_mut(job_id) {
                        job.status = OperationStatus::Completed;
                        job.progress = 100;
                        job.exported_messages = exported;
                        job.output_path = Some(output_path.to_string_lossy().to_string());
                        job.file_size = file_size;
                        job.completed_at = Some(Utc::now());
                    }
                }

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.total_exports += 1;
                    stats.messages_exported += exported;
                    if let Some(size) = file_size {
                        stats.bytes_exported += size;
                    }
                }
            }
            Err(e) => {
                self.update_export_status(job_id, OperationStatus::Failed, Some(e.to_string())).await;
                // Clean up partial output
                let _ = fs::remove_file(&output_path);
            }
        }

        Ok(())
    }

    /// Export to MBOX format
    async fn export_mbox(
        &self,
        job_id: &str,
        user_maildir: &Path,
        output_path: &Path,
        request: &ExportRequest,
    ) -> Result<u64> {
        let file = File::create(output_path)?;
        let mut writer = MboxWriter::new(BufWriter::new(file));

        let folders = self.get_folders_to_export(user_maildir, &request.folders)?;
        let mut exported = 0u64;

        for folder in folders {
            let folder_path = user_maildir.join(&folder);

            // Read from cur and new directories
            for subdir in &["cur", "new"] {
                let subdir_path = folder_path.join(subdir);
                if !subdir_path.exists() {
                    continue;
                }

                for entry in fs::read_dir(&subdir_path)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_file() {
                        let content = fs::read(&path)?;

                        // Extract From header
                        let from = extract_from_header(&content).unwrap_or_default();

                        // Extract Date header
                        let date = extract_date_header(&content);

                        writer.write_message(&from, date, &content)?;
                        exported += 1;

                        // Update progress
                        self.update_export_progress(job_id, exported).await;
                    }
                }
            }
        }

        Ok(exported)
    }

    /// Export to EML format
    async fn export_eml(
        &self,
        job_id: &str,
        user_maildir: &Path,
        output_path: &Path,
        request: &ExportRequest,
    ) -> Result<u64> {
        let folders = self.get_folders_to_export(user_maildir, &request.folders)?;
        let mut exported = 0u64;

        if request.format == ExportFormat::EmlZip {
            let file = File::create(output_path)?;
            let mut zip = ZipWriter::new(file);
            let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            for folder in folders {
                let folder_path = user_maildir.join(&folder);

                for subdir in &["cur", "new"] {
                    let subdir_path = folder_path.join(subdir);
                    if !subdir_path.exists() {
                        continue;
                    }

                    for entry in fs::read_dir(&subdir_path)? {
                        let entry = entry?;
                        let path = entry.path();

                        if path.is_file() {
                            let content = fs::read(&path)?;
                            let filename = format!("{}/{}.eml", folder, exported + 1);

                            zip.start_file(&filename, options)?;
                            zip.write_all(&content)?;
                            exported += 1;

                            self.update_export_progress(job_id, exported).await;
                        }
                    }
                }
            }

            zip.finish()?;
        } else {
            // Single EML file - just copy first message
            for folder in folders {
                let folder_path = user_maildir.join(&folder);

                for subdir in &["cur", "new"] {
                    let subdir_path = folder_path.join(subdir);
                    if !subdir_path.exists() {
                        continue;
                    }

                    for entry in fs::read_dir(&subdir_path)? {
                        let entry = entry?;
                        let path = entry.path();

                        if path.is_file() {
                            fs::copy(&path, output_path)?;
                            return Ok(1);
                        }
                    }
                }
            }
        }

        Ok(exported)
    }

    /// Start an import job
    pub async fn start_import(&self, request: ImportRequest, data: Vec<u8>) -> Result<ImportJob> {
        let job_id = Uuid::new_v4().to_string();

        // Count messages in import
        let total_messages = match request.format {
            ImportFormat::Mbox => {
                let cursor = std::io::Cursor::new(&data);
                count_messages(cursor)?
            }
            ImportFormat::Eml => 1,
            ImportFormat::EmlZip => {
                let cursor = std::io::Cursor::new(&data);
                let archive = ZipArchive::new(cursor)?;
                archive.len() as u64
            }
        };

        let target_folder = request.target_folder.clone().unwrap_or_else(|| "INBOX".to_string());

        let job = ImportJob {
            id: job_id.clone(),
            email: request.email.clone(),
            format: request.format,
            target_folder: target_folder.clone(),
            status: OperationStatus::Pending,
            progress: 0,
            total_messages,
            imported_messages: 0,
            skipped_messages: 0,
            error: None,
            created_at: Utc::now(),
            completed_at: None,
        };

        {
            let mut jobs = self.import_jobs.write().await;
            jobs.insert(job_id.clone(), job.clone());
        }

        // Start import in background
        let manager = self.clone_state();
        tokio::spawn(async move {
            let _ = manager.run_import(&job_id, request, data).await;
        });

        Ok(job)
    }

    /// Run the actual import
    async fn run_import(&self, job_id: &str, request: ImportRequest, data: Vec<u8>) -> Result<()> {
        self.update_import_status(job_id, OperationStatus::Running, None).await;

        let user_maildir = self.maildir_root.join(&request.email);
        let target_folder = request.target_folder.clone().unwrap_or_else(|| "INBOX".to_string());
        let target_path = if target_folder == "INBOX" {
            user_maildir.clone()
        } else {
            user_maildir.join(format!(".{}", target_folder))
        };

        // Ensure target directories exist
        fs::create_dir_all(target_path.join("new"))?;
        fs::create_dir_all(target_path.join("cur"))?;
        fs::create_dir_all(target_path.join("tmp"))?;

        let result = match request.format {
            ImportFormat::Mbox => {
                self.import_mbox(job_id, &target_path, &data, request.skip_duplicates).await
            }
            ImportFormat::Eml => {
                self.import_eml(job_id, &target_path, &data).await
            }
            ImportFormat::EmlZip => {
                self.import_eml_zip(job_id, &target_path, &data, request.skip_duplicates).await
            }
        };

        match result {
            Ok((imported, skipped)) => {
                {
                    let mut jobs = self.import_jobs.write().await;
                    if let Some(job) = jobs.get_mut(job_id) {
                        job.status = OperationStatus::Completed;
                        job.progress = 100;
                        job.imported_messages = imported;
                        job.skipped_messages = skipped;
                        job.completed_at = Some(Utc::now());
                    }
                }

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.total_imports += 1;
                    stats.messages_imported += imported;
                    stats.bytes_imported += data.len() as u64;
                }
            }
            Err(e) => {
                self.update_import_status(job_id, OperationStatus::Failed, Some(e.to_string())).await;
            }
        }

        Ok(())
    }

    /// Import from MBOX format
    async fn import_mbox(&self, job_id: &str, target_path: &Path, data: &[u8], skip_duplicates: bool) -> Result<(u64, u64)> {
        let cursor = std::io::Cursor::new(data);
        let mut reader = MboxReader::new(cursor);
        let mut imported = 0u64;
        let mut skipped = 0u64;

        while let Some(message) = reader.read_message()? {
            // Generate unique filename
            let filename = generate_maildir_filename();
            let new_path = target_path.join("new").join(&filename);

            // Check for duplicates if needed
            if skip_duplicates && self.is_duplicate(target_path, &message.content) {
                skipped += 1;
            } else {
                fs::write(&new_path, &message.content)?;
                imported += 1;
            }

            self.update_import_progress(job_id, imported, skipped).await;
        }

        Ok((imported, skipped))
    }

    /// Import single EML file
    async fn import_eml(&self, job_id: &str, target_path: &Path, data: &[u8]) -> Result<(u64, u64)> {
        let filename = generate_maildir_filename();
        let new_path = target_path.join("new").join(&filename);

        fs::write(&new_path, data)?;
        self.update_import_progress(job_id, 1, 0).await;

        Ok((1, 0))
    }

    /// Import from ZIP of EML files
    async fn import_eml_zip(&self, job_id: &str, target_path: &Path, data: &[u8], skip_duplicates: bool) -> Result<(u64, u64)> {
        let cursor = std::io::Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)?;
        let mut imported = 0u64;
        let mut skipped = 0u64;

        // First extract all files synchronously
        let mut files_to_import: Vec<Vec<u8>> = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            if file.is_file() && file.name().ends_with(".eml") {
                let mut content = Vec::new();
                file.read_to_end(&mut content)?;
                files_to_import.push(content);
            }
        }

        // Now process files with async progress updates
        for content in files_to_import {
            if skip_duplicates && self.is_duplicate(target_path, &content) {
                skipped += 1;
            } else {
                let filename = generate_maildir_filename();
                let new_path = target_path.join("new").join(&filename);
                fs::write(&new_path, &content)?;
                imported += 1;
            }

            self.update_import_progress(job_id, imported, skipped).await;
        }

        Ok((imported, skipped))
    }

    /// Check if a message is a duplicate
    fn is_duplicate(&self, target_path: &Path, content: &[u8]) -> bool {
        // Simple duplicate check based on Message-ID header
        if let Some(message_id) = extract_message_id(content) {
            for subdir in &["cur", "new"] {
                let subdir_path = target_path.join(subdir);
                if let Ok(entries) = fs::read_dir(subdir_path) {
                    for entry in entries.flatten() {
                        if let Ok(existing_content) = fs::read(entry.path()) {
                            if let Some(existing_id) = extract_message_id(&existing_content) {
                                if message_id == existing_id {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Get folders to export
    fn get_folders_to_export(&self, user_maildir: &Path, folders: &Option<Vec<String>>) -> Result<Vec<String>> {
        if let Some(folders) = folders {
            Ok(folders.clone())
        } else {
            // Get all folders
            let mut result = vec!["INBOX".to_string()];

            for entry in fs::read_dir(user_maildir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();

                if name.starts_with('.') && entry.path().is_dir() {
                    result.push(name[1..].to_string());
                }
            }

            Ok(result)
        }
    }

    /// Count messages in user's maildir
    fn count_user_messages(&self, user_maildir: &Path, folders: &Option<Vec<String>>) -> Result<u64> {
        let folders = self.get_folders_to_export(user_maildir, folders)?;
        let mut count = 0u64;

        for folder in folders {
            let folder_path = if folder == "INBOX" {
                user_maildir.to_path_buf()
            } else {
                user_maildir.join(format!(".{}", folder))
            };

            for subdir in &["cur", "new"] {
                let subdir_path = folder_path.join(subdir);
                if subdir_path.exists() {
                    count += fs::read_dir(subdir_path)?
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_file())
                        .count() as u64;
                }
            }
        }

        Ok(count)
    }

    /// Update export job status
    async fn update_export_status(&self, job_id: &str, status: OperationStatus, error: Option<String>) {
        let mut jobs = self.export_jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = status;
            job.error = error;
            if status == OperationStatus::Completed || status == OperationStatus::Failed {
                job.completed_at = Some(Utc::now());
            }
        }
    }

    /// Update export progress
    async fn update_export_progress(&self, job_id: &str, exported: u64) {
        let mut jobs = self.export_jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.exported_messages = exported;
            if job.total_messages > 0 {
                job.progress = ((exported * 100) / job.total_messages).min(99) as u8;
            }
        }
    }

    /// Update import job status
    async fn update_import_status(&self, job_id: &str, status: OperationStatus, error: Option<String>) {
        let mut jobs = self.import_jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = status;
            job.error = error;
            if status == OperationStatus::Completed || status == OperationStatus::Failed {
                job.completed_at = Some(Utc::now());
            }
        }
    }

    /// Update import progress
    async fn update_import_progress(&self, job_id: &str, imported: u64, skipped: u64) {
        let mut jobs = self.import_jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.imported_messages = imported;
            job.skipped_messages = skipped;
            let total = imported + skipped;
            if job.total_messages > 0 {
                job.progress = ((total * 100) / job.total_messages).min(99) as u8;
            }
        }
    }

    /// Get export job by ID
    pub async fn get_export_job(&self, job_id: &str) -> Option<ExportJob> {
        let jobs = self.export_jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// Get import job by ID
    pub async fn get_import_job(&self, job_id: &str) -> Option<ImportJob> {
        let jobs = self.import_jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// List export jobs for a user
    pub async fn list_export_jobs(&self, email: Option<&str>) -> Vec<ExportJob> {
        let jobs = self.export_jobs.read().await;
        jobs.values()
            .filter(|j| email.map_or(true, |e| j.email == e))
            .cloned()
            .collect()
    }

    /// List import jobs for a user
    pub async fn list_import_jobs(&self, email: Option<&str>) -> Vec<ImportJob> {
        let jobs = self.import_jobs.read().await;
        jobs.values()
            .filter(|j| email.map_or(true, |e| j.email == e))
            .cloned()
            .collect()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> ImportExportStats {
        let mut stats = self.stats.read().await.clone();

        // Count active jobs
        stats.active_exports = self.export_jobs.read().await
            .values()
            .filter(|j| j.status == OperationStatus::Running || j.status == OperationStatus::Pending)
            .count() as u32;

        stats.active_imports = self.import_jobs.read().await
            .values()
            .filter(|j| j.status == OperationStatus::Running || j.status == OperationStatus::Pending)
            .count() as u32;

        stats
    }

    /// Download export file
    pub async fn get_export_file(&self, job_id: &str) -> Result<Option<(String, Vec<u8>)>> {
        let jobs = self.export_jobs.read().await;
        if let Some(job) = jobs.get(job_id) {
            if let Some(ref path) = job.output_path {
                let content = fs::read(path)?;
                let filename = Path::new(path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                return Ok(Some((filename, content)));
            }
        }
        Ok(None)
    }

    /// Delete export file
    pub async fn delete_export(&self, job_id: &str) -> Result<()> {
        let mut jobs = self.export_jobs.write().await;
        if let Some(job) = jobs.remove(job_id) {
            if let Some(ref path) = job.output_path {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }

    /// Cancel a running job
    pub async fn cancel_job(&self, job_id: &str, is_export: bool) -> Result<()> {
        if is_export {
            let mut jobs = self.export_jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                if job.status == OperationStatus::Running || job.status == OperationStatus::Pending {
                    job.status = OperationStatus::Cancelled;
                    job.completed_at = Some(Utc::now());
                }
            }
        } else {
            let mut jobs = self.import_jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                if job.status == OperationStatus::Running || job.status == OperationStatus::Pending {
                    job.status = OperationStatus::Cancelled;
                    job.completed_at = Some(Utc::now());
                }
            }
        }
        Ok(())
    }
}

/// Generate a unique Maildir filename
fn generate_maildir_filename() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let pid = std::process::id();
    let unique = Uuid::new_v4().to_string();

    format!("{}.P{}M{}.localhost:2,", timestamp, pid, &unique[..8])
}

/// Extract From header from raw message
fn extract_from_header(content: &[u8]) -> Option<String> {
    let content_str = String::from_utf8_lossy(content);
    for line in content_str.lines() {
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("From:") {
            let value = value.trim();
            // Extract email from "Name <email>" format
            if let Some(start) = value.find('<') {
                if let Some(end) = value.find('>') {
                    return Some(value[start + 1..end].to_string());
                }
            }
            return Some(value.to_string());
        }
    }
    None
}

/// Extract Date header from raw message
fn extract_date_header(content: &[u8]) -> Option<chrono::DateTime<Utc>> {
    let content_str = String::from_utf8_lossy(content);
    for line in content_str.lines() {
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Date:") {
            let value = value.trim();
            // Try common date formats
            if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(value) {
                return Some(dt.with_timezone(&Utc));
            }
        }
    }
    None
}

/// Extract Message-ID header from raw message
fn extract_message_id(content: &[u8]) -> Option<String> {
    let content_str = String::from_utf8_lossy(content);
    for line in content_str.lines() {
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Message-ID:").or_else(|| line.strip_prefix("Message-Id:")) {
            return Some(value.trim().to_string());
        }
    }
    None
}

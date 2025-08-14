// P1.2.7: Auto-diff Support for Tools Platform 2.0
// Automatic detection and visualization of changes made by tools

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// File system snapshot for diff comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemSnapshot {
    /// Timestamp when snapshot was taken
    pub timestamp: SystemTime,
    /// Map of file paths to their metadata and content hashes
    pub files: HashMap<PathBuf, FileSnapshot>,
    /// Directories that were monitored
    pub monitored_paths: Vec<PathBuf>,
    /// Snapshot ID for reference
    pub id: String,
}

/// Individual file snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    /// File size in bytes
    pub size: u64,
    /// Last modified time
    pub modified: SystemTime,
    /// Content hash (SHA-256)
    pub content_hash: String,
    /// File permissions (Unix-style octal)
    pub permissions: Option<u32>,
    /// Whether the file is a directory
    pub is_directory: bool,
}

/// Diff result between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    /// Snapshot before changes
    pub before_snapshot_id: String,
    /// Snapshot after changes
    pub after_snapshot_id: String,
    /// Files that were added
    pub added_files: Vec<FileChange>,
    /// Files that were modified
    pub modified_files: Vec<FileChange>,
    /// Files that were deleted
    pub deleted_files: Vec<FileChange>,
    /// Summary statistics
    pub summary: DiffSummary,
    /// Time when diff was computed
    pub diff_time: SystemTime,
}

/// Individual file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// Path of the changed file
    pub path: PathBuf,
    /// Type of change
    pub change_type: FileChangeType,
    /// Before state (None for added files)
    pub before: Option<FileSnapshot>,
    /// After state (None for deleted files)
    pub after: Option<FileSnapshot>,
    /// Detailed change description
    pub description: String,
}

/// Type of file system change
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    PermissionsChanged,
    Renamed,
}

/// Summary of all changes in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Total number of files added
    pub files_added: u32,
    /// Total number of files modified
    pub files_modified: u32,
    /// Total number of files deleted
    pub files_deleted: u32,
    /// Total bytes added
    pub bytes_added: u64,
    /// Total bytes removed
    pub bytes_removed: u64,
    /// Total bytes modified
    pub bytes_modified: u64,
}

/// Auto-diff engine that tracks file system changes
pub struct AutoDiffEngine {
    /// Base directory for monitoring
    base_path: PathBuf,
    /// Whether to monitor subdirectories recursively
    recursive: bool,
    /// File patterns to include (if empty, include all)
    include_patterns: Vec<String>,
    /// File patterns to exclude
    exclude_patterns: Vec<String>,
    /// Maximum file size to track (bytes)
    max_file_size: u64,
    /// Stored snapshots
    snapshots: HashMap<String, FileSystemSnapshot>,
}

impl AutoDiffEngine {
    /// Create new auto-diff engine
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            recursive: true,
            include_patterns: Vec::new(),
            exclude_patterns: vec![
                "*.tmp".to_string(),
                "*.log".to_string(),
                ".git/*".to_string(),
                "target/*".to_string(),
                "node_modules/*".to_string(),
            ],
            max_file_size: 100 * 1024 * 1024, // 100MB default limit
            snapshots: HashMap::new(),
        }
    }

    /// Configure recursive monitoring
    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Add include pattern
    pub fn include_pattern(mut self, pattern: String) -> Self {
        self.include_patterns.push(pattern);
        self
    }

    /// Add exclude pattern
    pub fn exclude_pattern(mut self, pattern: String) -> Self {
        self.exclude_patterns.push(pattern);
        self
    }

    /// Set maximum file size to track
    pub fn with_max_file_size(mut self, max_size: u64) -> Self {
        self.max_file_size = max_size;
        self
    }

    /// Take a snapshot of the current file system state
    pub async fn take_snapshot(&mut self, snapshot_id: String) -> Result<()> {
        debug!("Taking file system snapshot: {}", snapshot_id);

        let start_time = std::time::Instant::now();
        let mut files = HashMap::new();

        self.scan_directory(&self.base_path.clone(), &mut files)
            .await?;

        let snapshot = FileSystemSnapshot {
            timestamp: SystemTime::now(),
            files,
            monitored_paths: vec![self.base_path.clone()],
            id: snapshot_id.clone(),
        };

        let files_count = snapshot.files.len();
        self.snapshots.insert(snapshot_id.clone(), snapshot);

        let scan_time = start_time.elapsed();
        info!(
            "Snapshot '{}' taken in {:?} ({} files)",
            snapshot_id, scan_time, files_count
        );

        Ok(())
    }

    /// Compute diff between two snapshots
    pub fn compute_diff(&self, before_id: &str, after_id: &str) -> Result<DiffResult> {
        let before = self
            .snapshots
            .get(before_id)
            .ok_or_else(|| anyhow::anyhow!("Snapshot '{}' not found", before_id))?;

        let after = self
            .snapshots
            .get(after_id)
            .ok_or_else(|| anyhow::anyhow!("Snapshot '{}' not found", after_id))?;

        debug!("Computing diff: {} -> {}", before_id, after_id);

        let mut added_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut deleted_files = Vec::new();

        // Find added and modified files
        for (path, after_snapshot) in &after.files {
            match before.files.get(path) {
                None => {
                    // File was added
                    added_files.push(FileChange {
                        path: path.clone(),
                        change_type: FileChangeType::Added,
                        before: None,
                        after: Some(after_snapshot.clone()),
                        description: format!("Added file: {}", path.display()),
                    });
                }
                Some(before_snapshot) => {
                    // Check if file was modified
                    if before_snapshot.content_hash != after_snapshot.content_hash {
                        modified_files.push(FileChange {
                            path: path.clone(),
                            change_type: FileChangeType::Modified,
                            before: Some(before_snapshot.clone()),
                            after: Some(after_snapshot.clone()),
                            description: self
                                .describe_modification(before_snapshot, after_snapshot),
                        });
                    } else if before_snapshot.permissions != after_snapshot.permissions {
                        modified_files.push(FileChange {
                            path: path.clone(),
                            change_type: FileChangeType::PermissionsChanged,
                            before: Some(before_snapshot.clone()),
                            after: Some(after_snapshot.clone()),
                            description: format!("Permissions changed: {}", path.display()),
                        });
                    }
                }
            }
        }

        // Find deleted files
        for (path, before_snapshot) in &before.files {
            if !after.files.contains_key(path) {
                deleted_files.push(FileChange {
                    path: path.clone(),
                    change_type: FileChangeType::Deleted,
                    before: Some(before_snapshot.clone()),
                    after: None,
                    description: format!("Deleted file: {}", path.display()),
                });
            }
        }

        // Compute summary statistics
        let summary = self.compute_summary(&added_files, &modified_files, &deleted_files);

        let diff_result = DiffResult {
            before_snapshot_id: before_id.to_string(),
            after_snapshot_id: after_id.to_string(),
            added_files,
            modified_files,
            deleted_files,
            summary,
            diff_time: SystemTime::now(),
        };

        info!(
            "Diff computed: +{} ~{} -{} files",
            diff_result.summary.files_added,
            diff_result.summary.files_modified,
            diff_result.summary.files_deleted
        );

        Ok(diff_result)
    }

    /// Get formatted diff output
    pub fn format_diff(&self, diff: &DiffResult) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "Diff: {} -> {}\n",
            diff.before_snapshot_id, diff.after_snapshot_id
        ));
        output.push_str("=".repeat(50).as_str());
        output.push_str("\n\n");

        // Summary
        output.push_str("SUMMARY:\n");
        output.push_str(&format!(
            "  Added:    {} files (+{} bytes)\n",
            diff.summary.files_added, diff.summary.bytes_added
        ));
        output.push_str(&format!(
            "  Modified: {} files (~{} bytes)\n",
            diff.summary.files_modified, diff.summary.bytes_modified
        ));
        output.push_str(&format!(
            "  Deleted:  {} files (-{} bytes)\n\n",
            diff.summary.files_deleted, diff.summary.bytes_removed
        ));

        // Added files
        if !diff.added_files.is_empty() {
            output.push_str("ADDED FILES:\n");
            for change in &diff.added_files {
                output.push_str(&format!("  + {}\n", change.path.display()));
            }
            output.push('\n');
        }

        // Modified files
        if !diff.modified_files.is_empty() {
            output.push_str("MODIFIED FILES:\n");
            for change in &diff.modified_files {
                output.push_str(&format!(
                    "  ~ {} ({})\n",
                    change.path.display(),
                    change.description
                ));
            }
            output.push('\n');
        }

        // Deleted files
        if !diff.deleted_files.is_empty() {
            output.push_str("DELETED FILES:\n");
            for change in &diff.deleted_files {
                output.push_str(&format!("  - {}\n", change.path.display()));
            }
            output.push('\n');
        }

        output
    }

    /// Scan directory recursively
    async fn scan_directory(
        &self,
        dir: &Path,
        files: &mut HashMap<PathBuf, FileSnapshot>,
    ) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Skip if excluded
            if self.is_excluded(&path) {
                continue;
            }

            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                // Record directory
                files.insert(
                    path.clone(),
                    FileSnapshot {
                        size: 0,
                        modified: metadata.modified()?,
                        content_hash: "directory".to_string(),
                        permissions: self.get_permissions(&metadata),
                        is_directory: true,
                    },
                );

                // Recurse into subdirectory if enabled
                if self.recursive {
                    Box::pin(self.scan_directory(&path, files)).await?;
                }
            } else {
                // Skip large files
                if metadata.len() > self.max_file_size {
                    warn!(
                        "Skipping large file: {} ({} bytes)",
                        path.display(),
                        metadata.len()
                    );
                    continue;
                }

                // Record file
                let content_hash = self.compute_file_hash(&path).await.unwrap_or_else(|e| {
                    warn!("Failed to compute hash for {}: {}", path.display(), e);
                    "error".to_string()
                });

                files.insert(
                    path.clone(),
                    FileSnapshot {
                        size: metadata.len(),
                        modified: metadata.modified()?,
                        content_hash,
                        permissions: self.get_permissions(&metadata),
                        is_directory: false,
                    },
                );
            }
        }

        Ok(())
    }

    /// Check if path should be excluded
    fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check include patterns first (if any)
        if !self.include_patterns.is_empty() {
            let included = self
                .include_patterns
                .iter()
                .any(|pattern| self.matches_pattern(&path_str, pattern));
            if !included {
                return true;
            }
        }

        // Check exclude patterns
        self.exclude_patterns
            .iter()
            .any(|pattern| self.matches_pattern(&path_str, pattern))
    }

    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.is_empty() {
                return true;
            }

            let mut start = 0;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }

                if let Some(pos) = text[start..].find(part) {
                    start += pos + part.len();
                } else {
                    return false;
                }
            }
            true
        } else {
            text.contains(pattern)
        }
    }

    /// Compute SHA-256 hash of file content
    async fn compute_file_hash(&self, path: &Path) -> Result<String> {
        let content = tokio::fs::read(path).await?;
        let digest = sha2::Sha256::digest(&content);
        Ok(format!("{:x}", digest))
    }

    /// Get file permissions (Unix-style)
    #[cfg(unix)]
    fn get_permissions(&self, metadata: &std::fs::Metadata) -> Option<u32> {
        use std::os::unix::fs::PermissionsExt;
        Some(metadata.permissions().mode())
    }

    #[cfg(not(unix))]
    fn get_permissions(&self, _metadata: &std::fs::Metadata) -> Option<u32> {
        None // Windows doesn't have Unix-style permissions
    }

    /// Describe file modification
    fn describe_modification(&self, before: &FileSnapshot, after: &FileSnapshot) -> String {
        let size_change = after.size as i64 - before.size as i64;

        if size_change > 0 {
            format!("Content changed (+{} bytes)", size_change)
        } else if size_change < 0 {
            format!("Content changed ({} bytes)", size_change)
        } else {
            "Content changed".to_string()
        }
    }

    /// Compute diff summary statistics
    fn compute_summary(
        &self,
        added: &[FileChange],
        modified: &[FileChange],
        deleted: &[FileChange],
    ) -> DiffSummary {
        let files_added = added.len() as u32;
        let files_modified = modified.len() as u32;
        let files_deleted = deleted.len() as u32;

        let bytes_added = added
            .iter()
            .filter_map(|change| change.after.as_ref())
            .map(|snapshot| snapshot.size)
            .sum();

        let bytes_removed = deleted
            .iter()
            .filter_map(|change| change.before.as_ref())
            .map(|snapshot| snapshot.size)
            .sum();

        let bytes_modified = modified
            .iter()
            .filter_map(|change| match (&change.before, &change.after) {
                (Some(before), Some(after)) => Some(if after.size > before.size {
                    after.size - before.size
                } else {
                    before.size - after.size
                }),
                _ => None,
            })
            .sum();

        DiffSummary {
            files_added,
            files_modified,
            files_deleted,
            bytes_added,
            bytes_removed,
            bytes_modified,
        }
    }

    /// List available snapshots
    pub fn list_snapshots(&self) -> Vec<&String> {
        self.snapshots.keys().collect()
    }

    /// Remove snapshot
    pub fn remove_snapshot(&mut self, snapshot_id: &str) -> Option<FileSystemSnapshot> {
        self.snapshots.remove(snapshot_id)
    }

    /// Clear all snapshots
    pub fn clear_snapshots(&mut self) {
        self.snapshots.clear();
    }
}

// Include sha2 dependency in Cargo.toml if not already present
use sha2::Digest;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_snapshot_creation() {
        let temp_dir = tempdir().unwrap();
        let mut engine = AutoDiffEngine::new(temp_dir.path());

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file).await.unwrap();
        file.write_all(b"test content").await.unwrap();

        // Take snapshot
        engine.take_snapshot("before".to_string()).await.unwrap();

        let snapshots = engine.list_snapshots();
        assert_eq!(snapshots.len(), 1);
        assert!(snapshots.contains(&&"before".to_string()));
    }

    #[tokio::test]
    async fn test_diff_computation() {
        let temp_dir = tempdir().unwrap();
        let mut engine = AutoDiffEngine::new(temp_dir.path());

        // Take initial snapshot
        engine.take_snapshot("before".to_string()).await.unwrap();

        // Create a new file
        let new_file = temp_dir.path().join("new.txt");
        let mut file = File::create(&new_file).await.unwrap();
        file.write_all(b"new content").await.unwrap();

        // Take second snapshot
        engine.take_snapshot("after".to_string()).await.unwrap();

        // Compute diff
        let diff = engine.compute_diff("before", "after").unwrap();

        assert_eq!(diff.summary.files_added, 1);
        assert_eq!(diff.summary.files_modified, 0);
        assert_eq!(diff.summary.files_deleted, 0);
        assert_eq!(diff.added_files.len(), 1);
    }

    #[test]
    fn test_pattern_matching() {
        let engine = AutoDiffEngine::new(".");

        assert!(engine.matches_pattern("test.txt", "*.txt"));
        assert!(engine.matches_pattern("path/to/file.log", "*.log"));
        assert!(!engine.matches_pattern("test.txt", "*.log"));
        assert!(engine.matches_pattern("target/debug/test", "target/*"));
    }

    #[test]
    fn test_diff_formatting() {
        let engine = AutoDiffEngine::new(".");

        let diff = DiffResult {
            before_snapshot_id: "before".to_string(),
            after_snapshot_id: "after".to_string(),
            added_files: vec![FileChange {
                path: PathBuf::from("new.txt"),
                change_type: FileChangeType::Added,
                before: None,
                after: Some(FileSnapshot {
                    size: 100,
                    modified: SystemTime::now(),
                    content_hash: "hash".to_string(),
                    permissions: None,
                    is_directory: false,
                }),
                description: "Added file: new.txt".to_string(),
            }],
            modified_files: vec![],
            deleted_files: vec![],
            summary: DiffSummary {
                files_added: 1,
                files_modified: 0,
                files_deleted: 0,
                bytes_added: 100,
                bytes_removed: 0,
                bytes_modified: 0,
            },
            diff_time: SystemTime::now(),
        };

        let formatted = engine.format_diff(&diff);
        assert!(formatted.contains("ADDED FILES:"));
        assert!(formatted.contains("new.txt"));
    }
}

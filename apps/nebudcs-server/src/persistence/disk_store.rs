//! Atomic disk persistence engine ensuring projects, scenarios, and training records
//! survive server restarts and power outages without partial write corruption.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::security::sanitize_identifier;

/// Disk-backed atomic storage engine.
#[derive(Debug, Clone)]
pub struct DiskStore {
    base_dir: PathBuf,
}

impl DiskStore {
    /// Creates a new DiskStore at the specified base directory, creating folders if needed.
    pub fn new(base_dir: impl AsRef<Path>) -> io::Result<Self> {
        let base = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(base.join("projects"))?;
        fs::create_dir_all(base.join("scenarios"))?;
        fs::create_dir_all(base.join("training"))?;
        fs::create_dir_all(base.join("backups"))?;
        Ok(Self { base_dir: base })
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.base_dir.join("projects")
    }

    pub fn scenarios_dir(&self) -> PathBuf {
        self.base_dir.join("scenarios")
    }

    pub fn training_dir(&self) -> PathBuf {
        self.base_dir.join("training")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.base_dir.join("backups")
    }

    /// Atomically writes JSON content to `<category>/<sanitized_id>.json`.
    /// First writes to `.tmp` file, flushes/syncs, then atomically renames.
    pub fn save_json<T: serde::Serialize>(
        &self,
        category: &str,
        id: &str,
        data: &T,
    ) -> Result<PathBuf, String> {
        let safe_id = sanitize_identifier(id).map_err(|e| format!("{e}"))?;
        let dir = self.base_dir.join(category);
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create directory {}: {e}", dir.display()))?;

        let final_path = dir.join(format!("{safe_id}.json"));
        let tmp_path = dir.join(format!("{safe_id}.json.tmp"));

        let json_bytes = serde_json::to_vec_pretty(data)
            .map_err(|e| format!("Failed to serialize data for '{id}': {e}"))?;

        // Write to temporary file
        let mut file = File::create(&tmp_path).map_err(|e| {
            format!(
                "Failed to create temporary file {}: {e}",
                tmp_path.display()
            )
        })?;
        file.write_all(&json_bytes)
            .map_err(|e| format!("Failed to write bytes to {}: {e}", tmp_path.display()))?;
        file.sync_all()
            .map_err(|e| format!("Failed to sync file {}: {e}", tmp_path.display()))?;
        drop(file);

        // Atomic rename
        fs::rename(&tmp_path, &final_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            format!(
                "Atomic rename failed from {} to {}: {e}",
                tmp_path.display(),
                final_path.display()
            )
        })?;

        Ok(final_path)
    }

    /// Reads and deserializes JSON content from `<category>/<sanitized_id>.json`.
    pub fn load_json<T: serde::de::DeserializeOwned>(
        &self,
        category: &str,
        id: &str,
    ) -> Result<Option<T>, String> {
        let safe_id = sanitize_identifier(id).map_err(|e| format!("{e}"))?;
        let path = self.base_dir.join(category).join(format!("{safe_id}.json"));
        if !path.exists() {
            return Ok(None);
        }

        let bytes =
            fs::read(&path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let data: T = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Failed to deserialize {}: {e}", path.display()))?;

        Ok(Some(data))
    }

    /// Deletes `<category>/<sanitized_id>.json`.
    pub fn delete_json(&self, category: &str, id: &str) -> Result<bool, String> {
        let safe_id = sanitize_identifier(id).map_err(|e| format!("{e}"))?;
        let path = self.base_dir.join(category).join(format!("{safe_id}.json"));
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove {}: {e}", path.display()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Lists and deserializes all JSON files in a category directory.
    pub fn load_all_in_category<T: serde::de::DeserializeOwned>(
        &self,
        category: &str,
    ) -> Vec<(String, T)> {
        let dir = self.base_dir.join(category);
        if !dir.exists() {
            return Vec::new();
        }

        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(bytes) = fs::read(&path) {
                        if let Ok(item) = serde_json::from_slice::<T>(&bytes) {
                            results.push((stem.to_string(), item));
                        }
                    }
                }
            }
        }

        results
    }
}

use anyhow::Context;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn snapshot_dir() -> anyhow::Result<PathBuf> {
    let dir = dirs::data_dir()
        .or_else(dirs::config_dir)
        .context("Cannot determine app data directory")?
        .join("courseape")
        .join("snapshots");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub struct SnapshotArchive;

impl SnapshotArchive {
    /// Save a raw snapshot to disk. Returns (hash, file_path).
    pub fn save(source: &str, data: &[u8]) -> anyhow::Result<(String, PathBuf)> {
        let ext = if source.contains("pdf") {
            "pdf"
        } else if source.contains("html") || source.contains("grade") {
            "html"
        } else {
            "json"
        };
        Self::save_as(source.trim_end_matches(&format!(".{ext}")), ext, data)
    }

    pub fn save_as(
        source: &str,
        extension: &str,
        data: &[u8],
    ) -> anyhow::Result<(String, PathBuf)> {
        let dir = snapshot_dir()?;
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(data);
            hex::encode(hasher.finalize())
        };
        let filename = format!("{}_{}.{}", source, &hash[..16], extension);
        let path = dir.join(&filename);
        std::fs::write(&path, data)?;
        Ok((hash, path))
    }

    /// Purge all snapshots.
    pub fn purge() -> anyhow::Result<()> {
        let dir = snapshot_dir()?;
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn is_fresh(prefix: &str, max_age_hours: u64) -> anyhow::Result<bool> {
        let dir = snapshot_dir()?;
        let cutoff = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(max_age_hours * 3600))
            .unwrap_or(std::time::UNIX_EPOCH);
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with(prefix)
                && entry
                    .metadata()?
                    .modified()
                    .is_ok_and(|modified| modified >= cutoff)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn newest_valid_grade(max_age_hours: Option<u64>) -> anyhow::Result<Option<PathBuf>> {
        let cutoff = max_age_hours.map(|hours| {
            std::time::SystemTime::now()
                .checked_sub(std::time::Duration::from_secs(hours * 3600))
                .unwrap_or(std::time::UNIX_EPOCH)
        });
        let mut candidates = Vec::new();
        for entry in std::fs::read_dir(snapshot_dir()?)? {
            let entry = entry?;
            if !entry.file_name().to_string_lossy().starts_with("grades") {
                continue;
            }
            let modified = entry.metadata()?.modified()?;
            if cutoff.is_some_and(|cutoff| modified < cutoff) {
                continue;
            }
            let body = std::fs::read(entry.path())?;
            if crate::connectors::itouch::is_authenticated_grade_body(&body) {
                candidates.push((modified, entry.path()));
            }
        }
        candidates.sort_by_key(|(modified, _)| *modified);
        Ok(candidates.pop().map(|(_, path)| path))
    }

    /// List all snapshot filenames.
    #[allow(dead_code)]
    pub fn list() -> anyhow::Result<Vec<String>> {
        let dir = snapshot_dir()?;
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        Ok(names)
    }
}

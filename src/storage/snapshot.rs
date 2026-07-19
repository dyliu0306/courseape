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
        let dir = snapshot_dir()?;
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(data);
            hex::encode(hasher.finalize())
        };
        let ext = if source.contains("pdf") {
            "pdf"
        } else if source.contains("html") || source.contains("grade") {
            "html"
        } else {
            "json"
        };
        let filename = format!("{}_{}.{}", source, &hash[..16], ext);
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

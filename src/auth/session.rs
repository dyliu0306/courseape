use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub cookie: String,
    pub login_token: Option<String>,
    pub logged_in_at: chrono::DateTime<chrono::Utc>,
}

fn session_path() -> anyhow::Result<PathBuf> {
    let dir = dirs::data_dir()
        .or_else(dirs::config_dir)
        .context("Cannot determine app data directory")?
        .join("courseape");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("session.json"))
}

impl Session {
    pub fn save(&self) -> anyhow::Result<()> {
        let path = session_path()?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn load() -> anyhow::Result<Option<Self>> {
        let path = session_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path)?;
        let session: Self = serde_json::from_str(&data)?;
        Ok(Some(session))
    }

    pub fn delete() -> anyhow::Result<()> {
        let path = session_path()?;
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn has_login_token(&self) -> bool {
        self.login_token.is_some()
    }
}

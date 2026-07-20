use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SERVICE: &str = "courseape";
const ACCOUNT: &str = "cycu-itouch-session";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub cookie: String,
    pub login_token: Option<String>,
    pub logged_in_at: chrono::DateTime<chrono::Utc>,
}

fn legacy_session_path() -> anyhow::Result<PathBuf> {
    let dir = dirs::data_dir()
        .or_else(dirs::config_dir)
        .context("Cannot determine app data directory")?
        .join("courseape");
    Ok(dir.join("session.json"))
}

fn entry() -> anyhow::Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).context("Failed to open OS session store")
}

impl Session {
    pub fn save(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string(self)?;
        entry()?
            .set_password(&json)
            .context("Failed to save session to OS credential store")?;
        let legacy = legacy_session_path()?;
        if legacy.exists() {
            std::fs::remove_file(legacy)?;
        }
        Ok(())
    }

    pub fn load() -> anyhow::Result<Option<Self>> {
        match entry()?.get_password() {
            Ok(json) => Ok(Some(
                serde_json::from_str(&json).context("Invalid session payload")?,
            )),
            Err(keyring::Error::NoEntry) => Self::migrate_legacy(),
            Err(error) => Err(error).context("Failed to read session from OS credential store"),
        }
    }

    fn migrate_legacy() -> anyhow::Result<Option<Self>> {
        let path = legacy_session_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let session: Self = serde_json::from_str(&std::fs::read_to_string(&path)?)
            .context("Invalid legacy session file")?;
        session.save()?;
        Ok(Some(session))
    }

    pub fn delete() -> anyhow::Result<()> {
        match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(error) => return Err(error).context("Failed to delete OS session"),
        }
        let legacy = legacy_session_path()?;
        if legacy.exists() {
            std::fs::remove_file(legacy)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_payload_round_trips() {
        let session = Session {
            cookie: "JSESSIONID=secret".to_string(),
            login_token: None,
            logged_in_at: chrono::Utc::now(),
        };
        let decoded: Session =
            serde_json::from_str(&serde_json::to_string(&session).unwrap()).unwrap();
        assert_eq!(decoded.cookie, session.cookie);
    }
}

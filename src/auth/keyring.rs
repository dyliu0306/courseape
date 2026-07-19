use anyhow::Context;
use zeroize::Zeroize;

const SERVICE: &str = "courseape";
const ACCOUNT: &str = "cycu-itouch";

pub struct StoredCredentials {
    pub student_id: String,
    pub password: String,
}

impl StoredCredentials {
    pub fn new(mut student_id: String, mut password: String) -> anyhow::Result<Self> {
        if student_id.trim().is_empty() || password.is_empty() {
            student_id.zeroize();
            password.zeroize();
            anyhow::bail!("Student ID and password must not be empty.");
        }
        let normalized = student_id.trim().to_owned();
        student_id.zeroize();
        Ok(Self {
            student_id: normalized,
            password,
        })
    }

    fn entry() -> anyhow::Result<keyring::Entry> {
        keyring::Entry::new(SERVICE, ACCOUNT).context("Failed to open OS credential store")
    }

    fn encode(&self) -> anyhow::Result<String> {
        serde_json::to_string(&(&self.student_id, &self.password))
            .context("Failed to encode credentials")
    }

    fn decode(payload: &str) -> anyhow::Result<Self> {
        let (student_id, password): (String, String) =
            serde_json::from_str(payload).context("Invalid credential store payload")?;
        Self::new(student_id, password)
    }

    pub fn load() -> anyhow::Result<Option<Self>> {
        match Self::entry()?.get_password() {
            Ok(mut payload) => {
                let result = Self::decode(&payload).map(Some);
                payload.zeroize();
                result
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error).context("Failed to read OS credential store"),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let mut payload = self.encode()?;
        let result = match Self::entry() {
            Ok(entry) => entry
                .set_password(&payload)
                .context("Failed to save credentials to OS credential store"),
            Err(error) => Err(error),
        };
        payload.zeroize();
        result
    }

    pub fn delete() -> anyhow::Result<()> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error).context("Failed to delete OS credential"),
        }
    }
}

impl Drop for StoredCredentials {
    fn drop(&mut self) {
        self.student_id.zeroize();
        self.password.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_payload_round_trips() {
        let creds = StoredCredentials::new("11244151".into(), "p@ss\"\\\nword".into()).unwrap();
        let decoded = StoredCredentials::decode(&creds.encode().unwrap()).unwrap();
        assert_eq!(decoded.student_id, creds.student_id);
        assert_eq!(decoded.password, creds.password);
    }
}

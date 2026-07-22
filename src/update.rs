use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use fs2::FileExt;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::path::PathBuf;

const LATEST_URL: &str = "https://registry.npmjs.org/@dyliu0306%2Fcourseape/latest";
const CACHE_HOURS: i64 = 4;

#[derive(Debug, Deserialize, Serialize)]
struct UpdateCache {
    checked_at: DateTime<Utc>,
    latest_version: String,
}

#[derive(Deserialize)]
struct NpmLatest {
    version: String,
}

pub async fn check_and_notify() {
    if let Ok(Some(message)) = check_for_update().await {
        eprintln!("{message}");
    }
}

pub fn clear_cache() -> anyhow::Result<()> {
    let cache = cache_path()?;
    let Some(dir) = cache.parent() else {
        return Ok(());
    };
    clear_cache_in(dir)
}

fn clear_cache_in(dir: &std::path::Path) -> anyhow::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_update_file = name == "update-check.json"
            || name == "update-check.lock"
            || name.starts_with("update-check.json.") && name.ends_with(".tmp");
        if is_update_file {
            match std::fs::remove_file(entry.path()) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(())
}

async fn check_for_update() -> anyhow::Result<Option<String>> {
    let now = Utc::now();
    let latest = match read_fresh_cache(now) {
        Some(version) => version,
        None => {
            let Some(_lock) = UpdateLock::acquire()? else {
                return Ok(None);
            };
            // Another process may have refreshed the cache before this process got the lock.
            if let Some(version) = read_fresh_cache(Utc::now()) {
                return Ok(update_message(env!("CARGO_PKG_VERSION"), &version));
            }
            let version = fetch_latest().await?;
            let _ = write_cache(&UpdateCache {
                checked_at: now,
                latest_version: version.clone(),
            });
            version
        }
    };

    Ok(update_message(env!("CARGO_PKG_VERSION"), &latest))
}

async fn fetch_latest() -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let response = client.get(LATEST_URL).send().await?.error_for_status()?;
    let body = response.bytes().await?;
    parse_latest_response(&body)
}

fn parse_latest_response(body: &[u8]) -> anyhow::Result<String> {
    let latest: NpmLatest = serde_json::from_slice(body)?;
    Version::parse(&latest.version).context("npm latest returned an invalid version")?;
    Ok(latest.version)
}

fn update_message(current: &str, latest: &str) -> Option<String> {
    let current = Version::parse(current).ok()?;
    let latest_version = Version::parse(latest).ok()?;
    (latest_version > current).then(|| {
        format!(
            "CourseApe 有新版本：目前 {current}，最新 {latest_version}。\n\
             更新：npm install -g @dyliu0306/courseape@latest"
        )
    })
}

fn read_fresh_cache(now: DateTime<Utc>) -> Option<String> {
    let path = cache_path().ok()?;
    if !path.exists() {
        return None;
    }
    let raw = std::fs::read(&path).ok()?;
    parse_fresh_cache(&raw, now)
}

fn parse_fresh_cache(raw: &[u8], now: DateTime<Utc>) -> Option<String> {
    let cache: UpdateCache = serde_json::from_slice(raw).ok()?;
    if !cache_is_fresh(&cache, now) || Version::parse(&cache.latest_version).is_err() {
        return None;
    }
    Some(cache.latest_version)
}

fn cache_is_fresh(cache: &UpdateCache, now: DateTime<Utc>) -> bool {
    let age = now.signed_duration_since(cache.checked_at);
    age >= Duration::zero() && age < Duration::hours(CACHE_HOURS)
}

fn write_cache(cache: &UpdateCache) -> anyhow::Result<()> {
    let path = cache_path()?;
    let temp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    std::fs::write(&temp, serde_json::to_vec(cache)?)?;
    replace_file(&temp, &path)?;
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(source: &std::path::Path, destination: &std::path::Path) -> anyhow::Result<()> {
    std::fs::rename(source, destination)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(source: &std::path::Path, destination: &std::path::Path) -> anyhow::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, ReplaceFileW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        REPLACEFILE_WRITE_THROUGH,
    };

    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let succeeded = unsafe {
        if destination.exists() {
            ReplaceFileW(
                destination_wide.as_ptr(),
                source_wide.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } else {
            MoveFileExW(
                source_wide.as_ptr(),
                destination_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
    };
    if succeeded == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

fn cache_path() -> anyhow::Result<PathBuf> {
    let dir = dirs::data_dir()
        .or_else(dirs::config_dir)
        .context("Cannot determine app data directory")?
        .join("courseape");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("update-check.json"))
}

struct UpdateLock {
    file: std::fs::File,
}

impl UpdateLock {
    fn acquire() -> anyhow::Result<Option<Self>> {
        let path = cache_path()?.with_file_name("update-check.lock");
        Self::acquire_at(path)
    }

    fn acquire_at(path: PathBuf) -> anyhow::Result<Option<Self>> {
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if is_lock_contention(&error) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { file })),
            Err(error) if is_lock_contention(&error) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

fn is_lock_contention(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock
        || cfg!(windows) && matches!(error.raw_os_error(), Some(32 | 33))
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_version_comparison_is_not_lexical() {
        let message = update_message("1.0.2", "1.0.10").unwrap();
        assert!(message.contains("目前 1.0.2"));
        assert!(message.contains("最新 1.0.10"));
        assert!(message.contains("npm install -g @dyliu0306/courseape@latest"));
    }

    #[test]
    fn current_or_older_remote_version_has_no_notice() {
        assert!(update_message("1.0.2", "1.0.2").is_none());
        assert!(update_message("1.0.2", "1.0.1").is_none());
    }

    #[test]
    fn cache_expires_at_four_hours() {
        let now = Utc::now();
        let fresh = UpdateCache {
            checked_at: now - Duration::hours(4) + Duration::seconds(1),
            latest_version: "1.0.2".into(),
        };
        let stale = UpdateCache {
            checked_at: now - Duration::hours(4),
            latest_version: "1.0.2".into(),
        };
        assert!(cache_is_fresh(&fresh, now));
        assert!(!cache_is_fresh(&stale, now));
    }

    #[test]
    fn malformed_registry_response_fails_open() {
        assert!(parse_latest_response(br#"{"version":"not-semver"}"#).is_err());
        assert!(parse_latest_response(b"not-json").is_err());
    }

    #[test]
    fn malformed_cache_is_treated_as_missing() {
        assert!(parse_fresh_cache(b"not-json", Utc::now()).is_none());
        assert!(parse_fresh_cache(
            br#"{"checked_at":"2026-07-22T00:00:00Z","latest_version":"bad"}"#,
            Utc::now()
        )
        .is_none());
    }

    #[test]
    fn lock_allows_only_one_holder() {
        let path = std::env::temp_dir().join(format!(
            "courseape-update-test-{}-{}.lock",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let first = UpdateLock::acquire_at(path.clone()).unwrap().unwrap();
        assert!(UpdateLock::acquire_at(path.clone()).unwrap().is_none());
        drop(first);
        assert!(UpdateLock::acquire_at(path).unwrap().is_some());
    }

    #[test]
    fn clear_cache_removes_only_update_artifacts() {
        let dir = std::env::temp_dir().join(format!(
            "courseape-update-clear-test-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        for name in [
            "update-check.json",
            "update-check.lock",
            "update-check.json.123.tmp",
            "courseape.db",
        ] {
            std::fs::write(dir.join(name), b"test").unwrap();
        }

        clear_cache_in(&dir).unwrap();

        assert!(!dir.join("update-check.json").exists());
        assert!(!dir.join("update-check.lock").exists());
        assert!(!dir.join("update-check.json.123.tmp").exists());
        assert!(dir.join("courseape.db").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

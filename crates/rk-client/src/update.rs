use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/shawn-nabizada/rustkanban/releases/latest";
const GITHUB_DOWNLOAD_BASE: &str =
    "https://github.com/shawn-nabizada/rustkanban/releases/latest/download";
const PREF_LAST_UPDATE_CHECK: &str = "last_update_check";
const PREF_LATEST_VERSION: &str = "latest_version";
const CHECK_COOLDOWN_SECS: u64 = 86400;

#[derive(Debug)]
pub enum UpdateError {
    AlreadyUpToDate(String),
    UnsupportedPlatform(String),
    Network(String),
    ChecksumMismatch,
    Io(String),
    PermissionDenied,
    CargoInstall,
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateError::AlreadyUpToDate(v) => write!(f, "Already up to date (v{})", v),
            UpdateError::UnsupportedPlatform(p) => write!(f, "Unsupported platform: {}", p),
            UpdateError::Network(e) => write!(f, "Network error: {}", e),
            UpdateError::ChecksumMismatch => write!(f, "Checksum verification failed"),
            UpdateError::Io(e) => write!(f, "IO error: {}", e),
            UpdateError::PermissionDenied => {
                if cfg!(unix) {
                    write!(f, "Permission denied — try `sudo rk update`")
                } else {
                    write!(f, "Permission denied — try running as administrator")
                }
            }
            UpdateError::CargoInstall => {
                writeln!(f, "It looks like you installed via `cargo install`.")?;
                writeln!(f, "Consider running: cargo install rustkanban")?;
                write!(f, "To update anyway, run: rk update --force")
            }
        }
    }
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Check GitHub for the latest version, respecting a 24h cooldown.
/// Returns Some(version) if a newer version is available.
pub fn check_for_update(conn: &Connection) -> Option<String> {
    let now = now_unix();

    if let Some(last_check_str) = crate::db::get_preference(conn, PREF_LAST_UPDATE_CHECK) {
        if let Ok(last_check) = last_check_str.parse::<u64>() {
            if now.saturating_sub(last_check) < CHECK_COOLDOWN_SECS {
                return get_cached_update(conn);
            }
        }
    }

    match fetch_latest_version() {
        Some(version) => {
            let _ = crate::db::set_preference(conn, PREF_LATEST_VERSION, &version);
            let _ = crate::db::set_preference(conn, PREF_LAST_UPDATE_CHECK, &now.to_string());
            if is_newer(&version, current_version()) {
                Some(version)
            } else {
                None
            }
        }
        None => {
            // Don't update cooldown on failure — retry next launch
            get_cached_update(conn)
        }
    }
}

fn get_cached_update(conn: &Connection) -> Option<String> {
    let version = crate::db::get_preference(conn, PREF_LATEST_VERSION)?;
    if is_newer(&version, current_version()) {
        Some(version)
    } else {
        None
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn make_agent(timeout: Duration) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build();
    ureq::Agent::new_with_config(config)
}

fn fetch_latest_version() -> Option<String> {
    let agent = make_agent(Duration::from_secs(5));

    let mut resp = agent
        .get(GITHUB_API_URL)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", &format!("rk/{}", current_version()))
        .call()
        .ok()?;

    let text = resp.body_mut().read_to_string().ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let tag = json["tag_name"].as_str()?;
    Some(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

fn is_newer(remote: &str, current: &str) -> bool {
    let parse = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    };
    match (parse(remote), parse(current)) {
        (Some(r), Some(c)) => r > c,
        _ => false,
    }
}

fn platform_asset_name() -> Result<&'static str, UpdateError> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("rk-linux-x86_64"),
        ("linux", "aarch64") => Ok("rk-linux-aarch64"),
        ("macos", "x86_64") => Ok("rk-macos-x86_64"),
        ("macos", "aarch64") => Ok("rk-macos-aarch64"),
        ("windows", "x86_64") => Ok("rk-windows-x86_64.exe"),
        (os, arch) => Err(UpdateError::UnsupportedPlatform(format!("{}-{}", os, arch))),
    }
}

fn download_bytes(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, UpdateError> {
    let mut resp = agent
        .get(url)
        .header("User-Agent", &format!("rk/{}", current_version()))
        .call()
        .map_err(|e| UpdateError::Network(e.to_string()))?;

    resp.body_mut()
        .with_config()
        .limit(50_000_000) // 50MB limit for binary downloads
        .read_to_vec()
        .map_err(|e| UpdateError::Io(e.to_string()))
}

fn verify_checksum(
    agent: &ureq::Agent,
    binary: &[u8],
    asset_name: &str,
    checksums_url: &str,
) -> Result<(), UpdateError> {
    let checksums_bytes = match download_bytes(agent, checksums_url) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Warning: could not fetch checksums, skipping verification.");
            return Ok(());
        }
    };

    let checksums_text =
        String::from_utf8(checksums_bytes).map_err(|e| UpdateError::Io(e.to_string()))?;

    let expected = checksums_text
        .lines()
        .find(|line| line.contains(asset_name))
        .and_then(|line| line.split_whitespace().next());

    let expected = match expected {
        Some(hash) => hash,
        None => {
            eprintln!(
                "Warning: no checksum found for {}, skipping verification.",
                asset_name
            );
            return Ok(());
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(binary);
    let actual = format!("{:x}", hasher.finalize());

    if actual != expected {
        return Err(UpdateError::ChecksumMismatch);
    }

    Ok(())
}

fn map_io_error(e: std::io::Error) -> UpdateError {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        UpdateError::PermissionDenied
    } else {
        UpdateError::Io(e.to_string())
    }
}

pub fn perform_update(force: bool) -> Result<String, UpdateError> {
    let version = fetch_latest_version()
        .ok_or_else(|| UpdateError::Network("Failed to check latest version".into()))?;

    if !is_newer(&version, current_version()) {
        return Err(UpdateError::AlreadyUpToDate(current_version().to_string()));
    }

    if !force {
        if let Ok(exe) = std::env::current_exe() {
            let exe_str = exe.to_string_lossy();
            if exe_str.contains(".cargo/bin") || exe_str.contains(".cargo\\bin") {
                return Err(UpdateError::CargoInstall);
            }
        }
    }

    let asset = platform_asset_name()?;
    let agent = make_agent(Duration::from_secs(60));

    let download_url = format!("{}/{}", GITHUB_DOWNLOAD_BASE, asset);
    println!("Downloading v{}...", version);
    let binary = download_bytes(&agent, &download_url)?;

    let checksums_url = format!("{}/checksums.sha256", GITHUB_DOWNLOAD_BASE);
    verify_checksum(&agent, &binary, asset, &checksums_url)?;
    println!("Checksum verified.");

    replace_executable(&binary)?;

    Ok(version)
}

#[cfg(unix)]
fn replace_executable(binary: &[u8]) -> Result<(), UpdateError> {
    use std::os::unix::fs::PermissionsExt;

    let exe_path = std::env::current_exe().map_err(|e| UpdateError::Io(e.to_string()))?;

    let tmp_path = exe_path.with_extension("new");
    std::fs::write(&tmp_path, binary).map_err(map_io_error)?;

    let perms = std::fs::Permissions::from_mode(0o755);
    if let Err(e) = std::fs::set_permissions(&tmp_path, perms) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(UpdateError::Io(e.to_string()));
    }

    if let Err(e) = std::fs::rename(&tmp_path, &exe_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(map_io_error(e));
    }

    Ok(())
}

#[cfg(windows)]
fn replace_executable(binary: &[u8]) -> Result<(), UpdateError> {
    let exe_path = std::env::current_exe().map_err(|e| UpdateError::Io(e.to_string()))?;

    let old_path = exe_path.with_extension("old.exe");
    let tmp_path = exe_path.with_extension("new.exe");

    std::fs::write(&tmp_path, binary).map_err(map_io_error)?;

    let _ = std::fs::remove_file(&old_path);
    std::fs::rename(&exe_path, &old_path).map_err(|e| UpdateError::Io(e.to_string()))?;

    if let Err(e) = std::fs::rename(&tmp_path, &exe_path) {
        let _ = std::fs::rename(&old_path, &exe_path);
        return Err(UpdateError::Io(e.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("1.0.0", "0.9.0"));
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("invalid", "0.1.0"));
        assert!(!is_newer("0.1.0", "invalid"));
    }

    #[test]
    fn test_platform_asset_name() {
        let result = platform_asset_name();
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.starts_with("rk-"));
    }

    #[test]
    fn test_cached_update() {
        let conn = crate::db::init_db_memory();

        // No cached version — returns None
        assert!(get_cached_update(&conn).is_none());

        // Cached version same as current — returns None
        crate::db::set_preference(&conn, PREF_LATEST_VERSION, current_version()).unwrap();
        assert!(get_cached_update(&conn).is_none());

        // Cached version newer — returns Some
        crate::db::set_preference(&conn, PREF_LATEST_VERSION, "99.0.0").unwrap();
        assert_eq!(get_cached_update(&conn), Some("99.0.0".to_string()));
    }
}

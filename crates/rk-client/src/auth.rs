use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const DEFAULT_SERVER: &str = "https://sync.rustkanban.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub token: String,
    pub device_id: String,
    pub device_name: String,
    pub server_url: String,
    pub last_synced_at: Option<String>,
}

pub fn credentials_path() -> PathBuf {
    let config_dir = dirs::config_dir().expect("Could not determine config directory");
    config_dir.join("rustkanban").join("credentials.json")
}

pub fn load_credentials() -> Option<Credentials> {
    let path = credentials_path();
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_credentials(creds: &Credentials) -> Result<(), Box<dyn std::error::Error>> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(creds)?;
    fs::write(&path, &json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn delete_credentials() -> Result<(), Box<dyn std::error::Error>> {
    let path = credentials_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn is_logged_in() -> bool {
    load_credentials().is_some()
}

#[allow(dead_code)]
pub fn update_last_synced(synced_at: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(mut creds) = load_credentials() {
        creds.last_synced_at = Some(synced_at.to_string());
        save_credentials(&creds)?;
    }
    Ok(())
}

pub fn default_device_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".into())
}

pub fn login(
    server_url: Option<&str>,
    device_name: Option<&str>,
) -> Result<Credentials, Box<dyn std::error::Error>> {
    if let Some(creds) = load_credentials() {
        return Err(format!(
            "Already logged in as device '{}'. Run `rk logout` first.",
            creds.device_name
        )
        .into());
    }

    let server = server_url.unwrap_or(DEFAULT_SERVER);
    let name = device_name
        .map(String::from)
        .unwrap_or_else(default_device_name);

    // Bind localhost callback server
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();

    let login_url = format!(
        "{}/login?redirect_port={}&device_name={}",
        server,
        port,
        urlencoding::encode(&name)
    );

    // Try to open browser
    if open::that(&login_url).is_err() {
        // Headless fallback
        let headless_url = format!(
            "{}/login?device_name={}&mode=headless",
            server,
            urlencoding::encode(&name)
        );
        println!("Open this URL in any browser:");
        println!("  {}", headless_url);
        println!("\nThen paste the token here:");

        let mut token_input = String::new();
        std::io::stdin().read_line(&mut token_input)?;
        let token = token_input.trim().to_string();

        println!("Enter device ID:");
        let mut device_input = String::new();
        std::io::stdin().read_line(&mut device_input)?;
        let device_id = device_input.trim().to_string();

        let creds = Credentials {
            token,
            device_id,
            device_name: name,
            server_url: server.to_string(),
            last_synced_at: None,
        };
        save_credentials(&creds)?;
        return Ok(creds);
    }

    println!("Waiting for authentication... (Ctrl+C to cancel)");

    // Wait for callback (blocking — user can Ctrl+C to cancel)
    let (mut stream, _) = listener.accept()?;

    let reader = BufReader::new(&stream);
    let request_line = reader
        .lines()
        .next()
        .ok_or("No request received")?
        .map_err(|e| format!("Read error: {}", e))?;

    // Parse GET /callback?token=...&device_id=... HTTP/1.1
    let url_part = request_line.split_whitespace().nth(1).unwrap_or("");
    let query = url_part.split('?').nth(1).unwrap_or("");
    let params: HashMap<&str, &str> = query
        .split('&')
        .filter_map(|p| {
            let mut parts = p.splitn(2, '=');
            Some((parts.next()?, parts.next()?))
        })
        .collect();

    let token = params
        .get("token")
        .ok_or("Missing token in callback")?
        .to_string();
    let device_id = params
        .get("device_id")
        .ok_or("Missing device_id in callback")?
        .to_string();

    // Send response — redirect to server's styled success page
    let response = format!(
        "HTTP/1.1 302 Found\r\nLocation: {}/login/success\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        server
    );
    stream.write_all(response.as_bytes()).ok();

    let creds = Credentials {
        token,
        device_id,
        device_name: name,
        server_url: server.to_string(),
        last_synced_at: None,
    };
    save_credentials(&creds)?;
    Ok(creds)
}

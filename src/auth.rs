use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

fn cookies_db_path() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        let home = dirs::home_dir()?;
        Some(home.join("Library/Application Support/Capacities/Cookies"))
    } else if cfg!(target_os = "linux") {
        let config = dirs::config_dir()?;
        Some(config.join("Capacities/Cookies"))
    } else if cfg!(target_os = "windows") {
        let appdata = dirs::data_dir()?;
        Some(appdata.join("Capacities/Cookies"))
    } else {
        None
    }
}

pub fn get_token() -> Result<String> {
    if let Ok(token) = env::var("CAP_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }
    extract_auth_token()
}

fn extract_auth_token() -> Result<String> {
    let db_path = cookies_db_path().ok_or_else(|| {
        anyhow::anyhow!("Unsupported platform for auto-auth. Set CAP_TOKEN or use --token.")
    })?;

    if !db_path.exists() {
        bail!(
            "Capacities cookie database not found at {}.\n\
             Is the Capacities desktop app installed and logged in?\n\
             Alternatively, set CAP_TOKEN or use --token.",
            db_path.display()
        );
    }

    let tmp_path = env::temp_dir().join(format!(
        "capacities-cookies-{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    fs::copy(&db_path, &tmp_path)
        .with_context(|| format!("Failed to copy cookie DB to {}", tmp_path.display()))?;

    let result = (|| -> Result<String> {
        let conn = rusqlite::Connection::open_with_flags(
            &tmp_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        let token: String = conn
            .query_row(
                "SELECT value FROM cookies WHERE host_key = 'app.capacities.io' AND name = 'auth-token' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .context("auth-token cookie not found. Please log into Capacities desktop app first.")?;

        Ok(token)
    })();

    let _ = fs::remove_file(&tmp_path);
    result
}

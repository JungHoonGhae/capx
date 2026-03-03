use anyhow::{Context, Result, bail};
use std::env;
use std::fs;
use std::path::PathBuf;

fn cookies_db_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join("Library/Application Support/Capacities/Cookies")
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
    let db_path = cookies_db_path();
    if !db_path.exists() {
        bail!(
            "Capacities cookie database not found at {}. Is Capacities installed?",
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

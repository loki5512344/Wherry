//! Таблица произвольных настроек приложения (ключ-значение) — используется
//! диалогом Settings для того, что должно переживать перезапуск (в отличие
//! от видимости панелей, которая остаётся только на время сессии).
use anyhow::Result;
use rusqlite::{params, Connection};

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .ok()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_bool(conn: &Connection, key: &str, default: bool) -> bool {
    get_setting(conn, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub fn get_u32(conn: &Connection, key: &str, default: u32) -> u32 {
    get_setting(conn, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub fn set_bool(conn: &Connection, key: &str, value: bool) {
    let _ = set_setting(conn, key, &value.to_string());
}

pub fn set_u32(conn: &Connection, key: &str, value: u32) {
    let _ = set_setting(conn, key, &value.to_string());
}

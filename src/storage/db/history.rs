//! Таблица истории подключений (последние 20 целей, уникальных по host/port/user).
use anyhow::Result;
use rusqlite::{Connection, params};

use super::{protocol_from_str, protocol_to_str};
use crate::domain::connection::Protocol;

/// Одна запись истории: host, port, username, время, conn_id (стабильный id —
/// под ним же лежит пароль в keychain), протокол, key_path.
pub type HistoryRow = (
    String,
    u16,
    String,
    String,
    String,
    Protocol,
    Option<String>,
);

/// Ищет уже существующий conn_id для этой цели (host,port,username), чтобы
/// повторные подключения писали пароль в keychain под тем же id, а не заводили новый.
pub fn find_history_conn_id(
    conn: &Connection,
    host: &str,
    port: u16,
    username: &str,
) -> Result<Option<String>> {
    let id: Option<String> = conn
        .query_row(
            "SELECT conn_id FROM connection_history WHERE host = ?1 AND port = ?2 AND username = ?3",
            params![host, port, username],
            |row| row.get(0),
        )
        .ok();
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
pub fn add_history_entry(
    conn: &Connection,
    host: &str,
    port: u16,
    username: &str,
    conn_id: &str,
    protocol: &Protocol,
    key_path: Option<&str>,
) -> Result<()> {
    use chrono::Local;
    let connected_at = Local::now().format("%H:%M %d.%m").to_string();
    let proto = protocol_to_str(protocol);
    conn.execute(
        "INSERT INTO connection_history (host, port, username, connected_at, conn_id, protocol, key_path)
         VALUES (?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(host, port, username) DO UPDATE SET
            connected_at = excluded.connected_at,
            protocol = excluded.protocol,
            key_path = excluded.key_path,
            conn_id = COALESCE(connection_history.conn_id, excluded.conn_id)",
        params![host, port, username, connected_at, conn_id, proto, key_path],
    )?;
    // Храним только последние 20 записей
    conn.execute(
        "DELETE FROM connection_history WHERE id NOT IN (
            SELECT id FROM connection_history ORDER BY id DESC LIMIT 20
        )",
        [],
    )?;
    Ok(())
}

/// Полностью очищает историю подключений (Settings → History → Clear All).
pub fn clear_history(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM connection_history", [])?;
    Ok(())
}

pub fn get_history(conn: &Connection) -> Result<Vec<HistoryRow>> {
    let mut stmt = conn.prepare(
        "SELECT host, port, username, connected_at, conn_id, protocol, key_path
         FROM connection_history ORDER BY id DESC LIMIT 20",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let proto_str: String = row.get(5)?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                protocol_from_str(&proto_str),
                row.get(6)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

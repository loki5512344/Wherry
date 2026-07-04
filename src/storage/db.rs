use crate::domain::connection::Protocol;
use crate::domain::site::Site;
use anyhow::Result;
use rusqlite::{Connection, params};

fn protocol_to_str(protocol: &Protocol) -> &'static str {
    match protocol {
        Protocol::Sftp => "sftp",
        Protocol::Ftp => "ftp",
        Protocol::Ftps => "ftps",
    }
}

fn protocol_from_str(s: &str) -> Protocol {
    match s {
        "sftp" => Protocol::Sftp,
        "ftps" => Protocol::Ftps,
        _ => Protocol::Ftp,
    }
}

pub fn init_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS sites (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            protocol    TEXT NOT NULL,
            host        TEXT NOT NULL,
            port        INTEGER NOT NULL,
            username    TEXT NOT NULL,
            key_path    TEXT,
            folder      TEXT,
            note        TEXT
        );

        CREATE TABLE IF NOT EXISTS connection_history (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            host         TEXT NOT NULL,
            port         INTEGER NOT NULL,
            username     TEXT NOT NULL,
            connected_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS bookmarks (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            path TEXT NOT NULL
        );
    ",
    )?;

    // Миграция: старые базы созданы без conn_id/protocol/key_path — добавляем,
    // если их ещё нет (ошибку "duplicate column" просто игнорируем).
    let _ = conn.execute("ALTER TABLE connection_history ADD COLUMN conn_id TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE connection_history ADD COLUMN protocol TEXT NOT NULL DEFAULT 'sftp'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE connection_history ADD COLUMN key_path TEXT",
        [],
    );
    // На случай старых записей без уникальности (host,port,username) — оставляем
    // только самую свежую строку на каждую цель перед созданием индекса.
    conn.execute(
        "DELETE FROM connection_history WHERE id NOT IN (
            SELECT MAX(id) FROM connection_history GROUP BY host, port, username
        )",
        [],
    )?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_history_target
         ON connection_history(host, port, username)",
        [],
    )?;

    Ok(())
}

pub fn get_bookmarks(conn: &Connection) -> Result<Vec<(i64, String, String)>> {
    let mut stmt = conn.prepare("SELECT id, name, path FROM bookmarks ORDER BY id")?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn add_bookmark(conn: &Connection, name: &str, path: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO bookmarks (name, path) VALUES (?1, ?2)",
        params![name, path],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn remove_bookmark(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
    Ok(())
}

/// Одна запись истории подключений: host, port, username, время, conn_id
/// (стабильный id — под ним же лежит пароль в keychain), протокол, key_path.
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

pub fn get_sites(conn: &Connection) -> Result<Vec<Site>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, protocol, host, port, username, key_path, folder, note FROM sites ORDER BY name",
    )?;
    let sites = stmt
        .query_map([], |row| {
            let proto_str: String = row.get(2)?;
            let protocol = protocol_from_str(&proto_str);
            Ok(Site {
                id: row.get(0)?,
                name: row.get(1)?,
                protocol,
                host: row.get(3)?,
                port: row.get(4)?,
                username: row.get(5)?,
                key_path: row.get(6)?,
                folder: row.get(7)?,
                note: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(sites)
}

pub fn save_site(conn: &Connection, site: &Site) -> Result<()> {
    let proto = protocol_to_str(&site.protocol);
    conn.execute(
        "INSERT OR REPLACE INTO sites
         (id, name, protocol, host, port, username, key_path, folder, note)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            site.id,
            site.name,
            proto,
            site.host,
            site.port,
            site.username,
            site.key_path,
            site.folder,
            site.note
        ],
    )?;
    Ok(())
}

pub fn delete_site(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM sites WHERE id = ?1", params![id])?;
    Ok(())
}

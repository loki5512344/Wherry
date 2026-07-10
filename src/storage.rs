use crate::domain::{Protocol, Site};
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
            password    TEXT,
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

        CREATE TABLE IF NOT EXISTS app_settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
    ",
    )?;

    let _ = conn.execute("ALTER TABLE connection_history ADD COLUMN conn_id TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE connection_history ADD COLUMN protocol TEXT NOT NULL DEFAULT 'sftp'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE connection_history ADD COLUMN key_path TEXT",
        [],
    );
    let _ = conn.execute("ALTER TABLE sites ADD COLUMN password TEXT", []);
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

// ── Sites ──

pub fn get_sites(conn: &Connection) -> Result<Vec<Site>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, protocol, host, port, username, password, key_path, folder, note FROM sites ORDER BY name",
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
                password: row.get(6)?,
                key_path: row.get(7)?,
                folder: row.get(8)?,
                note: row.get(9)?,
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
         (id, name, protocol, host, port, username, password, key_path, folder, note)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            site.id,
            site.name,
            proto,
            site.host,
            site.port,
            site.username,
            site.password,
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

// ── History ──

pub type HistoryRow = (
    String,
    u16,
    String,
    String,
    String,
    Protocol,
    Option<String>,
);

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
    conn.execute(
        "DELETE FROM connection_history WHERE id NOT IN (
            SELECT id FROM connection_history ORDER BY id DESC LIMIT 20
        )",
        [],
    )?;
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        init_tables(&conn).unwrap();
        conn
    }

    #[test]
    fn test_init_tables() {
        let conn = test_db();
        conn.execute("SELECT 1 FROM sites", []).unwrap();
        conn.execute("SELECT 1 FROM connection_history", [])
            .unwrap();
        conn.execute("SELECT 1 FROM bookmarks", []).unwrap();
        conn.execute("SELECT 1 FROM app_settings", []).unwrap();
    }

    #[test]
    fn test_save_get_delete_site() {
        let conn = test_db();
        let site = Site {
            id: "site-1".into(),
            name: "Test Server".into(),
            protocol: Protocol::Sftp,
            host: "example.com".into(),
            port: 22,
            username: "admin".into(),
            password: Some("secret".into()),
            key_path: None,
            folder: Some("/srv".into()),
            note: Some("test".into()),
        };
        save_site(&conn, &site).unwrap();
        let sites = get_sites(&conn).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].id, "site-1");
        assert_eq!(sites[0].protocol, Protocol::Sftp);

        delete_site(&conn, "site-1").unwrap();
        assert!(get_sites(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_save_site_update() {
        let conn = test_db();
        let site = Site {
            id: "s1".into(),
            name: "Original".into(),
            protocol: Protocol::Ftp,
            host: "a.com".into(),
            port: 21,
            username: "u".into(),
            password: None,
            key_path: None,
            folder: None,
            note: None,
        };
        save_site(&conn, &site).unwrap();
        let updated = Site {
            name: "Updated".into(),
            ..site
        };
        save_site(&conn, &updated).unwrap();
        let sites = get_sites(&conn).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].name, "Updated");
    }

    #[test]
    fn test_add_get_clear_history() {
        let conn = test_db();
        let protocol = Protocol::Sftp;
        add_history_entry(&conn, "example.com", 22, "admin", "conn-1", &protocol, None).unwrap();
        let history = get_history(&conn).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].0, "example.com");
        assert_eq!(history[0].1, 22);
        assert_eq!(history[0].2, "admin");

        clear_history(&conn).unwrap();
        assert!(get_history(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_find_history_conn_id() {
        let conn = test_db();
        let protocol = Protocol::Sftp;
        add_history_entry(&conn, "example.com", 22, "admin", "conn-1", &protocol, None).unwrap();

        let found = find_history_conn_id(&conn, "example.com", 22, "admin").unwrap();
        assert_eq!(found, Some("conn-1".into()));

        let not_found = find_history_conn_id(&conn, "other.com", 22, "admin").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_history_dedup() {
        let conn = test_db();
        let protocol = Protocol::Sftp;
        add_history_entry(&conn, "example.com", 22, "admin", "conn-1", &protocol, None).unwrap();
        add_history_entry(&conn, "example.com", 22, "admin", "conn-2", &protocol, None).unwrap();
        let history = get_history(&conn).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].4, "conn-1");
    }
}

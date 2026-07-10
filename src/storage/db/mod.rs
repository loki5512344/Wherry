//! SQLite-хранилище: схема + операции по таблицам (bookmarks/history/sites).
use crate::domain::connection::Protocol;
use anyhow::Result;
use rusqlite::Connection;

mod bookmarks;
mod history;
mod settings;
mod sites;

pub use bookmarks::{add_bookmark, get_bookmarks, remove_bookmark};
pub use history::{
    add_history_entry, clear_history, find_history_conn_id, get_history, HistoryRow,
};
pub use settings::{get_bool, get_setting, get_u32, set_bool, set_setting, set_u32};
pub use sites::{delete_site, get_sites, save_site};

pub(super) fn protocol_to_str(protocol: &Protocol) -> &'static str {
    match protocol {
        Protocol::Sftp => "sftp",
        Protocol::Ftp => "ftp",
        Protocol::Ftps => "ftps",
    }
}

pub(super) fn protocol_from_str(s: &str) -> Protocol {
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

    // Миграции
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::site::Site;

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
        // same target keeps only the latest; conn_id preserved from first insert
        let history = get_history(&conn).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].4, "conn-1");
    }

    #[test]
    fn test_add_get_remove_bookmark() {
        let conn = test_db();
        let id = add_bookmark(&conn, "Projects", "/home/user/projects").unwrap();
        assert!(id > 0);

        let bookmarks = get_bookmarks(&conn).unwrap();
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].1, "Projects");
        assert_eq!(bookmarks[0].2, "/home/user/projects");

        remove_bookmark(&conn, id).unwrap();
        assert!(get_bookmarks(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_settings_string() {
        let conn = test_db();
        assert!(get_setting(&conn, "theme").is_none());
        set_setting(&conn, "theme", "dark").unwrap();
        assert_eq!(get_setting(&conn, "theme").unwrap(), "dark");
        set_setting(&conn, "theme", "light").unwrap();
        assert_eq!(get_setting(&conn, "theme").unwrap(), "light");
    }

    #[test]
    fn test_settings_bool() {
        let conn = test_db();
        assert!(!get_bool(&conn, "show_hidden", false));
        assert!(get_bool(&conn, "show_hidden", true));
        set_bool(&conn, "show_hidden", true);
        assert!(get_bool(&conn, "show_hidden", false));
    }

    #[test]
    fn test_settings_u32() {
        let conn = test_db();
        assert_eq!(get_u32(&conn, "timeout", 30), 30);
        set_u32(&conn, "timeout", 60);
        assert_eq!(get_u32(&conn, "timeout", 30), 60);
    }
}

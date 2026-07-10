use anyhow::Result;
use rusqlite::{Connection, params};

// ── App settings ──

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

// ── Bookmarks ──

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::storage::init_tables(&conn).unwrap();
        conn
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

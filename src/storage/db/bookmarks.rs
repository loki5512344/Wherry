//! Таблица закладок локальных папок.
use anyhow::Result;
use rusqlite::{params, Connection};

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

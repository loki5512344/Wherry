//! Таблица сохранённых серверов (Sites).
use anyhow::Result;
use rusqlite::{Connection, params};

use super::{protocol_from_str, protocol_to_str};
use crate::domain::site::Site;

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

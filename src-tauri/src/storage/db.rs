use anyhow::Result;
use rusqlite::{Connection, params};
use tauri::{AppHandle, Manager};
use std::sync::Mutex;
use crate::domain::site::Site;
use crate::domain::connection::Protocol;

pub struct Db(pub Mutex<Connection>);

pub fn init(app: &AppHandle) -> Result<()> {
    let path = app.path().app_data_dir()
        .expect("no app data dir")
        .join("loflum.db");

    std::fs::create_dir_all(path.parent().unwrap())?;
    let conn = Connection::open(&path)?;

    conn.execute_batch("
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
    ")?;

    app.manage(Db(Mutex::new(conn)));
    Ok(())
}

pub fn get_sites(conn: &Connection) -> Result<Vec<Site>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, protocol, host, port, username, key_path, folder, note FROM sites ORDER BY name"
    )?;
    let sites = stmt.query_map([], |row| {
        let proto_str: String = row.get(2)?;
        let protocol = match proto_str.as_str() {
            "sftp" => Protocol::Sftp,
            "ftps" => Protocol::Ftps,
            _ => Protocol::Ftp,
        };
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
    })?.filter_map(|r| r.ok()).collect();
    Ok(sites)
}

pub fn save_site(conn: &Connection, site: &Site) -> Result<()> {
    let proto = match site.protocol {
        Protocol::Sftp => "sftp",
        Protocol::Ftp => "ftp",
        Protocol::Ftps => "ftps",
    };
    conn.execute(
        "INSERT OR REPLACE INTO sites
         (id, name, protocol, host, port, username, key_path, folder, note)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            site.id, site.name, proto, site.host, site.port,
            site.username, site.key_path, site.folder, site.note
        ],
    )?;
    Ok(())
}

pub fn delete_site(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM sites WHERE id = ?1", params![id])?;
    Ok(())
}

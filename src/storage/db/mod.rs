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
    HistoryRow, add_history_entry, clear_history, find_history_conn_id, get_history,
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

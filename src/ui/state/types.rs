//! Вспомогательные типы состояния UI (вкладки, отложенные операции, история).
use std::sync::Arc;

use crate::domain::connection::{ConnectionParams, ConnectionStatus, Protocol};
use crate::domain::file_entry::FileEntry;

#[derive(Clone)]
pub struct ConnectionTab {
    pub id: String,
    pub label: String,
    pub params: ConnectionParams,
    pub status: ConnectionStatus,
    pub remote_path: String,
    pub remote_entries: Vec<FileEntry>,
    pub remote_selected: Option<String>,
    pub loading: bool,
}

/// Какая панель последней принимала клик — используется тулбаром,
/// чтобы New Folder/Rename/Delete применялись к нужной стороне (local/remote).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pane {
    Local,
    Remote,
}

/// Активный раздел диалога Settings (popup: меню слева — содержимое справа).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsSection {
    General,
    Appearance,
    Connections,
    History,
    Security,
    Transfers,
    About,
}

pub type PendingResult<T> = Arc<std::sync::Mutex<Option<Result<T, String>>>>;

pub struct PendingConnect {
    pub result: PendingResult<(ConnectionParams, Vec<FileEntry>)>,
}

pub struct PendingRemoteList {
    pub tab_idx: usize,
    pub result: PendingResult<Vec<FileEntry>>,
}

#[derive(Clone)]
pub struct Bookmark {
    pub id: i64,
    pub name: String,
    pub path: String,
}

/// Фиксированный пункт быстрого доступа (Home/Desktop/...) — не удаляется, не хранится в БД.
#[derive(Clone)]
pub struct QuickAccess {
    pub name: String,
    pub path: String,
}

#[derive(Clone)]
pub struct HistoryEntry {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub time: String,
    /// Стабильный id этой цели — под ним же лежит пароль в keychain.
    pub conn_id: String,
    pub protocol: Protocol,
    pub key_path: Option<String>,
}

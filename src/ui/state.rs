use std::sync::Arc;

use crate::domain::connection::{ConnectionParams, ConnectionStatus, Protocol};
use crate::domain::file_entry::FileEntry;
use crate::domain::transfer::TransferTask;

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

type PendingResult<T> = Arc<std::sync::Mutex<Option<Result<T, String>>>>;

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

pub struct AppState {
    pub tabs: Vec<ConnectionTab>,
    pub active_tab: usize,

    pub local_path: String,
    pub local_entries: Vec<FileEntry>,
    pub local_selected: Option<String>,

    /// Панель, в которой был последний клик — New Folder/Rename/Delete в тулбаре
    /// применяются к ней.
    pub active_pane: Pane,
    /// К какой стороне относится открытый сейчас диалог mkdir/rename/delete.
    pub op_target: Pane,

    pub show_connect_dialog: bool,
    pub show_bookmarks: bool,
    pub show_history: bool,
    pub show_settings_dialog: bool,

    /// Фиксированные пункты быстрого доступа (Home/Desktop/...) — не удаляются, не хранятся в БД.
    pub quick_access: Vec<QuickAccess>,
    /// Закладки, добавленные пользователем — хранятся в БД, можно удалять.
    pub bookmarks: Vec<Bookmark>,
    pub history: Vec<HistoryEntry>,

    pub connect_label: String,
    pub connect_host: String,
    pub connect_port: String,
    pub connect_user: String,
    pub connect_pass: String,
    pub connect_key_path: String,
    pub connect_protocol: usize,
    pub connect_error: String,
    pub connect_loading: bool,

    pub show_queue: bool,
    pub queue_tasks: Vec<TransferTask>,

    pub status_message: String,
    pub connected_count: usize,

    // action flags — выставляются тулбаром, обрабатываются в app.rs
    pub pending_refresh: bool,
    /// Клик по записи истории — переподключиться (новая вкладка, без диалога).
    pub pending_history_reconnect: Option<HistoryEntry>,
    /// ПКМ → "Save" на записи истории — сохранить как постоянный Site.
    pub pending_history_save: Option<HistoryEntry>,

    // диалоги операций над удалённой ФС
    pub show_mkdir_dialog: bool,
    pub show_delete_dialog: bool,
    pub show_rename_dialog: bool,

    pub mkdir_name: String,
    pub delete_name: String,
    pub rename_old_name: String,
    pub rename_new_name: String,

    pub pending_mkdir_result: Option<PendingResult<()>>,
    pub pending_delete_result: Option<PendingResult<()>>,
    pub pending_rename_result: Option<PendingResult<()>>,

    pub pending_connect: Option<PendingConnect>,
    pub pending_remote_list: Vec<PendingRemoteList>,
}

impl Default for AppState {
    fn default() -> Self {
        let home = dirs::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            local_path: home.clone(),
            local_entries: Vec::new(),
            local_selected: None,
            active_pane: Pane::Local,
            op_target: Pane::Local,
            show_connect_dialog: false,
            show_bookmarks: false,
            show_history: false,
            show_settings_dialog: false,
            quick_access: vec![
                QuickAccess {
                    name: "Home".into(),
                    path: home.clone(),
                },
                QuickAccess {
                    name: "Desktop".into(),
                    path: format!("{}/Desktop", home),
                },
                QuickAccess {
                    name: "Documents".into(),
                    path: format!("{}/Documents", home),
                },
                QuickAccess {
                    name: "Downloads".into(),
                    path: dirs::download_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| format!("{}/Downloads", home)),
                },
                QuickAccess {
                    name: "Pictures".into(),
                    path: format!("{}/Pictures", home),
                },
                QuickAccess {
                    name: "Music".into(),
                    path: format!("{}/Music", home),
                },
                QuickAccess {
                    name: "Videos".into(),
                    path: format!("{}/Videos", home),
                },
            ],
            bookmarks: Vec::new(),
            history: Vec::new(),
            connect_label: String::new(),
            connect_host: String::new(),
            connect_port: "22".into(),
            connect_user: String::new(),
            connect_pass: String::new(),
            connect_key_path: String::new(),
            connect_protocol: 0,
            connect_error: String::new(),
            connect_loading: false,
            show_queue: false,
            queue_tasks: Vec::new(),
            status_message: "Ready".into(),
            connected_count: 0,
            pending_refresh: false,
            pending_history_reconnect: None,
            pending_history_save: None,
            show_mkdir_dialog: false,
            show_delete_dialog: false,
            show_rename_dialog: false,
            mkdir_name: String::new(),
            delete_name: String::new(),
            rename_old_name: String::new(),
            rename_new_name: String::new(),
            pending_mkdir_result: None,
            pending_delete_result: None,
            pending_rename_result: None,
            pending_connect: None,
            pending_remote_list: Vec::new(),
        }
    }
}

impl AppState {
    pub fn active_tab_mut(&mut self) -> Option<&mut ConnectionTab> {
        if self.tabs.is_empty() {
            return None;
        }
        let idx = self.active_tab.min(self.tabs.len() - 1);
        Some(&mut self.tabs[idx])
    }

    pub fn active_tab_ref(&self) -> Option<&ConnectionTab> {
        if self.tabs.is_empty() {
            return None;
        }
        let idx = self.active_tab.min(self.tabs.len() - 1);
        Some(&self.tabs[idx])
    }

    /// Перечитывает историю подключений из БД (после записи новой попытки).
    pub fn reload_history(&mut self, db: &Arc<std::sync::Mutex<rusqlite::Connection>>) {
        if let Ok(conn) = db.lock()
            && let Ok(rows) = crate::storage::db::get_history(&conn)
        {
            self.history = rows
                .into_iter()
                .map(
                    |(host, port, user, time, conn_id, protocol, key_path)| HistoryEntry {
                        host,
                        port,
                        user,
                        time,
                        conn_id,
                        protocol,
                        key_path,
                    },
                )
                .collect();
        }
    }
}

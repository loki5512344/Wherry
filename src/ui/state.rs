use std::sync::Arc;

use crate::domain::connection::{ConnectionParams, ConnectionStatus};
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
    pub name: String,
    pub path: String,
}

#[derive(Clone)]
pub struct HistoryEntry {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub time: String,
}

pub struct AppState {
    pub tabs: Vec<ConnectionTab>,
    pub active_tab: usize,

    pub local_path: String,
    pub local_entries: Vec<FileEntry>,
    pub local_selected: Option<String>,

    pub show_connect_dialog: bool,
    pub show_bookmarks: bool,
    pub show_history: bool,

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
    pub pending_mkdir: bool,
    pub pending_delete: bool,
    pub pending_rename: bool,

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
            show_connect_dialog: false,
            show_bookmarks: false,
            show_history: false,
            bookmarks: vec![
                Bookmark {
                    name: "Home".into(),
                    path: home.clone(),
                },
                Bookmark {
                    name: "Desktop".into(),
                    path: format!("{}/Desktop", home),
                },
                Bookmark {
                    name: "Documents".into(),
                    path: format!("{}/Documents", home),
                },
                Bookmark {
                    name: "Downloads".into(),
                    path: dirs::download_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| format!("{}/Downloads", home)),
                },
                Bookmark {
                    name: "Pictures".into(),
                    path: format!("{}/Pictures", home),
                },
                Bookmark {
                    name: "Music".into(),
                    path: format!("{}/Music", home),
                },
                Bookmark {
                    name: "Videos".into(),
                    path: format!("{}/Videos", home),
                },
            ],
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
            pending_mkdir: false,
            pending_delete: false,
            pending_rename: false,
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

    pub fn add_history(&mut self, host: &str, port: u16, user: &str) {
        use chrono::Local;
        let now = Local::now().format("%H:%M %d.%m").to_string();
        self.history.insert(
            0,
            HistoryEntry {
                host: host.into(),
                port,
                user: user.into(),
                time: now,
            },
        );
        if self.history.len() > 20 {
            self.history.truncate(20);
        }
    }
}

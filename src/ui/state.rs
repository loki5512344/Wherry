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

pub struct AppState {
    pub tabs: Vec<ConnectionTab>,
    pub active_tab: usize,
    pub local_path: String,
    pub local_entries: Vec<FileEntry>,
    pub local_selected: Option<String>,
    pub local_tree_open: bool,
    pub show_connect_dialog: bool,
    pub onboarding_host: String,
    pub onboarding_user: String,
    pub onboarding_pass: String,
    pub onboarding_port: String,
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
    pub agg_speed: u64,
    pub connected_count: usize,
    pub pending_connect: Option<PendingConnect>,
    pub pending_remote_list: Vec<PendingRemoteList>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            local_path: dirs::home_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            local_entries: Vec::new(),
            local_selected: None,
            local_tree_open: false,
            show_connect_dialog: false,
            onboarding_host: String::new(),
            onboarding_user: String::new(),
            onboarding_pass: String::new(),
            onboarding_port: String::new(),
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
            agg_speed: 0,
            connected_count: 0,
            pending_connect: None,
            pending_remote_list: Vec::new(),
        }
    }
}

impl AppState {
    pub fn active_tab_mut(&mut self) -> Option<&mut ConnectionTab> {
        if self.tabs.is_empty() {
            None
        } else {
            let idx = self.active_tab.min(self.tabs.len() - 1);
            Some(&mut self.tabs[idx])
        }
    }

    pub fn active_tab_ref(&self) -> Option<&ConnectionTab> {
        if self.tabs.is_empty() {
            None
        } else {
            let idx = self.active_tab.min(self.tabs.len() - 1);
            Some(&self.tabs[idx])
        }
    }
}

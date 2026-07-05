//! Начальные значения [`AppState`] (стартовые панели, быстрый доступ, поля формы).
use super::{AppState, Pane, QuickAccess, SettingsSection};
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

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
            settings_section: SettingsSection::General,
            settings_prev_section: SettingsSection::General,
            settings_section_changed_at: f64::NEG_INFINITY,
            show_toolbar: true,
            show_sidebar: true,
            show_status_bar: true,
            show_queue_panel: true,
            confirm_before_delete: true,
            default_local_folder: String::new(),
            auto_clear_completed_secs: 0,
            max_concurrent: Arc::new(AtomicU32::new(2)),
            history_clear_confirm: false,
            quick_access: quick_access(&home),
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

/// Фиксированные пункты быстрого доступа относительно домашней папки.
fn quick_access(home: &str) -> Vec<QuickAccess> {
    let downloads = dirs::download_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{}/Downloads", home));
    let item = |name: &str, path: String| QuickAccess {
        name: name.into(),
        path,
    };
    vec![
        item("Home", home.to_string()),
        item("Desktop", format!("{}/Desktop", home)),
        item("Documents", format!("{}/Documents", home)),
        item("Downloads", downloads),
        item("Pictures", format!("{}/Pictures", home)),
        item("Music", format!("{}/Music", home)),
        item("Videos", format!("{}/Videos", home)),
    ]
}

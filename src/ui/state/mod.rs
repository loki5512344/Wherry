//! Состояние UI: главная структура [`AppState`], её типы ([`types`]) и
//! начальные значения ([`defaults`]).
use std::sync::Arc;

use crate::domain::file_entry::FileEntry;
use crate::domain::transfer::TransferTask;

mod defaults;
mod types;

pub use types::*;

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
    /// Активный раздел открытого диалога Settings.
    pub settings_section: SettingsSection,
    /// Прошлый раздел + момент переключения (ui time) — для кросс-фейда
    /// содержимого при смене раздела.
    pub settings_prev_section: SettingsSection,
    pub settings_section_changed_at: f64,

    /// Видимость панелей — настраивается в Window (macOS) или в Settings.
    pub show_toolbar: bool,
    pub show_sidebar: bool,
    pub show_status_bar: bool,
    pub show_queue_panel: bool,

    // ── Настройки, сохраняемые между запусками (Settings) ────────────────
    /// Спрашивать подтверждение перед удалением файла/папки.
    pub confirm_before_delete: bool,
    /// Стартовая локальная папка; пусто — домашняя папка ОС.
    pub default_local_folder: String,
    /// Через сколько секунд автоматически убирать завершённые задачи из очереди
    /// передач; 0 — никогда.
    pub auto_clear_completed_secs: u32,
    /// Сколько передач может выполняться параллельно — общий с воркером атомик,
    /// чтобы Settings → Transfers менял параллелизм на лету.
    pub max_concurrent: Arc<std::sync::atomic::AtomicU32>,
    /// Инлайн-подтверждение "Clear All History" в разделе Settings → History.
    pub history_clear_confirm: bool,

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

impl AppState {
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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use crate::domain::connection::ConnectionStatus;
use crate::domain::site::Site;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::dialogs::connection;
use crate::ui::dock::PaneTab;
use crate::ui::panels::local_pane;
use crate::ui::widgets::overlay;

mod dialogs;
mod ops;
mod poll;
mod results;
mod screens;
mod settings;
mod visuals;
mod welcome;

pub struct FileManagerApp {
    pub state: crate::ui::state::AppState,
    pub registry: Arc<RemoteRegistry>,
    pub queue: TransferQueue,
    pub rt: tokio::runtime::Runtime,
    pub db: Arc<std::sync::Mutex<rusqlite::Connection>>,
    pub sites: Vec<Site>,
    pub first_frame: bool,
    pub dock_state: egui_dock::DockState<PaneTab>,
    /// Момент, когда задача передачи впервые замечена в состоянии Completed —
    /// нужен для авто-очистки очереди (Settings → Transfers).
    completed_since: HashMap<String, Instant>,
    /// Какой экран был в прошлом кадре (welcome/main) + момент переключения —
    /// для фейда при переходе между экранами. None до первого кадра.
    prev_is_welcome: Option<bool>,
    screen_changed_at: f64,
}

impl FileManagerApp {
    pub fn new(
        registry: Arc<RemoteRegistry>,
        queue: TransferQueue,
        rt: tokio::runtime::Runtime,
        db: Arc<std::sync::Mutex<rusqlite::Connection>>,
        sites: Vec<Site>,
        max_concurrent: Arc<AtomicU32>,
    ) -> Self {
        let prefs = settings::prefs::load(&db);
        max_concurrent.store(prefs.max_concurrent, Ordering::Relaxed);
        crate::i18n::set_lang(prefs.language.unwrap_or_else(crate::i18n::detect_system_lang));

        let start_path = if !prefs.default_local_folder.is_empty()
            && std::path::Path::new(&prefs.default_local_folder).is_dir()
        {
            prefs.default_local_folder.clone()
        } else {
            crate::fs::local::home_dir()
        };

        let mut state = crate::ui::state::AppState {
            local_path: start_path,
            confirm_before_delete: prefs.confirm_before_delete,
            default_local_folder: prefs.default_local_folder,
            auto_clear_completed_secs: prefs.auto_clear_completed_secs,
            max_concurrent,
            ..Default::default()
        };
        local_pane::refresh_local(&mut state);

        state.reload_history(&db);

        if let Ok(rows) = crate::storage::db::get_bookmarks(&db.lock().unwrap()) {
            state.bookmarks = rows
                .into_iter()
                .map(|(id, name, path)| crate::ui::state::Bookmark { id, name, path })
                .collect();
        }

        Self {
            state,
            registry,
            queue,
            rt,
            db,
            sites,
            first_frame: true,
            dock_state: egui_dock::DockState::new(vec![PaneTab::Local]),
            completed_since: HashMap::new(),
            prev_is_welcome: None,
            screen_changed_at: f64::NEG_INFINITY,
        }
    }

    fn active_tab_idx(&self) -> Option<usize> {
        if self.state.tabs.is_empty() {
            return None;
        }
        let idx = self.state.active_tab.min(self.state.tabs.len() - 1);
        if self.state.tabs[idx].status == ConnectionStatus::Connected {
            Some(idx)
        } else {
            None
        }
    }
}

impl eframe::App for FileManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.first_frame {
            self.first_frame = false;
            egui_extras::install_image_loaders(ctx);
            self.apply_visuals(ctx);
            ctx.request_repaint();
        }

        self.poll_pending();
        crate::ui::menu::poll_menu_events(&mut self.state);
        self.state.queue_tasks = self.queue.all();
        self.sweep_completed_tasks();

        // ── Esc закрывает верхний из открытых диалогов ───────────────────────
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            let s = &mut self.state;
            if s.show_mkdir_dialog {
                s.show_mkdir_dialog = false;
            } else if s.show_delete_dialog {
                s.show_delete_dialog = false;
            } else if s.show_rename_dialog {
                s.show_rename_dialog = false;
            } else if s.show_connect_dialog && !s.connect_loading {
                s.show_connect_dialog = false;
            } else if s.show_settings_dialog {
                s.show_settings_dialog = false;
            }
        }

        // ── Переход welcome ↔ main с фейдом ──────────────────────────────────
        let is_welcome = self.state.tabs.is_empty();
        if self.prev_is_welcome != Some(is_welcome) {
            // На самом первом кадре не анимируем — окно и так только появилось.
            if self.prev_is_welcome.is_some() {
                self.screen_changed_at = ctx.input(|i| i.time);
            }
            self.prev_is_welcome = Some(is_welcome);
        }
        let screen_fade =
            (((ctx.input(|i| i.time) - self.screen_changed_at) / 0.2).clamp(0.0, 1.0)) as f32;
        if screen_fade < 1.0 {
            ctx.request_repaint();
        }

        if is_welcome {
            self.render_welcome_screen(ctx, screen_fade);
        } else {
            self.render_main_ui(ctx, screen_fade);
        }

        // «Открытость» диалогов тикает каждый кадр (см. док к overlay::openness),
        // окна рисуются, пока не доиграл fade-out.
        // ── Connection dialog ────────────────────────────────────────────────
        let t_connect = overlay::openness(ctx, "connect_dlg", self.state.show_connect_dialog);
        if t_connect > 0.0 {
            connection::render(
                ctx,
                &mut self.state,
                &self.registry,
                self.rt.handle(),
                t_connect,
            );
        }

        // ── Remote operation dialogs ─────────────────────────────────────────
        self.render_remote_op_dialogs(ctx);

        // ── Settings ─────────────────────────────────────────────────────────
        let t_settings = overlay::openness(ctx, "settings_dlg", self.state.show_settings_dialog);
        if t_settings > 0.0 {
            self.render_settings_dialog(ctx, t_settings);
        }

        // 200ms repaint для обновления прогресса
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

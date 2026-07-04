use std::sync::Arc;

use egui::{CentralPanel, RichText, SidePanel, TopBottomPanel, Visuals};

use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::domain::site::Site;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::dialogs::connection;
use crate::ui::panels::{local_pane, queue, remote_pane, sidebar, status_bar, tabs, toolbar};
use crate::ui::theme::*;

pub struct FileManagerApp {
    pub state: crate::ui::state::AppState,
    pub registry: Arc<RemoteRegistry>,
    pub queue: TransferQueue,
    pub rt: tokio::runtime::Runtime,
    pub db: Arc<std::sync::Mutex<rusqlite::Connection>>,
    pub sites: Vec<Site>,
    pub first_frame: bool,
}

impl FileManagerApp {
    pub fn new(
        registry: Arc<RemoteRegistry>,
        queue: TransferQueue,
        rt: tokio::runtime::Runtime,
        db: Arc<std::sync::Mutex<rusqlite::Connection>>,
        sites: Vec<Site>,
    ) -> Self {
        let mut state = crate::ui::state::AppState {
            local_path: crate::fs::local::home_dir(),
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
        }
    }

    fn poll_pending(&mut self) {
        // --- Connect result ---
        let opt = self.state.pending_connect.take();
        if let Some(pc) = opt.as_ref() {
            let mut guard = pc.result.lock().unwrap();
            if let Some(res) = guard.take() {
                drop(guard);
                match res {
                    Ok((params, entries)) => {
                        let path = "/".to_string();
                        let mut list = vec![FileEntry {
                            name: "..".into(),
                            path: path.clone(),
                            kind: EntryKind::Dir,
                            size: None,
                            modified: None,
                            permissions: None,
                        }];
                        list.extend(entries);

                        let tab = crate::ui::state::ConnectionTab {
                            id: params.id.clone(),
                            label: params.label.clone(),
                            params: params.clone(),
                            status: ConnectionStatus::Connected,
                            remote_path: path,
                            remote_entries: list,
                            remote_selected: None,
                            loading: false,
                        };
                        self.state.tabs.push(tab);
                        self.state.active_tab = self.state.tabs.len() - 1;
                        self.state.status_message = format!("Connected to {}", params.host);
                        self.state.connect_loading = false;
                        self.state.show_connect_dialog = false;

                        // Один и тот же host/port/username всегда должен использовать
                        // один conn_id — под ним лежит пароль в keychain, иначе повторное
                        // подключение через "New Connection" развело бы историю и keychain.
                        if let Ok(conn) = self.db.lock() {
                            let canonical_id = crate::storage::db::find_history_conn_id(
                                &conn,
                                &params.host,
                                params.port,
                                &params.username,
                            )
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| params.id.clone());

                            if let Some(password) = &params.password {
                                let _ = crate::storage::keychain::store_password(
                                    &canonical_id,
                                    password,
                                );
                            }

                            let _ = crate::storage::db::add_history_entry(
                                &conn,
                                &params.host,
                                params.port,
                                &params.username,
                                &canonical_id,
                                &params.protocol,
                                params.key_path.as_deref(),
                            );
                        }
                        self.state.reload_history(&self.db);
                    }
                    Err(e) => {
                        self.state.status_message = format!("Connection failed: {}", e);
                        self.state.connect_error = e;
                        self.state.connect_loading = false;
                    }
                }
            } else {
                drop(guard);
                self.state.pending_connect = opt;
            }
        }

        // --- Remote list results ---
        let mut done: Vec<usize> = Vec::new();
        for (i, pending) in self.state.pending_remote_list.iter().enumerate() {
            let mut guard = pending.result.lock().unwrap();
            if let Some(res) = guard.take() {
                let tab_idx = pending.tab_idx;
                match res {
                    Ok(entries) => {
                        let path = if tab_idx < self.state.tabs.len() {
                            self.state.tabs[tab_idx].remote_path.clone()
                        } else {
                            "/".into()
                        };
                        let mut list = Vec::new();
                        if path != "/" {
                            let parent = remote_pane::remote_parent(&path);
                            list.push(FileEntry {
                                name: "..".into(),
                                path: parent,
                                kind: EntryKind::Dir,
                                size: None,
                                modified: None,
                                permissions: None,
                            });
                        }
                        list.extend(entries);
                        if tab_idx < self.state.tabs.len() {
                            self.state.tabs[tab_idx].remote_entries = list;
                            self.state.tabs[tab_idx].loading = false;
                        }
                    }
                    Err(e) => {
                        if tab_idx < self.state.tabs.len() {
                            self.state.tabs[tab_idx].loading = false;
                        }
                        self.state.status_message = format!("Remote list error: {}", e);
                    }
                }
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.state.pending_remote_list.remove(i);
        }

        // --- Toolbar action flags ---
        if self.state.pending_refresh {
            self.state.pending_refresh = false;
            if let Some(idx) = self.active_tab_idx() {
                remote_pane::trigger_list(&mut self.state, idx, &self.registry, self.rt.handle());
            }
        }

        // --- История: переподключение по клику ---
        if let Some(entry) = self.state.pending_history_reconnect.take() {
            connection::reconnect_from_history(
                &mut self.state,
                &self.registry,
                self.rt.handle(),
                &entry,
            );
        }

        // --- История: "Save" → постоянный Site ---
        if let Some(entry) = self.state.pending_history_save.take() {
            match connection::save_history_as_site(&self.db, &mut self.sites, &entry) {
                Ok(()) => self.state.status_message = "Saved to sites".into(),
                Err(e) => self.state.status_message = format!("Save failed: {}", e),
            }
        }

        // --- Mkdir result ---
        let opt = self.state.pending_mkdir_result.take();
        if let Some(res) = opt.as_ref() {
            let mut guard = res.lock().unwrap();
            if let Some(r) = guard.take() {
                drop(guard);
                match r {
                    Ok(()) => {
                        self.state.status_message = "Folder created".into();
                        self.state.mkdir_name.clear();
                        if let Some(idx) = self.active_tab_idx() {
                            remote_pane::trigger_list(
                                &mut self.state,
                                idx,
                                &self.registry,
                                self.rt.handle(),
                            );
                        }
                    }
                    Err(e) => {
                        self.state.status_message = format!("Create folder failed: {}", e);
                    }
                }
            } else {
                drop(guard);
                self.state.pending_mkdir_result = opt;
            }
        }

        // --- Delete result ---
        let opt = self.state.pending_delete_result.take();
        if let Some(res) = opt.as_ref() {
            let mut guard = res.lock().unwrap();
            if let Some(r) = guard.take() {
                drop(guard);
                match r {
                    Ok(()) => {
                        self.state.status_message = "Deleted".into();
                        self.state.delete_name.clear();
                        if let Some(idx) = self.active_tab_idx() {
                            remote_pane::trigger_list(
                                &mut self.state,
                                idx,
                                &self.registry,
                                self.rt.handle(),
                            );
                        }
                    }
                    Err(e) => {
                        self.state.status_message = format!("Delete failed: {}", e);
                    }
                }
            } else {
                drop(guard);
                self.state.pending_delete_result = opt;
            }
        }

        // --- Rename result ---
        let opt = self.state.pending_rename_result.take();
        if let Some(res) = opt.as_ref() {
            let mut guard = res.lock().unwrap();
            if let Some(r) = guard.take() {
                drop(guard);
                match r {
                    Ok(()) => {
                        self.state.status_message = "Renamed".into();
                        self.state.rename_old_name.clear();
                        self.state.rename_new_name.clear();
                        if let Some(idx) = self.active_tab_idx() {
                            remote_pane::trigger_list(
                                &mut self.state,
                                idx,
                                &self.registry,
                                self.rt.handle(),
                            );
                        }
                    }
                    Err(e) => {
                        self.state.status_message = format!("Rename failed: {}", e);
                    }
                }
            } else {
                drop(guard);
                self.state.pending_rename_result = opt;
            }
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

    fn start_mkdir(&mut self, tab_idx: usize, name: String) {
        let tab = &self.state.tabs[tab_idx];
        let connection_id = tab.id.clone();
        let remote_path = tab.remote_path.clone();
        let registry = self.registry.clone();
        let path = format!("{}/{}", remote_path.trim_end_matches('/'), name);

        let result = Arc::new(std::sync::Mutex::new(None::<Result<(), String>>));
        let result_clone = result.clone();

        self.rt.handle().spawn(async move {
            let fs = match registry.get(&connection_id) {
                Some(fs) => fs,
                None => {
                    *result_clone.lock().unwrap() = Some(Err("connection not found".into()));
                    return;
                }
            };
            let r = fs.mkdir(&path).await.map_err(|e| e.to_string());
            *result_clone.lock().unwrap() = Some(r);
        });

        self.state.pending_mkdir_result = Some(result);
    }

    fn start_delete(&mut self, tab_idx: usize, name: String) {
        let tab = &self.state.tabs[tab_idx];
        let connection_id = tab.id.clone();
        let registry = self.registry.clone();

        let entry_path = tab
            .remote_entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.path.clone());

        if name == ".." {
            self.state.status_message = "Cannot delete parent entry".into();
            return;
        }

        let Some(path) = entry_path else {
            self.state.status_message = "Selected entry not found".into();
            return;
        };

        let result = Arc::new(std::sync::Mutex::new(None::<Result<(), String>>));
        let result_clone = result.clone();

        self.rt.handle().spawn(async move {
            let fs = match registry.get(&connection_id) {
                Some(fs) => fs,
                None => {
                    *result_clone.lock().unwrap() = Some(Err("connection not found".into()));
                    return;
                }
            };
            let r = fs.delete(&path).await.map_err(|e| e.to_string());
            *result_clone.lock().unwrap() = Some(r);
        });

        self.state.pending_delete_result = Some(result);
    }

    fn start_rename(&mut self, tab_idx: usize, old_name: String, new_name: String) {
        let tab = &self.state.tabs[tab_idx];
        let connection_id = tab.id.clone();
        let registry = self.registry.clone();

        let Some(entry) = tab.remote_entries.iter().find(|e| e.name == old_name) else {
            self.state.status_message = "Selected entry not found".into();
            return;
        };

        let from = entry.path.clone();
        let parent_dir = remote_pane::remote_parent(&from);
        let to = format!("{}/{}", parent_dir.trim_end_matches('/'), new_name);

        let result = Arc::new(std::sync::Mutex::new(None::<Result<(), String>>));
        let result_clone = result.clone();

        self.rt.handle().spawn(async move {
            let fs = match registry.get(&connection_id) {
                Some(fs) => fs,
                None => {
                    *result_clone.lock().unwrap() = Some(Err("connection not found".into()));
                    return;
                }
            };
            let r = fs.rename(&from, &to).await.map_err(|e| e.to_string());
            *result_clone.lock().unwrap() = Some(r);
        });

        self.state.pending_rename_result = Some(result);
    }

    fn start_local_mkdir(&mut self, name: String) {
        let path = format!("{}/{}", self.state.local_path.trim_end_matches('/'), name);
        match crate::fs::local::mkdir(&path) {
            Ok(()) => {
                self.state.status_message = "Folder created".into();
                local_pane::refresh_local(&mut self.state);
            }
            Err(e) => self.state.status_message = format!("Create folder failed: {}", e),
        }
    }

    fn start_local_delete(&mut self, name: String) {
        if name == ".." {
            self.state.status_message = "Cannot delete parent entry".into();
            return;
        }
        let Some(entry) = self.state.local_entries.iter().find(|e| e.name == name) else {
            self.state.status_message = "Selected entry not found".into();
            return;
        };
        match crate::fs::local::delete(&entry.path) {
            Ok(()) => {
                self.state.status_message = "Deleted".into();
                local_pane::refresh_local(&mut self.state);
            }
            Err(e) => self.state.status_message = format!("Delete failed: {}", e),
        }
    }

    fn start_local_rename(&mut self, old_name: String, new_name: String) {
        let Some(entry) = self.state.local_entries.iter().find(|e| e.name == old_name) else {
            self.state.status_message = "Selected entry not found".into();
            return;
        };
        let from = entry.path.clone();
        let parent_dir = std::path::Path::new(&from)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| self.state.local_path.clone());
        let to = format!("{}/{}", parent_dir.trim_end_matches('/'), new_name);
        match crate::fs::local::rename(&from, &to) {
            Ok(()) => {
                self.state.status_message = "Renamed".into();
                local_pane::refresh_local(&mut self.state);
            }
            Err(e) => self.state.status_message = format!("Rename failed: {}", e),
        }
    }

    fn apply_visuals(&self, ctx: &egui::Context) {
        let mut vis = Visuals::dark();
        vis.window_fill = BG_PANEL;
        vis.panel_fill = BG_CONTENT;
        vis.extreme_bg_color = BG_BASE;
        vis.code_bg_color = BG_BASE;
        vis.override_text_color = Some(TEXT_PRIMARY);
        vis.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        vis.widgets.inactive.bg_fill = BG_CONTENT;
        vis.widgets.hovered.bg_fill = BG_ROW_HOVER;
        vis.widgets.active.bg_fill = ACCENT_DIM;
        vis.selection.bg_fill = BG_ROW_SEL;
        vis.selection.stroke = egui::Stroke::new(1.0, ACCENT);
        vis.window_rounding = egui::Rounding::same(8.0);
        ctx.set_visuals(vis);

        ctx.set_fonts(system_fonts());

        let mut style = ctx.style().as_ref().clone();
        style.spacing.item_spacing = egui::vec2(4.0, 2.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        ctx.set_style(style);
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
        self.state.connected_count = self
            .state
            .tabs
            .iter()
            .filter(|t| t.status == ConnectionStatus::Connected)
            .count();

        if self.state.tabs.is_empty() {
            self.render_welcome_screen(ctx);
        } else {
            self.render_main_ui(ctx);
        }

        // ── Connection dialog ────────────────────────────────────────────────
        if self.state.show_connect_dialog {
            connection::render(ctx, &mut self.state, &self.registry, self.rt.handle());
        }

        // ── Remote operation dialogs ─────────────────────────────────────────
        self.render_remote_op_dialogs(ctx);

        // ── Settings ─────────────────────────────────────────────────────────
        if self.state.show_settings_dialog {
            self.render_settings_dialog(ctx);
        }

        // 200ms repaint для обновления прогресса
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

impl FileManagerApp {
    /// Стартовый экран — показывается, пока нет ни одного открытого соединения.
    /// Только фон, логотип, кнопка нового подключения и список недавних серверов.
    fn render_welcome_screen(&mut self, ctx: &egui::Context) {
        CentralPanel::default()
            .frame(egui::Frame::none().fill(BG_BASE))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let avail_h = ui.available_height();
                    ui.add_space((avail_h * 0.16).max(24.0));

                    let icon_bytes: &[u8] = include_bytes!("icons/app_icon.png");
                    let icon_img = egui::Image::from_bytes("bytes://welcome_app_icon", icon_bytes)
                        .rounding(RADIUS_LG)
                        .fit_to_exact_size(egui::vec2(72.0, 72.0));
                    ui.add(icon_img);

                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("LoFlum")
                            .color(TEXT_PRIMARY)
                            .size(20.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new("FTP/SFTP client").color(TEXT_HINT).size(12.5));

                    ui.add_space(20.0);
                    let btn = egui::Button::image_and_text(
                        crate::ui::icons::image(
                            crate::ui::icons::Icon::AddCircleBold,
                            15.0,
                            ON_ACCENT,
                        ),
                        RichText::new("New Connection")
                            .color(ON_ACCENT)
                            .size(12.5)
                            .strong(),
                    )
                    .fill(ACCENT)
                    .rounding(RADIUS_MD)
                    .min_size(egui::vec2(170.0, 36.0));
                    if ui.add(btn).clicked() {
                        self.state.show_connect_dialog = true;
                    }

                    ui.add_space(28.0);

                    let list_w = 320.0_f32.min(ui.available_width() - 40.0);
                    if !self.state.history.is_empty() {
                        ui.label(
                            RichText::new("RECENT CONNECTIONS")
                                .color(TEXT_HINT)
                                .size(10.5)
                                .strong(),
                        );
                        ui.add_space(8.0);

                        const ROW_H: f32 = 34.0;
                        egui::Frame::none().show(ui, |ui| {
                            ui.set_width(list_w);
                            let history: Vec<_> =
                                self.state.history.iter().take(8).cloned().collect();
                            for entry in &history {
                                let row_rect = egui::Rect::from_min_size(
                                    ui.cursor().min,
                                    egui::vec2(list_w, ROW_H),
                                );
                                let is_hovered = ui.rect_contains_pointer(row_rect);
                                let bg = if is_hovered {
                                    BG_ROW_HOVER
                                } else {
                                    egui::Color32::TRANSPARENT
                                };

                                let label = format!("{}@{}:{}", entry.user, entry.host, entry.port);
                                let row = egui::Frame::none()
                                    .fill(bg)
                                    .rounding(RADIUS_MD)
                                    .inner_margin(egui::Margin::symmetric(10.0, 7.0))
                                    .show(ui, |ui| {
                                        ui.set_width(list_w - 20.0);
                                        ui.horizontal(|ui| {
                                            crate::ui::icons::icon(
                                                ui,
                                                crate::ui::icons::Icon::ServerSquare,
                                                13.0,
                                                TEXT_DIM,
                                            );
                                            ui.add_space(8.0);
                                            ui.label(
                                                RichText::new(&label)
                                                    .color(TEXT_PRIMARY)
                                                    .size(12.5),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new(&entry.time)
                                                            .color(TEXT_HINT)
                                                            .size(10.5),
                                                    );
                                                },
                                            );
                                        });
                                    });

                                let resp = row.response.interact(egui::Sense::click());
                                if is_hovered {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if resp.clicked() {
                                    self.state.pending_history_reconnect = Some(entry.clone());
                                }
                                resp.context_menu(|ui| {
                                    if ui.button("Edit").clicked() {
                                        connection::edit_history_entry(&mut self.state, entry);
                                        ui.close_menu();
                                    }
                                    if ui.button("Save").clicked() {
                                        self.state.pending_history_save = Some(entry.clone());
                                        ui.close_menu();
                                    }
                                });
                            }
                        });
                    }
                });
            });
    }

    fn render_main_ui(&mut self, ctx: &egui::Context) {
        // ── Toolbar ──────────────────────────────────────────────────────────
        TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none().fill(BG_TOOLBAR))
            .show(ctx, |ui| {
                toolbar::render(ui, &mut self.state, &self.queue);
            });

        // ── Tab bar ──────────────────────────────────────────────────────────
        TopBottomPanel::top("tabbar")
            .frame(egui::Frame::none().fill(BG_BASE))
            .show(ctx, |ui| {
                tabs::render(ui, &mut self.state);
            });

        // ── Status bar ───────────────────────────────────────────────────────
        TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none().fill(BG_BASE))
            .exact_height(STATUS_H)
            .show(ctx, |ui| {
                status_bar::render(ui, &self.state);
            });

        // ── Queue panel ───────────────────────────────────────────────────────
        let tasks = self.state.queue_tasks.clone();
        let q_h = if self.state.show_queue && !tasks.is_empty() {
            QUEUE_EXPANDED_H
        } else {
            QUEUE_COLLAPSED_H
        };
        TopBottomPanel::bottom("queue_panel")
            .frame(
                egui::Frame::none()
                    .fill(BG_QUEUE)
                    .stroke(egui::Stroke::new(1.0, BORDER)),
            )
            .resizable(false)
            .exact_height(q_h)
            .show(ctx, |ui| {
                queue::render(ui, &mut self.state, &tasks);
            });

        // ── Sidebar ───────────────────────────────────────────────────────────
        SidePanel::left("sidebar")
            .frame(
                egui::Frame::none()
                    .fill(BG_PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER)),
            )
            .resizable(true)
            .default_width(SIDEBAR_W)
            .width_range(120.0..=260.0)
            .show(ctx, |ui| {
                sidebar::render(ui, &mut self.state, &self.db);
            });

        // ── Main content: Local | Remote ─────────────────────────────────────
        CentralPanel::default()
            .frame(egui::Frame::none().fill(BG_CONTENT))
            .show(ctx, |ui| {
                let has_connection = self.active_tab_idx().is_some();
                let total_w = ui.available_width();
                let total_h = ui.available_height();
                let sep_w = 1.0;
                let half = (total_w - sep_w) * 0.5;

                ui.horizontal(|ui| {
                    // ── LOCAL PANE ──────────────────────────────────────────
                    ui.allocate_ui_with_layout(
                        egui::vec2(half, total_h),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            local_pane::render(
                                ui,
                                &mut self.state,
                                &self.queue,
                                &self.registry,
                                self.rt.handle(),
                                &self.db,
                            );
                        },
                    );

                    // разделитель
                    let (_, sep_rect) = ui.allocate_space(egui::vec2(sep_w, total_h));
                    ui.painter().rect_filled(sep_rect, 0.0, BORDER);

                    // ── REMOTE PANE ─────────────────────────────────────────
                    ui.allocate_ui_with_layout(
                        egui::vec2(half, total_h),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            if has_connection {
                                let idx = self.state.active_tab.min(self.state.tabs.len() - 1);

                                remote_pane::render(
                                    ui,
                                    &mut self.state,
                                    idx,
                                    &self.registry,
                                    self.rt.handle(),
                                    &self.queue,
                                );
                            } else {
                                // Empty state
                                pane_header(ui, "REMOTE", half);
                                ui.with_layout(
                                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                    |ui| {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(ui.available_height() * 0.28);

                                            let circle_d = 56.0;
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(circle_d, circle_d),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().circle_filled(
                                                rect.center(),
                                                circle_d / 2.0,
                                                BG_TAB_ACTIVE,
                                            );
                                            let icon_img = crate::ui::icons::image(
                                                crate::ui::icons::Icon::ServerSquare,
                                                26.0,
                                                TEXT_HINT,
                                            );
                                            icon_img.paint_at(
                                                ui,
                                                egui::Rect::from_center_size(
                                                    rect.center(),
                                                    egui::vec2(26.0, 26.0),
                                                ),
                                            );

                                            ui.add_space(10.0);
                                            ui.label(
                                                RichText::new("Remote")
                                                    .color(TEXT_DIM)
                                                    .size(14.0)
                                                    .strong(),
                                            );
                                            ui.add_space(4.0);
                                            ui.label(
                                                RichText::new(
                                                    "Connect to a server to browse files",
                                                )
                                                .color(TEXT_HINT)
                                                .size(12.0),
                                            );
                                            ui.add_space(16.0);
                                            let btn = egui::Button::image_and_text(
                                                crate::ui::icons::image(
                                                    crate::ui::icons::Icon::AddCircleBold,
                                                    15.0,
                                                    ON_ACCENT,
                                                ),
                                                RichText::new("New Connection")
                                                    .color(ON_ACCENT)
                                                    .size(12.5)
                                                    .strong(),
                                            )
                                            .fill(ACCENT)
                                            .rounding(RADIUS_MD)
                                            .min_size(egui::vec2(150.0, 34.0));
                                            if ui.add(btn).clicked() {
                                                self.state.show_connect_dialog = true;
                                            }
                                        });
                                    },
                                );
                            }
                        },
                    );
                });
            });
    }
}

impl FileManagerApp {
    fn render_remote_op_dialogs(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let center = screen.center();

        // затемнённый оверлей под диалогами
        if self.state.show_mkdir_dialog
            || self.state.show_delete_dialog
            || self.state.show_rename_dialog
        {
            egui::Area::new(egui::Id::new("remote_op_overlay"))
                .order(egui::Order::Background)
                .show(ctx, |ui| {
                    ui.painter().rect_filled(
                        screen,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 120),
                    );
                });
        }

        // --- New Folder ---
        if self.state.show_mkdir_dialog {
            let mut open = true;
            let mut clicked_ok = false;
            egui::Window::new("new_folder_dialog")
                .collapsible(false)
                .title_bar(false)
                .resizable(false)
                .default_pos(center)
                .pivot(egui::Align2::CENTER_CENTER)
                .frame(
                    egui::Frame::none()
                        .fill(BG_PANEL)
                        .stroke(egui::Stroke::new(1.0, BORDER))
                        .rounding(RADIUS_LG)
                        .inner_margin(egui::Margin::same(16.0)),
                )
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_width(260.0);
                    ui.label(RichText::new("New Folder").color(TEXT_PRIMARY).strong());
                    ui.add_space(12.0);
                    ui.text_edit_singleline(&mut self.state.mkdir_name);
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ok =
                                egui::Button::new(RichText::new("OK").color(ON_ACCENT).size(12.5))
                                    .fill(ACCENT)
                                    .rounding(RADIUS_MD)
                                    .min_size(egui::vec2(70.0, 30.0));
                            if ui.add(ok).clicked() {
                                clicked_ok = true;
                            }
                            ui.add_space(8.0);
                            let cancel = egui::Button::new(
                                RichText::new("Cancel").color(TEXT_DIM).size(12.5),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .min_size(egui::vec2(70.0, 30.0));
                            if ui.add(cancel).clicked() {
                                self.state.show_mkdir_dialog = false;
                                self.state.mkdir_name.clear();
                            }
                        });
                    });
                });

            if clicked_ok && !self.state.mkdir_name.is_empty() {
                let name = self.state.mkdir_name.clone();
                match self.state.op_target {
                    crate::ui::state::Pane::Local => self.start_local_mkdir(name),
                    crate::ui::state::Pane::Remote => {
                        if let Some(idx) = self.active_tab_idx() {
                            self.start_mkdir(idx, name);
                        }
                    }
                }
                self.state.show_mkdir_dialog = false;
            }
            if !open {
                self.state.show_mkdir_dialog = false;
                self.state.mkdir_name.clear();
            }
        }

        // --- Delete ---
        if self.state.show_delete_dialog {
            let mut open = true;
            let mut clicked_ok = false;
            let name = self.state.delete_name.clone();
            egui::Window::new("delete_dialog")
                .collapsible(false)
                .title_bar(false)
                .resizable(false)
                .default_pos(center)
                .pivot(egui::Align2::CENTER_CENTER)
                .frame(
                    egui::Frame::none()
                        .fill(BG_PANEL)
                        .stroke(egui::Stroke::new(1.0, BORDER))
                        .rounding(RADIUS_LG)
                        .inner_margin(egui::Margin::same(16.0)),
                )
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_width(260.0);
                    ui.horizontal(|ui| {
                        crate::ui::icons::icon(
                            ui,
                            crate::ui::icons::Icon::DangerTriangle,
                            18.0,
                            RED,
                        );
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("Delete \"{}\"?", name))
                                    .color(TEXT_PRIMARY)
                                    .strong(),
                            );
                            ui.label(
                                RichText::new("This action cannot be undone.")
                                    .color(TEXT_DIM)
                                    .size(11.0),
                            );
                        });
                    });
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ok =
                                egui::Button::new(RichText::new("OK").color(ON_ACCENT).size(12.5))
                                    .fill(RED)
                                    .rounding(RADIUS_MD)
                                    .min_size(egui::vec2(70.0, 30.0));
                            if ui.add(ok).clicked() {
                                clicked_ok = true;
                            }
                            ui.add_space(8.0);
                            let cancel = egui::Button::new(
                                RichText::new("Cancel").color(TEXT_DIM).size(12.5),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .min_size(egui::vec2(70.0, 30.0));
                            if ui.add(cancel).clicked() {
                                self.state.show_delete_dialog = false;
                                self.state.delete_name.clear();
                            }
                        });
                    });
                });

            if clicked_ok {
                match self.state.op_target {
                    crate::ui::state::Pane::Local => self.start_local_delete(name),
                    crate::ui::state::Pane::Remote => {
                        if let Some(idx) = self.active_tab_idx() {
                            self.start_delete(idx, name);
                        }
                    }
                }
                self.state.show_delete_dialog = false;
            }
            if !open {
                self.state.show_delete_dialog = false;
                self.state.delete_name.clear();
            }
        }

        // --- Rename ---
        if self.state.show_rename_dialog {
            let mut open = true;
            let mut clicked_ok = false;
            egui::Window::new("rename_dialog")
                .collapsible(false)
                .title_bar(false)
                .resizable(false)
                .default_pos(center)
                .pivot(egui::Align2::CENTER_CENTER)
                .frame(
                    egui::Frame::none()
                        .fill(BG_PANEL)
                        .stroke(egui::Stroke::new(1.0, BORDER))
                        .rounding(RADIUS_LG)
                        .inner_margin(egui::Margin::same(16.0)),
                )
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_width(260.0);
                    ui.label(RichText::new("Rename").color(TEXT_PRIMARY).strong());
                    ui.add_space(12.0);
                    ui.text_edit_singleline(&mut self.state.rename_new_name);
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ok =
                                egui::Button::new(RichText::new("OK").color(ON_ACCENT).size(12.5))
                                    .fill(ACCENT)
                                    .rounding(RADIUS_MD)
                                    .min_size(egui::vec2(70.0, 30.0));
                            if ui.add(ok).clicked() {
                                clicked_ok = true;
                            }
                            ui.add_space(8.0);
                            let cancel = egui::Button::new(
                                RichText::new("Cancel").color(TEXT_DIM).size(12.5),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .min_size(egui::vec2(70.0, 30.0));
                            if ui.add(cancel).clicked() {
                                self.state.show_rename_dialog = false;
                                self.state.rename_new_name.clear();
                                self.state.rename_old_name.clear();
                            }
                        });
                    });
                });

            if clicked_ok && !self.state.rename_new_name.is_empty() {
                let old_name = self.state.rename_old_name.clone();
                let new_name = self.state.rename_new_name.clone();
                match self.state.op_target {
                    crate::ui::state::Pane::Local => self.start_local_rename(old_name, new_name),
                    crate::ui::state::Pane::Remote => {
                        if let Some(idx) = self.active_tab_idx() {
                            self.start_rename(idx, old_name, new_name);
                        }
                    }
                }
                self.state.show_rename_dialog = false;
            }
            if !open {
                self.state.show_rename_dialog = false;
                self.state.rename_new_name.clear();
                self.state.rename_old_name.clear();
            }
        }
    }

    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let center = screen.center();

        egui::Area::new(egui::Id::new("settings_overlay"))
            .fixed_pos(egui::Pos2::ZERO)
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                let r = ui.allocate_rect(screen, egui::Sense::click());
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(160));
                if r.clicked() {
                    self.state.show_settings_dialog = false;
                }
            });

        let mut open = true;
        egui::Window::new("settings_dialog")
            .collapsible(false)
            .title_bar(false)
            .resizable(false)
            .default_pos(center)
            .pivot(egui::Align2::CENTER_CENTER)
            .frame(
                egui::Frame::none()
                    .fill(BG_PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .rounding(RADIUS_LG)
                    .inner_margin(egui::Margin::same(20.0)),
            )
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.horizontal(|ui| {
                    crate::ui::icons::icon(ui, crate::ui::icons::Icon::Settings, 16.0, ACCENT);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Settings")
                            .color(TEXT_PRIMARY)
                            .size(15.0)
                            .strong(),
                    );
                });
                ui.add_space(16.0);

                settings_row(ui, "Version", env!("CARGO_PKG_VERSION"));
                let db_path = dirs::data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("loflum")
                    .join("loflum.db");
                settings_row(ui, "Sites database", &db_path.to_string_lossy());

                ui.add_space(16.0);
                ui.label(
                    RichText::new("More settings are on the way.")
                        .color(TEXT_HINT)
                        .size(11.0),
                );

                ui.add_space(16.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close =
                        egui::Button::new(RichText::new("Close").color(ON_ACCENT).size(12.5))
                            .fill(ACCENT)
                            .rounding(RADIUS_MD)
                            .min_size(egui::vec2(80.0, 30.0));
                    if ui.add(close).clicked() {
                        self.state.show_settings_dialog = false;
                    }
                });
            });

        if !open {
            self.state.show_settings_dialog = false;
        }
    }
}

fn settings_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_DIM).size(11.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(TEXT_HINT).monospace().size(10.5));
        });
    });
    ui.add_space(6.0);
}

/// Загружает системный шрифт (SF Pro / SF Mono на macOS) вместо egui-дефолта,
/// чтобы интерфейс выглядел нативно, как в исходном макете. На других ОС
/// файлы просто отсутствуют — тогда остаются встроенные шрифты egui.
fn system_fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();

    let candidates: &[(&str, &str, egui::FontFamily)] = &[
        (
            "system-sans",
            "/System/Library/Fonts/SFNS.ttf",
            egui::FontFamily::Proportional,
        ),
        (
            "system-mono",
            "/System/Library/Fonts/SFNSMono.ttf",
            egui::FontFamily::Monospace,
        ),
    ];

    for (name, path, family) in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert((*name).to_owned(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .entry(family.clone())
                .or_default()
                .insert(0, (*name).to_owned());
        }
    }

    fonts
}

fn pane_header(ui: &mut egui::Ui, label: &str, width: f32) {
    egui::Frame::none()
        .fill(BG_BASE)
        .inner_margin(egui::Margin::symmetric(12.0, 0.0))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(width - 24.0, 30.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(RichText::new(label).color(TEXT_DIM).size(10.5).strong());
                },
            );
        });

    let (_, sep) = ui.allocate_space(egui::vec2(width, 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);
}

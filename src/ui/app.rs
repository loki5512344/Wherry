use std::sync::Arc;

use egui::{CentralPanel, Color32, RichText, SidePanel, TopBottomPanel, Visuals};

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
                        self.state
                            .add_history(&params.host, params.port, &params.username);
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

    fn apply_visuals(&self, ctx: &egui::Context) {
        let mut vis = Visuals::dark();
        vis.window_fill = BG_PANEL;
        vis.panel_fill = BG_CONTENT;
        vis.extreme_bg_color = BG_BASE;
        vis.code_bg_color = BG_BASE;
        vis.override_text_color = Some(TEXT_PRIMARY);
        vis.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        vis.widgets.inactive.bg_fill = Color32::from_rgb(36, 36, 40);
        vis.widgets.hovered.bg_fill = BG_ROW_HOVER;
        vis.widgets.active.bg_fill = ACCENT_DIM;
        vis.selection.bg_fill = BG_ROW_SEL;
        vis.selection.stroke = egui::Stroke::new(1.0, ACCENT);
        vis.window_rounding = egui::Rounding::same(8.0);
        ctx.set_visuals(vis);

        let fonts = egui::FontDefinitions::default();
        ctx.set_fonts(fonts);

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
            self.apply_visuals(ctx);
            ctx.request_repaint();
        }

        self.poll_pending();
        self.state.queue_tasks = self.queue.all();
        self.state.connected_count = self
            .state
            .tabs
            .iter()
            .filter(|t| t.status == ConnectionStatus::Connected)
            .count();

        // ── Toolbar ──────────────────────────────────────────────────────────
        TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none().fill(BG_TOOLBAR))
            .show(ctx, |ui| {
                toolbar::render(ui, &mut self.state);
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
                sidebar::render(ui, &mut self.state);
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
                            // Хедер панели
                            pane_header(ui, "LOCAL", half);

                            local_pane::render(
                                ui,
                                &mut self.state,
                                &self.queue,
                                &self.registry,
                                self.rt.handle(),
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

                                // Хедер панели
                                let label = self.state.tabs[idx].params.host.clone();
                                pane_header(ui, &format!("REMOTE  ·  {}", label), half);

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
                                            ui.add_space(ui.available_height() * 0.3);
                                            ui.label(
                                                RichText::new("Remote")
                                                    .color(TEXT_HINT)
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
                                            let btn = egui::Button::new(
                                                RichText::new("+ New Connection")
                                                    .color(Color32::WHITE)
                                                    .size(12.0),
                                            )
                                            .fill(ACCENT)
                                            .rounding(6.0)
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

        // ── Connection dialog ────────────────────────────────────────────────
        if self.state.show_connect_dialog {
            connection::render(ctx, &mut self.state, &self.registry, self.rt.handle());
        }

        // ── Remote operation dialogs ─────────────────────────────────────────
        self.render_remote_op_dialogs(ctx);

        // 200ms repaint для обновления прогресса
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
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
                        .rounding(8.0)
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
                            let ok = egui::Button::new(
                                RichText::new("OK").color(Color32::WHITE).size(12.0),
                            )
                            .fill(ACCENT)
                            .rounding(4.0);
                            if ui.add(ok).clicked() {
                                clicked_ok = true;
                            }
                            ui.add_space(8.0);
                            let cancel = egui::Button::new(
                                RichText::new("Cancel").color(TEXT_DIM).size(12.0),
                            )
                            .fill(egui::Color32::TRANSPARENT);
                            if ui.add(cancel).clicked() {
                                self.state.show_mkdir_dialog = false;
                                self.state.mkdir_name.clear();
                            }
                        });
                    });
                });

            if clicked_ok && !self.state.mkdir_name.is_empty() {
                if let Some(idx) = self.active_tab_idx() {
                    let name = self.state.mkdir_name.clone();
                    self.start_mkdir(idx, name);
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
                        .rounding(8.0)
                        .inner_margin(egui::Margin::same(16.0)),
                )
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_width(260.0);
                    ui.label(
                        RichText::new(format!("Delete '{}' ?", name))
                            .color(TEXT_PRIMARY)
                            .strong(),
                    );
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ok = egui::Button::new(
                                RichText::new("OK").color(Color32::WHITE).size(12.0),
                            )
                            .fill(RED)
                            .rounding(4.0);
                            if ui.add(ok).clicked() {
                                clicked_ok = true;
                            }
                            ui.add_space(8.0);
                            let cancel = egui::Button::new(
                                RichText::new("Cancel").color(TEXT_DIM).size(12.0),
                            )
                            .fill(egui::Color32::TRANSPARENT);
                            if ui.add(cancel).clicked() {
                                self.state.show_delete_dialog = false;
                                self.state.delete_name.clear();
                            }
                        });
                    });
                });

            if clicked_ok {
                if let Some(idx) = self.active_tab_idx() {
                    self.start_delete(idx, name);
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
                        .rounding(8.0)
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
                            let ok = egui::Button::new(
                                RichText::new("OK").color(Color32::WHITE).size(12.0),
                            )
                            .fill(ACCENT)
                            .rounding(4.0);
                            if ui.add(ok).clicked() {
                                clicked_ok = true;
                            }
                            ui.add_space(8.0);
                            let cancel = egui::Button::new(
                                RichText::new("Cancel").color(TEXT_DIM).size(12.0),
                            )
                            .fill(egui::Color32::TRANSPARENT);
                            if ui.add(cancel).clicked() {
                                self.state.show_rename_dialog = false;
                                self.state.rename_new_name.clear();
                                self.state.rename_old_name.clear();
                            }
                        });
                    });
                });

            if clicked_ok && !self.state.rename_new_name.is_empty() {
                if let Some(idx) = self.active_tab_idx() {
                    let old_name = self.state.rename_old_name.clone();
                    let new_name = self.state.rename_new_name.clone();
                    self.start_rename(idx, old_name, new_name);
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
}

fn pane_header(ui: &mut egui::Ui, label: &str, width: f32) {
    egui::Frame::none()
        .fill(BG_BASE)
        .inner_margin(egui::Margin {
            left: 10.0,
            right: 6.0,
            top: 4.0,
            bottom: 4.0,
        })
        .show(ui, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).color(TEXT_DIM).size(10.0).strong());
            });
        });

    let (_, sep) = ui.allocate_space(egui::vec2(width, 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);
}

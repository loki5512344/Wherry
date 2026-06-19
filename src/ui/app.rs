use std::sync::Arc;

use egui::{Color32, CentralPanel, RichText, SidePanel, TopBottomPanel, Visuals};

use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::domain::site::Site;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::dialogs::connection;
use crate::ui::panels::{
    local_pane, queue, remote_pane, sidebar, status_bar, tabs, toolbar,
};
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
                        self.state.add_history(&params.host, params.port, &params.username);
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
    }

    fn active_tab_idx(&self) -> Option<usize> {
        if self.state.tabs.is_empty() { return None; }
        let idx = self.state.active_tab.min(self.state.tabs.len() - 1);
        if self.state.tabs[idx].status == ConnectionStatus::Connected {
            Some(idx)
        } else {
            None
        }
    }

    fn apply_visuals(&self, ctx: &egui::Context) {
        let mut vis = Visuals::dark();
        vis.window_fill    = BG_PANEL;
        vis.panel_fill     = BG_CONTENT;
        vis.extreme_bg_color = BG_BASE;
        vis.code_bg_color  = BG_BASE;
        vis.override_text_color = Some(TEXT_PRIMARY);
        vis.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        vis.widgets.inactive.bg_fill         = Color32::from_rgb(36, 36, 40);
        vis.widgets.hovered.bg_fill          = BG_ROW_HOVER;
        vis.widgets.active.bg_fill           = ACCENT_DIM;
        vis.selection.bg_fill                = BG_ROW_SEL;
        vis.selection.stroke                 = egui::Stroke::new(1.0, ACCENT);
        vis.window_rounding                  = egui::Rounding::same(8.0);
        ctx.set_visuals(vis);

        let fonts = egui::FontDefinitions::default();
        ctx.set_fonts(fonts);

        let mut style = ctx.style().as_ref().clone();
        style.spacing.item_spacing  = egui::vec2(4.0, 2.0);
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
            .frame(egui::Frame::none().fill(BG_QUEUE).stroke(egui::Stroke::new(1.0, BORDER)))
            .resizable(false)
            .exact_height(q_h)
            .show(ctx, |ui| {
                queue::render(ui, &mut self.state, &tasks);
            });

        // ── Sidebar ───────────────────────────────────────────────────────────
        SidePanel::left("sidebar")
            .frame(egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BORDER)))
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
                                let label = self.state.tabs[idx]
                                    .params
                                    .host
                                    .clone();
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
                                    egui::Layout::centered_and_justified(
                                        egui::Direction::TopDown,
                                    ),
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
                                                RichText::new("Connect to a server to browse files")
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

        // 200ms repaint для обновления прогресса
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

fn pane_header(ui: &mut egui::Ui, label: &str, width: f32) {
    egui::Frame::none()
        .fill(BG_BASE)
        .inner_margin(egui::Margin { left: 10.0, right: 6.0, top: 4.0, bottom: 4.0 })
        .show(ui, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(label)
                        .color(TEXT_DIM)
                        .size(10.0)
                        .strong(),
                );
            });
        });

    let (_, sep) = ui.allocate_space(egui::vec2(width, 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);
}

//! Основная рабочая область: тулбар, статус-бар, очередь, сайдбар и док-вкладки.
use egui::{CentralPanel, SidePanel, TopBottomPanel};

use super::FileManagerApp;
use crate::ui::dock::{AppTabViewer, PaneTab};
use crate::ui::panels::{queue, sidebar, status_bar, toolbar};
use crate::ui::theme::*;

impl FileManagerApp {
    pub(super) fn render_main_ui(&mut self, ctx: &egui::Context, fade: f32) {
        // Панели используют show_animated: тумблеры View → Toolbar/Sidebar/...
        // плавно въезжают/выезжают вместо мгновенного скачка раскладки.
        // ── Toolbar ──────────────────────────────────────────────────────────
        TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none().fill(BG_TOOLBAR))
            .show_animated(ctx, self.state.show_toolbar, |ui| {
                toolbar::render(ui, &mut self.state, &self.queue);
            });

        // ── Status bar ───────────────────────────────────────────────────────
        TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none().fill(BG_BASE))
            .exact_height(STATUS_H)
            .show_animated(ctx, self.state.show_status_bar, |ui| {
                status_bar::render(ui, &self.state);
            });

        // ── Queue panel ──────────────────────────────────────────────────────
        if self.state.show_queue_panel {
            let tasks = self.state.queue_tasks.clone();
            // Плавное раскрытие/сворачивание очереди по клику на заголовок.
            let expanded = self.state.show_queue && !tasks.is_empty();
            let t = ctx.animate_bool_with_time(egui::Id::new("queue_expanded"), expanded, 0.18);
            let q_h = egui::lerp(QUEUE_COLLAPSED_H..=QUEUE_EXPANDED_H, t);
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
        }

        // ── Sidebar ──────────────────────────────────────────────────────────
        SidePanel::left("sidebar")
            .frame(
                egui::Frame::none()
                    .fill(BG_PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER)),
            )
            .resizable(true)
            .default_width(SIDEBAR_W)
            .width_range(120.0..=260.0)
            .show_animated(ctx, self.state.show_sidebar, |ui| {
                sidebar::render(ui, &mut self.state, &self.db);
            });

        // ── Main content: модульная рабочая область (Local + подключения) ────
        // Каждое подключение — своя вкладка, которую можно перетаскивать,
        // разбивать на сплиты или закрывать независимо от остальных.
        CentralPanel::default()
            .frame(egui::Frame::none().fill(BG_CONTENT))
            .show(ctx, |ui| {
                ui.multiply_opacity(crate::ui::widgets::overlay::ease_out(fade));
                let mut closed_remote: Option<String> = None;
                let mut viewer = AppTabViewer {
                    state: &mut self.state,
                    queue: &self.queue,
                    registry: &self.registry,
                    rt_handle: self.rt.handle(),
                    db: &self.db,
                    closed_remote: &mut closed_remote,
                };
                let style = egui_dock::Style::from_egui(ui.style());
                egui_dock::DockArea::new(&mut self.dock_state)
                    .style(style)
                    .show_add_buttons(false)
                    .show_close_buttons(true)
                    .show_inside(ui, &mut viewer);

                if let Some((_rect, tab)) = self.dock_state.find_active_focused() {
                    match tab {
                        PaneTab::Local => self.state.active_pane = crate::ui::state::Pane::Local,
                        PaneTab::Remote(id) => {
                            if let Some(idx) = self.state.tabs.iter().position(|t| &t.id == id) {
                                self.state.active_tab = idx;
                                self.state.active_pane = crate::ui::state::Pane::Remote;
                            }
                        }
                    }
                }

                if let Some(id) = closed_remote {
                    self.registry.remove(&id);
                    self.state.tabs.retain(|t| t.id != id);
                    if self.state.active_tab >= self.state.tabs.len() {
                        self.state.active_tab = self.state.tabs.len().saturating_sub(1);
                    }
                }
            });
    }
}

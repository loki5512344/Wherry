use egui::{Align, Layout, RichText, Ui};
use crate::ui::state::AppState;
use crate::ui::theme::*;

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let frame = egui::Frame::none()
        .fill(BG_TOOLBAR)
        .inner_margin(egui::Margin::symmetric(8.0, 0.0));

    frame.show(ui, |ui| {
        ui.set_min_height(TOOLBAR_H);
        ui.horizontal_centered(|ui| {
            // Логотип
            ui.label(
                RichText::new("LoFlum")
                    .color(TEXT_PRIMARY)
                    .size(14.0)
                    .strong(),
            );

            ui.add_space(8.0);
            separator_v(ui);
            ui.add_space(8.0);

            // Новое подключение
            toolbar_btn(ui, "+ New Connection", false, || {
                state.show_connect_dialog = true;
            });

            separator_v(ui);

            // Кнопки работы с файлами
            let has_conn = state
                .active_tab_ref()
                .map(|t| t.status == crate::domain::connection::ConnectionStatus::Connected)
                .unwrap_or(false);

            let has_local_sel = state.local_selected.is_some();
            let has_remote_sel = state
                .active_tab_ref()
                .and_then(|t| t.remote_selected.as_ref())
                .is_some();

            toolbar_btn_enabled(ui, "⬆ Upload", has_conn && has_local_sel, || {
                state.status_message = "Upload: select file and use drag & drop".into();
            });
            toolbar_btn_enabled(ui, "⬇ Download", has_conn && has_remote_sel, || {
                state.status_message = "Download: select file and use drag & drop".into();
            });

            separator_v(ui);

            // Очередь — с счётчиком
            let q_count = state.queue_tasks.len();
            let q_label = if q_count > 0 {
                format!("⏳ Queue  {}", q_count)
            } else {
                "⏳ Queue".to_string()
            };
            let q_active = state.show_queue;
            toolbar_btn(ui, &q_label, q_active, || {
                state.show_queue = !state.show_queue;
            });

            separator_v(ui);

            toolbar_btn_enabled(ui, "⟳ Refresh", has_conn, || {
                // refresh remote — вызывается снаружи через state флаг
                state.pending_refresh = true;
            });
            toolbar_btn_enabled(ui, "📁 New Folder", has_conn, || {
                state.pending_mkdir = true;
            });
            toolbar_btn_enabled(ui, "× Delete", has_conn && has_remote_sel, || {
                state.pending_delete = true;
            });
            toolbar_btn_enabled(ui, "✎ Rename", has_conn && has_remote_sel, || {
                state.pending_rename = true;
            });

            // Правая часть — History / Bookmarks
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                toolbar_btn(ui, "⏱ History", state.show_history, || {
                    state.show_history = !state.show_history;
                    state.show_bookmarks = false;
                });
            });
        });
    });

    // История поверх
    if state.show_history {
        render_history_popup(ui, state);
    }
}

fn render_history_popup(ui: &mut Ui, state: &mut AppState) {
    let available = ui.clip_rect();
    let popup_rect = egui::Rect::from_min_size(
        egui::pos2(available.max.x - 280.0, TOOLBAR_H),
        egui::vec2(270.0, 200.0),
    );

    egui::Area::new(egui::Id::new("history_popup_area"))
        .fixed_pos(popup_rect.min)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(32, 32, 36))
                .stroke(egui::Stroke::new(1.0, BORDER))
                .rounding(6.0)
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.set_width(260.0);
                    ui.label(RichText::new("Recent Connections").color(TEXT_DIM).size(11.0));
                    ui.add_space(4.0);
                    if state.history.is_empty() {
                        ui.label(RichText::new("No history yet").color(TEXT_HINT));
                    } else {
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            for entry in &state.history.clone() {
                                let label = format!("{}@{}:{}", entry.user, entry.host, entry.port);
                                let time = RichText::new(&entry.time).color(TEXT_HINT).size(10.0);
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(&label).color(TEXT_PRIMARY).size(12.0),
                                        )
                                        .fill(egui::Color32::TRANSPARENT)
                                        .min_size(egui::vec2(240.0, 24.0)),
                                    )
                                    .clicked()
                                {
                                    state.connect_host = entry.host.clone();
                                    state.connect_user = entry.user.clone();
                                    state.connect_port = entry.port.to_string();
                                    state.show_connect_dialog = true;
                                    state.show_history = false;
                                }
                                ui.label(time);
                            }
                        });
                    }
                    ui.add_space(4.0);
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Close").color(TEXT_DIM).size(11.0))
                                .fill(egui::Color32::TRANSPARENT),
                        )
                        .clicked()
                    {
                        state.show_history = false;
                    }
                });
        });
}

fn toolbar_btn(ui: &mut Ui, label: &str, active: bool, mut on_click: impl FnMut()) {
    let fill = if active { BG_TAB_ACTIVE } else { egui::Color32::TRANSPARENT };
    let text_col = if active { ACCENT } else { TEXT_PRIMARY };
    let btn = egui::Button::new(RichText::new(label).color(text_col).size(12.0))
        .fill(fill)
        .rounding(4.0)
        .min_size(egui::vec2(0.0, 26.0));
    if ui.add(btn).clicked() {
        on_click();
    }
}

fn toolbar_btn_enabled(ui: &mut Ui, label: &str, enabled: bool, on_click: impl FnMut()) {
    ui.add_enabled_ui(enabled, |ui| {
        toolbar_btn(ui, label, false, on_click);
    });
}

fn separator_v(ui: &mut Ui) {
    let (_, rect) = ui.allocate_space(egui::vec2(1.0, 20.0));
    ui.painter().rect_filled(rect, 0.0, BORDER);
}

use crate::domain::connection::ConnectionStatus;
use crate::domain::transfer::TaskState;
use crate::i18n::t;
use crate::transfer::queue::TransferQueue;
use crate::ui::icons::{self, Icon};
use crate::ui::state::{AppState, Pane};
use crate::ui::theme::*;
use crate::ui::widgets::{button, overlay};
use egui::{Align, Layout, RichText, Ui};

mod actions;
mod buttons;
mod history;

use buttons::{icon_btn, icon_toggle_btn, separator_v, text_btn, text_btn_enabled};

pub fn render(ui: &mut Ui, state: &mut AppState, queue: &TransferQueue) {
    let frame = egui::Frame::none()
        .fill(BG_TOOLBAR)
        .inner_margin(egui::Margin::symmetric(8.0, 0.0));

    frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, TOOLBAR_H),
            Layout::left_to_right(Align::Center),
            |ui| {
                // Логотип
                icons::icon(ui, Icon::Folder, 17.0, ACCENT);
                ui.add_space(4.0);
                ui.label(
                    RichText::new("LoFlum")
                        .color(TEXT_PRIMARY)
                        .size(14.0)
                        .strong(),
                );

                ui.add_space(8.0);
                separator_v(ui);
                ui.add_space(8.0);

                // Новое подключение — единственная акцентная кнопка в тулбаре
                let connect_btn = egui::Button::image_and_text(
                    icons::image(Icon::AddCircleLinear, 15.0, TEXT_PRIMARY),
                    RichText::new(t("toolbar.new_connection"))
                        .color(TEXT_PRIMARY)
                        .size(12.5),
                )
                .rounding(RADIUS_MD)
                .min_size(egui::vec2(0.0, 30.0));
                if button::accent_dim(ui, connect_btn).clicked() {
                    state.show_connect_dialog = true;
                }

                separator_v(ui);

                // Кнопки работы с файлами
                let has_conn = state
                    .active_tab_ref()
                    .map(|t| t.status == ConnectionStatus::Connected)
                    .unwrap_or(false);

                let has_local_sel = state.local_selected.is_some();
                let has_remote_sel = state
                    .active_tab_ref()
                    .and_then(|t| t.remote_selected.as_ref())
                    .is_some();

                text_btn_enabled(
                    ui,
                    Icon::Upload,
                    t("common.upload"),
                    has_conn && has_local_sel,
                    || {
                        actions::queue_selected_upload(state, queue);
                    },
                );
                text_btn_enabled(
                    ui,
                    Icon::Download,
                    t("common.download"),
                    has_conn && has_remote_sel,
                    || {
                        actions::queue_selected_download(state, queue);
                    },
                );

                separator_v(ui);

                // Очередь — счётчик активных (в согласии с заголовком панели очереди)
                let q_pending = state
                    .queue_tasks
                    .iter()
                    .filter(|t| matches!(t.state, TaskState::Running | TaskState::Queued))
                    .count();
                let q_label = if q_pending > 0 {
                    format!("{}  {}", t("toolbar.queue_label"), q_pending)
                } else {
                    t("toolbar.queue_label").to_string()
                };
                let q_active = state.show_queue;
                text_btn(ui, Icon::Clock, &q_label, q_active, || {
                    state.show_queue = !state.show_queue;
                });

                separator_v(ui);

                let on_remote = state.active_pane == Pane::Remote;
                let can_new_folder = if on_remote { has_conn } else { true };
                let can_delete_rename = if on_remote {
                    has_conn && has_remote_sel
                } else {
                    has_local_sel
                };

                icon_btn(ui, Icon::Refresh, has_conn, t("common.refresh"), || {
                    // refresh remote — вызывается снаружи через state флаг
                    state.pending_refresh = true;
                });
                icon_btn(
                    ui,
                    Icon::FolderWithFiles,
                    can_new_folder,
                    t("common.new_folder"),
                    || {
                        state.op_target = state.active_pane;
                        state.show_mkdir_dialog = true;
                        state.mkdir_name.clear();
                    },
                );
                icon_btn(ui, Icon::Trash, can_delete_rename, t("common.delete"), || {
                    let name = selected_name(state, on_remote);
                    if let Some(name) = name {
                        state.op_target = state.active_pane;
                        state.show_delete_dialog = true;
                        state.delete_name = name;
                    }
                });
                icon_btn(ui, Icon::Pen, can_delete_rename, t("common.rename"), || {
                    let name = selected_name(state, on_remote);
                    if let Some(name) = name {
                        state.op_target = state.active_pane;
                        state.show_rename_dialog = true;
                        state.rename_old_name = name.clone();
                        state.rename_new_name = name;
                    }
                });

                // Правая часть — History / Settings
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // На macOS кнопка настроек живёт в нативном верхнем меню
                    // (LoFlum → Settings…), здесь она не нужна.
                    #[cfg(not(target_os = "macos"))]
                    icon_btn(ui, Icon::Settings, true, t("common.settings"), || {
                        state.show_settings_dialog = true;
                    });
                    icon_toggle_btn(
                        ui,
                        Icon::History,
                        state.show_history,
                        t("common.history"),
                        || {
                            state.show_history = !state.show_history;
                            state.show_bookmarks = false;
                        },
                    );
                });
            },
        );
    });

    // «Открытость» тикает каждый кадр (в т.ч. пока скрыто), иначе первое
    // открытие происходит без анимации — см. overlay::openness.
    let t_history = overlay::openness(ui.ctx(), "history_popup", state.show_history);
    if t_history > 0.0 {
        history::render_history_popup(ui, state, t_history);
    }
}

/// Имя выбранного элемента в активной панели (remote/local).
fn selected_name(state: &AppState, on_remote: bool) -> Option<String> {
    if on_remote {
        state
            .active_tab_ref()
            .and_then(|t| t.remote_selected.clone())
    } else {
        state.local_selected.clone()
    }
}

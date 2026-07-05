//! Верхняя (путь + статус) и нижняя (счётчик + refresh) полоски удалённого пейна.
use egui::{RichText, Ui};
use std::sync::Arc;

use super::trigger_list;
use crate::domain::connection::ConnectionStatus;
use crate::fs::remote::RemoteRegistry;
use crate::ui::icons::{self, Icon};
use crate::ui::panels::file_pane::format_size;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use crate::ui::widgets::button;

pub(super) fn render_path_bar(
    ui: &mut Ui,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    let frame = egui::Frame::none()
        .fill(BG_BASE)
        .inner_margin(egui::Margin::symmetric(12.0, 0.0));

    frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, 30.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let host = state.tabs[tab_idx].params.host.clone();
                ui.label(
                    RichText::new(crate::i18n::tf("panels.remote_label", &[("{host}", &host)]))
                        .color(TEXT_DIM)
                        .size(10.5)
                        .strong(),
                );
                ui.add_space(10.0);

                // статус точка
                let (dot, dot_col) = match state.tabs[tab_idx].status {
                    ConnectionStatus::Connected => ("●", GREEN),
                    ConnectionStatus::Disconnected => ("○", TEXT_HINT),
                    ConnectionStatus::Connecting => ("◐", YELLOW),
                    ConnectionStatus::Error(_) => ("×", RED),
                };
                ui.label(RichText::new(dot).color(dot_col).size(10.0));
                ui.add_space(4.0);

                breadcrumbs(ui, state, tab_idx, registry, rt_handle);

                // Индикатор загрузки — справа, не сдвигает таблицу под баром
                if state.tabs[tab_idx].loading {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(crate::i18n::t("panels.loading"))
                                .color(TEXT_HINT)
                                .size(10.5),
                        );
                        ui.add_space(4.0);
                        ui.spinner();
                    });
                }
            },
        );
    });
}

/// Хлебные крошки: root + сегменты пути. Каждый сегмент — ghost-кнопка с
/// подсветкой и курсором-рукой, чтобы кликабельность читалась сразу.
fn breadcrumbs(
    ui: &mut Ui,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    let path_clone = state.tabs[tab_idx].remote_path.clone();
    let parts: Vec<&str> = path_clone.split('/').filter(|s| !s.is_empty()).collect();
    let mut acc = String::new();

    let root = egui::Button::new(RichText::new("/").color(TEXT_DIM).size(12.0))
        .min_size(egui::vec2(12.0, 22.0));
    if button::ghost(ui, root).clicked() {
        navigate(state, tab_idx, "/".into(), registry, rt_handle);
    }

    for (i, part) in parts.iter().enumerate() {
        acc.push('/');
        acc.push_str(part);
        let is_last = i == parts.len() - 1;

        if is_last {
            ui.label(RichText::new(*part).color(TEXT_PRIMARY).size(12.0).strong());
        } else {
            let path_snap = acc.clone();
            let seg = egui::Button::new(RichText::new(*part).color(TEXT_DIM).size(12.0))
                .min_size(egui::vec2(0.0, 22.0));
            if button::ghost(ui, seg).clicked() {
                navigate(state, tab_idx, path_snap, registry, rt_handle);
            }
            ui.label(RichText::new("/").color(TEXT_HINT).size(12.0));
        }
    }
}

fn navigate(
    state: &mut AppState,
    tab_idx: usize,
    path: String,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    state.tabs[tab_idx].remote_path = path;
    if !state.tabs[tab_idx].loading {
        trigger_list(state, tab_idx, registry, rt_handle);
    }
}

pub(super) fn render_footer(
    ui: &mut Ui,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);

    let frame = egui::Frame::none()
        .fill(BG_PANEL)
        .inner_margin(egui::Margin::symmetric(8.0, 3.0));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let tab = &state.tabs[tab_idx];
            let count = tab.remote_entries.iter().filter(|e| e.name != "..").count();
            let total_size: u64 = tab.remote_entries.iter().filter_map(|e| e.size).sum();

            let label = if tab.loading {
                crate::i18n::t("panels.loading").to_string()
            } else {
                crate::i18n::tf("panels.count_items", &[("{n}", &count.to_string())])
            };

            ui.label(RichText::new(label).color(TEXT_DIM).size(11.0));

            if total_size > 0 {
                ui.label(
                    RichText::new(format!("({})", format_size(Some(total_size))))
                        .color(TEXT_HINT)
                        .size(11.0),
                );
            }

            // Refresh — тот же серый, что и в тулбаре (единый стиль)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let refresh = egui::Button::image(icons::image(Icon::Refresh, 13.0, TEXT_DIM))
                    .min_size(egui::vec2(22.0, 22.0));
                if button::ghost(ui, refresh)
                    .on_hover_text(crate::i18n::t("common.refresh"))
                    .clicked()
                    && !state.tabs[tab_idx].loading
                {
                    trigger_list(state, tab_idx, registry, rt_handle);
                }
            });
        });
    });
}

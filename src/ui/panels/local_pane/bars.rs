//! Верхняя (путь + home) и нижняя (счётчик + закладка) полоски локального пейна.
use egui::{RichText, Ui};
use std::sync::{Arc, Mutex};

use super::actions::add_bookmark;
use super::refresh_local;
use crate::ui::icons::{self, Icon};
use crate::ui::panels::file_pane::format_size;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use crate::ui::widgets::button;

pub(super) fn render_path_bar(ui: &mut Ui, state: &mut AppState) {
    let frame = egui::Frame::none()
        .fill(BG_BASE)
        .inner_margin(egui::Margin::symmetric(12.0, 0.0));

    frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, 30.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(
                    RichText::new(crate::i18n::t("panels.local_label"))
                        .color(TEXT_DIM)
                        .size(10.5)
                        .strong(),
                );
                ui.add_space(10.0);

                // иконка home
                let home = dirs::home_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let is_home = state.local_path == home;
                let home_col = if is_home { ACCENT } else { TEXT_DIM };
                let home_btn = egui::Button::image(icons::image(Icon::Home, 13.0, home_col))
                    .rounding(RADIUS_SM)
                    .min_size(egui::vec2(22.0, 22.0));
                if button::toggle(ui, home_btn, is_home).clicked() {
                    state.local_path = home;
                    refresh_local(state);
                }

                ui.add_space(4.0);

                breadcrumbs(ui, state);
            },
        );
    });
}

/// Хлебные крошки локального пути — каждый сегмент ghost-кнопка (hover + курсор-рука).
fn breadcrumbs(ui: &mut Ui, state: &mut AppState) {
    let path_clone = state.local_path.clone();
    let parts: Vec<&str> = path_clone.split('/').filter(|s| !s.is_empty()).collect();
    let mut acc = String::new();

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
                state.local_path = path_snap;
                refresh_local(state);
            }
            ui.label(RichText::new("/").color(TEXT_HINT).size(12.0));
        }
    }
}

pub(super) fn render_footer(
    ui: &mut Ui,
    state: &mut AppState,
    db: &Arc<Mutex<rusqlite::Connection>>,
) {
    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);

    let frame = egui::Frame::none()
        .fill(BG_PANEL)
        .inner_margin(egui::Margin::symmetric(8.0, 3.0));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let count = state
                .local_entries
                .iter()
                .filter(|e| e.name != "..")
                .count();
            let total_size: u64 = state.local_entries.iter().filter_map(|e| e.size).sum();

            ui.label(
                RichText::new(crate::i18n::tf(
                    "panels.count_items",
                    &[("{n}", &count.to_string())],
                ))
                .color(TEXT_DIM)
                .size(11.0),
            );

            if total_size > 0 {
                ui.label(
                    RichText::new(format!("({})", format_size(Some(total_size))))
                        .color(TEXT_HINT)
                        .size(11.0),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // кнопка добавить закладку
                let star = egui::Button::image(icons::image(Icon::Star, 12.0, YELLOW))
                    .min_size(egui::vec2(22.0, 22.0));
                if button::ghost(ui, star)
                    .on_hover_text(crate::i18n::t("panels.sidebar.bookmark_this_folder"))
                    .clicked()
                {
                    let name = std::path::Path::new(&state.local_path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Bookmark".into());
                    let path = state.local_path.clone();
                    add_bookmark(state, db, &name, &path);
                }
            });
        });
    });
}

//! Отрисовка шапки колонок и содержимого одной строки таблицы файлов.
use egui::{Id, RichText, Ui};

use super::sort::{sort_header, toggle_sort};
use super::{SortCol, SortDir, format_size, format_time};
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::ui::drag::DragPayload;
use crate::ui::icons::{self, Icon, file_icon_for};
use crate::ui::theme::*;

/// Шапка с кликабельными заголовками колонок.
pub(super) fn render_header(
    ui: &mut Ui,
    sort_col: SortCol,
    sort_dir: SortDir,
    new_sort: &mut Option<(SortCol, SortDir)>,
) {
    let header_frame = egui::Frame::none()
        .fill(BG_HEADER)
        .inner_margin(egui::Margin::symmetric(12.0, 0.0));

    header_frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, 24.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                use crate::i18n::t;
                if sort_header(ui, t("panels.col_name"), 0.0, sort_col == SortCol::Name, sort_dir)
                {
                    *new_sort = Some(toggle_sort(SortCol::Name, sort_col, sort_dir));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if sort_header(
                        ui,
                        t("panels.col_modified"),
                        140.0,
                        sort_col == SortCol::Modified,
                        sort_dir,
                    ) {
                        *new_sort = Some(toggle_sort(SortCol::Modified, sort_col, sort_dir));
                    }
                    if sort_header(ui, t("panels.col_type"), 40.0, sort_col == SortCol::Kind, sort_dir) {
                        *new_sort = Some(toggle_sort(SortCol::Kind, sort_col, sort_dir));
                    }
                    if sort_header(ui, t("panels.col_size"), 70.0, sort_col == SortCol::Size, sort_dir) {
                        *new_sort = Some(toggle_sort(SortCol::Size, sort_col, sort_dir));
                    }
                });
            },
        );
    });
}

/// Содержимое одной строки: иконка+имя (drag source) слева, метаданные справа.
pub(super) fn draw_row_content<F>(ui: &mut Ui, entry: &FileEntry, row_id: Id, make_drag_payload: &F)
where
    F: Fn(&FileEntry) -> Option<DragPayload>,
{
    let (icon, icon_col) = if entry.name == ".." {
        (Icon::ArrowUp, TEXT_HINT)
    } else {
        match entry.kind {
            EntryKind::Dir => (Icon::Folder, YELLOW),
            EntryKind::Symlink => (Icon::Link, YELLOW),
            EntryKind::File => {
                let ext = entry.name.rsplit('.').next().unwrap_or("").to_lowercase();
                file_icon_for(&ext)
            }
        }
    };
    let name_col = if entry.name == ".." {
        TEXT_HINT
    } else {
        TEXT_PRIMARY
    };

    let draw = |ui: &mut Ui| {
        icons::icon(ui, icon, 14.0, icon_col);
        ui.add_space(6.0);
        ui.label(RichText::new(&entry.name).color(name_col).size(12.5));
    };

    if let Some(payload) = make_drag_payload(entry) {
        ui.dnd_drag_source(row_id, payload, |ui| {
            ui.horizontal(draw);
        });
    } else {
        draw(ui);
    }

    // правые колонки
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(
            RichText::new(format_time(entry.modified))
                .color(TEXT_HINT)
                .monospace()
                .size(10.5),
        );
        ui.add_space(6.0);

        let (type_str, type_col) = match entry.kind {
            EntryKind::Dir => (crate::i18n::t("panels.type_dir"), TEXT_DIM),
            EntryKind::File => (crate::i18n::t("panels.type_file"), TEXT_HINT),
            EntryKind::Symlink => (crate::i18n::t("panels.type_link"), YELLOW),
        };
        let type_label =
            egui::Label::new(RichText::new(type_str).color(type_col).size(11.0)).truncate();
        ui.add_sized([38.0, ROW_H], type_label);

        ui.label(
            RichText::new(format_size(entry.size))
                .color(TEXT_DIM)
                .monospace()
                .size(11.0),
        );
        ui.add_space(4.0);
    });
}

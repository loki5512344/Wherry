//! Рендер строк таблицы файлов (шапка, список, drag&drop, контекстное меню).
use egui::{Color32, Id, Ui};

use super::row::{draw_row_content, render_header};
use super::sort::sort_entries;
use super::{FileTableResponse, RowAction, SortCol, SortDir};
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::ui::drag::DragPayload;
use crate::ui::theme::*;

pub(super) fn file_table_inner<F, C>(
    ui: &mut Ui,
    id: &str,
    entries: &[FileEntry],
    selected: &mut Option<String>,
    make_drag_payload: F,
    mut context_menu: C,
) -> FileTableResponse
where
    F: Fn(&FileEntry) -> Option<DragPayload>,
    C: FnMut(&mut Ui, &FileEntry) -> Option<RowAction>,
{
    // Сортировка — читаем из egui memory
    let sort_id = Id::new((id, "sort"));
    let (sort_col, sort_dir) = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or(sort_id, (SortCol::Name, SortDir::Asc)));

    let sorted = sort_entries(entries, sort_col, sort_dir);

    let mut double_clicked: Option<String> = None;
    let mut clicked: Option<String> = None;
    let mut new_sort: Option<(SortCol, SortDir)> = None;
    let mut dropped_on_dir: Option<(FileEntry, std::sync::Arc<DragPayload>)> = None;
    let mut context_action: Option<(FileEntry, RowAction)> = None;

    render_header(ui, sort_col, sort_dir, &mut new_sort);

    if let Some(s) = new_sort {
        ui.ctx().data_mut(|d| {
            d.insert_temp(sort_id, s);
        });
    }

    // разделитель
    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);

    // Список файлов
    egui::ScrollArea::vertical()
        .id_salt((id, "scroll"))
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for entry in &sorted {
                let is_selected = selected.as_deref() == Some(&entry.name);
                let row_id = Id::new((id, "row", &entry.name));

                // Строка, которую сейчас тащат — её содержимое рисуется в
                // плавающем слое поверх курсора (см. dnd_drag_source), поэтому
                // сама она не может быть целью для дропа, а на исходном месте
                // не должно оставаться подсветки выбора/ховера — там пусто.
                let is_being_dragged = ui.ctx().is_being_dragged(row_id);

                let row_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), ROW_H),
                );
                let is_hovered = !is_being_dragged && ui.rect_contains_pointer(row_rect);
                let is_drop_target = !is_being_dragged
                    && entry.kind == EntryKind::Dir
                    && egui::DragAndDrop::has_payload_of_type::<DragPayload>(ui.ctx());
                let is_drop_hover = is_drop_target && is_hovered;

                let bg = if is_being_dragged {
                    Color32::TRANSPARENT
                } else if is_drop_hover {
                    ACCENT_DIM
                } else if is_selected {
                    BG_ROW_SEL
                } else if is_hovered {
                    BG_ROW_HOVER
                } else {
                    Color32::TRANSPARENT
                };
                let stroke = if is_drop_hover {
                    egui::Stroke::new(1.5, ACCENT)
                } else {
                    egui::Stroke::NONE
                };

                let row_width = ui.available_width();
                let response = egui::Frame::none()
                    .fill(bg)
                    .stroke(stroke)
                    .rounding(RADIUS_SM)
                    .inner_margin(egui::Margin::symmetric(12.0, 0.0))
                    .show(ui, |ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(row_width - 24.0, ROW_H),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| draw_row_content(ui, entry, row_id, &make_drag_payload),
                        );
                    })
                    .response;

                // Frame::show() only senses hover, not clicks — add a real click
                // sense on the full row rect (separate id from the drag source,
                // which only senses drag over the icon+name sub-area). Without
                // this, every press was interpreted as a drag start and plain
                // clicks/double-clicks never fired.
                let click_response =
                    ui.interact(response.rect, row_id.with("click"), egui::Sense::click());
                let response = response | click_response;

                if response.clicked() {
                    *selected = Some(entry.name.clone());
                    clicked = Some(entry.name.clone());
                }
                if response.double_clicked() {
                    double_clicked = Some(entry.name.clone());
                }
                if is_drop_target
                    && let Some(payload) = response.dnd_release_payload::<DragPayload>()
                {
                    dropped_on_dir = Some((entry.clone(), payload));
                }

                if entry.name != ".." {
                    // Правый клик выбирает строку — иначе меню открывается
                    // над предыдущим выделением, что сбивает с толку.
                    if response.secondary_clicked() {
                        *selected = Some(entry.name.clone());
                    }
                    crate::ui::widgets::context_menu::context_menu(&response, |ui| {
                        if let Some(action) = context_menu(ui, entry) {
                            context_action = Some((entry.clone(), action));
                            ui.close_menu();
                        }
                    });
                }
            }
        });

    FileTableResponse {
        double_clicked,
        clicked,
        dropped_on_dir,
        context_action,
    }
}

//! Переиспользуемая таблица файлов с нормальными колонками
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::ui::drag::DragPayload;
use crate::ui::icons::{self, Icon, file_icon_for};
use crate::ui::theme::*;
use egui::{Color32, Id, RichText, Ui};

pub fn format_size(bytes: Option<u64>) -> String {
    match bytes {
        None => String::new(),
        Some(b) if b < 1024 => format!("{} B", b),
        Some(b) if b < 1024 * 1024 => format!("{:.1} KB", b as f64 / 1024.0),
        Some(b) if b < 1024 * 1024 * 1024 => format!("{:.1} MB", b as f64 / (1024.0 * 1024.0)),
        Some(b) => format!("{:.2} GB", b as f64 / (1024.0 * 1024.0 * 1024.0)),
    }
}

pub fn format_time(ts: Option<i64>) -> String {
    match ts.and_then(|t| chrono::DateTime::from_timestamp(t, 0)) {
        Some(dt) => dt.format("%d.%m.%Y %H:%M").to_string(),
        None => String::new(),
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SortCol {
    Name,
    Size,
    Kind,
    Modified,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SortDir {
    Asc,
    Desc,
}

/// Действие, запрошенное через контекстное меню (ПКМ) на строке.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RowAction {
    Open,
    Rename,
    Delete,
    Bookmark,
    CopyPath,
    /// Загрузка/выгрузка этой конкретной строки — направление решает вызывающая сторона.
    Transfer,
}

pub struct FileTableResponse {
    pub double_clicked: Option<String>,
    pub clicked: Option<String>,
    /// Payload, перетащенный на папку (или "..") в этой таблице — (целевая папка, что бросили).
    pub dropped_on_dir: Option<(FileEntry, std::sync::Arc<DragPayload>)>,
    /// Действие из контекстного меню (ПКМ) — (строка, действие).
    pub context_action: Option<(FileEntry, RowAction)>,
}

pub fn file_table<F, C>(
    ui: &mut Ui,
    id: &str,
    entries: &[FileEntry],
    selected: &mut Option<String>,
    make_drag_payload: F,
    context_menu: C,
) -> FileTableResponse
where
    F: Fn(&FileEntry) -> Option<DragPayload>,
    C: FnMut(&mut Ui, &FileEntry) -> Option<RowAction>,
{
    ui.push_id(id, |ui| {
        file_table_inner(ui, id, entries, selected, make_drag_payload, context_menu)
    })
    .inner
}

fn file_table_inner<F, C>(
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

    let mut sorted = entries.to_vec();
    let dotdot_entry = sorted.iter().position(|e| e.name == "..");
    let dotdot = dotdot_entry.map(|i| sorted.remove(i));

    sorted.sort_by(|a, b| {
        // папки всегда выше
        let dir_cmp = match (&b.kind, &a.kind) {
            (EntryKind::Dir, EntryKind::Dir) => std::cmp::Ordering::Equal,
            (EntryKind::Dir, _) => std::cmp::Ordering::Greater,
            (_, EntryKind::Dir) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        };
        if dir_cmp != std::cmp::Ordering::Equal {
            return dir_cmp;
        }

        let ord = match sort_col {
            SortCol::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortCol::Size => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
            SortCol::Kind => format!("{:?}", a.kind).cmp(&format!("{:?}", b.kind)),
            SortCol::Modified => a.modified.unwrap_or(0).cmp(&b.modified.unwrap_or(0)),
        };
        if sort_dir == SortDir::Desc {
            ord.reverse()
        } else {
            ord
        }
    });

    if let Some(dd) = dotdot {
        sorted.insert(0, dd);
    }

    let mut double_clicked: Option<String> = None;
    let mut clicked: Option<String> = None;
    let mut new_sort: Option<(SortCol, SortDir)> = None;
    let mut dropped_on_dir: Option<(FileEntry, std::sync::Arc<DragPayload>)> = None;
    let mut context_action: Option<(FileEntry, RowAction)> = None;

    // Заголовки колонок
    let header_frame = egui::Frame::none()
        .fill(BG_HEADER)
        .inner_margin(egui::Margin::symmetric(12.0, 0.0));

    header_frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, 24.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let name_r = sort_header(ui, "Name", 0.0, sort_col == SortCol::Name, sort_dir);
                if name_r {
                    new_sort = Some(toggle_sort(SortCol::Name, sort_col, sort_dir));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mod_r = sort_header(
                        ui,
                        "Modified",
                        140.0,
                        sort_col == SortCol::Modified,
                        sort_dir,
                    );
                    if mod_r {
                        new_sort = Some(toggle_sort(SortCol::Modified, sort_col, sort_dir));
                    }

                    let type_r = sort_header(ui, "Type", 40.0, sort_col == SortCol::Kind, sort_dir);
                    if type_r {
                        new_sort = Some(toggle_sort(SortCol::Kind, sort_col, sort_dir));
                    }

                    let size_r = sort_header(ui, "Size", 70.0, sort_col == SortCol::Size, sort_dir);
                    if size_r {
                        new_sort = Some(toggle_sort(SortCol::Size, sort_col, sort_dir));
                    }
                });
            },
        );
    });

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
                            |ui| {
                                // иконка (цвет по типу) + имя (единый цвет, как в спеке)
                                let (icon, icon_col) = if entry.name == ".." {
                                    (Icon::ArrowUp, TEXT_HINT)
                                } else {
                                    match entry.kind {
                                        EntryKind::Dir => (Icon::Folder, YELLOW),
                                        EntryKind::Symlink => (Icon::Link, YELLOW),
                                        EntryKind::File => {
                                            let ext = entry
                                                .name
                                                .rsplit('.')
                                                .next()
                                                .unwrap_or("")
                                                .to_lowercase();
                                            file_icon_for(&ext)
                                        }
                                    }
                                };
                                let name_col = if entry.name == ".." {
                                    TEXT_HINT
                                } else {
                                    TEXT_PRIMARY
                                };

                                let draw_row = |ui: &mut Ui| {
                                    icons::icon(ui, icon, 14.0, icon_col);
                                    ui.add_space(6.0);
                                    ui.label(RichText::new(&entry.name).color(name_col).size(12.5));
                                };

                                if let Some(payload) = make_drag_payload(entry) {
                                    ui.dnd_drag_source(row_id, payload, |ui| {
                                        ui.horizontal(draw_row);
                                    });
                                } else {
                                    draw_row(ui);
                                }

                                // правые колонки
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format_time(entry.modified))
                                                .color(TEXT_HINT)
                                                .monospace()
                                                .size(10.5),
                                        );
                                        ui.add_space(6.0);

                                        let (type_str, type_col) = match entry.kind {
                                            EntryKind::Dir => ("Dir", TEXT_DIM),
                                            EntryKind::File => ("File", TEXT_HINT),
                                            EntryKind::Symlink => ("Link", YELLOW),
                                        };
                                        let type_label = egui::Label::new(
                                            RichText::new(type_str).color(type_col).size(11.0),
                                        )
                                        .truncate();
                                        ui.add_sized([38.0, ROW_H], type_label);

                                        ui.label(
                                            RichText::new(format_size(entry.size))
                                                .color(TEXT_DIM)
                                                .monospace()
                                                .size(11.0),
                                        );
                                        ui.add_space(4.0);
                                    },
                                );
                            },
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
                    response.context_menu(|ui| {
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

/// Единообразная кнопка-пункт для контекстных меню (ПКМ на строке).
pub fn context_menu_item(ui: &mut Ui, icon: Icon, label: &str, danger: bool) -> bool {
    let color = if danger { RED } else { TEXT_PRIMARY };
    let btn = egui::Button::image_and_text(
        icons::image(icon, 13.0, color),
        RichText::new(label).size(12.0).color(color),
    )
    .fill(Color32::TRANSPARENT)
    .min_size(egui::vec2(170.0, 26.0));
    ui.add(btn).clicked()
}

fn sort_header(ui: &mut Ui, label: &str, min_w: f32, active: bool, dir: SortDir) -> bool {
    let arrow = if active {
        if dir == SortDir::Asc { " ↑" } else { " ↓" }
    } else {
        ""
    };
    let full = format!("{}{}", label, arrow);
    let col = if active { ACCENT } else { TEXT_DIM };
    let btn = egui::Button::new(RichText::new(full).color(col).size(10.5).strong())
        .fill(Color32::TRANSPARENT)
        .min_size(egui::vec2(min_w, 20.0));
    ui.add(btn).clicked()
}

fn toggle_sort(col: SortCol, cur_col: SortCol, cur_dir: SortDir) -> (SortCol, SortDir) {
    if col == cur_col {
        let new_dir = if cur_dir == SortDir::Asc {
            SortDir::Desc
        } else {
            SortDir::Asc
        };
        (col, new_dir)
    } else {
        (col, SortDir::Asc)
    }
}

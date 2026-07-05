//! Переиспользуемая таблица файлов: публичный API, типы и форматтеры.
//! Рендер строк — в [`table`], сортировка — в [`sort`].
use crate::domain::file_entry::FileEntry;
use crate::ui::drag::DragPayload;
use crate::ui::icons::{self, Icon};
use crate::ui::theme::*;
use crate::ui::widgets::button;
use egui::{RichText, Ui};

mod row;
mod sort;
mod table;

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
        table::file_table_inner(ui, id, entries, selected, make_drag_payload, context_menu)
    })
    .inner
}

/// Единообразный пункт для контекстных меню (ПКМ на строке) с подсветкой на hover.
pub fn context_menu_item(ui: &mut Ui, icon: Icon, label: &str, danger: bool) -> bool {
    let color = if danger { RED } else { TEXT_PRIMARY };
    let btn = egui::Button::image_and_text(
        icons::image(icon, 13.0, color),
        RichText::new(label).size(12.0).color(color),
    )
    .min_size(egui::vec2(170.0, 26.0));
    button::ghost(ui, btn).clicked()
}

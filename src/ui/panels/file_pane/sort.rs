//! Сортировка таблицы файлов: заголовки-переключатели и сам компаратор.
use egui::{RichText, Ui};

use super::{SortCol, SortDir};
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::ui::theme::*;
use crate::ui::widgets::button;

/// Сортирует записи по колонке/направлению, держа папки выше файлов, а ".." — первым.
pub(super) fn sort_entries(
    entries: &[FileEntry],
    sort_col: SortCol,
    sort_dir: SortDir,
) -> Vec<FileEntry> {
    let mut sorted = entries.to_vec();
    let dotdot_pos = sorted.iter().position(|e| e.name == "..");
    let dotdot = dotdot_pos.map(|i| sorted.remove(i));

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
    sorted
}

/// Кликабельный заголовок колонки со стрелкой активной сортировки.
pub(super) fn sort_header(
    ui: &mut Ui,
    label: &str,
    min_w: f32,
    active: bool,
    dir: SortDir,
) -> bool {
    let arrow = if active {
        if dir == SortDir::Asc { " ↑" } else { " ↓" }
    } else {
        ""
    };
    let full = format!("{}{}", label, arrow);
    let col = if active { ACCENT } else { TEXT_DIM };
    let btn = egui::Button::new(RichText::new(full).color(col).size(10.5).strong())
        .min_size(egui::vec2(min_w, 20.0));
    button::ghost(ui, btn).clicked()
}

pub(super) fn toggle_sort(col: SortCol, cur_col: SortCol, cur_dir: SortDir) -> (SortCol, SortDir) {
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

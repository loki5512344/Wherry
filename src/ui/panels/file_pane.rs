//! Переиспользуемая таблица файлов с нормальными колонками
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::ui::drag::DragPayload;
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

pub struct FileTableResponse {
    pub double_clicked: Option<String>,
    pub clicked: Option<String>,
}

pub fn file_table<F>(
    ui: &mut Ui,
    id: &str,
    entries: &[FileEntry],
    selected: &mut Option<String>,
    make_drag_payload: F,
) -> FileTableResponse
where
    F: Fn(&FileEntry) -> Option<DragPayload>,
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

    // Заголовки колонок
    let header_frame = egui::Frame::none()
        .fill(BG_HEADER)
        .inner_margin(egui::Margin {
            left: 6.0,
            right: 6.0,
            top: 0.0,
            bottom: 0.0,
        });

    header_frame.show(ui, |ui| {
        ui.set_min_height(22.0);
        ui.horizontal(|ui| {
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
        });
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
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for entry in &sorted {
                let is_selected = selected.as_deref() == Some(&entry.name);
                let row_id = Id::new((id, "row", &entry.name));

                let row_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), ROW_H),
                );
                let is_hovered = ui.rect_contains_pointer(row_rect);

                let bg = if is_selected {
                    BG_ROW_SEL
                } else if is_hovered {
                    BG_ROW_HOVER
                } else {
                    Color32::TRANSPARENT
                };

                let response = egui::Frame::none()
                    .fill(bg)
                    .inner_margin(egui::Margin {
                        left: 6.0,
                        right: 6.0,
                        top: 0.0,
                        bottom: 0.0,
                    })
                    .show(ui, |ui| {
                        ui.set_min_height(ROW_H);
                        ui.horizontal_centered(|ui| {
                            // иконка + имя
                            let name_text = if entry.name == ".." {
                                "⬆  ..".to_string()
                            } else {
                                match entry.kind {
                                    EntryKind::Dir => format!("📁  {}", entry.name),
                                    EntryKind::Symlink => format!("🔗  {}", entry.name),
                                    EntryKind::File => {
                                        let icon = file_icon(&entry.name);
                                        format!("{}  {}", icon, entry.name)
                                    }
                                }
                            };

                            let name_col = if entry.name == ".." {
                                TEXT_HINT
                            } else if entry.kind == EntryKind::Dir {
                                ACCENT
                            } else {
                                TEXT_PRIMARY
                            };

                            if let Some(payload) = make_drag_payload(entry) {
                                ui.dnd_drag_source(row_id, payload, |ui| {
                                    ui.label(RichText::new(&name_text).color(name_col).size(12.0));
                                });
                            } else {
                                ui.label(RichText::new(&name_text).color(name_col).size(12.0));
                            }

                            // правые колонки
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format_time(entry.modified))
                                            .color(TEXT_DIM)
                                            .size(11.0),
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
                                            .size(11.0),
                                    );
                                    ui.add_space(4.0);
                                },
                            );
                        });
                    })
                    .response;

                if response.clicked() {
                    *selected = Some(entry.name.clone());
                    clicked = Some(entry.name.clone());
                }
                if response.double_clicked() {
                    double_clicked = Some(entry.name.clone());
                }
            }
        });

    FileTableResponse {
        double_clicked,
        clicked,
    }
}

fn sort_header(ui: &mut Ui, label: &str, min_w: f32, active: bool, dir: SortDir) -> bool {
    let arrow = if active {
        if dir == SortDir::Asc { " ↑" } else { " ↓" }
    } else {
        ""
    };
    let full = format!("{}{}", label, arrow);
    let col = if active { ACCENT } else { TEXT_DIM };
    let btn = egui::Button::new(RichText::new(full).color(col).size(11.0).strong())
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

fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "🦀",
        "toml" | "yaml" | "yml" | "json" => "⚙",
        "md" | "txt" | "log" => "📝",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "🖼",
        "mp4" | "mkv" | "avi" | "mov" => "🎬",
        "mp3" | "ogg" | "flac" | "wav" => "🎵",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" => "📦",
        "pdf" => "📕",
        "sh" | "bash" | "fish" | "zsh" => "⚡",
        "html" | "css" | "js" | "ts" => "🌐",
        "py" => "🐍",
        "java" | "class" | "jar" => "☕",
        _ => "📄",
    }
}

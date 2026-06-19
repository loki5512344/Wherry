use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::domain::transfer::{TransferKind, TransferTask};
use crate::fs::local;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::drag::{DragPayload, make_local_payload};
use crate::ui::panels::file_pane::{FileTableResponse, file_table, format_size};
use crate::ui::state::AppState;
use crate::ui::theme::*;
use egui::{RichText, Ui};
use std::sync::Arc;

fn parent_path(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn make_parent_entry(path: &str) -> FileEntry {
    FileEntry {
        name: "..".into(),
        path: parent_path(path),
        kind: EntryKind::Dir,
        size: None,
        modified: None,
        permissions: None,
    }
}

pub fn render(
    ui: &mut Ui,
    state: &mut AppState,
    queue: &TransferQueue,
    registry: &Arc<RemoteRegistry>,
    _rt_handle: &tokio::runtime::Handle,
) {
    let frame = egui::Frame::none()
        .fill(BG_CONTENT)
        .inner_margin(egui::Margin::same(0.0));

    let (_inner, dropped) = ui.dnd_drop_zone::<DragPayload, _>(frame, |ui| {
        render_inner(ui, state);
    });

    if let Some(payload_arc) = dropped {
        match &*payload_arc {
            DragPayload::RemoteFile(remote_path, file_name, conn_id) => {
                if registry.get(conn_id).is_some() {
                    let local_path_str =
                        format!("{}/{}", state.local_path.trim_end_matches('/'), file_name);
                    let task = TransferTask::new(
                        TransferKind::Download,
                        conn_id.clone(),
                        local_path_str,
                        remote_path.clone(),
                        file_name.clone(),
                        0,
                    );
                    queue.push(task);
                    state.status_message = format!("Download queued: {}", file_name);
                } else {
                    state.status_message = "Connection no longer active".into();
                }
            }
            DragPayload::LocalFile(_, _) => {
                state.status_message = "Cannot drop local onto local".into();
            }
        }
    }
}

fn render_inner(ui: &mut Ui, state: &mut AppState) {
    // Хлебные крошки + путь
    render_path_bar(ui, state);

    // разделитель
    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);

    // Таблица
    let entries = state.local_entries.clone();
    let mut selected = state.local_selected.clone();

    let has_connection = state
        .active_tab_ref()
        .map(|t| t.status == ConnectionStatus::Connected)
        .unwrap_or(false);

    let FileTableResponse { double_clicked, .. } =
        file_table(ui, "local_table", &entries, &mut selected, |entry| {
            if has_connection {
                make_local_payload(entry)
            } else {
                None
            }
        });

    state.local_selected = selected;

    if let Some(name) = double_clicked {
        if name == ".." {
            state.local_path = parent_path(&state.local_path);
        } else if let Some(entry) = state.local_entries.iter().find(|e| e.name == name)
            && entry.kind == EntryKind::Dir
        {
            state.local_path = entry.path.clone();
        }
        refresh_local(state);
    }

    // Нижняя полоска
    render_footer(ui, state);
}

fn render_path_bar(ui: &mut Ui, state: &mut AppState) {
    let frame = egui::Frame::none()
        .fill(BG_PANEL)
        .inner_margin(egui::Margin {
            left: 8.0,
            right: 8.0,
            top: 4.0,
            bottom: 4.0,
        });

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // иконка home
            let home = dirs::home_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_home = state.local_path == home;
            let home_btn = egui::Button::new(RichText::new("🏠").size(13.0))
                .fill(if is_home {
                    BG_ROW_SEL
                } else {
                    egui::Color32::TRANSPARENT
                })
                .rounding(4.0)
                .min_size(egui::vec2(22.0, 22.0));
            if ui.add(home_btn).clicked() {
                state.local_path = home;
                refresh_local(state);
            }

            ui.add_space(4.0);

            // Крошки
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
                    let link = egui::Label::new(RichText::new(*part).color(TEXT_DIM).size(12.0))
                        .sense(egui::Sense::click());
                    if ui.add(link).clicked() {
                        state.local_path = path_snap;
                        refresh_local(state);
                    }
                    ui.label(RichText::new("/").color(TEXT_HINT).size(12.0));
                }
            }
        });
    });
}

fn render_footer(ui: &mut Ui, state: &mut AppState) {
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
                RichText::new(format!("{} items", count))
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
                let star = egui::Button::new(RichText::new("★").color(YELLOW).size(12.0))
                    .fill(egui::Color32::TRANSPARENT)
                    .min_size(egui::vec2(20.0, 20.0));
                if ui.add(star).on_hover_text("Bookmark this folder").clicked() {
                    let name = std::path::Path::new(&state.local_path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Bookmark".into());
                    state.bookmarks.push(crate::ui::state::Bookmark {
                        name,
                        path: state.local_path.clone(),
                    });
                    state.status_message = "Bookmark added".into();
                }
            });
        });
    });
}

pub fn refresh_local(state: &mut AppState) {
    let mut entries = Vec::new();
    if state.local_path != "/" {
        entries.push(make_parent_entry(&state.local_path));
    }
    match local::list(&state.local_path) {
        Ok(mut list) => {
            entries.append(&mut list);
            state.local_entries = entries;
        }
        Err(e) => {
            state.status_message = format!("Local error: {}", e);
        }
    }
}

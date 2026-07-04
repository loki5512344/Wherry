use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::domain::transfer::{TransferKind, TransferTask};
use crate::fs::local;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::drag::{DragPayload, make_local_payload};
use crate::ui::icons::{self, Icon};
use crate::ui::panels::file_pane::{
    FileTableResponse, RowAction, context_menu_item, file_table, format_size,
};
use crate::ui::state::{AppState, Pane};
use crate::ui::theme::*;
use egui::{RichText, Ui};
use std::sync::{Arc, Mutex};

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
    db: &Arc<Mutex<rusqlite::Connection>>,
) {
    let frame = egui::Frame::none()
        .fill(BG_CONTENT)
        .inner_margin(egui::Margin::same(0.0));

    let (_inner, dropped) = ui.dnd_drop_zone::<DragPayload, _>(frame, |ui| {
        render_inner(ui, state, queue, registry, db);
    });

    if let Some(payload_arc) = dropped {
        let dest_dir = state.local_path.clone();
        handle_drop(state, queue, registry, &payload_arc, &dest_dir);
    }
}

/// Обрабатывает бросок файла в текущую директорию (dest_dir) — общий код
/// для дропа на весь пейн и дропа на конкретную папку в списке.
fn handle_drop(
    state: &mut AppState,
    queue: &TransferQueue,
    registry: &Arc<RemoteRegistry>,
    payload: &DragPayload,
    dest_dir: &str,
) {
    match payload {
        DragPayload::RemoteFile(remote_path, file_name, conn_id) => {
            if registry.get(conn_id).is_some() {
                let local_path_str = format!("{}/{}", dest_dir.trim_end_matches('/'), file_name);
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
        DragPayload::LocalFile(src_path, file_name) => {
            if std::path::Path::new(dest_dir)
                == std::path::Path::new(src_path)
                    .parent()
                    .unwrap_or(std::path::Path::new(""))
            {
                return; // бросили туда же, откуда взяли
            }
            match local::move_into(src_path, dest_dir) {
                Ok(()) => {
                    state.status_message = format!("Moved {}", file_name);
                    refresh_local(state);
                }
                Err(e) => state.status_message = format!("Move failed: {}", e),
            }
        }
    }
}

fn render_inner(
    ui: &mut Ui,
    state: &mut AppState,
    queue: &TransferQueue,
    registry: &Arc<RemoteRegistry>,
    db: &Arc<Mutex<rusqlite::Connection>>,
) {
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

    let FileTableResponse {
        double_clicked,
        clicked,
        dropped_on_dir,
        context_action,
    } = file_table(
        ui,
        "local_table",
        &entries,
        &mut selected,
        |entry| {
            if has_connection {
                make_local_payload(entry)
            } else {
                None
            }
        },
        |ui, entry| build_context_menu(ui, entry, has_connection),
    );

    state.local_selected = selected;
    if clicked.is_some() {
        state.active_pane = Pane::Local;
    }

    if let Some((target_entry, payload)) = dropped_on_dir {
        handle_drop(state, queue, registry, &payload, &target_entry.path);
    }

    if let Some(name) = double_clicked {
        open_entry(state, &name);
    }

    if let Some((entry, action)) = context_action {
        handle_row_action(ui.ctx(), state, queue, db, &entry, action);
    }

    // Нижняя полоска
    render_footer(ui, state, db);
}

fn build_context_menu(ui: &mut Ui, entry: &FileEntry, has_connection: bool) -> Option<RowAction> {
    if context_menu_item(ui, Icon::Folder, "Open", false) {
        return Some(RowAction::Open);
    }
    if entry.name != ".." {
        if context_menu_item(ui, Icon::Pen, "Rename", false) {
            return Some(RowAction::Rename);
        }
        if context_menu_item(ui, Icon::Trash, "Delete", true) {
            return Some(RowAction::Delete);
        }
        ui.separator();
        if entry.kind == EntryKind::Dir
            && context_menu_item(ui, Icon::Star, "Add to Bookmarks", false)
        {
            return Some(RowAction::Bookmark);
        }
        if context_menu_item(ui, Icon::Document, "Copy Path", false) {
            return Some(RowAction::CopyPath);
        }
        if entry.kind == EntryKind::File && has_connection {
            ui.separator();
            if context_menu_item(ui, Icon::Upload, "Upload to Server", false) {
                return Some(RowAction::Transfer);
            }
        }
    }
    None
}

fn open_entry(state: &mut AppState, name: &str) {
    if name == ".." {
        state.local_path = parent_path(&state.local_path);
        refresh_local(state);
    } else if let Some(entry) = state.local_entries.iter().find(|e| e.name == name) {
        if entry.kind == EntryKind::Dir {
            state.local_path = entry.path.clone();
            refresh_local(state);
        } else {
            match local::open(&entry.path) {
                Ok(()) => state.status_message = format!("Opening {}", entry.name),
                Err(e) => state.status_message = format!("Open failed: {}", e),
            }
        }
    }
}

fn handle_row_action(
    ctx: &egui::Context,
    state: &mut AppState,
    queue: &TransferQueue,
    db: &Arc<Mutex<rusqlite::Connection>>,
    entry: &FileEntry,
    action: RowAction,
) {
    match action {
        RowAction::Open => open_entry(state, &entry.name),
        RowAction::Rename => {
            state.op_target = Pane::Local;
            state.show_rename_dialog = true;
            state.rename_old_name = entry.name.clone();
            state.rename_new_name = entry.name.clone();
        }
        RowAction::Delete => {
            state.op_target = Pane::Local;
            state.show_delete_dialog = true;
            state.delete_name = entry.name.clone();
        }
        RowAction::Bookmark => add_bookmark(state, db, &entry.name, &entry.path),
        RowAction::CopyPath => {
            ctx.copy_text(entry.path.clone());
            state.status_message = "Path copied".into();
        }
        RowAction::Transfer => {
            if let Some(tab) = state.active_tab_ref() {
                let connection_id = tab.id.clone();
                let remote_path =
                    format!("{}/{}", tab.remote_path.trim_end_matches('/'), entry.name);
                let task = TransferTask::new(
                    TransferKind::Upload,
                    connection_id,
                    entry.path.clone(),
                    remote_path,
                    entry.name.clone(),
                    0,
                );
                queue.push(task);
                state.status_message = format!("Upload queued: {}", entry.name);
            }
        }
    }
}

fn add_bookmark(
    state: &mut AppState,
    db: &Arc<Mutex<rusqlite::Connection>>,
    name: &str,
    path: &str,
) {
    if state.bookmarks.iter().any(|b| b.path == path) {
        state.status_message = "Already bookmarked".into();
        return;
    }
    let Ok(conn) = db.lock() else {
        return;
    };
    match crate::storage::db::add_bookmark(&conn, name, path) {
        Ok(id) => {
            state.bookmarks.push(crate::ui::state::Bookmark {
                id,
                name: name.to_string(),
                path: path.to_string(),
            });
            state.status_message = "Bookmark added".into();
        }
        Err(e) => state.status_message = format!("Bookmark failed: {}", e),
    }
}

fn render_path_bar(ui: &mut Ui, state: &mut AppState) {
    let frame = egui::Frame::none()
        .fill(BG_BASE)
        .inner_margin(egui::Margin::symmetric(12.0, 0.0));

    frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, 30.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new("LOCAL").color(TEXT_DIM).size(10.5).strong());
                ui.add_space(10.0);

                // иконка home
                let home = dirs::home_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let is_home = state.local_path == home;
                let home_col = if is_home { ACCENT } else { TEXT_DIM };
                let home_btn = egui::Button::image(icons::image(Icon::Home, 13.0, home_col))
                    .fill(if is_home {
                        BG_ROW_SEL
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .rounding(RADIUS_SM)
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
                        let link =
                            egui::Label::new(RichText::new(*part).color(TEXT_DIM).size(12.0))
                                .sense(egui::Sense::click());
                        if ui.add(link).clicked() {
                            state.local_path = path_snap;
                            refresh_local(state);
                        }
                        ui.label(RichText::new("/").color(TEXT_HINT).size(12.0));
                    }
                }
            },
        );
    });
}

fn render_footer(ui: &mut Ui, state: &mut AppState, db: &Arc<Mutex<rusqlite::Connection>>) {
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
                let star = egui::Button::image(icons::image(Icon::Star, 12.0, YELLOW))
                    .fill(egui::Color32::TRANSPARENT)
                    .min_size(egui::vec2(20.0, 20.0));
                if ui.add(star).on_hover_text("Bookmark this folder").clicked() {
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

use egui::{RichText, Ui};
use std::sync::Arc;

use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::domain::transfer::{TransferKind, TransferTask};
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::drag::{DragPayload, make_remote_payload};
use crate::ui::icons::{self, Icon};
use crate::ui::panels::file_pane::{
    FileTableResponse, RowAction, context_menu_item, file_table, format_size,
};
use crate::ui::state::{AppState, Pane, PendingRemoteList};
use crate::ui::theme::*;

pub fn remote_parent(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return "/".into();
    }
    if let Some(idx) = trimmed.rfind('/') {
        let p = &trimmed[..idx];
        if p.is_empty() {
            "/".into()
        } else {
            p.to_string()
        }
    } else {
        "/".into()
    }
}

pub fn render(
    ui: &mut Ui,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    queue: &TransferQueue,
) {
    let frame = egui::Frame::none()
        .fill(BG_CONTENT)
        .inner_margin(egui::Margin::same(0.0));

    let (_inner, dropped) = ui.dnd_drop_zone::<DragPayload, _>(frame, |ui| {
        render_content(ui, state, tab_idx, registry, rt_handle, queue);
    });

    let remote_path = state.tabs[tab_idx].remote_path.clone();

    if let Some(payload_arc) = dropped {
        handle_drop(
            state,
            tab_idx,
            queue,
            registry,
            rt_handle,
            &payload_arc,
            &remote_path,
        );
    }
}

/// Обрабатывает бросок файла в директорию dest_dir на удалённой стороне —
/// общий код для дропа на весь пейн и дропа на конкретную папку в списке.
fn handle_drop(
    state: &mut AppState,
    tab_idx: usize,
    queue: &TransferQueue,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    payload: &DragPayload,
    dest_dir: &str,
) {
    let is_connected = state.tabs[tab_idx].status == ConnectionStatus::Connected;
    let connection_id = state.tabs[tab_idx].id.clone();

    match payload {
        DragPayload::LocalFile(local_path, file_name) => {
            if is_connected {
                let dest = format!("{}/{}", dest_dir.trim_end_matches('/'), file_name);
                let task = TransferTask::new(
                    TransferKind::Upload,
                    connection_id,
                    local_path.clone(),
                    dest,
                    file_name.clone(),
                    0,
                );
                queue.push(task);
                state.status_message = format!("Upload queued: {}", file_name);
            } else {
                state.status_message = "No active connection".into();
            }
        }
        DragPayload::RemoteFile(remote_path, file_name, conn_id) => {
            if *conn_id != connection_id {
                state.status_message = "Cannot move between different connections".into();
                return;
            }
            let dest = format!("{}/{}", dest_dir.trim_end_matches('/'), file_name);
            if dest == *remote_path {
                return;
            }
            let registry = registry.clone();
            let from = remote_path.clone();
            let conn_id_clone = conn_id.clone();
            let result = Arc::new(std::sync::Mutex::new(None::<Result<(), String>>));
            let result_clone = result.clone();
            rt_handle.spawn(async move {
                let fs = match registry.get(&conn_id_clone) {
                    Some(fs) => fs,
                    None => {
                        *result_clone.lock().unwrap() = Some(Err("connection not found".into()));
                        return;
                    }
                };
                let r = fs.rename(&from, &dest).await.map_err(|e| e.to_string());
                *result_clone.lock().unwrap() = Some(r);
            });
            state.pending_rename_result = Some(result);
            state.status_message = format!("Moving {}…", file_name);
        }
    }
}

fn render_content(
    ui: &mut Ui,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    queue: &TransferQueue,
) {
    // Path bar
    render_path_bar(ui, state, tab_idx, registry, rt_handle);

    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);

    // Loading overlay hint
    if state.tabs[tab_idx].loading {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.spinner();
            ui.label(RichText::new("Loading...").color(TEXT_DIM).size(11.0));
        });
    }

    // Table
    let entries;
    let mut selected;
    {
        let tab = &state.tabs[tab_idx];
        entries = tab.remote_entries.clone();
        selected = tab.remote_selected.clone();
    }

    let is_connected = state.tabs[tab_idx].status == ConnectionStatus::Connected;
    let conn_id = state.tabs[tab_idx].id.clone();

    let table_id = format!("remote_table_{}", tab_idx);
    let conn_id_for_drag = conn_id.clone();
    let FileTableResponse {
        double_clicked,
        clicked,
        dropped_on_dir,
        context_action,
    } = file_table(
        ui,
        &table_id,
        &entries,
        &mut selected,
        move |entry| {
            if is_connected {
                make_remote_payload(entry, &conn_id_for_drag)
            } else {
                None
            }
        },
        build_context_menu,
    );

    state.tabs[tab_idx].remote_selected = selected;
    if clicked.is_some() {
        state.active_pane = Pane::Remote;
    }

    if let Some((target_entry, payload)) = dropped_on_dir {
        handle_drop(
            state,
            tab_idx,
            queue,
            registry,
            rt_handle,
            &payload,
            &target_entry.path,
        );
    }

    if let Some(name) = double_clicked {
        open_entry(state, tab_idx, registry, rt_handle, queue, &name);
    }

    if let Some((entry, action)) = context_action {
        handle_row_action(
            ui.ctx(),
            state,
            tab_idx,
            registry,
            rt_handle,
            queue,
            &entry,
            action,
        );
    }

    // Footer
    render_footer(ui, state, tab_idx, registry, rt_handle);
}

fn build_context_menu(ui: &mut Ui, entry: &FileEntry) -> Option<RowAction> {
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
        if context_menu_item(ui, Icon::Document, "Copy Path", false) {
            return Some(RowAction::CopyPath);
        }
        if entry.kind == EntryKind::File
            && context_menu_item(ui, Icon::Download, "Download to Local", false)
        {
            return Some(RowAction::Transfer);
        }
    }
    None
}

fn open_entry(
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    queue: &TransferQueue,
    name: &str,
) {
    let (nav_path, is_dir) = if name == ".." {
        let parent = remote_parent(&state.tabs[tab_idx].remote_path);
        (parent, true)
    } else {
        let entry = state.tabs[tab_idx]
            .remote_entries
            .iter()
            .find(|e| e.name == name);
        match entry {
            Some(e) if e.kind == EntryKind::Dir => (e.path.clone(), true),
            _ => (state.tabs[tab_idx].remote_path.clone(), false),
        }
    };

    if is_dir {
        state.tabs[tab_idx].remote_path = nav_path;
        if !state.tabs[tab_idx].loading {
            trigger_list(state, tab_idx, registry, rt_handle);
        }
    } else if let Some(entry) = state.tabs[tab_idx]
        .remote_entries
        .iter()
        .find(|e| e.name == name)
        .cloned()
    {
        queue_download(state, tab_idx, queue, &entry);
    }
}

fn queue_download(state: &mut AppState, tab_idx: usize, queue: &TransferQueue, entry: &FileEntry) {
    let connection_id = state.tabs[tab_idx].id.clone();
    let task = TransferTask::new(
        TransferKind::Download,
        connection_id,
        format!("{}/{}", state.local_path.trim_end_matches('/'), entry.name),
        entry.path.clone(),
        entry.name.clone(),
        0,
    );
    queue.push(task);
    state.status_message = format!("Download queued: {}", entry.name);
}

#[allow(clippy::too_many_arguments)]
fn handle_row_action(
    ctx: &egui::Context,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    queue: &TransferQueue,
    entry: &FileEntry,
    action: RowAction,
) {
    match action {
        RowAction::Open => open_entry(state, tab_idx, registry, rt_handle, queue, &entry.name),
        RowAction::Rename => {
            state.op_target = Pane::Remote;
            state.show_rename_dialog = true;
            state.rename_old_name = entry.name.clone();
            state.rename_new_name = entry.name.clone();
        }
        RowAction::Delete => {
            state.op_target = Pane::Remote;
            state.show_delete_dialog = true;
            state.delete_name = entry.name.clone();
        }
        RowAction::CopyPath => {
            ctx.copy_text(entry.path.clone());
            state.status_message = "Path copied".into();
        }
        RowAction::Transfer => queue_download(state, tab_idx, queue, entry),
        RowAction::Bookmark => {}
    }
}

fn render_path_bar(
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
                    RichText::new(format!("REMOTE · {}", host))
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

                // Хлебные крошки для remote path
                let path_clone = state.tabs[tab_idx].remote_path.clone();
                let parts: Vec<&str> = path_clone.split('/').filter(|s| !s.is_empty()).collect();
                let mut acc = String::new();

                // Root
                let root_btn = egui::Button::new(RichText::new("/").color(TEXT_DIM).size(12.0))
                    .fill(egui::Color32::TRANSPARENT)
                    .min_size(egui::vec2(12.0, 22.0));
                if ui.add(root_btn).clicked() {
                    state.tabs[tab_idx].remote_path = "/".into();
                    if !state.tabs[tab_idx].loading {
                        trigger_list(state, tab_idx, registry, rt_handle);
                    }
                }

                for (i, part) in parts.iter().enumerate() {
                    acc.push('/');
                    acc.push_str(part);
                    let is_last = i == parts.len() - 1;
                    let path_snap = acc.clone();

                    if is_last {
                        ui.label(RichText::new(*part).color(TEXT_PRIMARY).size(12.0).strong());
                    } else {
                        let link =
                            egui::Label::new(RichText::new(*part).color(TEXT_DIM).size(12.0))
                                .sense(egui::Sense::click());
                        if ui.add(link).clicked() {
                            state.tabs[tab_idx].remote_path = path_snap;
                            if !state.tabs[tab_idx].loading {
                                trigger_list(state, tab_idx, registry, rt_handle);
                            }
                        }
                        ui.label(RichText::new("/").color(TEXT_HINT).size(12.0));
                    }
                }
            },
        );
    });
}

fn render_footer(
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
                "Loading…".to_string()
            } else {
                format!("{} items", count)
            };

            ui.label(RichText::new(label).color(TEXT_DIM).size(11.0));

            if total_size > 0 {
                ui.label(
                    RichText::new(format!("({})", format_size(Some(total_size))))
                        .color(TEXT_HINT)
                        .size(11.0),
                );
            }

            // Refresh кнопка
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let refresh = egui::Button::image(icons::image(Icon::Refresh, 13.0, ACCENT))
                    .fill(egui::Color32::TRANSPARENT)
                    .min_size(egui::vec2(20.0, 20.0));
                if ui.add(refresh).on_hover_text("Refresh").clicked()
                    && !state.tabs[tab_idx].loading
                {
                    trigger_list(state, tab_idx, registry, rt_handle);
                }
            });
        });
    });
}

pub fn trigger_list(
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    if state.tabs[tab_idx].status != ConnectionStatus::Connected {
        state.status_message = "Not connected".into();
        return;
    }
    state.tabs[tab_idx].loading = true;

    let connection_id = state.tabs[tab_idx].id.clone();
    let path = state.tabs[tab_idx].remote_path.clone();
    let registry = registry.clone();

    let result = Arc::new(std::sync::Mutex::new(
        None::<Result<Vec<FileEntry>, String>>,
    ));
    let result_clone = result.clone();

    rt_handle.spawn(async move {
        let fs = match registry.get(&connection_id) {
            Some(fs) => fs,
            None => {
                *result_clone.lock().unwrap() = Some(Err("connection not found".into()));
                return;
            }
        };
        let r = fs.list(&path).await.map_err(|e| e.to_string());
        *result_clone.lock().unwrap() = Some(r);
    });

    state
        .pending_remote_list
        .push(PendingRemoteList { tab_idx, result });
}

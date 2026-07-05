//! Действия над удалённой ФС: drop-обработка, открытие записей, контекстное меню.
use egui::Ui;
use std::sync::Arc;

use super::{remote_parent, trigger_list};
use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::domain::transfer::{TransferKind, TransferTask};
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::drag::DragPayload;
use crate::ui::icons::Icon;
use crate::ui::panels::file_pane::{RowAction, context_menu_item};
use crate::ui::state::{AppState, Pane};

/// Обрабатывает бросок файла в директорию `dest_dir` на удалённой стороне —
/// общий код для дропа на весь пейн и дропа на конкретную папку в списке.
pub(super) fn handle_drop(
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
                state.status_message =
                    crate::i18n::tf("toolbar.upload_queued", &[("{name}", file_name)]);
            } else {
                state.status_message = crate::i18n::t("panels.remote.no_active_connection").into();
            }
        }
        DragPayload::RemoteFile(remote_path, file_name, conn_id) => {
            if *conn_id != connection_id {
                state.status_message =
                    crate::i18n::t("panels.remote.cannot_move_cross_connection").into();
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
            state.status_message =
                crate::i18n::tf("panels.remote.moving", &[("{name}", file_name)]);
        }
    }
}

pub(super) fn build_context_menu(ui: &mut Ui, entry: &FileEntry) -> Option<RowAction> {
    use crate::i18n::t;
    if context_menu_item(ui, Icon::Folder, t("common.open"), false) {
        return Some(RowAction::Open);
    }
    if entry.name != ".." {
        if context_menu_item(ui, Icon::Pen, t("common.rename"), false) {
            return Some(RowAction::Rename);
        }
        if context_menu_item(ui, Icon::Trash, t("common.delete"), true) {
            return Some(RowAction::Delete);
        }
        ui.separator();
        if context_menu_item(ui, Icon::Document, t("common.copy_path"), false) {
            return Some(RowAction::CopyPath);
        }
        if entry.kind == EntryKind::File
            && context_menu_item(ui, Icon::Download, t("panels.remote.download_to_local"), false)
        {
            return Some(RowAction::Transfer);
        }
    }
    None
}

pub(super) fn open_entry(
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
    state.status_message = crate::i18n::tf("toolbar.download_queued", &[("{name}", &entry.name)]);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_row_action(
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
            state.status_message = crate::i18n::t("panels.path_copied").into();
        }
        RowAction::Transfer => queue_download(state, tab_idx, queue, entry),
        RowAction::Bookmark => {}
    }
}

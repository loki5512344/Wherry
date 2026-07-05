//! Действия над локальной ФС: drop, открытие, контекстное меню, закладки.
use std::sync::{Arc, Mutex};

use super::{parent_path, refresh_local};
use crate::domain::file_entry::EntryKind;
use crate::domain::file_entry::FileEntry;
use crate::domain::transfer::{TransferKind, TransferTask};
use crate::fs::local;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::drag::DragPayload;
use crate::ui::icons::Icon;
use crate::ui::panels::file_pane::{RowAction, context_menu_item};
use crate::ui::state::{AppState, Pane};

/// Обрабатывает бросок файла в директорию `dest_dir` — общий код для дропа
/// на весь пейн и дропа на конкретную папку в списке.
pub(super) fn handle_drop(
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
                state.status_message =
                    crate::i18n::tf("toolbar.download_queued", &[("{name}", file_name)]);
            } else {
                state.status_message = crate::i18n::t("panels.local.conn_inactive").into();
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
                    state.status_message =
                        crate::i18n::tf("panels.local.moved", &[("{name}", file_name)]);
                    refresh_local(state);
                }
                Err(e) => {
                    state.status_message =
                        crate::i18n::tf("panels.local.move_failed", &[("{err}", &e.to_string())])
                }
            }
        }
    }
}

pub(super) fn build_context_menu(
    ui: &mut egui::Ui,
    entry: &FileEntry,
    has_connection: bool,
) -> Option<RowAction> {
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
        if entry.kind == EntryKind::Dir
            && context_menu_item(ui, Icon::Star, t("panels.local.add_to_bookmarks"), false)
        {
            return Some(RowAction::Bookmark);
        }
        if context_menu_item(ui, Icon::Document, t("common.copy_path"), false) {
            return Some(RowAction::CopyPath);
        }
        if entry.kind == EntryKind::File && has_connection {
            ui.separator();
            if context_menu_item(ui, Icon::Upload, t("panels.local.upload_to_server"), false) {
                return Some(RowAction::Transfer);
            }
        }
    }
    None
}

pub(super) fn open_entry(state: &mut AppState, name: &str) {
    if name == ".." {
        state.local_path = parent_path(&state.local_path);
        refresh_local(state);
    } else if let Some(entry) = state.local_entries.iter().find(|e| e.name == name) {
        if entry.kind == EntryKind::Dir {
            state.local_path = entry.path.clone();
            refresh_local(state);
        } else {
            match local::open(&entry.path) {
                Ok(()) => {
                    state.status_message =
                        crate::i18n::tf("panels.local.opening", &[("{name}", &entry.name)])
                }
                Err(e) => {
                    state.status_message =
                        crate::i18n::tf("panels.local.open_failed", &[("{err}", &e.to_string())])
                }
            }
        }
    }
}

pub(super) fn handle_row_action(
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
            state.status_message = crate::i18n::t("panels.path_copied").into();
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
                state.status_message =
                    crate::i18n::tf("toolbar.upload_queued", &[("{name}", &entry.name)]);
            }
        }
    }
}

pub(super) fn add_bookmark(
    state: &mut AppState,
    db: &Arc<Mutex<rusqlite::Connection>>,
    name: &str,
    path: &str,
) {
    if state.bookmarks.iter().any(|b| b.path == path) {
        state.status_message = crate::i18n::t("panels.local.already_bookmarked").into();
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
            state.status_message = crate::i18n::t("panels.local.bookmark_added").into();
        }
        Err(e) => {
            state.status_message =
                crate::i18n::tf("panels.local.bookmark_failed", &[("{err}", &e.to_string())])
        }
    }
}

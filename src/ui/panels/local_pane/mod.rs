use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::fs::local;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::drag::{DragPayload, make_local_payload};
use crate::ui::panels::file_pane::{FileTableResponse, file_table};
use crate::ui::state::{AppState, Pane};
use crate::ui::theme::*;
use egui::Ui;
use std::sync::{Arc, Mutex};

mod actions;
mod bars;

pub(super) fn parent_path(path: &str) -> String {
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
        actions::handle_drop(state, queue, registry, &payload_arc, &dest_dir);
    }
}

fn render_inner(
    ui: &mut Ui,
    state: &mut AppState,
    queue: &TransferQueue,
    registry: &Arc<RemoteRegistry>,
    db: &Arc<Mutex<rusqlite::Connection>>,
) {
    bars::render_path_bar(ui, state);

    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);

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
        |ui, entry| actions::build_context_menu(ui, entry, has_connection),
    );

    state.local_selected = selected;
    if clicked.is_some() {
        state.active_pane = Pane::Local;
    }

    if let Some((target_entry, payload)) = dropped_on_dir {
        actions::handle_drop(state, queue, registry, &payload, &target_entry.path);
    }

    if let Some(name) = double_clicked {
        actions::open_entry(state, &name);
    }

    if let Some((entry, action)) = context_action {
        actions::handle_row_action(ui.ctx(), state, queue, db, &entry, action);
    }

    bars::render_footer(ui, state, db);
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
            state.status_message =
                crate::i18n::tf("panels.local.error", &[("{err}", &e.to_string())]);
        }
    }
}

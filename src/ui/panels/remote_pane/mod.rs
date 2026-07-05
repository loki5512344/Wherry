use egui::Ui;
use std::sync::Arc;

use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::FileEntry;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::drag::{DragPayload, make_remote_payload};
use crate::ui::panels::file_pane::{FileTableResponse, file_table};
use crate::ui::state::{AppState, Pane, PendingRemoteList};

mod actions;
mod bars;

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
        .fill(crate::ui::theme::BG_CONTENT)
        .inner_margin(egui::Margin::same(0.0));

    let (_inner, dropped) = ui.dnd_drop_zone::<DragPayload, _>(frame, |ui| {
        render_content(ui, state, tab_idx, registry, rt_handle, queue);
    });

    let remote_path = state.tabs[tab_idx].remote_path.clone();

    if let Some(payload_arc) = dropped {
        actions::handle_drop(
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

fn render_content(
    ui: &mut Ui,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    queue: &TransferQueue,
) {
    bars::render_path_bar(ui, state, tab_idx, registry, rt_handle);

    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, crate::ui::theme::BORDER);

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
        actions::build_context_menu,
    );

    state.tabs[tab_idx].remote_selected = selected;
    if clicked.is_some() {
        state.active_pane = Pane::Remote;
    }

    if let Some((target_entry, payload)) = dropped_on_dir {
        actions::handle_drop(
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
        actions::open_entry(state, tab_idx, registry, rt_handle, queue, &name);
    }

    if let Some((entry, action)) = context_action {
        actions::handle_row_action(
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

    bars::render_footer(ui, state, tab_idx, registry, rt_handle);
}

pub fn trigger_list(
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    if state.tabs[tab_idx].status != ConnectionStatus::Connected {
        state.status_message = crate::i18n::t("status.not_connected").into();
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

use egui::{RichText, Ui};
use std::sync::Arc;

use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::domain::transfer::{TransferKind, TransferTask};
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::drag::{make_remote_payload, DragPayload};
use crate::ui::panels::file_pane::{file_table, FileTableResponse, format_size};
use crate::ui::state::{AppState, PendingRemoteList};
use crate::ui::theme::*;

pub fn remote_parent(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" { return "/".into(); }
    if let Some(idx) = trimmed.rfind('/') {
        let p = &trimmed[..idx];
        if p.is_empty() { "/".into() } else { p.to_string() }
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
        render_content(ui, state, tab_idx, registry, rt_handle);
    });

    let is_connected = state.tabs[tab_idx].status == ConnectionStatus::Connected;
    let connection_id = state.tabs[tab_idx].id.clone();
    let remote_path = state.tabs[tab_idx].remote_path.clone();

    if let Some(payload_arc) = dropped {
        match &*payload_arc {
            DragPayload::LocalFile(local_path, file_name) => {
                if is_connected {
                    let dest = format!("{}/{}", remote_path.trim_end_matches('/'), file_name);
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
            DragPayload::RemoteFile(_, _, _) => {
                state.status_message = "Cannot drop remote onto remote".into();
            }
        }
    }
}

fn render_content(
    ui: &mut Ui,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
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
    let FileTableResponse { double_clicked, .. } =
        file_table(ui, &table_id, &entries, &mut selected, move |entry| {
            if is_connected {
                make_remote_payload(entry, &conn_id)
            } else {
                None
            }
        });

    state.tabs[tab_idx].remote_selected = selected;

    if let Some(name) = double_clicked {
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
        }
    }

    // Footer
    render_footer(ui, state, tab_idx, registry, rt_handle);
}

fn render_path_bar(
    ui: &mut Ui,
    state: &mut AppState,
    tab_idx: usize,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    let frame = egui::Frame::none()
        .fill(BG_PANEL)
        .inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 4.0, bottom: 4.0 });

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // статус точка
            let (dot, dot_col) = match state.tabs[tab_idx].status {
                ConnectionStatus::Connected    => ("●", GREEN),
                ConnectionStatus::Disconnected => ("○", TEXT_HINT),
                ConnectionStatus::Connecting   => ("◐", YELLOW),
                ConnectionStatus::Error(_)     => ("×", RED),
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
                    let link = egui::Label::new(
                        RichText::new(*part).color(TEXT_DIM).size(12.0)
                    ).sense(egui::Sense::click());
                    if ui.add(link).clicked() {
                        state.tabs[tab_idx].remote_path = path_snap;
                        if !state.tabs[tab_idx].loading {
                            trigger_list(state, tab_idx, registry, rt_handle);
                        }
                    }
                    ui.label(RichText::new("/").color(TEXT_HINT).size(12.0));
                }
            }
        });
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
                        .size(11.0)
                );
            }

            // Refresh кнопка
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let refresh = egui::Button::new(RichText::new("⟳").color(ACCENT).size(14.0))
                    .fill(egui::Color32::TRANSPARENT)
                    .min_size(egui::vec2(20.0, 20.0));
                if ui.add(refresh).on_hover_text("Refresh").clicked() {
                    if !state.tabs[tab_idx].loading {
                        trigger_list(state, tab_idx, registry, rt_handle);
                    }
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

    state.pending_remote_list.push(PendingRemoteList { tab_idx, result });
}

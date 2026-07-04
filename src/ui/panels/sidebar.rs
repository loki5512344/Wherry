//! Левая боковая панель — bookmarks + места
use crate::ui::icons::{self, Icon};
use crate::ui::panels::local_pane::refresh_local;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use egui::{RichText, Ui};
use std::sync::{Arc, Mutex};

pub fn render(ui: &mut Ui, state: &mut AppState, db: &Arc<Mutex<rusqlite::Connection>>) {
    let frame = egui::Frame::none()
        .fill(BG_PANEL)
        .inner_margin(egui::Margin::symmetric(0.0, 0.0));

    frame.show(ui, |ui| {
        ui.set_width(SIDEBAR_W);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(8.0);

                section_header(ui, "LOCAL");
                ui.add_space(4.0);

                for qa in &state.quick_access.clone() {
                    let is_active = state.local_path == qa.path;
                    let icon = quick_access_icon(&qa.name);
                    sidebar_item(ui, icon, &qa.name, is_active, || {
                        state.local_path = qa.path.clone();
                        refresh_local(state);
                    });
                }

                ui.add_space(10.0);
                section_divider(ui);
                ui.add_space(6.0);

                section_header(ui, "DRIVES");
                ui.add_space(4.0);

                // Root + data drives
                let root_active = state.local_path == "/";
                sidebar_item(ui, Icon::Ssd, "/ (root)", root_active, || {
                    state.local_path = "/".into();
                    refresh_local(state);
                });

                // попытаемся найти /media и /mnt точки монтирования
                for mount_path in find_mounts() {
                    let label = std::path::Path::new(&mount_path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| mount_path.clone());
                    let is_active = state.local_path == mount_path;
                    sidebar_item(ui, Icon::Database, &label, is_active, || {
                        state.local_path = mount_path.clone();
                        refresh_local(state);
                    });
                }

                ui.add_space(10.0);
                section_divider(ui);
                ui.add_space(6.0);

                section_header(ui, "BOOKMARKS");
                ui.add_space(4.0);

                if state.bookmarks.is_empty() {
                    ui.label(
                        RichText::new("  No bookmarks yet")
                            .color(TEXT_HINT)
                            .size(11.0),
                    );
                    ui.label(
                        RichText::new("  Use ★ in file list")
                            .color(TEXT_HINT)
                            .size(10.0),
                    );
                } else {
                    let mut to_remove: Option<i64> = None;
                    for bm in &state.bookmarks.clone() {
                        let is_active = state.local_path == bm.path;
                        sidebar_item(ui, Icon::Star, &bm.name, is_active, || {
                            state.local_path = bm.path.clone();
                            refresh_local(state);
                        })
                        .context_menu(|ui| {
                            if ui.button("Remove bookmark").clicked() {
                                to_remove = Some(bm.id);
                                ui.close_menu();
                            }
                        });
                    }
                    if let Some(id) = to_remove {
                        state.bookmarks.retain(|b| b.id != id);
                        if let Ok(conn) = db.lock() {
                            let _ = crate::storage::db::remove_bookmark(&conn, id);
                        }
                    }
                }

                ui.add_space(8.0);
            });
    });
}

fn section_header(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(RichText::new(text).color(TEXT_HINT).size(10.5).strong());
    });
}

fn section_divider(ui: &mut Ui) {
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(rect, 0.0, SEPARATOR);
}

fn sidebar_item(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    active: bool,
    mut on_click: impl FnMut(),
) -> egui::Response {
    let bg = if active {
        BG_ROW_SEL
    } else {
        egui::Color32::TRANSPARENT
    };
    let text_col = TEXT_PRIMARY;
    let icon_col = if active { ACCENT } else { TEXT_DIM };

    let resp = egui::Frame::none()
        .fill(bg)
        .rounding(RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(8.0, 0.0))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(SIDEBAR_W - 32.0, 28.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    icons::icon(ui, icon, 14.0, icon_col);
                    ui.add_space(8.0);
                    ui.label(RichText::new(label).color(text_col).size(12.5));
                },
            );
        });

    let response = resp.response.interact(egui::Sense::click());
    if response.clicked() {
        on_click();
    }
    response
}

fn quick_access_icon(name: &str) -> Icon {
    match name {
        "Home" => Icon::Home,
        "Desktop" => Icon::Monitor,
        "Documents" => Icon::Document,
        "Downloads" => Icon::DownloadSquare,
        "Pictures" => Icon::Gallery,
        "Music" => Icon::MusicNote,
        "Videos" => Icon::Videocamera,
        _ => Icon::Folder,
    }
}

fn find_mounts() -> Vec<String> {
    let mut result = Vec::new();
    for prefix in ["/media", "/mnt"] {
        if let Ok(rd) = std::fs::read_dir(prefix) {
            for e in rd.flatten() {
                let p = e.path().to_string_lossy().to_string();
                result.push(p);
            }
        }
    }
    result
}

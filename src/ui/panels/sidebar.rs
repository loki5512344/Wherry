//! Левая боковая панель — bookmarks + места
use crate::i18n::t;
use crate::ui::icons::{self, Icon};
use crate::ui::panels::local_pane::refresh_local;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use crate::ui::widgets::row::clickable_row;
use egui::{RichText, Ui};
use std::sync::{Arc, Mutex};

pub fn render(ui: &mut Ui, state: &mut AppState, db: &Arc<Mutex<rusqlite::Connection>>) {
    let frame = egui::Frame::none()
        .fill(BG_PANEL)
        .inner_margin(egui::Margin::symmetric(0.0, 0.0));

    frame.show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(8.0);

                section_header(ui, t("panels.local_label"));
                ui.add_space(4.0);

                for qa in &state.quick_access.clone() {
                    let is_active = state.local_path == qa.path;
                    let icon = quick_access_icon(&qa.name);
                    sidebar_item(ui, icon, quick_access_label(&qa.name), is_active, || {
                        state.local_path = qa.path.clone();
                        refresh_local(state);
                    });
                }

                ui.add_space(10.0);
                section_divider(ui);
                ui.add_space(6.0);

                section_header(ui, t("panels.sidebar.drives"));
                ui.add_space(4.0);

                // Root + data drives
                let root_active = state.local_path == "/";
                sidebar_item(
                    ui,
                    Icon::Ssd,
                    t("panels.sidebar.root_label"),
                    root_active,
                    || {
                        state.local_path = "/".into();
                        refresh_local(state);
                    },
                );

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

                if !state.bookmarks.is_empty() {
                    ui.add_space(10.0);
                    section_divider(ui);
                    ui.add_space(6.0);

                    section_header(ui, t("panels.sidebar.bookmarks"));
                    ui.add_space(4.0);

                    let mut to_remove: Option<i64> = None;
                    for bm in &state.bookmarks.clone() {
                        let is_active = state.local_path == bm.path;
                        let resp = sidebar_item(ui, Icon::Star, &bm.name, is_active, || {
                            state.local_path = bm.path.clone();
                            refresh_local(state);
                        });
                        crate::ui::widgets::context_menu::context_menu(&resp, |ui| {
                            if ui.button(t("panels.sidebar.remove_bookmark")).clicked() {
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
    let icon_col = if active { ACCENT } else { TEXT_DIM };
    let resp = clickable_row(ui, active, 28.0, |ui| {
        icons::icon(ui, icon, 14.0, icon_col);
        ui.add_space(8.0);
        ui.label(RichText::new(label).color(TEXT_PRIMARY).size(12.5));
    });
    if resp.clicked() {
        on_click();
    }
    resp
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

/// `state.quick_access[].name` — стабильный английский идентификатор (нужен
/// для матчинга иконки в [`quick_access_icon`] и не должен зависеть от языка);
/// здесь он превращается в переведённую подпись для показа в сайдбаре.
fn quick_access_label(name: &str) -> &'static str {
    match name {
        "Home" => t("panels.sidebar.qa_home"),
        "Desktop" => t("panels.sidebar.qa_desktop"),
        "Documents" => t("panels.sidebar.qa_documents"),
        "Downloads" => t("panels.sidebar.qa_downloads"),
        "Pictures" => t("panels.sidebar.qa_pictures"),
        "Music" => t("panels.sidebar.qa_music"),
        "Videos" => t("panels.sidebar.qa_videos"),
        _ => "",
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

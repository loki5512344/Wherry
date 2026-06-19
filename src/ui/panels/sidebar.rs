//! Левая боковая панель — bookmarks + места
use crate::ui::panels::local_pane::refresh_local;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use egui::{RichText, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
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

                for bm in &state.bookmarks.clone() {
                    let is_active = state.local_path == bm.path;
                    let icon = bookmark_icon(&bm.name);
                    sidebar_item(ui, icon, &bm.name, is_active, || {
                        state.local_path = bm.path.clone();
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
                sidebar_item(ui, "🖥", "/ (root)", root_active, || {
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
                    sidebar_item(ui, "💾", &label, is_active, || {
                        state.local_path = mount_path.clone();
                        refresh_local(state);
                    });
                }

                ui.add_space(10.0);
                section_divider(ui);
                ui.add_space(6.0);

                section_header(ui, "BOOKMARKS");
                ui.add_space(4.0);

                // Пользовательские закладки (после 3 дефолтных)
                let custom: Vec<_> = state.bookmarks.iter().skip(3).cloned().collect();
                if custom.is_empty() {
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
                    for bm in &custom {
                        let is_active = state.local_path == bm.path;
                        sidebar_item(ui, "📌", &bm.name, is_active, || {
                            state.local_path = bm.path.clone();
                            refresh_local(state);
                        });
                    }
                }

                ui.add_space(8.0);
            });
    });
}

fn section_header(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(RichText::new(text).color(TEXT_HINT).size(10.0).strong());
    });
}

fn section_divider(ui: &mut Ui) {
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(rect, 0.0, SEPARATOR);
}

fn sidebar_item(ui: &mut Ui, icon: &str, label: &str, active: bool, mut on_click: impl FnMut()) {
    let bg = if active {
        BG_ROW_SEL
    } else {
        egui::Color32::TRANSPARENT
    };
    let text_col = if active {
        egui::Color32::WHITE
    } else {
        TEXT_PRIMARY
    };

    let resp = egui::Frame::none()
        .fill(bg)
        .rounding(4.0)
        .inner_margin(egui::Margin {
            left: 10.0,
            right: 6.0,
            top: 2.0,
            bottom: 2.0,
        })
        .show(ui, |ui| {
            ui.set_min_width(SIDEBAR_W - 8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(12.0));
                ui.add_space(6.0);
                ui.label(RichText::new(label).color(text_col).size(12.0));
            });
        });

    if resp.response.interact(egui::Sense::click()).clicked() {
        on_click();
    }
}

fn bookmark_icon(name: &str) -> &'static str {
    match name {
        "Home" => "🏠",
        "Desktop" => "🖥",
        "Documents" => "📄",
        "Downloads" => "⬇",
        "Pictures" => "🖼",
        "Music" => "🎵",
        "Videos" => "🎬",
        _ => "📁",
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

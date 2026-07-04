use crate::domain::connection::ConnectionStatus;
use crate::ui::icons::{self, Icon};
use crate::ui::panels::file_pane::format_size;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use egui::RichText;

pub fn render(ui: &mut egui::Ui, state: &AppState) {
    let frame = egui::Frame::none()
        .fill(BG_BASE)
        .inner_margin(egui::Margin::symmetric(10.0, 2.0));

    frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, STATUS_H),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                // статус сообщение
                ui.label(
                    RichText::new(&state.status_message)
                        .color(TEXT_DIM)
                        .size(11.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // аггрегированная скорость
                    let agg: u64 = state
                        .queue_tasks
                        .iter()
                        .filter(|t| t.state == crate::domain::transfer::TaskState::Running)
                        .filter_map(|t| t.speed)
                        .sum();

                    if agg > 0 {
                        ui.label(
                            RichText::new(format!("{}/s", format_size(Some(agg))))
                                .color(GREEN)
                                .monospace()
                                .size(11.0),
                        );
                        ui.add_space(10.0);
                    }

                    // соединения
                    let connected = state
                        .tabs
                        .iter()
                        .filter(|t| t.status == ConnectionStatus::Connected)
                        .count();
                    let total = state.tabs.len();

                    if total > 0 {
                        let dot_col = if connected > 0 { GREEN } else { TEXT_HINT };
                        if connected > 0 {
                            icons::icon(ui, Icon::LockPassword, 11.0, GREEN);
                            ui.add_space(4.0);
                        }
                        ui.label(
                            RichText::new(format!("{}/{}", connected, total))
                                .color(dot_col)
                                .size(11.0),
                        );
                    } else {
                        ui.label(RichText::new("Not connected").color(TEXT_HINT).size(11.0));
                    }
                });
            },
        );
    });
}

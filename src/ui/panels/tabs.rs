use crate::domain::connection::ConnectionStatus;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use egui::{Color32, RichText, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState) {
    let frame = egui::Frame::none()
        .fill(BG_BASE)
        .inner_margin(egui::Margin {
            left: 6.0,
            right: 6.0,
            top: 0.0,
            bottom: 0.0,
        });

    frame.show(ui, |ui| {
        ui.set_min_height(TABBAR_H);
        ui.horizontal(|ui| {
            let mut to_close: Option<usize> = None;

            if state.tabs.is_empty() {
                ui.label(RichText::new("No connections").color(TEXT_HINT).size(11.0));
            }

            for (i, tab) in state.tabs.iter().enumerate() {
                let is_active = i == state.active_tab;

                let (dot_col, dot) = match &tab.status {
                    ConnectionStatus::Connected => (GREEN, "●"),
                    ConnectionStatus::Disconnected => (TEXT_HINT, "○"),
                    ConnectionStatus::Connecting => (YELLOW, "◐"),
                    ConnectionStatus::Error(_) => (RED, "×"),
                };

                let bg = if is_active {
                    BG_TAB_ACTIVE
                } else {
                    BG_TAB_IDLE
                };
                let tcol = if is_active { TEXT_PRIMARY } else { TEXT_DIM };

                let tab_response = egui::Frame::none()
                    .fill(bg)
                    .rounding(egui::Rounding {
                        nw: 4.0,
                        ne: 4.0,
                        sw: 0.0,
                        se: 0.0,
                    })
                    .inner_margin(egui::Margin {
                        left: 10.0,
                        right: 4.0,
                        top: 0.0,
                        bottom: 0.0,
                    })
                    .show(ui, |ui| {
                        ui.set_min_height(TABBAR_H);
                        ui.horizontal_centered(|ui| {
                            // цветная точка статуса
                            ui.label(RichText::new(dot).color(dot_col).size(9.0));
                            ui.add_space(4.0);
                            // название вкладки
                            let lbl =
                                egui::Label::new(RichText::new(&tab.label).color(tcol).size(12.0))
                                    .sense(egui::Sense::click());
                            if ui.add(lbl).clicked() {
                                // обрабатывается ниже
                            }
                            ui.add_space(6.0);
                            // кнопка закрытия
                            let close =
                                egui::Label::new(RichText::new("×").color(TEXT_HINT).size(14.0))
                                    .sense(egui::Sense::click());
                            if ui.add(close).clicked() {
                                to_close = Some(i);
                            }
                            ui.add_space(4.0);
                        });
                    });

                if tab_response.response.clicked() {
                    state.active_tab = i;
                }
            }

            // кнопка "+"
            ui.add_space(4.0);
            let plus = egui::Button::new(RichText::new("+").color(TEXT_DIM).size(16.0))
                .fill(Color32::TRANSPARENT)
                .rounding(4.0)
                .min_size(egui::vec2(28.0, TABBAR_H));
            if ui.add(plus).clicked() {
                state.show_connect_dialog = true;
            }

            if let Some(idx) = to_close {
                state.tabs.remove(idx);
                if !state.tabs.is_empty() && state.active_tab >= state.tabs.len() {
                    state.active_tab = state.tabs.len() - 1;
                }
            }
        });
    });

    // нижняя граница таббара
    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);
}

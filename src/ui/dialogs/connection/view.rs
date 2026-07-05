//! Отрисовка диалога «New Connection» (оболочка окна; поля — в [`super::form`]).
use egui::{Align, Color32, Layout, RichText, Ui};
use std::sync::Arc;

use super::actions::do_connect;
use super::form;
// Алиас `tr`: параметр `t` в этой функции — анимационная «открытость» (f32).
use crate::i18n::t as tr;
use crate::fs::remote::RemoteRegistry;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use crate::ui::widgets::{button, overlay};

const DIALOG_WIDTH: f32 = 332.0;

pub fn render(
    ctx: &egui::Context,
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    t: f32,
) {
    let protocols = ["SFTP", "FTP", "FTPS"];
    let default_ports: [u16; 3] = [22, 21, 990];

    // Клик по затемнению закрывает диалог (кроме момента подключения).
    if overlay::dim(ctx, "connect_overlay", t) && !state.connect_loading {
        state.show_connect_dialog = false;
    }

    // `egui::Window` с anchor(CENTER_CENTER) авто-подгоняет размер под контент
    // и всегда центрирует его заново — в отличие от Area с заранее посчитанным
    // размером окна, где реальная высота полей могла разойтись с предположенной
    // и диалог "уезжал" от центра.
    egui::Window::new("connect_dialog")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .anchor(egui::Align2::CENTER_CENTER, overlay::slide_offset(t))
        .interactable(t >= 1.0)
        .frame(
            egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .rounding(RADIUS_LG)
                .shadow(egui::Shadow {
                    offset: egui::vec2(0.0, 10.0),
                    blur: 28.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(130),
                })
                .inner_margin(egui::Margin::same(22.0)),
        )
        .show(ctx, |ui| {
            ui.set_opacity(overlay::ease_out(t));
            ui.set_width(DIALOG_WIDTH);
            header(ui, state);
            ui.add_space(18.0);
            form::protocol_row(ui, state, &protocols, &default_ports);
            ui.add_space(16.0);
            form::fields(ui, state);
            form::error_box(ui, state);
            ui.add_space(18.0);
            buttons(ui, state, registry, rt_handle);

            // Enter из любого поля запускает подключение (submit формы).
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter && t >= 1.0 && state.show_connect_dialog && !state.connect_loading {
                do_connect(state, registry, rt_handle);
            }
        });
}

fn header(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        crate::ui::icons::icon(ui, crate::ui::icons::Icon::ServerSquare, 15.0, ACCENT);
        ui.add_space(8.0);
        ui.label(
            RichText::new("New Connection")
                .color(TEXT_PRIMARY)
                .size(15.5)
                .strong(),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let close = egui::Button::new(RichText::new("✕").color(TEXT_DIM).size(12.0))
                .rounding(RADIUS_SM)
                .min_size(egui::vec2(24.0, 24.0));
            if button::ghost(ui, close).clicked() {
                state.show_connect_dialog = false;
            }
        });
    });
}

fn buttons(
    ui: &mut Ui,
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    ui.horizontal(|ui| {
        let cancel = egui::Button::new(RichText::new("Cancel").color(TEXT_DIM).size(12.5))
            .rounding(RADIUS_MD)
            .min_size(egui::vec2(90.0, 32.0));
        if button::ghost(ui, cancel).clicked() && !state.connect_loading {
            state.show_connect_dialog = false;
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_enabled_ui(!state.connect_loading, |ui| {
                let connect_label = if state.connect_loading {
                    "Connecting…"
                } else {
                    "Connect"
                };
                let connect_btn = egui::Button::new(
                    RichText::new(connect_label)
                        .color(ON_ACCENT)
                        .size(12.5)
                        .strong(),
                )
                .rounding(RADIUS_MD)
                .min_size(egui::vec2(114.0, 32.0));

                if button::accent(ui, connect_btn).clicked() {
                    do_connect(state, registry, rt_handle);
                }
            });

            if state.connect_loading {
                ui.add_space(8.0);
                ui.spinner();
            }
        });
    });
}

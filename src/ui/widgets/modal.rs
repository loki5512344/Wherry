//! Общий каркас модальных диалогов подтверждения/ввода (New Folder, Delete,
//! Rename и подобные) — раскладка окна и ряд кнопок OK/Cancel вынесены сюда,
//! чтобы не дублироваться в каждом диалоге.
use egui::{Align2, Context, RichText, Ui};

use super::{button, overlay};
use crate::ui::theme::*;

pub enum Outcome {
    Idle,
    Confirm,
    Cancel,
}

/// Центрированное окно модалки в общей рамке панели. `t` — анимированная
/// «открытость» из [`overlay::openness`]: прозрачность и лёгкий подъезд снизу;
/// пока окно доигрывает fade-out (t < 1 при закрытии), ввод отключён.
pub fn window(ctx: &Context, id: &str, t: f32, add_contents: impl FnOnce(&mut Ui)) {
    egui::Window::new(id)
        .collapsible(false)
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .anchor(Align2::CENTER_CENTER, overlay::slide_offset(t))
        .interactable(t >= 1.0)
        .frame(overlay::panel_frame())
        .show(ctx, |ui| {
            ui.set_opacity(overlay::ease_out(t));
            ui.set_width(260.0);
            add_contents(ui);
        });
}

/// Ряд кнопок OK/Cancel справа. `danger` — красная кнопка подтверждения.
pub fn ok_cancel_row(ui: &mut Ui, danger: bool) -> Outcome {
    let mut result = Outcome::Idle;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ok = egui::Button::new(
                RichText::new(crate::i18n::t("common.ok"))
                    .color(ON_ACCENT)
                    .size(12.5),
            )
            .rounding(RADIUS_MD)
            .min_size(egui::vec2(70.0, 30.0));
            let ok_resp = if danger {
                button::danger(ui, ok)
            } else {
                button::accent(ui, ok)
            };
            if ok_resp.clicked() {
                result = Outcome::Confirm;
            }
            ui.add_space(8.0);
            let cancel = egui::Button::new(
                RichText::new(crate::i18n::t("common.cancel"))
                    .color(TEXT_DIM)
                    .size(12.5),
            )
            .min_size(egui::vec2(70.0, 30.0));
            if button::ghost(ui, cancel).clicked() {
                result = Outcome::Cancel;
            }
        });
    });
    result
}

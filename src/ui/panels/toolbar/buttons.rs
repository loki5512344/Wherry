//! Кнопки тулбара. Все построены поверх `widgets::button`, поэтому мгновенно
//! реагируют на наведение и нажатие (заливка по состояниям), а не только по клику.
use egui::{RichText, Ui, Vec2};

use crate::ui::icons::{self, Icon};
use crate::ui::theme::*;
use crate::ui::widgets::button;

/// Кнопка с иконкой и текстом; `active` — «включённое» состояние (акцентный текст + подложка).
pub fn text_btn(ui: &mut Ui, icon: Icon, label: &str, active: bool, mut on_click: impl FnMut()) {
    let text_col = if active { ACCENT } else { TEXT_PRIMARY };
    let btn = egui::Button::image_and_text(
        icons::image(icon, 15.0, text_col),
        RichText::new(label).color(text_col).size(12.5),
    )
    .rounding(RADIUS_MD)
    .min_size(Vec2::new(0.0, 30.0));
    if button::toggle(ui, btn, active).clicked() {
        on_click();
    }
}

/// Иконка-переключатель без подписи (History): подсказка по наведению вместо
/// текста рядом — компактнее в узком тулбаре, поведение как у `text_btn`.
pub fn icon_toggle_btn(
    ui: &mut Ui,
    icon: Icon,
    active: bool,
    hover: &str,
    mut on_click: impl FnMut(),
) {
    let col = if active { ACCENT } else { TEXT_DIM };
    let btn = egui::Button::image(icons::image(icon, 16.0, col))
        .rounding(RADIUS_MD)
        .min_size(Vec2::new(30.0, 30.0));
    if button::toggle(ui, btn, active)
        .on_hover_text(hover)
        .clicked()
    {
        on_click();
    }
}

/// Кнопка с иконкой и текстом, которую можно отключить (Upload/Download).
pub fn text_btn_enabled(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    on_click: impl FnMut(),
) {
    ui.add_enabled_ui(enabled, |ui| text_btn(ui, icon, label, false, on_click));
}

/// Иконочная кнопка с подсказкой; серая при disabled.
pub fn icon_btn(ui: &mut Ui, icon: Icon, enabled: bool, hover: &str, mut on_click: impl FnMut()) {
    ui.add_enabled_ui(enabled, |ui| {
        let col = if enabled { TEXT_DIM } else { TEXT_HINT };
        let btn = egui::Button::image(icons::image(icon, 16.0, col))
            .rounding(RADIUS_MD)
            .min_size(Vec2::new(30.0, 30.0));
        if button::ghost(ui, btn).on_hover_text(hover).clicked() {
            on_click();
        }
    });
}

/// Вертикальный разделитель между группами кнопок.
pub fn separator_v(ui: &mut Ui) {
    let (_, rect) = ui.allocate_space(Vec2::new(1.0, 20.0));
    ui.painter().rect_filled(rect, 0.0, BORDER);
}

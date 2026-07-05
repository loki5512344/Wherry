//! Кнопки с корректной обратной связью на наведение/нажатие.
//!
//! egui анимирует фон кнопки только когда её `fill` не задан явно: как только
//! вызван `.fill(...)`, egui выбрасывает визуалы наведения/нажатия и заливка
//! замирает во всех состояниях (см. egui `button.rs`: `fill.unwrap_or(...)`).
//! Поэтому вместо хардкода `.fill(...)` на каждом вызове мы задаём заливку через
//! `weak_bg_fill` по состояниям в scoped-стиле — так каждая кнопка мгновенно
//! реагирует на hover и press.
use egui::{Button, Color32, Response, Stroke, Ui};

use crate::ui::theme::*;

/// Множит RGB на коэффициент (сохраняя непрозрачность): `<1` — темнее, `>1` — светлее.
fn shade(c: Color32, f: f32) -> Color32 {
    let m = |v: u8| ((v as f32) * f).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(m(c.r()), m(c.g()), m(c.b()))
}

/// Добавляет кнопку в scoped-стиле, где заливка по состояниям = переданным цветам,
/// а рамка убрана (плоский вид, обратная связь только заливкой).
fn scoped(
    ui: &mut Ui,
    inactive: Color32,
    hovered: Color32,
    active: Color32,
    btn: Button,
) -> Response {
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        w.inactive.weak_bg_fill = inactive;
        w.inactive.bg_stroke = Stroke::NONE;
        w.hovered.weak_bg_fill = hovered;
        w.hovered.bg_stroke = Stroke::NONE;
        w.active.weak_bg_fill = active;
        w.active.bg_stroke = Stroke::NONE;
        ui.add(btn)
    })
    .inner
}

/// Плоская кнопка: прозрачная в покое, лёгкая подсветка на hover, ACCENT_DIM на press.
pub fn ghost(ui: &mut Ui, btn: Button) -> Response {
    scoped(ui, Color32::TRANSPARENT, BG_ROW_HOVER, ACCENT_DIM, btn)
}

/// Основное действие (Connect / OK / New Connection): сплошной ACCENT, темнеет на hover/press.
pub fn accent(ui: &mut Ui, btn: Button) -> Response {
    scoped(ui, ACCENT, shade(ACCENT, 0.9), shade(ACCENT, 0.78), btn)
}

/// Вторичное акцентное действие (тулбар «New Connection»).
pub fn accent_dim(ui: &mut Ui, btn: Button) -> Response {
    scoped(
        ui,
        ACCENT_DIM,
        shade(ACCENT_DIM, 1.18),
        shade(ACCENT_DIM, 0.85),
        btn,
    )
}

/// Деструктивное действие (Delete): сплошной RED, темнеет на hover/press.
pub fn danger(ui: &mut Ui, btn: Button) -> Response {
    scoped(ui, RED, shade(RED, 0.9), shade(RED, 0.78), btn)
}

/// Кнопка-переключатель: `on` — активна (акцентная подложка), иначе плоская.
pub fn toggle(ui: &mut Ui, btn: Button, on: bool) -> Response {
    if on {
        scoped(
            ui,
            BG_TAB_ACTIVE,
            shade(BG_TAB_ACTIVE, 1.15),
            ACCENT_DIM,
            btn,
        )
    } else {
        ghost(ui, btn)
    }
}

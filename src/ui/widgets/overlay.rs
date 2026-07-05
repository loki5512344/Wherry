//! Общие примитивы модальных окон: затемняющая подложка, рамка панели и
//! анимация «открытости». Раньше каждый диалог (connect/settings/mkdir/delete/
//! rename) строил их копипастой — теперь это одно место (DRY).
use egui::{Color32, Context, Frame, Id, Margin, Order, Sense, Stroke};

use crate::ui::theme::*;

/// Длительность анимации открытия/закрытия модалок.
pub const OPEN_ANIM_SECS: f32 = 0.15;

/// Анимированная «открытость» модалки: 0.0 — закрыта, 1.0 — открыта.
///
/// ВАЖНО: вызывать каждый кадр, в том числе пока диалог закрыт — свежий id в
/// egui::animate_bool сразу прыгает к целевому значению, поэтому без
/// «прогрева» на закрытом состоянии первое открытие происходит без анимации.
/// Окно стоит рисовать, пока значение > 0 (это даёт fade-out после закрытия).
pub fn openness(ctx: &Context, id: &str, open: bool) -> f32 {
    ctx.animate_bool_with_time(Id::new(id).with("openness"), open, OPEN_ANIM_SECS)
}

/// Ease-out для входа: быстро стартует, мягко доезжает.
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Рисует полноэкранную затемняющую подложку под модальным окном; `t` —
/// «открытость» (масштабирует прозрачность). Возвращает `true`, если по ней
/// кликнули (для закрытия по клику вне окна) — клики учитываются только когда
/// подложка уже видима, чтобы догоняющий fade-out не ловил случайные клики.
pub fn dim(ctx: &Context, id: &str, t: f32) -> bool {
    let screen = ctx.screen_rect();
    let alpha = (160.0 * t.clamp(0.0, 1.0)) as u8;
    egui::Area::new(Id::new(id))
        .fixed_pos(egui::Pos2::ZERO)
        .order(Order::Background)
        .show(ctx, |ui| {
            let r = ui.allocate_rect(screen, Sense::click());
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(alpha));
            r.clicked() && t > 0.5
        })
        .inner
}

/// Стандартная рамка панели диалога (фон/рамка/скругление/отступы).
pub fn panel_frame() -> Frame {
    Frame::none()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(RADIUS_LG)
        .inner_margin(Margin::same(16.0))
}

/// Вертикальный сдвиг окна при появлении (лёгкий «подъезд» снизу).
pub fn slide_offset(t: f32) -> egui::Vec2 {
    egui::vec2(0.0, 8.0 * (1.0 - ease_out(t)))
}

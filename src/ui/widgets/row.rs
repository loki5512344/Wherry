//! Кликабельная строка списка (пункты сайдбара, «недавние подключения»).
//!
//! Собрана на `Frame` + `interact(click)`, потому что это не обычная кнопка,
//! а полноширинная строка. Наведение подсвечивается, а курсор становится
//! «рукой» — чтобы любая строка сразу читалась как интерактивная (см. правило
//! про мгновенный визуальный отклик).
use egui::{Align, Color32, Layout, Response, Sense, Ui, Vec2};

use crate::ui::theme::*;

/// Рисует кликабельную строку фиксированной высоты с hover-подсветкой и
/// курсором-рукой. `active` — визуально выделенное состояние (выбранный путь).
/// Возвращает `Response` клика по всей строке.
pub fn clickable_row(
    ui: &mut Ui,
    active: bool,
    height: f32,
    content: impl FnOnce(&mut Ui),
) -> Response {
    let width = ui.available_width();
    let row_rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(width, height));
    let hovered = ui.rect_contains_pointer(row_rect);

    // Hover въезжает/уходит коротким фейдом (~80мс): появление подсветки всё
    // ещё мгновенное на глаз (правило мгновенного отклика), но без резкого
    // мигания при проведении курсора по списку. Выделение (active) — сразу.
    let hover_t =
        ui.ctx()
            .animate_bool_with_time(ui.next_auto_id().with("row_hover"), hovered, 0.08);
    let bg = if active {
        BG_ROW_SEL
    } else if hover_t > 0.0 {
        BG_ROW_HOVER.gamma_multiply(hover_t)
    } else {
        Color32::TRANSPARENT
    };

    let resp = egui::Frame::none()
        .fill(bg)
        .rounding(RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(8.0, 0.0))
        .show(ui, |ui| {
            // На первом кадре, пока свежий (авто-размерный) egui::Window ещё
            // не определил свою ширину, `width` может быть меньше отступа —
            // без clamp'а egui паникует на assert desired_size >= 0.0.
            ui.allocate_ui_with_layout(
                Vec2::new((width - 16.0).max(0.0), height),
                Layout::left_to_right(Align::Center),
                content,
            );
        })
        .response
        .interact(Sense::click());

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

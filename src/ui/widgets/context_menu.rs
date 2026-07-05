//! Единая замена `Response::context_menu()` для всех меню по правому клику
//! (файлы, история, закладки): повторный клик ПКМ рядом с точкой, где меню
//! уже открыто — как в Finder/Explorer — закрывает его, а клик в другом
//! месте (по другой строке) сразу переоткрывает меню там.
//!
//! egui сам умеет закрывать меню кликом мимо и переоткрывать его на другом
//! элементе — этого не хватает только для случая "кликнули ПКМ второй раз
//! по тому же элементу": по умолчанию это просто пересоздаёт меню на месте
//! (no-op на вид), а не закрывает его.
use egui::{Id, Pos2, Response, Ui};

/// Если новый клик ПКМ по тому же элементу оказался ближе этого порога к
/// точке, где меню уже открыто — считаем, что курсор "не сдвинулся".
const SAME_SPOT_PX: f32 = 10.0;

pub fn context_menu(response: &Response, add_contents: impl FnOnce(&mut Ui)) {
    let ctx = &response.ctx;
    let anchor_id = Id::new("ctx_menu_anchor").with(response.id);
    let force_close_id = Id::new("ctx_menu_force_close").with(response.id);

    if response.secondary_clicked()
        && let Some(click_pos) = response.interact_pointer_pos()
    {
        let prev_anchor: Option<Pos2> = ctx.data(|d| d.get_temp(anchor_id));
        let toggle_close = response.context_menu_opened()
            && prev_anchor.is_some_and(|p| p.distance(click_pos) <= SAME_SPOT_PX);

        if toggle_close {
            ctx.data_mut(|d| d.insert_temp(force_close_id, true));
        } else {
            ctx.data_mut(|d| d.insert_temp(anchor_id, click_pos));
        }
    }

    response.context_menu(|ui| {
        let should_close = ui
            .ctx()
            .data_mut(|d| d.get_temp::<bool>(force_close_id))
            .unwrap_or(false);
        if should_close {
            ui.ctx().data_mut(|d| d.remove::<bool>(force_close_id));
            ui.close_menu();
            return;
        }
        add_contents(ui);
    });
}

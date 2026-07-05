//! Всплывающий список недавних подключений (кнопка History в тулбаре).
use egui::{RichText, Ui};

// Алиас `tr`: параметр `t` в этом файле — анимационная «открытость» (f32),
// а не перевод, так что `crate::i18n::t` импортирован под другим именем.
use crate::i18n::t as tr;
use crate::ui::icons::{self, Icon};
use crate::ui::state::AppState;
use crate::ui::theme::*;
use crate::ui::widgets::{button, overlay};

/// `t` — анимированная «открытость» (0 = скрыт, 1 = открыт): короткий fade
/// вместе с лёгким смещением вниз, тот же язык анимации, что и у диалогов.
pub fn render_history_popup(ui: &mut Ui, state: &mut AppState, t: f32) {
    let available = ui.clip_rect();
    let slide = 6.0 * (1.0 - overlay::ease_out(t));
    let popup_rect = egui::Rect::from_min_size(
        egui::pos2(available.max.x - 280.0, TOOLBAR_H + slide),
        egui::vec2(270.0, 200.0),
    );

    let area = egui::Area::new(egui::Id::new("history_popup_area"))
        .fixed_pos(popup_rect.min)
        .order(egui::Order::Foreground)
        .interactable(t >= 1.0)
        .show(ui.ctx(), |ui| {
            ui.set_opacity(overlay::ease_out(t));
            egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .rounding(RADIUS_LG)
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.set_width(260.0);
                    ui.horizontal(|ui| {
                        icons::icon(ui, Icon::ServerSquare, 12.0, TEXT_DIM);
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(tr("history.recent_connections"))
                                .color(TEXT_DIM)
                                .size(11.0)
                                .strong(),
                        );
                    });
                    ui.add_space(4.0);
                    if state.history.is_empty() {
                        ui.label(RichText::new(tr("history.no_history")).color(TEXT_HINT));
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(160.0)
                            .show(ui, |ui| {
                                for entry in &state.history.clone() {
                                    let label =
                                        format!("{}@{}:{}", entry.user, entry.host, entry.port);
                                    let time =
                                        RichText::new(&entry.time).color(TEXT_HINT).size(10.0);
                                    let btn = button::ghost(
                                        ui,
                                        egui::Button::new(
                                            RichText::new(&label).color(TEXT_PRIMARY).size(12.0),
                                        )
                                        .min_size(egui::vec2(240.0, 24.0)),
                                    );
                                    if btn.clicked() {
                                        state.pending_history_reconnect = Some(entry.clone());
                                        state.show_history = false;
                                    }
                                    crate::ui::widgets::context_menu::context_menu(&btn, |ui| {
                                        if ui.button(tr("common.edit")).clicked() {
                                            crate::ui::dialogs::connection::edit_history_entry(
                                                state, entry,
                                            );
                                            state.show_history = false;
                                            ui.close_menu();
                                        }
                                        if ui.button(tr("common.save")).clicked() {
                                            state.pending_history_save = Some(entry.clone());
                                            ui.close_menu();
                                        }
                                    });
                                    ui.label(time);
                                }
                            });
                    }
                });
        });

    // Клик по пустому пространству (где угодно за пределами попапа) закрывает
    // его — единственный способ убрать попап теперь, что кнопки Close нет.
    // Гейт по t>=1.0 (как и interactable выше) не даёт клику, которым попап
    // только открыли, тем же кадром его закрыть.
    if t >= 1.0
        && ui.ctx().input(|i| i.pointer.any_click())
        && ui
            .ctx()
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|pos| !area.response.rect.contains(pos))
    {
        state.show_history = false;
    }
}

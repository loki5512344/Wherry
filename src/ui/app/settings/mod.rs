//! Диалог Settings — popup с меню разделов слева и содержимым справа
//! (General/Appearance/Connections/History/Security/Transfers/About).
use egui::{Align2, Context, RichText, Ui};

use super::FileManagerApp;
use crate::ui::state::SettingsSection;
use crate::ui::theme::*;
use crate::ui::widgets::{button, overlay};

mod about;
mod connections;
mod general;
mod nav;
pub(super) mod prefs;
mod transfers;

const NAV_HEIGHT: f32 = 420.0;

impl FileManagerApp {
    pub(super) fn render_settings_dialog(&mut self, ctx: &Context, t: f32) {
        if overlay::dim(ctx, "settings_overlay", t) {
            self.state.show_settings_dialog = false;
        }

        egui::Window::new("settings_dialog")
            .collapsible(false)
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .anchor(Align2::CENTER_CENTER, overlay::slide_offset(t))
            .interactable(t >= 1.0)
            .frame(overlay::panel_frame())
            .show(ctx, |ui| {
                ui.set_opacity(overlay::ease_out(t));
                ui.set_width(600.0);
                header(ui, &mut self.state.show_settings_dialog);
                ui.add_space(12.0);
                let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
                ui.painter().rect_filled(sep, 0.0, BORDER);
                ui.add_space(14.0);

                ui.horizontal_top(|ui| {
                    // ScrollArea наследует layout родителя, а здесь он
                    // горизонтальный — без ui.vertical() содержимое скроллов
                    // растекается в одну строку и окно вылезает за экран.
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("settings_nav_scroll")
                            .max_height(NAV_HEIGHT)
                            .show(ui, |ui| {
                                nav::render(ui, &mut self.state.settings_section);
                            });
                    });

                    let (_, vsep) = ui.allocate_space(egui::vec2(1.0, NAV_HEIGHT));
                    ui.painter().rect_filled(vsep, 0.0, BORDER);
                    ui.add_space(18.0);

                    ui.vertical(|ui| {
                        // Кросс-фейд содержимого при переключении раздела:
                        // фиксируем момент смены и плавно поднимаем прозрачность.
                        if self.state.settings_section != self.state.settings_prev_section {
                            self.state.settings_prev_section = self.state.settings_section;
                            self.state.settings_section_changed_at = ui.input(|i| i.time);
                        }
                        let fade = ((ui.input(|i| i.time) - self.state.settings_section_changed_at)
                            / 0.18)
                            .clamp(0.0, 1.0) as f32;
                        if fade < 1.0 {
                            ui.ctx().request_repaint();
                        }
                        ui.multiply_opacity(overlay::ease_out(fade));

                        egui::ScrollArea::vertical()
                            .id_salt("settings_content_scroll")
                            .max_height(NAV_HEIGHT)
                            .show(ui, |ui| {
                                ui.set_width(370.0);
                                match self.state.settings_section {
                                    SettingsSection::General => self.render_general(ui),
                                    SettingsSection::Appearance => self.render_appearance(ui),
                                    SettingsSection::Connections => self.render_connections(ui),
                                    SettingsSection::History => self.render_history_section(ui),
                                    SettingsSection::Security => self.render_security(ui),
                                    SettingsSection::Transfers => self.render_transfers(ui),
                                    SettingsSection::About => self.render_about(ui),
                                }
                            });
                    });
                });
            });
    }
}

fn header(ui: &mut Ui, show: &mut bool) {
    ui.horizontal(|ui| {
        crate::ui::icons::icon(ui, crate::ui::icons::Icon::Settings, 16.0, ACCENT);
        ui.add_space(8.0);
        ui.label(
            RichText::new(crate::i18n::t("settings.title"))
                .color(TEXT_PRIMARY)
                .size(15.0)
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = egui::Button::new(RichText::new("✕").color(TEXT_DIM).size(12.0))
                .rounding(RADIUS_SM)
                .min_size(egui::vec2(24.0, 24.0));
            if button::ghost(ui, close).clicked() {
                *show = false;
            }
        });
    });
}

/// Заголовок раздела ("General", "Transfers"...), одинаковый во всех секциях.
pub(super) fn section_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(TEXT_PRIMARY).size(15.0).strong());
    ui.add_space(12.0);
}

/// Подсказка серым текстом под заголовком/контролом.
pub(super) fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(TEXT_HINT).size(11.0));
}

/// Строка "метка — значение" только для чтения (версия, путь к БД и т.п.).
/// Длинные значения (пути) усекаются с «…», полный текст — по наведению.
pub(super) fn value_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_DIM).size(11.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let text = RichText::new(value).color(TEXT_HINT).monospace().size(10.5);
            let resp = ui.add(egui::Label::new(text).truncate());
            if resp.hovered() && value.len() > 32 {
                resp.on_hover_text(value);
            }
        });
    });
    ui.add_space(6.0);
}

/// Пустое состояние списка ("No saved sites yet." и т.п.).
pub(super) fn empty_state(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(TEXT_HINT).size(11.5));
}

/// Строка элемента списка с кнопкой-иконкой справа (удалить/забыть и т.п.).
/// Возвращает `true`, если по кнопке кликнули.
pub(super) fn list_row_with_action(
    ui: &mut Ui,
    label: &str,
    sublabel: &str,
    action_icon: crate::ui::icons::Icon,
    action_hover: &str,
) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).color(TEXT_PRIMARY).size(12.5));
            if !sublabel.is_empty() {
                ui.label(RichText::new(sublabel).color(TEXT_HINT).size(10.5));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let btn = egui::Button::image(crate::ui::icons::image(action_icon, 13.0, TEXT_DIM))
                .rounding(RADIUS_SM)
                .min_size(egui::vec2(26.0, 26.0));
            if button::ghost(ui, btn).on_hover_text(action_hover).clicked() {
                clicked = true;
            }
        });
    });
    ui.add_space(4.0);
    clicked
}

/// Сегментированный ряд из равных по ширине кнопок (используется в Transfers
/// для выбора числа/интервала) — тот же паттерн, что и выбор протокола в
/// диалоге New Connection.
pub(super) fn segmented_row<T: Copy + PartialEq>(
    ui: &mut Ui,
    options: &[(&str, T)],
    current: T,
) -> Option<T> {
    let mut picked = None;
    let gap = 4.0;
    let n = options.len().max(1) as f32;
    // На первом кадре свежего egui::Window ширина ещё не устоялась и может
    // быть меньше суммарных отступов — clamp защищает от negative desired_size.
    let seg_w = ((ui.available_width() - gap * (n - 1.0) - 6.0) / n).max(0.0);

    egui::Frame::none()
        .fill(BG_CONTENT)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .rounding(RADIUS_MD)
        .inner_margin(egui::Margin::same(3.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.horizontal(|ui| {
                for (label, value) in options {
                    let active = *value == current;
                    let tc = if active { ON_ACCENT } else { TEXT_DIM };
                    let btn = egui::Button::new(RichText::new(*label).color(tc).size(11.5))
                        .rounding(RADIUS_SM)
                        .min_size(egui::vec2(seg_w, 26.0));
                    let resp = if active {
                        button::accent(ui, btn)
                    } else {
                        button::ghost(ui, btn)
                    };
                    if resp.clicked() {
                        picked = Some(*value);
                    }
                }
            });
        });
    picked
}

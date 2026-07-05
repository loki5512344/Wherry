//! Settings → General & Appearance: подтверждение удаления, стартовая папка,
//! язык интерфейса, видимость панелей (в согласии с меню Window на macOS).
use egui::{RichText, Ui};

use super::prefs::{KEY_CONFIRM_DELETE, KEY_DEFAULT_LOCAL_FOLDER, KEY_LANGUAGE};
use super::{FileManagerApp, hint, section_title};
use crate::i18n::{Lang, t};
use crate::ui::theme::*;
use crate::ui::widgets::button;

impl FileManagerApp {
    pub(super) fn render_general(&mut self, ui: &mut Ui) {
        section_title(ui, t("settings.nav.general"));

        let changed = ui
            .checkbox(
                &mut self.state.confirm_before_delete,
                RichText::new(t("settings.general.confirm_delete"))
                    .color(TEXT_PRIMARY)
                    .size(12.5),
            )
            .changed();
        hint(ui, t("settings.general.confirm_delete_hint"));
        if changed {
            let value = self.state.confirm_before_delete;
            self.save_pref_bool(KEY_CONFIRM_DELETE, value);
        }

        ui.add_space(18.0);
        ui.label(
            RichText::new(t("settings.general.language"))
                .color(TEXT_PRIMARY)
                .size(12.5)
                .strong(),
        );
        hint(ui, t("settings.general.language_hint"));
        ui.add_space(6.0);
        if let Some(lang) = language_picker(ui) {
            self.set_language(ui, lang);
        }

        ui.add_space(18.0);
        ui.label(
            RichText::new(t("settings.general.startup_folder"))
                .color(TEXT_PRIMARY)
                .size(12.5)
                .strong(),
        );
        hint(ui, t("settings.general.startup_folder_hint"));
        ui.add_space(6.0);

        let current = if self.state.default_local_folder.is_empty() {
            t("settings.general.home_directory").to_string()
        } else {
            self.state.default_local_folder.clone()
        };
        ui.label(
            RichText::new(&current)
                .color(TEXT_DIM)
                .monospace()
                .size(11.0),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let use_current = egui::Button::new(
                RichText::new(t("settings.general.use_current_folder"))
                    .color(TEXT_PRIMARY)
                    .size(11.5),
            )
            .rounding(RADIUS_MD)
            .min_size(egui::vec2(0.0, 28.0));
            if button::ghost(ui, use_current).clicked() {
                let path = self.state.local_path.clone();
                self.state.default_local_folder = path.clone();
                self.save_pref_str(KEY_DEFAULT_LOCAL_FOLDER, &path);
            }

            ui.add_space(8.0);
            let reset = egui::Button::new(
                RichText::new(t("settings.general.reset_to_home"))
                    .color(TEXT_DIM)
                    .size(11.5),
            )
            .rounding(RADIUS_MD)
            .min_size(egui::vec2(0.0, 28.0));
            if button::ghost(ui, reset).clicked() {
                self.state.default_local_folder.clear();
                self.save_pref_str(KEY_DEFAULT_LOCAL_FOLDER, "");
            }
        });
    }

    pub(super) fn render_appearance(&mut self, ui: &mut Ui) {
        section_title(ui, t("settings.nav.appearance"));
        hint(ui, t("settings.appearance.hint"));
        ui.add_space(10.0);

        appearance_checkbox(
            ui,
            t("settings.appearance.toolbar"),
            &mut self.state.show_toolbar,
        );
        appearance_checkbox(
            ui,
            t("settings.appearance.sidebar"),
            &mut self.state.show_sidebar,
        );
        appearance_checkbox(
            ui,
            t("settings.appearance.status_bar"),
            &mut self.state.show_status_bar,
        );
        appearance_checkbox(
            ui,
            t("settings.appearance.transfer_queue"),
            &mut self.state.show_queue_panel,
        );
    }

    /// Сохраняет выбор языка и сразу переключает `t()`/шрифты — эффект виден
    /// в этом же кадре, без перезапуска (см. правило про мгновенный отклик).
    fn set_language(&self, ui: &Ui, lang: Lang) {
        crate::i18n::set_lang(lang);
        self.refresh_fonts(ui.ctx());
        self.save_pref_str(KEY_LANGUAGE, lang.code());
    }
}

fn appearance_checkbox(ui: &mut Ui, label: &str, value: &mut bool) {
    ui.checkbox(value, RichText::new(label).color(TEXT_PRIMARY).size(12.5));
    ui.add_space(4.0);
}

/// Выпадающий список из всех 12 языков — названия на них самих (Español,
/// 日本語...), чтобы найти свой язык, даже не понимая текущего. Возвращает
/// выбранный язык, если пользователь только что кликнул по другому пункту.
fn language_picker(ui: &mut Ui) -> Option<Lang> {
    let current = crate::i18n::current();
    let mut picked = None;
    egui::ComboBox::from_id_salt("language_picker")
        .width(200.0)
        .selected_text(
            RichText::new(current.native_name())
                .color(TEXT_PRIMARY)
                .size(12.0),
        )
        .show_ui(ui, |ui| {
            for lang in Lang::ALL {
                let selected = lang == current;
                if ui.selectable_label(selected, lang.native_name()).clicked() && !selected {
                    picked = Some(lang);
                }
            }
        });
    picked
}

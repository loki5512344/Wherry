//! Settings → About: версия, путь к БД, лицензия.
use egui::{RichText, Ui};

use super::{FileManagerApp, section_title, value_row};
use crate::i18n::t;
use crate::ui::theme::*;
use crate::ui::widgets::button;

impl FileManagerApp {
    pub(super) fn render_about(&mut self, ui: &mut Ui) {
        section_title(ui, t("settings.about.title"));

        value_row(ui, t("settings.about.version"), env!("CARGO_PKG_VERSION"));
        value_row(ui, t("settings.about.license"), env!("CARGO_PKG_LICENSE"));

        let db_path = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("loflum")
            .join("loflum.db");
        value_row(ui, t("settings.about.database"), &db_path.to_string_lossy());

        ui.add_space(10.0);
        let reveal = egui::Button::new(
            RichText::new(t("settings.about.reveal_in_finder"))
                .color(TEXT_PRIMARY)
                .size(11.5),
        )
        .rounding(RADIUS_MD)
        .min_size(egui::vec2(0.0, 28.0));
        if button::ghost(ui, reveal).clicked()
            && let Some(dir) = db_path.parent()
        {
            let _ = crate::fs::local::open(&dir.to_string_lossy());
        }
    }
}

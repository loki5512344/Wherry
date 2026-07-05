//! Settings → Transfers: параллелизм передач и авто-очистка завершённых задач.
use std::sync::atomic::Ordering;

use egui::{RichText, Ui};

use super::prefs::{KEY_AUTO_CLEAR_SECS, KEY_MAX_CONCURRENT};
use super::{FileManagerApp, hint, section_title, segmented_row};
use crate::i18n::t;
use crate::ui::theme::TEXT_PRIMARY;

const CONCURRENCY_OPTIONS: &[(&str, u32)] =
    &[("1", 1), ("2", 2), ("3", 3), ("4", 4), ("5", 5), ("6", 6)];

impl FileManagerApp {
    pub(super) fn render_transfers(&mut self, ui: &mut Ui) {
        section_title(ui, t("settings.nav.transfers"));

        ui.label(
            RichText::new(t("settings.transfers.concurrent_title"))
                .color(TEXT_PRIMARY)
                .size(12.5)
                .strong(),
        );
        hint(ui, t("settings.transfers.concurrent_hint"));
        ui.add_space(6.0);

        let current = self
            .state
            .max_concurrent
            .load(Ordering::Relaxed)
            .clamp(1, 6);
        if let Some(picked) = segmented_row(ui, CONCURRENCY_OPTIONS, current) {
            self.state.max_concurrent.store(picked, Ordering::Relaxed);
            self.save_pref_u32(KEY_MAX_CONCURRENT, picked);
        }

        ui.add_space(18.0);
        ui.label(
            RichText::new(t("settings.transfers.auto_remove_title"))
                .color(TEXT_PRIMARY)
                .size(12.5)
                .strong(),
        );
        hint(ui, t("settings.transfers.auto_remove_hint"));
        ui.add_space(6.0);

        let auto_clear_options: &[(&str, u32)] = &[
            (t("settings.transfers.never"), 0),
            (t("settings.transfers.10s"), 10),
            (t("settings.transfers.30s"), 30),
            (t("settings.transfers.1min"), 60),
        ];
        let current = self.state.auto_clear_completed_secs;
        if let Some(picked) = segmented_row(ui, auto_clear_options, current) {
            self.state.auto_clear_completed_secs = picked;
            self.save_pref_u32(KEY_AUTO_CLEAR_SECS, picked);
        }
    }
}

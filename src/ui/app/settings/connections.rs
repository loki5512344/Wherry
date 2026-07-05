//! Settings → Sites & Bookmarks / History / Security — всё, что относится к
//! сохранённым подключениям, в одном файле (соответствует группе "CONNECTIONS"
//! в левом меню).
use egui::{RichText, Ui};

use super::{FileManagerApp, empty_state, hint, list_row_with_action, section_title};
use crate::i18n::{t, tf};
use crate::ui::icons::Icon;
use crate::ui::theme::*;
use crate::ui::widgets::button;

impl FileManagerApp {
    pub(super) fn render_connections(&mut self, ui: &mut Ui) {
        section_title(ui, t("settings.connections.sites"));
        if self.sites.is_empty() {
            empty_state(ui, t("settings.connections.no_sites"));
        } else {
            let mut remove_idx: Option<usize> = None;
            for (i, site) in self.sites.iter().enumerate() {
                let sublabel = format!("{}@{}:{}", site.username, site.host, site.port);
                if list_row_with_action(
                    ui,
                    &site.name,
                    &sublabel,
                    Icon::Trash,
                    t("settings.connections.delete_site_hover"),
                ) {
                    remove_idx = Some(i);
                }
            }
            if let Some(i) = remove_idx {
                let site = self.sites.remove(i);
                if let Ok(conn) = self.db.lock() {
                    let _ = crate::storage::db::delete_site(&conn, &site.id);
                }
                let _ = crate::storage::keychain::delete_password(&site.id);
                self.state.status_message =
                    tf("settings.connections.removed_site", &[("{name}", &site.name)]);
            }
        }

        ui.add_space(20.0);
        section_title(ui, t("settings.connections.bookmarks_title"));
        if self.state.bookmarks.is_empty() {
            empty_state(ui, t("settings.connections.no_bookmarks"));
        } else {
            let mut remove_id: Option<i64> = None;
            for bm in &self.state.bookmarks {
                if list_row_with_action(
                    ui,
                    &bm.name,
                    &bm.path,
                    Icon::Trash,
                    t("panels.sidebar.remove_bookmark"),
                ) {
                    remove_id = Some(bm.id);
                }
            }
            if let Some(id) = remove_id {
                self.state.bookmarks.retain(|b| b.id != id);
                if let Ok(conn) = self.db.lock() {
                    let _ = crate::storage::db::remove_bookmark(&conn, id);
                }
                self.state.status_message = t("settings.connections.bookmark_removed").into();
            }
        }
    }

    pub(super) fn render_history_section(&mut self, ui: &mut Ui) {
        section_title(ui, t("settings.history.title"));

        if self.state.history.is_empty() {
            empty_state(ui, t("settings.history.empty"));
        } else {
            for entry in &self.state.history {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}@{}:{}", entry.user, entry.host, entry.port))
                            .color(TEXT_PRIMARY)
                            .size(12.5),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(&entry.time).color(TEXT_HINT).size(10.5));
                    });
                });
                ui.add_space(4.0);
            }
        }

        ui.add_space(16.0);

        if self.state.history_clear_confirm {
            hint(ui, t("settings.history.confirm_clear"));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let confirm = egui::Button::new(
                    RichText::new(t("settings.history.delete_all"))
                        .color(ON_ACCENT)
                        .size(12.0),
                )
                .rounding(RADIUS_MD)
                .min_size(egui::vec2(0.0, 28.0));
                if button::danger(ui, confirm).clicked() {
                    if let Ok(conn) = self.db.lock() {
                        let _ = crate::storage::db::clear_history(&conn);
                    }
                    self.state.history.clear();
                    self.state.history_clear_confirm = false;
                    self.state.status_message = t("settings.history.cleared").into();
                }
                ui.add_space(8.0);
                let cancel = egui::Button::new(
                    RichText::new(t("common.cancel")).color(TEXT_DIM).size(12.0),
                )
                .rounding(RADIUS_MD)
                .min_size(egui::vec2(0.0, 28.0));
                if button::ghost(ui, cancel).clicked() {
                    self.state.history_clear_confirm = false;
                }
            });
        } else if !self.state.history.is_empty() {
            let clear = egui::Button::new(
                RichText::new(t("settings.history.clear_all"))
                    .color(RED)
                    .size(12.0),
            )
            .rounding(RADIUS_MD)
            .min_size(egui::vec2(0.0, 28.0));
            if button::ghost(ui, clear).clicked() {
                self.state.history_clear_confirm = true;
            }
        }
    }

    /// Не показываем, есть ли пароль у конкретной записи — это потребовало бы
    /// вызвать `keychain::get_password`, который может дёрнуть системный запрос
    /// доступа к keychain на каждую запись при каждом открытии Settings.
    pub(super) fn render_security(&mut self, ui: &mut Ui) {
        section_title(ui, t("settings.nav.security"));
        hint(ui, t("settings.security.hint"));
        ui.add_space(12.0);

        if self.sites.is_empty() {
            empty_state(ui, t("settings.security.no_connections"));
        } else {
            let mut forget_id: Option<String> = None;
            for site in &self.sites {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&site.name).color(TEXT_PRIMARY).size(12.5));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn = egui::Button::image(crate::ui::icons::image(
                            Icon::LockPassword,
                            13.0,
                            TEXT_DIM,
                        ))
                        .rounding(RADIUS_SM)
                        .min_size(egui::vec2(26.0, 26.0));
                        if button::ghost(ui, btn)
                            .on_hover_text(t("settings.security.forget_password_hover"))
                            .clicked()
                        {
                            forget_id = Some(site.id.clone());
                        }
                    });
                });
                ui.add_space(4.0);
            }
            if let Some(id) = forget_id {
                let _ = crate::storage::keychain::delete_password(&id);
                self.state.status_message = t("settings.security.password_forgotten").into();
            }
        }

        ui.add_space(18.0);
        let forget_all = egui::Button::new(
            RichText::new(t("settings.security.forget_all"))
                .color(RED)
                .size(12.0),
        )
        .rounding(RADIUS_MD)
        .min_size(egui::vec2(0.0, 28.0));
        if button::ghost(ui, forget_all).clicked() {
            for site in &self.sites {
                let _ = crate::storage::keychain::delete_password(&site.id);
            }
            for entry in &self.state.history {
                let _ = crate::storage::keychain::delete_password(&entry.conn_id);
            }
            self.state.status_message = t("settings.security.all_forgotten").into();
        }
    }
}

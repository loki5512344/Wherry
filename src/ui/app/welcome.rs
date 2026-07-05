//! Стартовый экран — пока нет ни одного открытого соединения: логотип,
//! кнопка нового подключения, индикатор подключения и список недавних серверов.
use egui::{CentralPanel, RichText};

use super::FileManagerApp;
use crate::ui::dialogs::connection;
use crate::ui::icons::{self, Icon};
use crate::ui::theme::*;
use crate::ui::widgets::{button, row::clickable_row};

impl FileManagerApp {
    pub(super) fn render_welcome_screen(&mut self, ctx: &egui::Context, fade: f32) {
        CentralPanel::default()
            .frame(egui::Frame::none().fill(BG_BASE))
            .show(ctx, |ui| {
                ui.multiply_opacity(crate::ui::widgets::overlay::ease_out(fade));
                ui.vertical_centered(|ui| {
                    let avail_h = ui.available_height();
                    ui.add_space((avail_h * 0.16).max(24.0));

                    let icon_bytes: &[u8] = include_bytes!("../icons/app_icon.png");
                    let icon_img = egui::Image::from_bytes("bytes://welcome_app_icon", icon_bytes)
                        .rounding(RADIUS_LG)
                        .fit_to_exact_size(egui::vec2(72.0, 72.0));
                    ui.add(icon_img);

                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("LoFlum")
                            .color(TEXT_PRIMARY)
                            .size(20.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new("FTP/SFTP client").color(TEXT_HINT).size(12.5));

                    ui.add_space(20.0);
                    let btn = egui::Button::image_and_text(
                        icons::image(Icon::AddCircleBold, 15.0, ON_ACCENT),
                        RichText::new("New Connection")
                            .color(ON_ACCENT)
                            .size(12.5)
                            .strong(),
                    )
                    .rounding(RADIUS_MD)
                    .min_size(egui::vec2(170.0, 36.0));
                    if button::accent(ui, btn).clicked() {
                        self.state.show_connect_dialog = true;
                    }

                    self.connect_status(ui);

                    ui.add_space(28.0);
                    self.recent_connections(ui);
                });
            });
    }

    /// Мгновенная обратная связь при подключении из истории/приветствия: спиннер,
    /// а при неудаче — текст ошибки (иначе провал был бы беззвучным, т.к. на
    /// welcome-экране нет статус-бара).
    ///
    /// Спиннер — единственный виджет, добавленный напрямую (без `ui.horizontal`):
    /// `ui.horizontal` по умолчанию растягивает свой прямоугольник на всю
    /// доступную ширину панели, и `vertical_centered` центрирует уже растянутый
    /// на всю ширину блок — то есть фактически не центрирует ничего, и
    /// содержимое просто прилипает к левому краю. Одиночный виджет всегда
    /// имеет свой собственный размер, поэтому центрируется корректно.
    fn connect_status(&self, ui: &mut egui::Ui) {
        if self.state.connect_loading {
            ui.add_space(14.0);
            ui.spinner();
        } else if !self.state.connect_error.is_empty() {
            ui.add_space(14.0);
            ui.label(
                RichText::new(&self.state.connect_error)
                    .color(RED)
                    .size(11.5),
            );
        }
    }

    fn recent_connections(&mut self, ui: &mut egui::Ui) {
        if self.state.history.is_empty() {
            return;
        }
        let list_w = 320.0_f32.min(ui.available_width() - 40.0);

        ui.label(
            RichText::new("RECENT CONNECTIONS")
                .color(TEXT_HINT)
                .size(10.5)
                .strong(),
        );
        ui.add_space(8.0);

        egui::Frame::none().show(ui, |ui| {
            ui.set_width(list_w);
            let history: Vec<_> = self.state.history.iter().take(8).cloned().collect();
            for entry in &history {
                let label = format!("{}@{}:{}", entry.user, entry.host, entry.port);
                let resp = clickable_row(ui, false, 34.0, |ui| {
                    icons::icon(ui, Icon::ServerSquare, 13.0, TEXT_DIM);
                    ui.add_space(8.0);
                    ui.label(RichText::new(&label).color(TEXT_PRIMARY).size(12.5));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(&entry.time).color(TEXT_HINT).size(10.5));
                    });
                });
                if resp.clicked() {
                    self.state.pending_history_reconnect = Some(entry.clone());
                }
                crate::ui::widgets::context_menu::context_menu(&resp, |ui| {
                    if ui.button("Edit").clicked() {
                        connection::edit_history_entry(&mut self.state, entry);
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.state.pending_history_save = Some(entry.clone());
                        ui.close_menu();
                    }
                });
            }
        });
    }
}

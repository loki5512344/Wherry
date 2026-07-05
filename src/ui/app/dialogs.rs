//! Модальные диалоги ФС (New Folder / Delete / Rename) — общий каркас модалки
//! (окно/кнопки) вынесен в [`crate::ui::widgets::modal`].
use egui::{Context, RichText, Ui};

use super::FileManagerApp;
// Алиас `tr`: в этом файле `t` — анимационная «открытость» (f32), не перевод.
use crate::i18n::{t as tr, tf};
use crate::ui::icons;
use crate::ui::state::Pane;
use crate::ui::theme::*;
use crate::ui::widgets::modal::{self, Outcome};
use crate::ui::widgets::overlay;

impl FileManagerApp {
    pub(super) fn render_remote_op_dialogs(&mut self, ctx: &Context) {
        // Settings → General → "Confirm before deleting" выключен — удаляем
        // сразу, до расчёта затемнения, чтобы не мелькнуть окном на 1 кадр.
        if self.state.show_delete_dialog && !self.state.confirm_before_delete {
            let name = self.state.delete_name.clone();
            self.dispatch_delete(name);
            self.state.show_delete_dialog = false;
        }

        // «Открытость» считается каждый кадр для каждого диалога (см. док к
        // overlay::openness) — окна дорисовываются, пока доигрывает fade-out.
        let t_mkdir = overlay::openness(ctx, "mkdir_dlg", self.state.show_mkdir_dialog);
        let t_delete = overlay::openness(ctx, "delete_dlg", self.state.show_delete_dialog);
        let t_rename = overlay::openness(ctx, "rename_dlg", self.state.show_rename_dialog);

        let t_any = t_mkdir.max(t_delete).max(t_rename);
        if t_any > 0.0 {
            let _ = overlay::dim(ctx, "remote_op_overlay", t_any);
        }

        if t_mkdir > 0.0 {
            match text_modal(
                ctx,
                "new_folder_dialog",
                t_mkdir,
                tr("common.new_folder"),
                &mut self.state.mkdir_name,
            ) {
                Outcome::Confirm if !self.state.mkdir_name.is_empty() => {
                    let name = self.state.mkdir_name.clone();
                    self.dispatch_mkdir(name);
                    self.state.show_mkdir_dialog = false;
                }
                // Поля не чистим при отмене: открыватели инициализируют их
                // сами, а очистка здесь заставила бы контент мигать в fade-out.
                Outcome::Cancel => self.state.show_mkdir_dialog = false,
                _ => {}
            }
        }

        if t_delete > 0.0 {
            let name = self.state.delete_name.clone();
            match confirm_modal(ctx, "delete_dialog", t_delete, &name) {
                Outcome::Confirm => {
                    self.dispatch_delete(name);
                    self.state.show_delete_dialog = false;
                }
                Outcome::Cancel => self.state.show_delete_dialog = false,
                _ => {}
            }
        }

        if t_rename > 0.0 {
            match text_modal(
                ctx,
                "rename_dialog",
                t_rename,
                tr("common.rename"),
                &mut self.state.rename_new_name,
            ) {
                Outcome::Confirm if !self.state.rename_new_name.is_empty() => {
                    let old_name = self.state.rename_old_name.clone();
                    let new_name = self.state.rename_new_name.clone();
                    self.dispatch_rename(old_name, new_name);
                    self.state.show_rename_dialog = false;
                }
                Outcome::Cancel => self.state.show_rename_dialog = false,
                _ => {}
            }
        }
    }

    fn dispatch_mkdir(&mut self, name: String) {
        match self.state.op_target {
            Pane::Local => self.start_local_mkdir(name),
            Pane::Remote => {
                if let Some(idx) = self.active_tab_idx() {
                    self.start_mkdir(idx, name);
                }
            }
        }
    }

    fn dispatch_delete(&mut self, name: String) {
        match self.state.op_target {
            Pane::Local => self.start_local_delete(name),
            Pane::Remote => {
                if let Some(idx) = self.active_tab_idx() {
                    self.start_delete(idx, name);
                }
            }
        }
    }

    fn dispatch_rename(&mut self, old_name: String, new_name: String) {
        match self.state.op_target {
            Pane::Local => self.start_local_rename(old_name, new_name),
            Pane::Remote => {
                if let Some(idx) = self.active_tab_idx() {
                    self.start_rename(idx, old_name, new_name);
                }
            }
        }
    }
}

/// Модалка с одним текстовым полем (New Folder / Rename): фокус в поле сразу,
/// Enter подтверждает.
fn text_modal(ctx: &Context, id: &str, t: f32, title: &str, value: &mut String) -> Outcome {
    let mut outcome = Outcome::Idle;
    modal::window(ctx, id, t, |ui| {
        ui.label(RichText::new(title).color(TEXT_PRIMARY).strong());
        ui.add_space(12.0);
        let resp = ui.text_edit_singleline(value);
        if t >= 1.0 {
            resp.request_focus();
        }
        let entered = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.add_space(16.0);
        let row = modal::ok_cancel_row(ui, false);
        outcome = if entered { Outcome::Confirm } else { row };
    });
    outcome
}

/// Диалог подтверждения удаления (красная кнопка OK).
fn confirm_modal(ctx: &Context, id: &str, t: f32, name: &str) -> Outcome {
    let mut outcome = Outcome::Idle;
    modal::window(ctx, id, t, |ui: &mut Ui| {
        ui.horizontal(|ui| {
            icons::icon(ui, icons::Icon::DangerTriangle, 18.0, RED);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(tf("dialogs.confirm_delete_title", &[("{name}", name)]))
                        .color(TEXT_PRIMARY)
                        .strong(),
                );
                ui.label(
                    RichText::new(tr("dialogs.cannot_undo"))
                        .color(TEXT_DIM)
                        .size(11.0),
                );
            });
        });
        ui.add_space(16.0);
        outcome = modal::ok_cancel_row(ui, true);
    });
    outcome
}

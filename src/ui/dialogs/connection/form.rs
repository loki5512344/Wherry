//! Поля диалога подключения: сегментированный выбор протокола, текстовые
//! поля с рамкой и подсветкой фокуса, блок ошибки.
use egui::{Color32, RichText, Stroke, TextEdit, Ui};

use crate::ui::state::AppState;
use crate::ui::theme::*;
use crate::ui::widgets::button;

const FIELD_HEIGHT: f32 = 32.0;

/// Сегментированный переключатель протокола — три равные по ширине секции
/// в одной рамке, а не отдельные кнопки вразнобой (было похоже на кривой
/// список из-за неравных промежутков и незаполненного остатка ширины).
pub(super) fn protocol_row(
    ui: &mut Ui,
    state: &mut AppState,
    protocols: &[&str; 3],
    default_ports: &[u16; 3],
) {
    let gap = 4.0;
    let total_w = ui.available_width();
    let seg_w = ((total_w - gap * 2.0 - 6.0) / 3.0).max(0.0);

    egui::Frame::none()
        .fill(BG_CONTENT)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(RADIUS_MD)
        .inner_margin(egui::Margin::same(3.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.horizontal(|ui| {
                for (i, p) in protocols.iter().enumerate() {
                    let active = state.connect_protocol == i;
                    let tc = if active { ON_ACCENT } else { TEXT_DIM };
                    let btn = egui::Button::new(RichText::new(*p).color(tc).size(11.5).strong())
                        .rounding(RADIUS_SM)
                        .min_size(egui::vec2(seg_w, 26.0));
                    let resp = if active {
                        button::accent(ui, btn)
                    } else {
                        button::ghost(ui, btn)
                    };
                    if resp.clicked() {
                        state.connect_protocol = i;
                        state.connect_port = default_ports[i].to_string();
                    }
                }
            });
        });
}

pub(super) fn fields(ui: &mut Ui, state: &mut AppState) {
    let field_w = ui.available_width();

    field(ui, "Host", &mut state.connect_host, field_w, false);
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        let user_w = (field_w - 80.0 - 8.0).max(0.0);
        field(ui, "Username", &mut state.connect_user, user_w, false);
        ui.add_space(8.0);
        field(ui, "Port", &mut state.connect_port, 80.0, false);
    });
    ui.add_space(10.0);

    field(ui, "Password", &mut state.connect_pass, field_w, true);

    if state.connect_protocol == 0 {
        ui.add_space(3.0);
        ui.label(
            RichText::new("Leave empty to sign in with an SSH key (agent or ~/.ssh).")
                .color(TEXT_HINT)
                .size(10.0),
        );
        ui.add_space(10.0);
        key_field(ui, state, field_w);
    }

    ui.add_space(10.0);
    field(
        ui,
        "Label (optional)",
        &mut state.connect_label,
        field_w,
        false,
    );
}

pub(super) fn error_box(ui: &mut Ui, state: &AppState) {
    if state.connect_error.is_empty() {
        return;
    }
    let bg = Color32::from_rgba_unmultiplied(RED.r(), RED.g(), RED.b(), 26);
    let border = Color32::from_rgba_unmultiplied(RED.r(), RED.g(), RED.b(), 100);
    ui.add_space(12.0);
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .rounding(RADIUS_MD)
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                crate::ui::icons::icon(ui, crate::ui::icons::Icon::DangerTriangle, 14.0, RED);
                ui.add_space(6.0);
                ui.label(RichText::new(&state.connect_error).color(RED).size(11.0));
            });
        });
}

/// Поле пути к SSH-ключу с кнопкой Browse… (нативный диалог выбора файла).
fn key_field(ui: &mut Ui, state: &mut AppState, field_w: f32) {
    const BROWSE_W: f32 = 72.0;
    ui.label(
        RichText::new("Key file (optional)")
            .color(TEXT_DIM)
            .size(11.0),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        field_input(
            ui,
            &mut state.connect_key_path,
            (field_w - BROWSE_W - 8.0).max(0.0),
            false,
        );
        ui.add_space(8.0);
        let browse = egui::Button::new(RichText::new("Browse…").color(TEXT_DIM).size(11.5))
            .rounding(RADIUS_MD)
            .min_size(egui::vec2(BROWSE_W, FIELD_HEIGHT));
        if button::ghost(ui, browse).clicked() {
            let ssh_dir = dirs::home_dir()
                .map(|h| h.join(".ssh"))
                .filter(|d| d.is_dir())
                .or_else(dirs::home_dir);
            let mut dlg = rfd::FileDialog::new().set_title("Choose SSH key");
            if let Some(dir) = ssh_dir {
                dlg = dlg.set_directory(dir);
            }
            if let Some(path) = dlg.pick_file() {
                state.connect_key_path = path.display().to_string();
            }
        }
    });
}

/// Текстовое поле с меткой, рамкой и подсветкой фокуса/hover в акцентном цвете —
/// раньше поля были обычными TextEdit без рамки, из-за чего вся форма выглядела
/// незакреплённой/неровной на фоне остального интерфейса с рамками у карточек.
fn field(ui: &mut Ui, label: &str, value: &mut String, width: f32, password: bool) {
    ui.label(RichText::new(label).color(TEXT_DIM).size(11.0));
    ui.add_space(4.0);
    field_input(ui, value, width, password);
}

/// Само поле ввода без метки (для составных рядов вроде «путь + Browse»).
fn field_input(ui: &mut Ui, value: &mut String, width: f32, password: bool) {
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        w.inactive.rounding = RADIUS_MD.into();
        w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        w.hovered.rounding = RADIUS_MD.into();
        w.hovered.bg_stroke = Stroke::new(1.0, ACCENT_DIM);
        ui.visuals_mut().selection.stroke = Stroke::new(1.0, ACCENT);

        let te = TextEdit::singleline(value)
            .password(password)
            .desired_width(width)
            .min_size(egui::vec2(width, FIELD_HEIGHT))
            .margin(egui::Margin::symmetric(10.0, 8.0))
            .font(egui::TextStyle::Body);
        ui.add(te);
    });
}

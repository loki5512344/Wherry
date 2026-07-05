use crate::domain::transfer::TransferTask;
use crate::domain::transfer::{TaskState, TransferKind};
use crate::ui::icons::{self, Icon};
use crate::ui::panels::file_pane::format_size;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use crate::ui::widgets::button;
use egui::{RichText, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState, tasks: &[TransferTask]) {
    // Заголовок очереди (всегда виден)
    let header_frame = egui::Frame::none()
        .fill(BG_QUEUE)
        .inner_margin(egui::Margin::symmetric(12.0, 0.0));

    header_frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(width, QUEUE_COLLAPSED_H),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let chevron = if state.show_queue {
                    Icon::ArrowDown
                } else {
                    Icon::ArrowUp
                };
                let n_active = tasks
                    .iter()
                    .filter(|t| matches!(t.state, TaskState::Running | TaskState::Queued))
                    .count();

                let n = if n_active > 0 { n_active } else { tasks.len() };
                let title = crate::i18n::tf("queue.title", &[("{n}", &n.to_string())]);

                let btn = egui::Button::image_and_text(
                    icons::image(chevron, 13.0, TEXT_DIM),
                    RichText::new(&title)
                        .color(TEXT_PRIMARY)
                        .size(11.5)
                        .strong(),
                )
                .min_size(egui::vec2(0.0, QUEUE_COLLAPSED_H));

                if button::ghost(ui, btn).clicked() {
                    state.show_queue = !state.show_queue;
                }

                // Аггрегированная скорость справа
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let agg_speed: u64 = tasks
                        .iter()
                        .filter(|t| t.state == TaskState::Running)
                        .filter_map(|t| t.speed)
                        .sum();

                    if agg_speed > 0 {
                        ui.label(
                            RichText::new(format!("{}/s", format_size(Some(agg_speed))))
                                .color(GREEN)
                                .size(11.0),
                        );
                    } else if !tasks.is_empty() {
                        let done = tasks
                            .iter()
                            .filter(|t| t.state == TaskState::Completed)
                            .count();
                        let failed = tasks
                            .iter()
                            .filter(|t| matches!(t.state, TaskState::Failed(_)))
                            .count();
                        ui.label(
                            RichText::new(crate::i18n::tf(
                                "queue.done_suffix",
                                &[("{done}", &done.to_string()), ("{total}", &tasks.len().to_string())],
                            ))
                            .color(TEXT_DIM)
                            .size(11.0),
                        );
                        if failed > 0 {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(crate::i18n::tf(
                                    "queue.failed_suffix",
                                    &[("{n}", &failed.to_string())],
                                ))
                                .color(RED)
                                .size(11.0),
                            );
                        }
                    }
                });
            },
        );
    });

    if !state.show_queue || tasks.is_empty() {
        return;
    }

    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, BORDER);

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .max_height(QUEUE_EXPANDED_H - QUEUE_COLLAPSED_H - 4.0)
        .show(ui, |ui| {
            ui.add_space(2.0);
            for (i, task) in tasks.iter().enumerate() {
                if i > 0 {
                    ui.add_space(6.0);
                }
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(12.0, 0.0))
                    .show(ui, |ui| render_task(ui, task));
            }
        });
}

fn render_task(ui: &mut Ui, task: &TransferTask) {
    // Плоская однострочная карточка, как в макете — без цветной подложки
    ui.horizontal(|ui| {
        ui.set_min_height(34.0);

        let (arrow, arrow_col) = match task.kind {
            TransferKind::Upload => (Icon::Upload, ACCENT),
            TransferKind::Download => (Icon::Download, GREEN),
        };
        icons::icon(ui, arrow, 15.0, arrow_col);
        ui.add_space(10.0);

        // имя файла — фиксированная ширина, как в спеке
        let name_label = egui::Label::new(
            RichText::new(&task.file_name)
                .color(TEXT_PRIMARY)
                .size(12.0),
        )
        .truncate();
        let name_resp = ui.add_sized([150.0, 18.0], name_label);
        if let TaskState::Failed(err) = &task.state {
            name_resp.on_hover_text(err);
        }
        ui.add_space(10.0);

        let pct = task.progress_pct() as f32 / 100.0;
        let bar_col = match &task.state {
            TaskState::Completed => GREEN,
            TaskState::Failed(_) => RED,
            TaskState::Paused => TEXT_HINT,
            _ => ACCENT,
        };
        let bar = egui::ProgressBar::new(pct)
            .desired_width(ui.available_width() - 130.0)
            .animate(matches!(task.state, TaskState::Running))
            .fill(bar_col);
        ui.add(bar);
        ui.add_space(10.0);

        ui.label(
            RichText::new(format!("{:.0}%", task.progress_pct()))
                .color(TEXT_DIM)
                .monospace()
                .size(11.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (state_str, state_col, state_icon) = match &task.state {
                TaskState::Queued => (crate::i18n::t("queue.state_queued").to_string(), TEXT_HINT, None),
                TaskState::Running => match task.speed {
                    Some(s) if s > 0 => (format!("{}/s", format_size(Some(s))), GREEN, None),
                    _ => {
                        let verb = match task.kind {
                            TransferKind::Upload => "queue.state_uploading",
                            TransferKind::Download => "queue.state_downloading",
                        };
                        (crate::i18n::t(verb).to_string(), ACCENT, None)
                    }
                },
                TaskState::Completed => (
                    crate::i18n::t("queue.state_complete").to_string(),
                    GREEN,
                    Some(Icon::CheckCircle),
                ),
                TaskState::Failed(_) => (crate::i18n::t("queue.state_failed").to_string(), RED, None),
                TaskState::Paused => (
                    crate::i18n::t("queue.state_paused").to_string(),
                    YELLOW,
                    Some(Icon::PlayCircle),
                ),
                TaskState::Cancelled => {
                    (crate::i18n::t("queue.state_cancelled").to_string(), TEXT_HINT, None)
                }
                TaskState::Retrying(_) => {
                    (crate::i18n::t("queue.state_retrying").to_string(), YELLOW, None)
                }
            };
            if let Some(ic) = state_icon {
                icons::icon(ui, ic, 13.0, state_col);
                ui.add_space(4.0);
            }
            ui.label(RichText::new(state_str).color(state_col).size(10.5));
        });
    });
}

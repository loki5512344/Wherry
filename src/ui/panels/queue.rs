use crate::domain::transfer::TransferTask;
use crate::domain::transfer::{TaskState, TransferKind};
use crate::ui::panels::file_pane::format_size;
use crate::ui::state::AppState;
use crate::ui::theme::*;
use egui::{Color32, RichText, Ui};

pub fn render(ui: &mut Ui, state: &mut AppState, tasks: &[TransferTask]) {
    // Заголовок очереди (всегда виден)
    let header_frame = egui::Frame::none()
        .fill(BG_QUEUE)
        .inner_margin(egui::Margin::symmetric(8.0, 0.0));

    header_frame.show(ui, |ui| {
        ui.set_min_height(QUEUE_COLLAPSED_H);
        ui.horizontal_centered(|ui| {
            let arrow = if state.show_queue { "▼" } else { "▲" };
            let n_active = tasks
                .iter()
                .filter(|t| matches!(t.state, TaskState::Running | TaskState::Queued))
                .count();

            let title = if n_active > 0 {
                format!("{} Transfer Queue  ({})", arrow, n_active)
            } else {
                format!("{} Transfer Queue  ({})", arrow, tasks.len())
            };

            let btn = egui::Button::new(
                RichText::new(&title)
                    .color(TEXT_PRIMARY)
                    .size(12.0)
                    .strong(),
            )
            .fill(Color32::TRANSPARENT)
            .min_size(egui::vec2(0.0, QUEUE_COLLAPSED_H));

            if ui.add(btn).clicked() {
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
                        RichText::new(format!("{}/{} done", done, tasks.len()))
                            .color(TEXT_DIM)
                            .size(11.0),
                    );
                    if failed > 0 {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!("{} failed", failed))
                                .color(RED)
                                .size(11.0),
                        );
                    }
                }
            });
        });
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
            for task in tasks {
                render_task(ui, task);
            }
        });
}

fn render_task(ui: &mut Ui, task: &TransferTask) {
    let bg = match &task.state {
        TaskState::Completed => Color32::from_rgb(22, 38, 24),
        TaskState::Failed(_) => Color32::from_rgb(38, 22, 22),
        TaskState::Running => Color32::from_rgb(22, 28, 42),
        TaskState::Queued => BG_QUEUE,
        _ => BG_QUEUE,
    };

    egui::Frame::none()
        .fill(bg)
        .inner_margin(egui::Margin {
            left: 10.0,
            right: 10.0,
            top: 4.0,
            bottom: 4.0,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // стрелка вида
                let (arrow, arrow_col) = match task.kind {
                    TransferKind::Upload => ("↑", ACCENT),
                    TransferKind::Download => ("↓", GREEN),
                };
                ui.label(RichText::new(arrow).color(arrow_col).size(13.0).strong());
                ui.add_space(6.0);

                // имя файла
                ui.label(
                    RichText::new(&task.file_name)
                        .color(TEXT_PRIMARY)
                        .size(12.0),
                );

                // статус справа
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (state_str, state_col) = match &task.state {
                        TaskState::Queued => ("Queued", TEXT_HINT),
                        TaskState::Running => ("Uploading", ACCENT),
                        TaskState::Completed => ("✓ Done", GREEN),
                        TaskState::Failed(_) => ("× Failed", RED),
                        TaskState::Paused => ("Paused", YELLOW),
                        TaskState::Cancelled => ("Cancelled", TEXT_HINT),
                        TaskState::Retrying(_) => ("Retrying", YELLOW),
                    };
                    ui.label(RichText::new(state_str).color(state_col).size(11.0));
                });
            });

            // прогресс
            ui.horizontal(|ui| {
                let pct = task.progress_pct() as f32 / 100.0;
                let bar_col = match &task.state {
                    TaskState::Completed => GREEN,
                    TaskState::Failed(_) => RED,
                    _ => ACCENT,
                };

                let bar = egui::ProgressBar::new(pct)
                    .text(format!("{:.0}%", task.progress_pct()))
                    .desired_width(ui.available_width() * 0.55)
                    .fill(bar_col);
                ui.add(bar);

                ui.label(
                    RichText::new(format!(
                        "  {}/{}",
                        format_size(Some(task.transferred_bytes)),
                        format_size(Some(task.total_bytes))
                    ))
                    .color(TEXT_DIM)
                    .size(11.0),
                );

                if let Some(speed) = task.speed
                    && speed > 0
                {
                    ui.label(
                        RichText::new(format!("  {}/s", format_size(Some(speed))))
                            .color(GREEN)
                            .size(11.0),
                    );
                }
                if let Some(eta) = task.eta_secs
                    && eta > 0
                {
                    ui.label(
                        RichText::new(format!("  ETA {}s", eta))
                            .color(TEXT_HINT)
                            .size(10.0),
                    );
                }
            });

            // ошибка если есть
            if let TaskState::Failed(err) = &task.state {
                ui.label(RichText::new(err).color(RED).size(10.0));
            }
        });

    // разделитель
    let (_, sep) = ui.allocate_space(egui::vec2(ui.available_width(), 1.0));
    ui.painter().rect_filled(sep, 0.0, SEPARATOR);
}

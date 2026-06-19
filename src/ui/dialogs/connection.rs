use egui::{Color32, RichText, TextEdit, Ui};
use std::sync::Arc;

use crate::domain::connection::{ConnectionParams, Protocol};
use crate::domain::file_entry::FileEntry;
use crate::fs::remote::RemoteRegistry;
use crate::protocols::{ftp::FtpClient, sftp::SftpClient, RemoteFs};
use crate::storage::keychain;
use crate::ui::state::{AppState, PendingConnect};
use crate::ui::theme::*;

pub fn render(
    ctx: &egui::Context,
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    let protocols = ["SFTP", "FTP", "FTPS"];
    let default_ports: [u16; 3] = [22, 21, 990];

    // Центрируем окно
    let screen = ctx.screen_rect();
    let win_size = egui::vec2(420.0, 340.0);
    let win_pos = egui::pos2(
        (screen.width() - win_size.x) * 0.5,
        (screen.height() - win_size.y) * 0.4,
    );

    egui::Area::new(egui::Id::new("connect_overlay"))
        .fixed_pos(egui::Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let r = ui.allocate_rect(screen, egui::Sense::click());
            ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(160));
            if r.clicked() && !state.connect_loading {
                state.show_connect_dialog = false;
            }
        });

    egui::Area::new(egui::Id::new("connect_dialog"))
        .fixed_pos(win_pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgb(30, 30, 34))
                .stroke(egui::Stroke::new(1.0, BORDER))
                .rounding(10.0)
                .shadow(egui::Shadow {
                    offset: egui::vec2(0.0, 8.0),
                    blur: 24.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(120),
                })
                .inner_margin(egui::Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_width(win_size.x - 48.0);

                    // Заголовок
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("New Connection").color(TEXT_PRIMARY).size(16.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close = egui::Button::new(RichText::new("×").color(TEXT_HINT).size(14.0))
                                .fill(Color32::TRANSPARENT);
                            if ui.add(close).clicked() {
                                state.show_connect_dialog = false;
                            }
                        });
                    });

                    ui.add_space(16.0);

                    // Протокол — переключатели
                    ui.horizontal(|ui| {
                        for (i, p) in protocols.iter().enumerate() {
                            let active = state.connect_protocol == i;
                            let bg = if active { ACCENT } else { Color32::from_rgb(40, 40, 44) };
                            let tc = if active { Color32::WHITE } else { TEXT_DIM };
                            let btn = egui::Button::new(RichText::new(*p).color(tc).size(12.0))
                                .fill(bg)
                                .rounding(4.0)
                                .min_size(egui::vec2(64.0, 28.0));
                            if ui.add(btn).clicked() {
                                state.connect_protocol = i;
                                state.connect_port = default_ports[i].to_string();
                            }
                        }
                    });

                    ui.add_space(14.0);

                    // Поля ввода
                    let field_w = ui.available_width();
                    field(ui, "Label (optional)", &mut state.connect_label, field_w, false);
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        let host_w = field_w - 80.0 - 8.0;
                        field(ui, "Host", &mut state.connect_host, host_w, false);
                        ui.add_space(8.0);
                        field(ui, "Port", &mut state.connect_port, 80.0, false);
                    });
                    ui.add_space(8.0);

                    field(ui, "Username", &mut state.connect_user, field_w, false);
                    ui.add_space(8.0);
                    field(ui, "Password", &mut state.connect_pass, field_w, true);

                    if state.connect_protocol == 0 {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            field(ui, "Key path", &mut state.connect_key_path, field_w - 80.0 - 8.0, false);
                            ui.add_space(8.0);
                            // Browse placeholder — open system picker if rfd added
                    let browse = egui::Button::new(RichText::new("Browse").color(TEXT_DIM).size(11.0))
                                .fill(Color32::from_rgb(40, 40, 44))
                                .min_size(egui::vec2(72.0, 32.0))
                                .rounding(4.0);
                            ui.add_enabled(false, browse);
                        });
                    }

                    // Ошибка
                    if !state.connect_error.is_empty() {
                        ui.add_space(10.0);
                        egui::Frame::none()
                            .fill(Color32::from_rgb(60, 25, 25))
                            .rounding(4.0)
                            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(&state.connect_error).color(RED).size(11.0));
                            });
                    }

                    ui.add_space(16.0);

                    // Кнопки
                    ui.horizontal(|ui| {
                        let cancel = egui::Button::new(RichText::new("Cancel").color(TEXT_DIM).size(12.0))
                            .fill(Color32::from_rgb(40, 40, 44))
                            .rounding(6.0)
                            .min_size(egui::vec2(90.0, 34.0));
                        if ui.add(cancel).clicked() && !state.connect_loading {
                            state.show_connect_dialog = false;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_enabled_ui(!state.connect_loading, |ui| {
                                let connect_label = if state.connect_loading {
                                    "Connecting…"
                                } else {
                                    "Connect"
                                };
                                let connect_btn = egui::Button::new(
                                    RichText::new(connect_label).color(Color32::WHITE).size(12.0).strong()
                                )
                                .fill(ACCENT)
                                .rounding(6.0)
                                .min_size(egui::vec2(110.0, 34.0));

                                if ui.add(connect_btn).clicked() {
                                    do_connect(state, registry, rt_handle);
                                }
                            });

                            if state.connect_loading {
                                ui.add_space(8.0);
                                ui.spinner();
                            }
                        });
                    });
                });
        });
}

fn field(ui: &mut Ui, label: &str, value: &mut String, width: f32, password: bool) {
    ui.label(RichText::new(label).color(TEXT_DIM).size(11.0));
    let te = TextEdit::singleline(value)
        .password(password)
        .desired_width(width)
        .margin(egui::Margin::symmetric(8.0, 6.0))
        .font(egui::TextStyle::Body);
    ui.add(te);
}

fn do_connect(
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    state.connect_error.clear();

    let protocol = match state.connect_protocol {
        0 => Protocol::Sftp,
        1 => Protocol::Ftp,
        _ => Protocol::Ftps,
    };

    let port: u16 = state.connect_port.parse().unwrap_or(match protocol {
        Protocol::Sftp => 22,
        Protocol::Ftp  => 21,
        Protocol::Ftps => 990,
    });

    let params = ConnectionParams {
        id: uuid::Uuid::new_v4().to_string(),
        label: if state.connect_label.is_empty() {
            format!("{} ({})", state.connect_host, match protocol {
                Protocol::Sftp => "SFTP",
                Protocol::Ftp  => "FTP",
                Protocol::Ftps => "FTPS",
            })
        } else {
            state.connect_label.clone()
        },
        protocol,
        host: state.connect_host.clone(),
        port,
        username: state.connect_user.clone(),
        password: if state.connect_pass.is_empty() { None } else { Some(state.connect_pass.clone()) },
        key_path: if state.connect_key_path.is_empty() { None } else { Some(state.connect_key_path.clone()) },
    };

    let registry = registry.clone();
    let params_clone = params.clone();
    let result = Arc::new(std::sync::Mutex::new(None));
    let result_clone = result.clone();

    state.connect_loading = true;
    rt_handle.spawn(async move {
        let r = connect_async(&registry, &params_clone).await;
        *result_clone.lock().unwrap() = Some(r);
    });

    state.pending_connect = Some(PendingConnect { result });
}

async fn connect_async(
    registry: &RemoteRegistry,
    params: &ConnectionParams,
) -> Result<(ConnectionParams, Vec<FileEntry>), String> {
    let password = if let Some(p) = &params.password {
        p.clone()
    } else {
        keychain::get_password(&params.id).map_err(|e| e.to_string())?
    };

    let fs: Arc<dyn RemoteFs> = match params.protocol {
        Protocol::Sftp => {
            if let Some(key_path) = &params.key_path {
                Arc::new(
                    SftpClient::connect_key(&params.host, params.port, &params.username, key_path)
                        .map_err(|e| e.to_string())?,
                )
            } else {
                Arc::new(
                    SftpClient::connect_password(&params.host, params.port, &params.username, &password)
                        .map_err(|e| e.to_string())?,
                )
            }
        }
        Protocol::Ftp => Arc::new(
            FtpClient::connect(&params.host, params.port, &params.username, &password)
                .await
                .map_err(|e| e.to_string())?,
        ),
        Protocol::Ftps => return Err("FTPS not yet implemented".into()),
    };

    registry.insert(params.id.clone(), fs.clone());
    let entries = fs.list("/").await.map_err(|e| e.to_string())?;
    Ok((params.clone(), entries))
}

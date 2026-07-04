use egui::{Color32, RichText, TextEdit, Ui};
use std::sync::Arc;

use crate::domain::connection::{ConnectionParams, Protocol};
use crate::domain::file_entry::FileEntry;
use crate::fs::remote::RemoteRegistry;
use crate::protocols::{RemoteFs, ftp::FtpClient, sftp::SftpClient};
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
    let win_size = egui::vec2(380.0, 340.0);
    let win_pos = egui::pos2(
        (screen.width() - win_size.x) * 0.5,
        (screen.height() - win_size.y) * 0.4,
    );

    egui::Area::new(egui::Id::new("connect_overlay"))
        .fixed_pos(egui::Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let r = ui.allocate_rect(screen, egui::Sense::click());
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(160));
            if r.clicked() && !state.connect_loading {
                state.show_connect_dialog = false;
            }
        });

    egui::Area::new(egui::Id::new("connect_dialog"))
        .fixed_pos(win_pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .rounding(RADIUS_LG)
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
                        ui.label(
                            RichText::new("New Connection")
                                .color(TEXT_PRIMARY)
                                .size(16.0)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close =
                                egui::Button::new(RichText::new("×").color(TEXT_HINT).size(14.0))
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
                            let bg = if active { ACCENT_DIM } else { BG_CONTENT };
                            let tc = if active { TEXT_PRIMARY } else { TEXT_DIM };
                            let btn = egui::Button::new(RichText::new(*p).color(tc).size(11.5))
                                .fill(bg)
                                .rounding(RADIUS_MD)
                                .min_size(egui::vec2(64.0, 28.0));
                            if ui.add(btn).clicked() {
                                state.connect_protocol = i;
                                state.connect_port = default_ports[i].to_string();
                            }
                        }
                    });

                    ui.add_space(14.0);

                    // Поля ввода — Host / Username+Port / Password, как в макете
                    let field_w = ui.available_width();

                    field(ui, "Host", &mut state.connect_host, field_w, false);
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        let user_w = field_w - 80.0 - 8.0;
                        field(ui, "Username", &mut state.connect_user, user_w, false);
                        ui.add_space(8.0);
                        field(ui, "Port", &mut state.connect_port, 80.0, false);
                    });
                    ui.add_space(8.0);

                    field(ui, "Password", &mut state.connect_pass, field_w, true);

                    if state.connect_protocol == 0 {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            field(
                                ui,
                                "Key path (optional)",
                                &mut state.connect_key_path,
                                field_w - 80.0 - 8.0,
                                false,
                            );
                            ui.add_space(8.0);
                            // Browse placeholder — open system picker if rfd added
                            let browse = egui::Button::new(
                                RichText::new("Browse").color(TEXT_DIM).size(11.0),
                            )
                            .fill(BG_CONTENT)
                            .min_size(egui::vec2(72.0, 32.0))
                            .rounding(RADIUS_MD);
                            ui.add_enabled(false, browse);
                        });
                    }

                    ui.add_space(8.0);
                    field(
                        ui,
                        "Label (optional)",
                        &mut state.connect_label,
                        field_w,
                        false,
                    );

                    // Ошибка
                    if !state.connect_error.is_empty() {
                        ui.add_space(10.0);
                        egui::Frame::none()
                            .fill(Color32::from_rgb(42, 27, 22))
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(70, 40, 34)))
                            .rounding(RADIUS_MD)
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    crate::ui::icons::icon(
                                        ui,
                                        crate::ui::icons::Icon::DangerTriangle,
                                        14.0,
                                        RED,
                                    );
                                    ui.add_space(6.0);
                                    ui.label(
                                        RichText::new(&state.connect_error).color(RED).size(11.0),
                                    );
                                });
                            });
                    }

                    ui.add_space(16.0);

                    // Кнопки
                    ui.horizontal(|ui| {
                        let cancel =
                            egui::Button::new(RichText::new("Cancel").color(TEXT_DIM).size(12.5))
                                .fill(Color32::TRANSPARENT)
                                .rounding(RADIUS_MD)
                                .min_size(egui::vec2(90.0, 30.0));
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
                                    RichText::new(connect_label)
                                        .color(ON_ACCENT)
                                        .size(12.5)
                                        .strong(),
                                )
                                .fill(ACCENT)
                                .rounding(RADIUS_MD)
                                .min_size(egui::vec2(110.0, 30.0));

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

    if state.connect_host.trim().is_empty() {
        state.connect_error = "Host is required.".into();
        return;
    }
    if state.connect_user.trim().is_empty() {
        state.connect_error = "Username is required.".into();
        return;
    }
    if !state.connect_port.trim().is_empty() && state.connect_port.trim().parse::<u16>().is_err() {
        state.connect_error = "Port must be a number between 1 and 65535.".into();
        return;
    }

    let protocol = match state.connect_protocol {
        0 => Protocol::Sftp,
        1 => Protocol::Ftp,
        _ => Protocol::Ftps,
    };

    let port: u16 = state.connect_port.parse().unwrap_or(match protocol {
        Protocol::Sftp => 22,
        Protocol::Ftp => 21,
        Protocol::Ftps => 990,
    });

    let params = ConnectionParams {
        id: uuid::Uuid::new_v4().to_string(),
        label: if state.connect_label.is_empty() {
            format!(
                "{} ({})",
                state.connect_host,
                match protocol {
                    Protocol::Sftp => "SFTP",
                    Protocol::Ftp => "FTP",
                    Protocol::Ftps => "FTPS",
                }
            )
        } else {
            state.connect_label.clone()
        },
        protocol,
        host: state.connect_host.clone(),
        port,
        username: state.connect_user.clone(),
        password: if state.connect_pass.is_empty() {
            None
        } else {
            Some(state.connect_pass.clone())
        },
        key_path: if state.connect_key_path.is_empty() {
            None
        } else {
            Some(state.connect_key_path.clone())
        },
    };

    spawn_connect(state, registry, rt_handle, params);
}

/// Общий код запуска подключения — используется и диалогом New Connection,
/// и переподключением из истории (см. `reconnect_from_history`).
pub fn spawn_connect(
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    params: ConnectionParams,
) {
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

/// Переподключение по клику на запись истории — пароль (если есть) уже лежит
/// в keychain под `entry.conn_id`, поэтому диалог не нужен.
pub fn reconnect_from_history(
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    entry: &crate::ui::state::HistoryEntry,
) {
    state.connect_error.clear();
    let params = ConnectionParams {
        id: entry.conn_id.clone(),
        label: format!("{}@{}", entry.user, entry.host),
        protocol: entry.protocol.clone(),
        host: entry.host.clone(),
        port: entry.port,
        username: entry.user.clone(),
        password: None,
        key_path: entry.key_path.clone(),
    };
    spawn_connect(state, registry, rt_handle, params);
}

fn protocol_index(protocol: &Protocol) -> usize {
    match protocol {
        Protocol::Sftp => 0,
        Protocol::Ftp => 1,
        Protocol::Ftps => 2,
    }
}

/// "Изменить" в меню истории — открывает диалог New Connection, предзаполненный
/// этой записью (включая пароль из keychain, если он там есть).
pub fn edit_history_entry(state: &mut AppState, entry: &crate::ui::state::HistoryEntry) {
    state.connect_error.clear();
    state.connect_host = entry.host.clone();
    state.connect_user = entry.user.clone();
    state.connect_port = entry.port.to_string();
    state.connect_protocol = protocol_index(&entry.protocol);
    state.connect_key_path = entry.key_path.clone().unwrap_or_default();
    state.connect_pass = keychain::get_password(&entry.conn_id).unwrap_or_default();
    state.connect_label.clear();
    state.show_connect_dialog = true;
}

/// "Сохранить" в меню истории — превращает разовое подключение в постоянный Site.
pub fn save_history_as_site(
    db: &Arc<std::sync::Mutex<rusqlite::Connection>>,
    sites: &mut Vec<crate::domain::site::Site>,
    entry: &crate::ui::state::HistoryEntry,
) -> Result<(), String> {
    let site = crate::domain::site::Site {
        id: entry.conn_id.clone(),
        name: format!("{}@{}", entry.user, entry.host),
        protocol: entry.protocol.clone(),
        host: entry.host.clone(),
        port: entry.port,
        username: entry.user.clone(),
        key_path: entry.key_path.clone(),
        folder: None,
        note: None,
    };
    let conn = db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    crate::storage::db::save_site(&conn, &site).map_err(|e| e.to_string())?;
    drop(conn);
    if let Some(existing) = sites.iter_mut().find(|s| s.id == site.id) {
        *existing = site;
    } else {
        sites.push(site);
    }
    Ok(())
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
                    SftpClient::connect_password(
                        &params.host,
                        params.port,
                        &params.username,
                        &password,
                    )
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

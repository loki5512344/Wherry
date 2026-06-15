use std::sync::Arc;

use egui::{CentralPanel, TopBottomPanel};

use crate::domain::connection::ConnectionStatus;
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::domain::site::Site;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::dialogs::connection;
use crate::ui::panels::local_pane;
use crate::ui::panels::queue;
use crate::ui::panels::remote_pane;
use crate::ui::panels::status_bar;
use crate::ui::panels::tabs;
use crate::ui::panels::toolbar;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(100, 80, 220);

pub struct FileManagerApp {
    pub state: crate::ui::state::AppState,
    pub registry: Arc<RemoteRegistry>,
    pub queue: TransferQueue,
    pub rt: tokio::runtime::Runtime,
    pub db: Arc<std::sync::Mutex<rusqlite::Connection>>,
    pub sites: Vec<Site>,
    pub first_frame: bool,
}

impl FileManagerApp {
    pub fn new(
        registry: Arc<RemoteRegistry>,
        queue: TransferQueue,
        rt: tokio::runtime::Runtime,
        db: Arc<std::sync::Mutex<rusqlite::Connection>>,
        sites: Vec<Site>,
    ) -> Self {
        let mut state = crate::ui::state::AppState {
            local_path: crate::fs::local::home_dir(),
            ..Default::default()
        };
        local_pane::refresh_local(&mut state);

        Self {
            state,
            registry,
            queue,
            rt,
            db,
            sites,
            first_frame: true,
        }
    }

    fn poll_pending(&mut self) {
        let opt = self.state.pending_connect.take();
        if let Some(pc) = opt.as_ref() {
            let mut guard = pc.result.lock().unwrap();
            if let Some(res) = guard.take() {
                drop(guard);
                match res {
                    Ok((params, entries)) => {
                        let path = "/".to_string();
                        let mut list = vec![FileEntry {
                            name: "..".into(),
                            path: path.clone(),
                            kind: EntryKind::Dir,
                            size: None,
                            modified: None,
                            permissions: None,
                        }];
                        list.extend(entries);

                        let tab = crate::ui::state::ConnectionTab {
                            id: params.id.clone(),
                            label: params.label.clone(),
                            params,
                            status: ConnectionStatus::Connected,
                            remote_path: path,
                            remote_entries: list,
                            remote_selected: None,
                            loading: false,
                        };
                        self.state.tabs.push(tab);
                        self.state.active_tab = self.state.tabs.len() - 1;
                        self.state.status_message = "Connected".into();
                        self.state.connect_loading = false;
                        self.state.show_connect_dialog = false;
                        self.state.onboarding_host.clear();
                        self.state.onboarding_user.clear();
                        self.state.onboarding_pass.clear();
                        self.state.onboarding_port.clear();
                    }
                    Err(e) => {
                        self.state.status_message = format!("Connection failed: {}", e);
                        self.state.connect_error = e;
                        self.state.connect_loading = false;
                    }
                }
            } else {
                drop(guard);
                self.state.pending_connect = opt;
            }
        }

        let mut done: Vec<usize> = Vec::new();
        for (i, pending) in self.state.pending_remote_list.iter().enumerate() {
            let mut guard = pending.result.lock().unwrap();
            if let Some(res) = guard.take() {
                let tab_idx = pending.tab_idx;
                match res {
                    Ok(entries) => {
                        let path = if tab_idx < self.state.tabs.len() {
                            self.state.tabs[tab_idx].remote_path.clone()
                        } else {
                            "/".into()
                        };
                        let mut list = Vec::new();
                        if path != "/" {
                            let parent = remote_pane::remote_parent(&path);
                            list.push(FileEntry {
                                name: "..".into(),
                                path: parent,
                                kind: EntryKind::Dir,
                                size: None,
                                modified: None,
                                permissions: None,
                            });
                        }
                        list.extend(entries);
                        if tab_idx < self.state.tabs.len() {
                            self.state.tabs[tab_idx].remote_entries = list;
                            self.state.tabs[tab_idx].loading = false;
                        }
                    }
                    Err(e) => {
                        if tab_idx < self.state.tabs.len() {
                            self.state.tabs[tab_idx].loading = false;
                        }
                        self.state.status_message = format!("Remote list error: {}", e);
                    }
                }
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.state.pending_remote_list.remove(i);
        }
    }
}

impl eframe::App for FileManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.first_frame {
            self.first_frame = false;

            let mut style = (*ctx.style()).clone();
            style.visuals.selection.stroke.color = ACCENT;
            style.visuals.widgets.active.bg_fill = ACCENT;
            ctx.set_style(style);

            ctx.request_repaint();
        }

        self.poll_pending();
        self.state.queue_tasks = self.queue.all();
        self.state.connected_count = self
            .state
            .tabs
            .iter()
            .filter(|t| t.status == ConnectionStatus::Connected)
            .count();

        TopBottomPanel::top("tabs").show(ctx, |ui| {
            tabs::render(ui, &mut self.state);
        });

        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            toolbar::render(ui, &mut self.state);
        });

        let has_connection = self.state.active_tab_ref().is_some()
            && self.state.active_tab_ref().unwrap().status == ConnectionStatus::Connected;

        if has_connection {
            CentralPanel::default().show(ctx, |ui| {
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        local_pane::render(
                            ui,
                            &mut self.state,
                            &self.queue,
                            &self.registry,
                            self.rt.handle(),
                        );
                    });

                    columns[1].vertical(|ui| {
                        let idx = self.state.active_tab.min(self.state.tabs.len() - 1);
                        remote_pane::render(
                            ui,
                            &mut self.state,
                            idx,
                            &self.registry,
                            self.rt.handle(),
                            &self.queue,
                        );
                    });
                });
            });
        } else {
            CentralPanel::default().show(ctx, |ui| {
                let available = ui.available_height();
                ui.vertical_centered(|ui| {
                    ui.add_space(available * 0.15);

                    ui.heading("LoFlum");
                    ui.label("FTP/SFTP client");
                    ui.add_space(24.0);

                    let card_frame = egui::Frame::window(&ctx.style());
                    card_frame.show(ui, |ui| {
                        ui.set_min_width(320.0);
                        ui.vertical_centered(|ui| {
                            ui.label("Connect to a server");
                            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.label("Host:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.onboarding_host)
                        .id("onb_host".into())
                        .desired_width(150.0)
                        .hint_text("hostname"),
                );
                ui.label("Port:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.onboarding_port)
                        .id("onb_port".into())
                        .desired_width(60.0)
                        .char_limit(5)
                        .hint_text("port"),
                );
            });

            ui.horizontal(|ui| {
                ui.label("User:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.onboarding_user)
                        .id("onb_user".into())
                        .desired_width(200.0)
                        .hint_text("username"),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Pass:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.onboarding_pass)
                        .password(true)
                        .id("onb_pass".into())
                        .desired_width(200.0)
                        .hint_text("password"),
                );
            });

            ui.add_space(12.0);
            let conn_btn = egui::Button::new("Connect")
                .fill(ACCENT)
                                .min_size(egui::vec2(200.0, 32.0));
                            if ui.add(conn_btn).clicked() {
                                self.state.connect_host = self.state.onboarding_host.clone();
                                self.state.connect_user = self.state.onboarding_user.clone();
                                self.state.connect_pass = self.state.onboarding_pass.clone();
                                self.state.connect_port = if self.state.onboarding_port.is_empty() {
                                    "22".into()
                                } else {
                                    self.state.onboarding_port.clone()
                                };
                                self.state.connect_protocol = 0;
                                self.state.show_connect_dialog = true;
                            }
                        });
                    });
                });
            });
        }

        let tasks = self.state.queue_tasks.clone();
        if self.state.show_queue || !tasks.is_empty() {
            TopBottomPanel::bottom("queue_panel")
                .resizable(true)
                .default_height(130.0)
                .min_height(60.0)
                .show(ctx, |ui| {
                    queue::render(ui, &mut self.state, &tasks);
                });
        }

        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            status_bar::render(ui, &mut self.state);
        });

        if self.state.show_connect_dialog {
            connection::render(ctx, &mut self.state, &self.registry, self.rt.handle());
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

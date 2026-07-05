//! Разбор результатов подключения и листинга удалённой директории.
use super::FileManagerApp;
use crate::domain::connection::{ConnectionParams, ConnectionStatus};
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::ui::dock::PaneTab;
use crate::ui::panels::remote_pane;
use crate::ui::state::ConnectionTab;

impl FileManagerApp {
    // --- Connect result ---
    pub(super) fn poll_connect(&mut self) {
        let opt = self.state.pending_connect.take();
        let Some(pc) = opt.as_ref() else {
            return;
        };
        let mut guard = pc.result.lock().unwrap();
        let Some(res) = guard.take() else {
            drop(guard);
            self.state.pending_connect = opt;
            return;
        };
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

                let tab = ConnectionTab {
                    id: params.id.clone(),
                    label: params.label.clone(),
                    params: params.clone(),
                    status: ConnectionStatus::Connected,
                    remote_path: path,
                    remote_entries: list,
                    remote_selected: None,
                    loading: false,
                };
                self.state.tabs.push(tab);
                self.state.active_tab = self.state.tabs.len() - 1;
                self.dock_state
                    .push_to_focused_leaf(PaneTab::Remote(params.id.clone()));
                self.state.status_message = format!("Connected to {}", params.host);
                self.state.connect_loading = false;
                self.state.show_connect_dialog = false;

                self.persist_connection(&params);
                self.state.reload_history(&self.db);
            }
            Err(e) => {
                self.state.status_message = format!("Connection failed: {}", e);
                self.state.connect_error = e;
                self.state.connect_loading = false;
            }
        }
    }

    /// Один и тот же host/port/username всегда должен использовать один conn_id —
    /// под ним лежит пароль в keychain, иначе повторное подключение через
    /// «New Connection» развело бы историю и keychain.
    fn persist_connection(&self, params: &ConnectionParams) {
        if let Ok(conn) = self.db.lock() {
            let canonical_id = crate::storage::db::find_history_conn_id(
                &conn,
                &params.host,
                params.port,
                &params.username,
            )
            .ok()
            .flatten()
            .unwrap_or_else(|| params.id.clone());

            if let Some(password) = &params.password {
                let _ = crate::storage::keychain::store_password(&canonical_id, password);
            }

            let _ = crate::storage::db::add_history_entry(
                &conn,
                &params.host,
                params.port,
                &params.username,
                &canonical_id,
                &params.protocol,
                params.key_path.as_deref(),
            );
        }
    }

    // --- Remote list results ---
    pub(super) fn poll_remote_list(&mut self) {
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

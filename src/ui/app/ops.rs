//! Запуск операций над ФС из диалогов (mkdir/delete/rename), локально и удалённо.
use std::sync::Arc;

use super::FileManagerApp;
use crate::ui::panels::{local_pane, remote_pane};

impl FileManagerApp {
    pub(super) fn start_mkdir(&mut self, tab_idx: usize, name: String) {
        let tab = &self.state.tabs[tab_idx];
        let connection_id = tab.id.clone();
        let remote_path = tab.remote_path.clone();
        let registry = self.registry.clone();
        let path = format!("{}/{}", remote_path.trim_end_matches('/'), name);

        let result = Arc::new(std::sync::Mutex::new(None::<Result<(), String>>));
        let result_clone = result.clone();

        self.rt.handle().spawn(async move {
            let fs = match registry.get(&connection_id) {
                Some(fs) => fs,
                None => {
                    *result_clone.lock().unwrap() = Some(Err("connection not found".into()));
                    return;
                }
            };
            let r = fs.mkdir(&path).await.map_err(|e| e.to_string());
            *result_clone.lock().unwrap() = Some(r);
        });

        self.state.pending_mkdir_result = Some(result);
    }

    pub(super) fn start_delete(&mut self, tab_idx: usize, name: String) {
        let tab = &self.state.tabs[tab_idx];
        let connection_id = tab.id.clone();
        let registry = self.registry.clone();

        let entry_path = tab
            .remote_entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.path.clone());

        if name == ".." {
            self.state.status_message = "Cannot delete parent entry".into();
            return;
        }

        let Some(path) = entry_path else {
            self.state.status_message = "Selected entry not found".into();
            return;
        };

        let result = Arc::new(std::sync::Mutex::new(None::<Result<(), String>>));
        let result_clone = result.clone();

        self.rt.handle().spawn(async move {
            let fs = match registry.get(&connection_id) {
                Some(fs) => fs,
                None => {
                    *result_clone.lock().unwrap() = Some(Err("connection not found".into()));
                    return;
                }
            };
            let r = fs.delete(&path).await.map_err(|e| e.to_string());
            *result_clone.lock().unwrap() = Some(r);
        });

        self.state.pending_delete_result = Some(result);
    }

    pub(super) fn start_rename(&mut self, tab_idx: usize, old_name: String, new_name: String) {
        let tab = &self.state.tabs[tab_idx];
        let connection_id = tab.id.clone();
        let registry = self.registry.clone();

        let Some(entry) = tab.remote_entries.iter().find(|e| e.name == old_name) else {
            self.state.status_message = "Selected entry not found".into();
            return;
        };

        let from = entry.path.clone();
        let parent_dir = remote_pane::remote_parent(&from);
        let to = format!("{}/{}", parent_dir.trim_end_matches('/'), new_name);

        let result = Arc::new(std::sync::Mutex::new(None::<Result<(), String>>));
        let result_clone = result.clone();

        self.rt.handle().spawn(async move {
            let fs = match registry.get(&connection_id) {
                Some(fs) => fs,
                None => {
                    *result_clone.lock().unwrap() = Some(Err("connection not found".into()));
                    return;
                }
            };
            let r = fs.rename(&from, &to).await.map_err(|e| e.to_string());
            *result_clone.lock().unwrap() = Some(r);
        });

        self.state.pending_rename_result = Some(result);
    }

    pub(super) fn start_local_mkdir(&mut self, name: String) {
        let path = format!("{}/{}", self.state.local_path.trim_end_matches('/'), name);
        match crate::fs::local::mkdir(&path) {
            Ok(()) => {
                self.state.status_message = "Folder created".into();
                local_pane::refresh_local(&mut self.state);
            }
            Err(e) => self.state.status_message = format!("Create folder failed: {}", e),
        }
    }

    pub(super) fn start_local_delete(&mut self, name: String) {
        if name == ".." {
            self.state.status_message = "Cannot delete parent entry".into();
            return;
        }
        let Some(entry) = self.state.local_entries.iter().find(|e| e.name == name) else {
            self.state.status_message = "Selected entry not found".into();
            return;
        };
        match crate::fs::local::delete(&entry.path) {
            Ok(()) => {
                self.state.status_message = "Deleted".into();
                local_pane::refresh_local(&mut self.state);
            }
            Err(e) => self.state.status_message = format!("Delete failed: {}", e),
        }
    }

    pub(super) fn start_local_rename(&mut self, old_name: String, new_name: String) {
        let Some(entry) = self.state.local_entries.iter().find(|e| e.name == old_name) else {
            self.state.status_message = "Selected entry not found".into();
            return;
        };
        let from = entry.path.clone();
        let parent_dir = std::path::Path::new(&from)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| self.state.local_path.clone());
        let to = format!("{}/{}", parent_dir.trim_end_matches('/'), new_name);
        match crate::fs::local::rename(&from, &to) {
            Ok(()) => {
                self.state.status_message = "Renamed".into();
                local_pane::refresh_local(&mut self.state);
            }
            Err(e) => self.state.status_message = format!("Rename failed: {}", e),
        }
    }
}

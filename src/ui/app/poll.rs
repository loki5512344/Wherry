//! Опрос результатов асинхронных операций раз в кадр из `update`.
//! Разбор connect/list — в [`super::results`], здесь — диспетчер, флаги действий
//! и результаты mkdir/delete/rename.
use std::sync::Arc;

use super::FileManagerApp;
use crate::ui::dialogs::connection;
use crate::ui::panels::remote_pane;

impl FileManagerApp {
    /// Убирает завершённые задачи из очереди передач через
    /// `auto_clear_completed_secs` секунд после завершения (0 — никогда).
    /// Вызывается каждый кадр, сразу после обновления `state.queue_tasks`.
    pub(super) fn sweep_completed_tasks(&mut self) {
        use crate::domain::transfer::TaskState;
        use std::collections::HashSet;

        let now = std::time::Instant::now();
        let live_ids: HashSet<&str> = self
            .state
            .queue_tasks
            .iter()
            .map(|t| t.id.as_str())
            .collect();
        self.completed_since
            .retain(|id, _| live_ids.contains(id.as_str()));

        for task in &self.state.queue_tasks {
            if task.state == TaskState::Completed {
                self.completed_since.entry(task.id.clone()).or_insert(now);
            }
        }

        let secs = self.state.auto_clear_completed_secs;
        if secs == 0 {
            return;
        }
        let expired: Vec<String> = self
            .completed_since
            .iter()
            .filter(|(_, since)| now.duration_since(**since).as_secs() >= secs as u64)
            .map(|(id, _)| id.clone())
            .collect();
        for id in expired {
            self.queue.remove(&id);
            self.completed_since.remove(&id);
        }
    }

    pub(super) fn poll_pending(&mut self) {
        self.poll_connect();
        self.poll_remote_list();
        self.poll_action_flags();
        self.poll_op_results();
    }

    // --- Toolbar/history action flags ---
    fn poll_action_flags(&mut self) {
        if self.state.pending_refresh {
            self.state.pending_refresh = false;
            if let Some(idx) = self.active_tab_idx() {
                remote_pane::trigger_list(&mut self.state, idx, &self.registry, self.rt.handle());
            }
        }

        // История: переподключение по клику
        if let Some(entry) = self.state.pending_history_reconnect.take() {
            connection::reconnect_from_history(
                &mut self.state,
                &self.registry,
                self.rt.handle(),
                &entry,
            );
        }

        // История: «Save» → постоянный Site
        if let Some(entry) = self.state.pending_history_save.take() {
            match connection::save_history_as_site(&self.db, &mut self.sites, &entry) {
                Ok(()) => self.state.status_message = "Saved to sites".into(),
                Err(e) => self.state.status_message = format!("Save failed: {}", e),
            }
        }
    }

    // --- mkdir/delete/rename results ---
    fn poll_op_results(&mut self) {
        if let Some(res) = take_ready(&mut self.state.pending_mkdir_result) {
            self.on_op_done(res, "Folder created", "Create folder failed");
        }
        if let Some(res) = take_ready(&mut self.state.pending_delete_result) {
            self.on_op_done(res, "Deleted", "Delete failed");
        }
        if let Some(res) = take_ready(&mut self.state.pending_rename_result) {
            self.on_op_done(res, "Renamed", "Rename failed");
        }
    }

    /// Общая обработка результата операции над ФС: статус + refresh списка при успехе.
    fn on_op_done(&mut self, res: Result<(), String>, ok_msg: &str, err_prefix: &str) {
        match res {
            Ok(()) => {
                self.state.status_message = ok_msg.into();
                self.state.mkdir_name.clear();
                self.state.delete_name.clear();
                self.state.rename_old_name.clear();
                self.state.rename_new_name.clear();
                if let Some(idx) = self.active_tab_idx() {
                    remote_pane::trigger_list(
                        &mut self.state,
                        idx,
                        &self.registry,
                        self.rt.handle(),
                    );
                }
            }
            Err(e) => self.state.status_message = format!("{}: {}", err_prefix, e),
        }
    }
}

/// Забирает готовый результат из отложенной операции (и очищает слот), если он есть.
type PendingResult = Arc<std::sync::Mutex<Option<Result<(), String>>>>;
fn take_ready(slot: &mut Option<PendingResult>) -> Option<Result<(), String>> {
    let opt = slot.take();
    let res = opt.as_ref()?;
    let mut guard = res.lock().unwrap();
    if let Some(r) = guard.take() {
        drop(guard);
        Some(r)
    } else {
        drop(guard);
        *slot = opt;
        None
    }
}

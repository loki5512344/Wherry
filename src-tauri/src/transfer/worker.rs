use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

use crate::domain::transfer::{TaskState, TransferKind};
use crate::fs::remote::RemoteRegistry;
use crate::transfer::manager::TransferManager;

/// Событие для фронтенда
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TransferProgress {
    task_id: String,
    state: String,
    transferred_bytes: u64,
    total_bytes: u64,
    speed: Option<u64>,
    eta_secs: Option<u64>,
    error: Option<String>,
}

pub fn spawn_worker(
    manager: Arc<TransferManager>,
    registry: Arc<RemoteRegistry>,
    app: AppHandle,
) {
    tokio::spawn(async move {
        loop {
            let task = {
                let task = manager.queue.pop();
                if let Some(ref t) = task {
                    if t.state != TaskState::Queued {
                        // paused/cancelled — не трогаем, вернём
                        manager.queue.push(t.clone());
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                }
                task
            };

            let mut task = match task {
                Some(t) => t,
                None => {
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };

            // Старт
            task.state = TaskState::Running;
            manager.queue.update_state(&task.id, TaskState::Running);
            emit_progress(&app, &task, None);

            let fs = match registry.get(&task.connection_id) {
                Some(fs) => fs,
                None => {
                    task.state = TaskState::Failed("connection not found".into());
                    manager.queue.update_state(&task.id, task.state.clone());
                    emit_progress(&app, &task, Some("connection not found"));
                    continue;
                }
            };

            let result = match task.kind {
                TransferKind::Upload => fs.upload(&task.local_path, &task.remote_path).await,
                TransferKind::Download => fs.download(&task.remote_path, &task.local_path).await,
            };

            match result {
                Ok(()) => {
                    task.state = TaskState::Completed;
                    task.transferred_bytes = task.total_bytes;
                    manager.queue.update_state(&task.id, TaskState::Completed);
                    manager.queue.update_progress(&task.id, task.total_bytes, 0);
                    emit_progress(&app, &task, None);
                }
                Err(e) => {
                    let msg = e.to_string();
                    task.state = TaskState::Failed(msg.clone());
                    manager.queue.update_state(&task.id, TaskState::Failed(msg.clone()));
                    emit_progress(&app, &task, Some(&msg));
                }
            }
        }
    });
}

fn emit_progress(app: &AppHandle, task: &crate::domain::transfer::TransferTask, error: Option<&str>) {
    let _ = app.emit("transfer-progress", TransferProgress {
        task_id: task.id.clone(),
        state: task.state.to_string(),
        transferred_bytes: task.transferred_bytes,
        total_bytes: task.total_bytes,
        speed: task.speed,
        eta_secs: task.eta_secs,
        error: error.map(|s| s.to_string()),
    });
}

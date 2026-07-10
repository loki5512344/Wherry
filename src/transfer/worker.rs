use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

use crate::domain::transfer::{TaskState, TransferKind, TransferTask};
use crate::fs::remote::RemoteRegistry;
use crate::protocols::ProgressAction;
use crate::transfer::progress::ProgressThrottle;
use crate::transfer::queue::TransferQueue;

const POLL_INTERVAL_MS: u64 = 200;

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    id: String,
    transferred_bytes: u64,
    speed: u64,
    eta_secs: Option<u64>,
}

#[derive(Clone, serde::Serialize)]
struct StatePayload {
    id: String,
    state: TaskState,
}

fn emit_progress(app: &AppHandle, queue: &TransferQueue, id: &str) {
    if let Some(t) = queue.get(id) {
        let _ = app.emit(
            "transfer-progress",
            ProgressPayload {
                id: t.id,
                transferred_bytes: t.transferred_bytes,
                speed: t.speed.unwrap_or(0),
                eta_secs: t.eta_secs,
            },
        );
    }
}

fn emit_state(app: &AppHandle, id: &str, state: TaskState) {
    let _ = app.emit(
        "transfer-state-changed",
        StatePayload {
            id: id.to_string(),
            state,
        },
    );
}

/// Уменьшает счётчик активных передач при выходе из скоупа — в том числе при
/// панике внутри задачи (Drop отрабатывает и во время размотки стека), иначе
/// упавшая задача навсегда съедала бы один слот параллелизма.
struct InFlightGuard(Arc<AtomicUsize>);
impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Запускает диспетчер передач: раз в тик подбирает из очереди задачи со
/// статусом `Queued` и запускает их параллельно, пока число активных передач
/// меньше `max_concurrent` — значение читается заново на каждой итерации, так
/// что Settings → Transfers меняет параллелизм на лету, без перезапуска.
pub fn spawn_worker(
    queue: TransferQueue,
    registry: Arc<RemoteRegistry>,
    rt_handle: tokio::runtime::Handle,
    max_concurrent: Arc<AtomicU32>,
    app: AppHandle,
) {
    let in_flight = Arc::new(AtomicUsize::new(0));
    rt_handle.spawn(async move {
        loop {
            let limit = max_concurrent.load(Ordering::Relaxed).max(1) as usize;
            while in_flight.load(Ordering::Relaxed) < limit {
                let Some(task) = queue
                    .all()
                    .into_iter()
                    .find(|t| t.state == TaskState::Queued)
                else {
                    break;
                };
                queue.update_state(&task.id, TaskState::Running);
                emit_state(&app, &task.id, TaskState::Running);
                in_flight.fetch_add(1, Ordering::Relaxed);

                let queue = queue.clone();
                let registry = registry.clone();
                let guard_counter = in_flight.clone();
                let app = app.clone();
                tokio::spawn(async move {
                    let _guard = InFlightGuard(guard_counter);
                    run_transfer(task, queue, registry, app).await;
                });
            }
            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    });
}

/// Выполняет одну задачу передачи целиком и записывает итог в очередь.
async fn run_transfer(
    task: TransferTask,
    queue: TransferQueue,
    registry: Arc<RemoteRegistry>,
    app: AppHandle,
) {
    let fs = match registry.get(&task.connection_id) {
        Some(fs) => fs,
        None => {
            queue.update_state(&task.id, TaskState::Failed("connection not found".into()));
            emit_state(
                &app,
                &task.id,
                TaskState::Failed("connection not found".into()),
            );
            return;
        }
    };

    // Progress callback: троттлит обновления очереди до ~10 FPS, считает
    // текущую скорость и реагирует на отмену/паузу, выставленные пользователем.
    let queue_for_progress = queue.clone();
    let task_id_for_progress = task.id.clone();
    let app_for_progress = app.clone();
    let throttle = Arc::new(std::sync::Mutex::new(ProgressThrottle::default()));
    let last_sample = Arc::new(std::sync::Mutex::new((Instant::now(), 0u64)));
    let on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>> =
        Some(Box::new(move |transferred: u64| {
            if let Some(t) = queue_for_progress.get(&task_id_for_progress) {
                match t.state {
                    TaskState::Cancelled => return ProgressAction::Cancel,
                    TaskState::Paused => return ProgressAction::Pause,
                    _ => {}
                }
            }

            let speed = {
                let mut guard = last_sample.lock().unwrap();
                let (last_time, last_bytes) = *guard;
                let now = Instant::now();
                let elapsed = now.duration_since(last_time).as_secs_f64();
                let speed = if elapsed > 0.0 {
                    ((transferred.saturating_sub(last_bytes)) as f64 / elapsed) as u64
                } else {
                    0
                };
                *guard = (now, transferred);
                speed
            };
            if throttle.lock().unwrap().should_emit() {
                queue_for_progress.update_progress(&task_id_for_progress, transferred, speed);
                emit_progress(&app_for_progress, &queue_for_progress, &task_id_for_progress);
            }
            ProgressAction::Continue
        }));

    let result = match task.kind {
        TransferKind::Upload => {
            fs.upload_with_progress(&task.local_path, &task.remote_path, on_progress)
                .await
        }
        TransferKind::Download => {
            fs.download_with_progress(&task.remote_path, &task.local_path, on_progress)
                .await
        }
    };

    match result {
        Ok(()) => {
            queue.update_state(&task.id, TaskState::Completed);
            queue.update_progress(&task.id, task.total_bytes, 0);
            emit_progress(&app, &queue, &task.id);
            emit_state(&app, &task.id, TaskState::Completed);
        }
        Err(e) => {
            let msg = e.to_string();
            if msg == "cancelled" {
                queue.update_state(&task.id, TaskState::Cancelled);
                queue.remove(&task.id);
                emit_state(&app, &task.id, TaskState::Cancelled);
            } else if msg == "paused" {
                let transferred = queue
                    .get(&task.id)
                    .map(|t| t.transferred_bytes)
                    .unwrap_or(0);
                queue.update_state(&task.id, TaskState::Paused);
                queue.update_progress(&task.id, transferred, 0);
                emit_state(&app, &task.id, TaskState::Paused);
            } else {
                queue.update_state(&task.id, TaskState::Failed(msg.clone()));
                emit_state(&app, &task.id, TaskState::Failed(msg));
            }
        }
    }
}

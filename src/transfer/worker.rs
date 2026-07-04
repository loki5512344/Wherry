use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use crate::domain::transfer::{TaskState, TransferKind};
use crate::fs::remote::RemoteRegistry;
use crate::protocols::ProgressAction;
use crate::transfer::progress::ProgressThrottle;
use crate::transfer::queue::TransferQueue;

const POLL_INTERVAL_MS: u64 = 500;
const PAUSED_SLEEP_MS: u64 = 1000;

pub fn spawn_worker(
    queue: TransferQueue,
    registry: Arc<RemoteRegistry>,
    rt_handle: tokio::runtime::Handle,
) {
    rt_handle.spawn(async move {
        loop {
            let front = queue.all().into_iter().next();
            match front.as_ref().map(|t| &t.state) {
                None => {
                    sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
                    continue;
                }
                Some(TaskState::Paused) => {
                    sleep(Duration::from_millis(PAUSED_SLEEP_MS)).await;
                    continue;
                }
                Some(TaskState::Cancelled) => {
                    if let Some(t) = queue.pop() {
                        queue.remove(&t.id);
                    }
                    continue;
                }
                Some(TaskState::Running) => {
                    // Anomalous state: a previous worker died or the task
                    // was left running. Pop it and mark failed so the queue
                    // does not spin forever.
                    if let Some(mut t) = queue.pop() {
                        t.state = TaskState::Failed("stale running task".into());
                        queue.update_state(&t.id, t.state.clone());
                    }
                    continue;
                }
                Some(TaskState::Completed) | Some(TaskState::Failed(_)) => {
                    // Terminal tasks should not block new work. Move them to
                    // the back of the queue so they remain visible in the UI.
                    if let Some(t) = queue.pop() {
                        queue.push(t);
                    }
                    continue;
                }
                Some(TaskState::Retrying(_)) => {
                    // Not implemented yet: demote to queued so it can run.
                    if let Some(mut t) = queue.pop() {
                        t.state = TaskState::Queued;
                        queue.push(t);
                    }
                    continue;
                }
                Some(TaskState::Queued) => {
                    // Pop and run below.
                }
            }

            let mut task = match queue.pop() {
                Some(t) => t,
                None => continue,
            };

            // State may have changed between peek and pop.
            if task.state == TaskState::Cancelled {
                queue.remove(&task.id);
                continue;
            }

            task.state = TaskState::Running;
            queue.update_state(&task.id, TaskState::Running);

            let fs = match registry.get(&task.connection_id) {
                Some(fs) => fs,
                None => {
                    task.state = TaskState::Failed("connection not found".into());
                    queue.update_state(&task.id, task.state.clone());
                    continue;
                }
            };

            // Progress callback: throttles queue updates to ~10 FPS,
            // computes current speed, and reacts to Cancel/Pause requests.
            let queue_for_progress = queue.clone();
            let task_id_for_progress = task.id.clone();
            let throttle = Arc::new(std::sync::Mutex::new(ProgressThrottle::default()));
            let last_sample = Arc::new(std::sync::Mutex::new((Instant::now(), 0u64)));
            let on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>> =
                Some(Box::new(move |transferred: u64| {
                    // Check for user-initiated cancel/pause first.
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
                        queue_for_progress.update_progress(
                            &task_id_for_progress,
                            transferred,
                            speed,
                        );
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
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg == "cancelled" {
                        queue.update_state(&task.id, TaskState::Cancelled);
                        queue.remove(&task.id);
                    } else if msg == "paused" {
                        let transferred = queue
                            .get(&task.id)
                            .map(|t| t.transferred_bytes)
                            .unwrap_or(0);
                        let mut paused_task = task.clone();
                        paused_task.state = TaskState::Paused;
                        paused_task.transferred_bytes = transferred;
                        paused_task.speed = None;
                        paused_task.eta_secs = None;
                        queue.update_state(&task.id, TaskState::Paused);
                        queue.update_progress(&task.id, transferred, 0);
                    } else {
                        queue.update_state(&task.id, TaskState::Failed(msg));
                    }
                }
            }
        }
    });
}

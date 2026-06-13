use tauri::State;
use std::sync::Arc;

use crate::domain::transfer::{TransferTask, TransferKind, TaskState};
use crate::transfer::manager::TransferManager;
use crate::domain::file_entry::EntryKind;

#[tauri::command]
pub async fn upload(
    manager: State<'_, Arc<TransferManager>>,
    connection_id: String,
    local_path: String,
    remote_path: String,
) -> Result<String, String> {
    let meta = std::fs::metadata(&local_path).map_err(|e| format!("cannot stat local file: {}", e))?;
    if !meta.is_file() {
        return Err("local path is not a file".into());
    }

    let file_name = std::path::Path::new(&local_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let task = TransferTask::new(
        TransferKind::Upload,
        connection_id,
        local_path,
        remote_path,
        file_name,
        meta.len(),
    );
    let task_id = task.id.clone();

    // Проверяем, что соединение живо
    if manager.registry().get(&task.connection_id).is_none() {
        return Err(format!("no connection: {}", task.connection_id));
    }

    manager.queue.push(task);
    Ok(task_id)
}

#[tauri::command]
pub async fn download(
    manager: State<'_, Arc<TransferManager>>,
    connection_id: String,
    remote_path: String,
    local_path: String,
) -> Result<String, String> {
    // Узнаём размер удалённого файла через stat
    let registry = manager.registry();
    let fs = registry.get(&connection_id)
        .ok_or_else(|| format!("no connection: {}", connection_id))?;
    let stat = fs.stat(&remote_path).await.map_err(|e| format!("stat failed: {}", e))?;

    let file_name = std::path::Path::new(&remote_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let total = match stat.kind {
        EntryKind::File => stat.size.unwrap_or(0),
        _ => return Err("remote path is not a file".into()),
    };

    let task = TransferTask::new(
        TransferKind::Download,
        connection_id,
        local_path,
        remote_path,
        file_name,
        total,
    );
    let task_id = task.id.clone();
    manager.queue.push(task);
    Ok(task_id)
}

#[tauri::command]
pub async fn get_queue(
    manager: State<'_, Arc<TransferManager>>,
) -> Result<Vec<TransferTask>, String> {
    Ok(manager.queue.all())
}

#[tauri::command]
pub async fn pause_task(
    manager: State<'_, Arc<TransferManager>>,
    task_id: String,
) -> Result<(), String> {
    manager.queue.update_state(&task_id, TaskState::Paused);
    Ok(())
}

#[tauri::command]
pub async fn cancel_task(
    manager: State<'_, Arc<TransferManager>>,
    task_id: String,
) -> Result<(), String> {
    manager.queue.update_state(&task_id, TaskState::Cancelled);
    Ok(())
}

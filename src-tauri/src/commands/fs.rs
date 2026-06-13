use std::sync::Arc;
use tauri::State;
use crate::domain::file_entry::FileEntry;
use crate::fs::{local, remote::RemoteRegistry};

#[tauri::command]
pub fn list_local(path: String) -> Result<Vec<FileEntry>, String> {
    local::list(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_remote(
    registry: State<'_, Arc<RemoteRegistry>>,
    connection_id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let fs = registry.get(&connection_id)
        .ok_or_else(|| format!("no connection: {}", connection_id))?;
    fs.list(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_mkdir(
    registry: State<'_, Arc<RemoteRegistry>>,
    connection_id: String,
    path: String,
) -> Result<(), String> {
    let fs = registry.get(&connection_id)
        .ok_or_else(|| format!("no connection: {}", connection_id))?;
    fs.mkdir(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_rename(
    registry: State<'_, Arc<RemoteRegistry>>,
    connection_id: String,
    from: String,
    to: String,
) -> Result<(), String> {
    let fs = registry.get(&connection_id)
        .ok_or_else(|| format!("no connection: {}", connection_id))?;
    fs.rename(&from, &to).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_delete(
    registry: State<'_, Arc<RemoteRegistry>>,
    connection_id: String,
    path: String,
) -> Result<(), String> {
    let fs = registry.get(&connection_id)
        .ok_or_else(|| format!("no connection: {}", connection_id))?;
    fs.delete(&path).await.map_err(|e| e.to_string())
}

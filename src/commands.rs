//! Tauri command surface — thin wrappers over domain/fs/protocols/storage/transfer.
//! See docs/TAURI_BACKEND_API.md for the design this implements.
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tauri::State;

use crate::domain::connection::{ConnectionParams, Protocol};
use crate::domain::file_entry::FileEntry;
use crate::domain::site::Site;
use crate::domain::transfer::{TaskState, TransferKind, TransferTask};
use crate::fs::remote::RemoteRegistry;
use crate::protocols::{RemoteFs, ftp::FtpClient, sftp::SftpClient};
use crate::storage::db::{self, HistoryRow};
use crate::transfer::queue::TransferQueue;

pub struct AppState {
    pub db: Arc<std::sync::Mutex<rusqlite::Connection>>,
    pub registry: Arc<RemoteRegistry>,
    pub queue: TransferQueue,
    pub max_concurrent: Arc<AtomicU32>,
}

// --- Connections ---------------------------------------------------------

#[tauri::command]
pub async fn connect(state: State<'_, AppState>, params: ConnectionParams) -> Result<String, String> {
    let registry = state.registry.clone();
    let db = state.db.clone();

    // Пароль приходит с фронтенда и хранится в БД (в sites.password).
    // Для истории используем канонический conn_id, чтобы повторные
    // подключения не плодили записи.
    let canonical_id = db
        .lock()
        .ok()
        .and_then(|conn| {
            db::find_history_conn_id(&conn, &params.host, params.port, &params.username)
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| params.id.clone());

    let fs: Arc<dyn RemoteFs> = match params.protocol {
        Protocol::Sftp => {
            if let Some(key_path) = &params.key_path {
                Arc::new(
                    SftpClient::connect_key(
                        &params.host,
                        params.port,
                        &params.username,
                        key_path,
                        params.password.as_deref(),
                    )
                    .map_err(|e| e.to_string())?,
                )
            } else if let Some(password) = &params.password {
                Arc::new(
                    SftpClient::connect_password(&params.host, params.port, &params.username, password)
                        .map_err(|e| e.to_string())?,
                )
            } else {
                Arc::new(
                    SftpClient::connect_auto(&params.host, params.port, &params.username)
                        .map_err(|e| e.to_string())?,
                )
            }
        }
        Protocol::Ftp => {
            let password = params
                .password
                .clone()
                .ok_or("Password is required for FTP connections.")?;
            Arc::new(
                FtpClient::connect(&params.host, params.port, &params.username, &password)
                    .await
                    .map_err(|e| e.to_string())?,
            )
        }
        Protocol::Ftps => return Err("FTPS not yet implemented".into()),
    };

    registry.insert(params.id.clone(), fs);

    if let Ok(conn) = db.lock() {
        let _ = db::add_history_entry(
            &conn,
            &params.host,
            params.port,
            &params.username,
            &canonical_id,
            &params.protocol,
            params.key_path.as_deref(),
        );
    }

    Ok(params.id)
}

#[tauri::command]
pub fn disconnect(state: State<'_, AppState>, connection_id: String) {
    state.registry.remove(&connection_id);
}

// --- Remote file ops -------------------------------------------------------

fn get_remote(state: &AppState, connection_id: &str) -> Result<Arc<dyn RemoteFs>, String> {
    state
        .registry
        .get(connection_id)
        .ok_or_else(|| "connection not found".to_string())
}

#[tauri::command]
pub async fn remote_list(
    state: State<'_, AppState>,
    connection_id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    get_remote(&state, &connection_id)?
        .list(&path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_stat(
    state: State<'_, AppState>,
    connection_id: String,
    path: String,
) -> Result<FileEntry, String> {
    get_remote(&state, &connection_id)?
        .stat(&path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_mkdir(
    state: State<'_, AppState>,
    connection_id: String,
    path: String,
) -> Result<(), String> {
    get_remote(&state, &connection_id)?
        .mkdir(&path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_rename(
    state: State<'_, AppState>,
    connection_id: String,
    from: String,
    to: String,
) -> Result<(), String> {
    get_remote(&state, &connection_id)?
        .rename(&from, &to)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_delete(
    state: State<'_, AppState>,
    connection_id: String,
    path: String,
) -> Result<(), String> {
    get_remote(&state, &connection_id)?
        .delete(&path)
        .await
        .map_err(|e| e.to_string())
}

// --- Local file ops --------------------------------------------------------

#[tauri::command]
pub fn local_list(path: String) -> Result<Vec<FileEntry>, String> {
    crate::fs::local::list(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn local_home_dir() -> String {
    crate::fs::local::home_dir()
}

#[tauri::command]
pub fn local_mkdir(path: String) -> Result<(), String> {
    crate::fs::local::mkdir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn local_rename(from: String, to: String) -> Result<(), String> {
    crate::fs::local::rename(&from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn local_delete(path: String) -> Result<(), String> {
    crate::fs::local::delete(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn local_open(path: String) -> Result<(), String> {
    crate::fs::local::open(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn local_move_into(src_path: String, dest_dir: String) -> Result<(), String> {
    crate::fs::local::move_into(&src_path, &dest_dir).map_err(|e| e.to_string())
}

// --- Transfers --------------------------------------------------------------

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn enqueue_transfer(
    state: State<'_, AppState>,
    kind: TransferKind,
    connection_id: String,
    local_path: String,
    remote_path: String,
    file_name: String,
    total_bytes: u64,
) -> String {
    let task = TransferTask::new(kind, connection_id, local_path, remote_path, file_name, total_bytes);
    let id = task.id.clone();
    state.queue.push(task);
    id
}

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>) -> Vec<TransferTask> {
    state.queue.all()
}

#[tauri::command]
pub fn pause_task(state: State<'_, AppState>, id: String) {
    state.queue.update_state(&id, TaskState::Paused);
}

#[tauri::command]
pub fn resume_task(state: State<'_, AppState>, id: String) {
    state.queue.update_state(&id, TaskState::Queued);
}

#[tauri::command]
pub fn cancel_task(state: State<'_, AppState>, id: String) {
    state.queue.update_state(&id, TaskState::Cancelled);
}

#[tauri::command]
pub fn remove_task(state: State<'_, AppState>, id: String) {
    state.queue.remove(&id);
}

#[tauri::command]
pub fn set_max_concurrent(state: State<'_, AppState>, n: u32) {
    state.max_concurrent.store(n.max(1), Ordering::Relaxed);
    if let Ok(conn) = state.db.lock() {
        db::set_u32(&conn, "max_concurrent_transfers", n.max(1));
    }
}

// --- Sites / bookmarks / history / settings ---------------------------------

#[tauri::command]
pub fn list_sites(state: State<'_, AppState>) -> Result<Vec<Site>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_sites(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_site(state: State<'_, AppState>, site: Site) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::save_site(&conn, &site).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_site(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::delete_site(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_bookmarks(state: State<'_, AppState>) -> Result<Vec<(i64, String, String)>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_bookmarks(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_bookmark(state: State<'_, AppState>, name: String, path: String) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::add_bookmark(&conn, &name, &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_bookmark(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::remove_bookmark(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_history(state: State<'_, AppState>) -> Result<Vec<HistoryRow>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_history(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::clear_history(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_history_conn_id(
    state: State<'_, AppState>,
    host: String,
    port: u16,
    username: String,
) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::find_history_conn_id(&conn, &host, port, &username).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pref(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(db::get_setting(&conn, &key))
}

#[tauri::command]
pub fn set_pref(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

/// Удалить пароль из сохранённого сайта (очистить поле password в БД).
#[tauri::command]
pub fn delete_password(state: State<'_, AppState>, site_id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sites SET password = NULL WHERE id = ?1",
        rusqlite::params![site_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// The folder holding the sqlite db (Settings → About "reveal in folder").
#[tauri::command]
pub fn app_data_dir() -> String {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("wherry")
        .to_string_lossy()
        .to_string()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub os: &'static str,
    /// Linux only: XDG_CURRENT_DESKTOP, lowercased ("kde", "gnome", "hyprland", …).
    pub desktop: Option<String>,
    /// Linux only: "wayland" | "x11".
    pub session: Option<String>,
}

/// Из webview на Linux не различить KDE/GNOME/Hyprland — окружение видно
/// только процессу; фронтенд вешает результат как data-атрибуты на <html>.
#[tauri::command]
pub fn platform_info() -> PlatformInfo {
    let os = std::env::consts::OS;
    let (desktop, session) = if os == "linux" {
        (
            std::env::var("XDG_CURRENT_DESKTOP")
                .ok()
                .map(|d| d.to_lowercase()),
            std::env::var("XDG_SESSION_TYPE").ok(),
        )
    } else {
        (None, None)
    };
    PlatformInfo { os, desktop, session }
}

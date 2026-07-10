pub mod commands;
pub mod domain;
pub mod error;
pub mod fs;
pub mod i18n;
pub mod protocols;
pub mod settings;
pub mod storage;
pub mod transfers;
pub mod window;

use crate::commands::AppState;
use crate::fs::remote::RemoteRegistry;
use crate::transfers::queue::TransferQueue;
use crate::window::WindowState;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use tauri::Manager;

const DEFAULT_MAX_CONCURRENT: u32 = 2;

pub fn run() {
    tracing_subscriber::fmt().with_target(false).init();

    let db_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("wherry")
        .join("wherry.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = rusqlite::Connection::open(&db_path).expect("failed to open wherry database");
    storage::init_tables(&conn).expect("failed to init db tables");
    let db = Arc::new(std::sync::Mutex::new(conn));

    let max_concurrent = {
        let c = db.lock().unwrap();
        Arc::new(AtomicU32::new(settings::get_u32(
            &c,
            "max_concurrent_transfers",
            DEFAULT_MAX_CONCURRENT,
        )))
    };

    let auto_clear_secs = {
        let c = db.lock().unwrap();
        Arc::new(AtomicU32::new(settings::get_u32(
            &c,
            "auto_clear_completed_secs",
            0,
        )))
    };

    let registry = Arc::new(RemoteRegistry::default());
    let queue = TransferQueue::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db,
            registry: registry.clone(),
            queue: queue.clone(),
            max_concurrent: max_concurrent.clone(),
            auto_clear_secs: auto_clear_secs.clone(),
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            transfers::worker::spawn_worker(
                queue.clone(),
                registry.clone(),
                tauri::async_runtime::handle().inner().clone(),
                max_concurrent.clone(),
                auto_clear_secs.clone(),
                handle.clone(),
            );

            // Restore main window geometry from saved state
            if let Some(window) = handle.get_webview_window("main") {
                let db = handle.state::<AppState>().db.clone();

                let label = window.label().to_string();
                let key = format!("window_state_{}", label);
                if let Ok(conn) = db.lock()
                    && let Some(json) = settings::get_setting(&conn, &key)
                    && let Ok(ws) = serde_json::from_str::<WindowState>(&json)
                {
                    drop(conn);
                    if let (Some(x), Some(y)) = (ws.x, ws.y) {
                        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                    }
                    let _ = window.set_size(tauri::PhysicalSize::new(ws.width, ws.height));
                    if ws.maximized {
                        let _ = window.maximize();
                    }
                }

                // Save geometry on close
                let db2 = db.clone();
                let handle2 = handle.clone();
                let label2 = label.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api: _, .. } = event
                        && let Some(win) = handle2.get_webview_window(&label2)
                    {
                        commands::save_window_state_internal(&win, &db2);
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::disconnect,
            commands::remote_list,
            commands::remote_stat,
            commands::remote_mkdir,
            commands::remote_rename,
            commands::remote_delete,
            commands::local_list,
            commands::local_home_dir,
            commands::local_mkdir,
            commands::local_rename,
            commands::local_delete,
            commands::local_open,
            commands::local_move_into,
            commands::enqueue_transfer,
            commands::list_tasks,
            commands::pause_task,
            commands::resume_task,
            commands::cancel_task,
            commands::remove_task,
            commands::set_max_concurrent,
            commands::list_sites,
            commands::save_site,
            commands::delete_site,
            commands::list_bookmarks,
            commands::add_bookmark,
            commands::remove_bookmark,
            commands::list_history,
            commands::clear_history,
            commands::find_history_conn_id,
            commands::get_pref,
            commands::set_pref,
            commands::delete_password,
            commands::app_data_dir,
            commands::platform_info,
            commands::save_window_state,
            commands::load_window_state,
            commands::new_window,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Wherry");
}

pub mod commands;
pub mod domain;
pub mod fs;
pub mod i18n;
pub mod protocols;
pub mod storage;
pub mod transfer;

use crate::commands::AppState;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

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
    storage::db::init_tables(&conn).expect("failed to init db tables");
    let db = Arc::new(std::sync::Mutex::new(conn));

    let max_concurrent = {
        let c = db.lock().unwrap();
        Arc::new(AtomicU32::new(storage::db::get_u32(
            &c,
            "max_concurrent_transfers",
            DEFAULT_MAX_CONCURRENT,
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
        })
        .setup(move |app| {
            transfer::worker::spawn_worker(
                queue.clone(),
                registry.clone(),
                tauri::async_runtime::handle().inner().clone(),
                max_concurrent.clone(),
                app.handle().clone(),
            );
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
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Wherry");
}

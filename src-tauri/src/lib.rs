pub mod commands;
pub mod domain;
pub mod fs;
pub mod protocols;
pub mod storage;
pub mod transfer;

use std::sync::Arc;
use tauri::Manager;
use crate::transfer::manager::TransferManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::fs::list_local,
            commands::fs::list_remote,
            commands::fs::remote_mkdir,
            commands::fs::remote_rename,
            commands::fs::remote_delete,
            commands::connection::connect,
            commands::connection::disconnect,
            commands::transfer::upload,
            commands::transfer::download,
            commands::transfer::get_queue,
            commands::transfer::pause_task,
            commands::transfer::cancel_task,
            commands::sites::get_sites,
            commands::sites::save_site,
            commands::sites::delete_site,
        ])
        .setup(|app| {
            let registry = Arc::new(crate::fs::remote::RemoteRegistry::default());
            let manager = TransferManager::new(registry.clone(), app.handle().clone());

            app.manage(registry);
            app.manage(manager);

            storage::db::init(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running loflum");
}

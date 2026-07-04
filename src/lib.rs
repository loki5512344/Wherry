pub mod domain;
pub mod fs;
pub mod protocols;
pub mod storage;
pub mod transfer;
pub mod ui;

use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::app::FileManagerApp;
use std::sync::Arc;

pub fn run() {
    tracing_subscriber::fmt().with_target(false).init();

    let db_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("loflum")
        .join("loflum.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = rusqlite::Connection::open(&db_path).expect("failed to open loflum database");
    storage::db::init_tables(&conn).expect("failed to init db tables");
    let db = Arc::new(std::sync::Mutex::new(conn));

    let sites = {
        let c = db.lock().unwrap();
        storage::db::get_sites(&c).unwrap_or_default()
    };

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let rt_handle = rt.handle().clone();

    let registry = Arc::new(RemoteRegistry::default());
    let queue = TransferQueue::default();

    transfer::worker::spawn_worker(queue.clone(), registry.clone(), rt_handle);

    let app = FileManagerApp::new(registry, queue, rt, db, sites);

    let icon = load_app_icon();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("LoFlum")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "LoFlum",
        native_options,
        Box::new(|_cc| {
            crate::ui::menu::setup_native_menu();
            Ok(Box::new(app))
        }),
    )
    .expect("failed to start LoFlum");
}

fn load_app_icon() -> egui::IconData {
    let bytes = include_bytes!("ui/icons/app_icon.png");
    let image = image::load_from_memory(bytes)
        .expect("failed to decode app icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

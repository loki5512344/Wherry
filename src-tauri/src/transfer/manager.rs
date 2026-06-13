use std::sync::Arc;
use tauri::AppHandle;
use crate::transfer::queue::TransferQueue;
use crate::transfer::worker::spawn_worker;
use crate::fs::remote::RemoteRegistry;

/// Хранит очередь передач и запускает воркер.
pub struct TransferManager {
    pub queue: TransferQueue,
    registry: Arc<RemoteRegistry>,
    app: AppHandle,
}

impl TransferManager {
    pub fn new(registry: Arc<RemoteRegistry>, app: AppHandle) -> Arc<Self> {
        let manager = Arc::new(Self {
            queue: TransferQueue::default(),
            registry: registry.clone(),
            app: app.clone(),
        });
        spawn_worker(manager.clone(), registry, app);
        manager
    }

    pub fn registry(&self) -> &RemoteRegistry {
        &self.registry
    }

    pub fn app(&self) -> &AppHandle {
        &self.app
    }
}

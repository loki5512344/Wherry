// Абстракция над RemoteFs для хранения активных соединений
// TODO: HashMap<connection_id, Arc<dyn RemoteFs>>

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::protocols::RemoteFs;

#[derive(Default)]
pub struct RemoteRegistry {
    connections: Mutex<HashMap<String, Arc<dyn RemoteFs>>>,
}

impl RemoteRegistry {
    pub fn insert(&self, id: String, fs: Arc<dyn RemoteFs>) {
        self.connections.lock().unwrap().insert(id, fs);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn RemoteFs>> {
        self.connections.lock().unwrap().get(id).cloned()
    }

    pub fn remove(&self, id: &str) {
        self.connections.lock().unwrap().remove(id);
    }
}

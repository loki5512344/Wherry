// Абстракция над RemoteFs для хранения активных соединений
// TODO: HashMap<connection_id, Arc<dyn RemoteFs>>

use crate::protocols::RemoteFs;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EntryKind, FileEntry};
    use crate::protocols::ProgressAction;

    struct MockFs;

    #[async_trait::async_trait]
    impl RemoteFs for MockFs {
        async fn list(&self, _path: &str) -> anyhow::Result<Vec<FileEntry>> {
            Ok(vec![])
        }
        async fn upload_with_progress(
            &self,
            _local: &str,
            _remote: &str,
            _on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn download_with_progress(
            &self,
            _remote: &str,
            _local: &str,
            _on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        async fn mkdir(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn rename(&self, _from: &str, _to: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn stat(&self, _path: &str) -> anyhow::Result<FileEntry> {
            Ok(FileEntry {
                name: "test".into(),
                path: "/test".into(),
                kind: EntryKind::File,
                size: None,
                modified: None,
                permissions: None,
            })
        }
    }

    #[test]
    fn test_insert_get_remove() {
        let registry = RemoteRegistry::default();
        let mock = Arc::new(MockFs);

        registry.insert("conn-1".into(), mock.clone());
        assert!(registry.get("conn-1").is_some());

        registry.remove("conn-1");
        assert!(registry.get("conn-1").is_none());
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = RemoteRegistry::default();
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_insert_multiple() {
        let registry = RemoteRegistry::default();
        registry.insert("a".into(), Arc::new(MockFs));
        registry.insert("b".into(), Arc::new(MockFs));
        assert!(registry.get("a").is_some());
        assert!(registry.get("b").is_some());
    }
}

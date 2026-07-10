use crate::domain::{TaskState, TransferKind, TransferTask};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub fn new_task(
    kind: TransferKind,
    connection_id: impl Into<String>,
    local_path: impl Into<String>,
    remote_path: impl Into<String>,
    file_name: impl Into<String>,
    total_bytes: u64,
) -> TransferTask {
    TransferTask::new(
        kind,
        connection_id.into(),
        local_path.into(),
        remote_path.into(),
        file_name.into(),
        total_bytes,
    )
}

#[derive(Clone, Default)]
pub struct TransferQueue {
    inner: Arc<Mutex<VecDeque<TransferTask>>>,
}

impl TransferQueue {
    pub fn push(&self, task: TransferTask) {
        self.inner.lock().unwrap().push_back(task);
    }

    pub fn pop(&self) -> Option<TransferTask> {
        self.inner.lock().unwrap().pop_front()
    }

    pub fn get(&self, id: &str) -> Option<TransferTask> {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    pub fn all(&self) -> Vec<TransferTask> {
        self.inner.lock().unwrap().iter().cloned().collect()
    }

    pub fn update_state(&self, id: &str, state: TaskState) {
        let mut q = self.inner.lock().unwrap();
        if let Some(t) = q.iter_mut().find(|t| t.id == id) {
            t.state = state;
        }
    }

    pub fn update_progress(&self, id: &str, transferred: u64, speed: u64) {
        let mut q = self.inner.lock().unwrap();
        if let Some(t) = q.iter_mut().find(|t| t.id == id) {
            t.transferred_bytes = transferred;
            t.speed = Some(speed);
            let remaining = t.total_bytes.saturating_sub(transferred);
            t.eta_secs = remaining.checked_div(speed);
        }
    }

    pub fn remove(&self, id: &str) {
        let mut q = self.inner.lock().unwrap();
        q.retain(|t| t.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TaskState, TransferKind, TransferTask};

    fn make_task(id: &str, total: u64) -> TransferTask {
        TransferTask {
            id: id.into(),
            kind: TransferKind::Download,
            connection_id: "conn-1".into(),
            local_path: "/local".into(),
            remote_path: "/remote".into(),
            file_name: "file.txt".into(),
            total_bytes: total,
            transferred_bytes: 0,
            state: TaskState::Queued,
            speed: None,
            eta_secs: None,
        }
    }

    #[test]
    fn test_push_pop_all() {
        let q = TransferQueue::default();
        assert!(q.all().is_empty());

        q.push(make_task("t1", 100));
        q.push(make_task("t2", 200));
        assert_eq!(q.all().len(), 2);

        let t1 = q.pop().unwrap();
        assert_eq!(t1.id, "t1");
        assert_eq!(q.all().len(), 1);

        let t2 = q.pop().unwrap();
        assert_eq!(t2.id, "t2");
        assert!(q.pop().is_none());
    }

    #[test]
    fn test_get() {
        let q = TransferQueue::default();
        q.push(make_task("t1", 100));
        q.push(make_task("t2", 200));

        assert_eq!(q.get("t2").unwrap().total_bytes, 200);
        assert!(q.get("nonexistent").is_none());
    }

    #[test]
    fn test_update_state() {
        let q = TransferQueue::default();
        q.push(make_task("t1", 100));

        for state in [
            TaskState::Running,
            TaskState::Paused,
            TaskState::Completed,
            TaskState::Failed("err".into()),
            TaskState::Cancelled,
            TaskState::Retrying(5),
        ] {
            q.update_state("t1", state.clone());
            assert_eq!(q.get("t1").unwrap().state, state);
        }
    }

    #[test]
    fn test_update_progress() {
        let q = TransferQueue::default();
        q.push(make_task("t1", 1000));

        q.update_progress("t1", 250, 50);
        let t = q.get("t1").unwrap();
        assert_eq!(t.transferred_bytes, 250);
        assert_eq!(t.speed, Some(50));
        assert_eq!(t.eta_secs, Some(15));
    }

    #[test]
    fn test_update_progress_zero_speed() {
        let q = TransferQueue::default();
        q.push(make_task("t1", 1000));

        q.update_progress("t1", 100, 0);
        let t = q.get("t1").unwrap();
        assert_eq!(t.transferred_bytes, 100);
        assert_eq!(t.speed, Some(0));
        assert_eq!(t.eta_secs, None);
    }

    #[test]
    fn test_remove() {
        let q = TransferQueue::default();
        q.push(make_task("t1", 100));
        q.push(make_task("t2", 200));
        q.push(make_task("t3", 300));

        q.remove("t2");
        let ids: Vec<String> = q.all().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, vec!["t1", "t3"]);
    }

    #[test]
    fn test_remove_nonexistent() {
        let q = TransferQueue::default();
        q.push(make_task("t1", 100));
        q.remove("does-not-exist");
        assert_eq!(q.all().len(), 1);
    }
}

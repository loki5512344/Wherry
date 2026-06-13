use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use crate::domain::transfer::{TransferTask, TaskState};

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
            if speed > 0 {
                let remaining = t.total_bytes.saturating_sub(transferred);
                t.eta_secs = Some(remaining / speed);
            }
        }
    }

    pub fn remove(&self, id: &str) {
        let mut q = self.inner.lock().unwrap();
        q.retain(|t| t.id != id);
    }
}

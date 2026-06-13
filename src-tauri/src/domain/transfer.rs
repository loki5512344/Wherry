use std::fmt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferKind {
    Upload,
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Queued,
    Running,
    Paused,
    Cancelled,
    Completed,
    Failed(String),
    Retrying(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferTask {
    pub id: String,
    pub kind: TransferKind,
    pub connection_id: String,
    pub local_path: String,
    pub remote_path: String,
    pub file_name: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub state: TaskState,
    /// bytes/sec, updated with throttle
    pub speed: Option<u64>,
    pub eta_secs: Option<u64>,
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskState::Queued => write!(f, "queued"),
            TaskState::Running => write!(f, "running"),
            TaskState::Paused => write!(f, "paused"),
            TaskState::Cancelled => write!(f, "cancelled"),
            TaskState::Completed => write!(f, "completed"),
            TaskState::Failed(e) => write!(f, "failed: {}", e),
            TaskState::Retrying(n) => write!(f, "retrying({})", n),
        }
    }
}

impl TransferTask {
    pub fn new(
        kind: TransferKind,
        connection_id: String,
        local_path: String,
        remote_path: String,
        file_name: String,
        total_bytes: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            connection_id,
            local_path,
            remote_path,
            file_name,
            total_bytes,
            transferred_bytes: 0,
            state: TaskState::Queued,
            speed: None,
            eta_secs: None,
        }
    }

    pub fn progress_pct(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0
    }
}

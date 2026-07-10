use serde::{Deserialize, Serialize};
use std::fmt;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_task_new() {
        let task = TransferTask::new(
            TransferKind::Download,
            "conn-1".into(),
            "/local".into(),
            "/remote".into(),
            "file.txt".into(),
            1000,
        );
        assert!(!task.id.is_empty());
        assert_eq!(task.kind, TransferKind::Download);
        assert_eq!(task.total_bytes, 1000);
        assert_eq!(task.transferred_bytes, 0);
        assert_eq!(task.state, TaskState::Queued);
        assert!(task.speed.is_none());
        assert!(task.eta_secs.is_none());
    }

    #[test]
    fn test_progress_pct() {
        let mut task = TransferTask::new(
            TransferKind::Upload,
            "conn-1".into(),
            "/local".into(),
            "/remote".into(),
            "file.txt".into(),
            200,
        );
        assert_eq!(task.progress_pct(), 0.0);
        task.transferred_bytes = 50;
        assert!((task.progress_pct() - 25.0).abs() < f64::EPSILON);
        task.transferred_bytes = 200;
        assert!((task.progress_pct() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_progress_pct_zero_total() {
        let task = TransferTask::new(
            TransferKind::Upload,
            "conn-1".into(),
            "/local".into(),
            "/remote".into(),
            "file.txt".into(),
            0,
        );
        assert_eq!(task.progress_pct(), 0.0);
    }

    #[test]
    fn test_task_state_display() {
        assert_eq!(TaskState::Queued.to_string(), "queued");
        assert_eq!(TaskState::Running.to_string(), "running");
        assert_eq!(TaskState::Paused.to_string(), "paused");
        assert_eq!(TaskState::Cancelled.to_string(), "cancelled");
        assert_eq!(TaskState::Completed.to_string(), "completed");
        assert_eq!(TaskState::Failed("err".into()).to_string(), "failed: err");
        assert_eq!(TaskState::Retrying(3).to_string(), "retrying(3)");
    }

    #[test]
    fn test_transfer_kind_serde() {
        assert_eq!(
            serde_json::to_string(&TransferKind::Upload).unwrap(),
            "\"upload\""
        );
        assert_eq!(
            serde_json::to_string(&TransferKind::Download).unwrap(),
            "\"download\""
        );
    }

    #[test]
    fn test_task_state_serde() {
        let json = serde_json::to_string(&TaskState::Retrying(2)).unwrap();
        assert_eq!(json, "{\"retrying\":2}");
        let deserialized: TaskState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, TaskState::Retrying(2));
    }
}

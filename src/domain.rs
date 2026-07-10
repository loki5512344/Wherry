use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ── Protocol & ConnectionParams ──
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Sftp,
    Ftp,
    Ftps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionParams {
    pub id: String,
    pub label: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// None = use keychain
    pub password: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

// ── EntryKind & FileEntry ──
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub modified: Option<i64>, // unix timestamp
    pub permissions: Option<String>,
}

// ── Site ──
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
    pub folder: Option<String>,
    pub note: Option<String>,
}

// ── TransferKind, TaskState, TransferTask ──
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

    // from connection.rs
    #[test]
    fn test_protocol_serde() {
        let sftp = Protocol::Sftp;
        let json = serde_json::to_string(&sftp).unwrap();
        assert_eq!(json, "\"sftp\"");
        let deserialized: Protocol = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Protocol::Sftp);

        let ftp = Protocol::Ftp;
        let json = serde_json::to_string(&ftp).unwrap();
        assert_eq!(json, "\"ftp\"");

        let ftps = Protocol::Ftps;
        let json = serde_json::to_string(&ftps).unwrap();
        assert_eq!(json, "\"ftps\"");
    }

    #[test]
    fn test_connection_params() {
        let params = ConnectionParams {
            id: "test-id".into(),
            label: "Test".into(),
            protocol: Protocol::Sftp,
            host: "example.com".into(),
            port: 22,
            username: "user".into(),
            password: Some("pass".into()),
            key_path: None,
        };
        assert_eq!(params.id, "test-id");
        assert_eq!(params.protocol, Protocol::Sftp);
        assert_eq!(params.port, 22);
        assert!(params.password.is_some());
        assert!(params.key_path.is_none());
    }

    #[test]
    fn test_connection_status_serde() {
        let json = serde_json::to_string(&ConnectionStatus::Connected).unwrap();
        assert_eq!(json, "\"connected\"");

        let json = serde_json::to_string(&ConnectionStatus::Error("timeout".into())).unwrap();
        assert_eq!(json, "{\"error\":\"timeout\"}");
    }

    // from file_entry.rs
    #[test]
    fn test_entry_kind_serde() {
        assert_eq!(serde_json::to_string(&EntryKind::File).unwrap(), "\"file\"");
        assert_eq!(serde_json::to_string(&EntryKind::Dir).unwrap(), "\"dir\"");
        assert_eq!(
            serde_json::to_string(&EntryKind::Symlink).unwrap(),
            "\"symlink\""
        );
    }

    #[test]
    fn test_file_entry() {
        let entry = FileEntry {
            name: "test.txt".into(),
            path: "/tmp/test.txt".into(),
            kind: EntryKind::File,
            size: Some(1024),
            modified: Some(1234567890),
            permissions: Some("rw-r--r--".into()),
        };
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.size, Some(1024));
    }

    #[test]
    fn test_file_entry_serde() {
        let entry = FileEntry {
            name: "f".into(),
            path: "/f".into(),
            kind: EntryKind::Dir,
            size: None,
            modified: None,
            permissions: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: FileEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "f");
        assert_eq!(deserialized.kind, EntryKind::Dir);
    }

    // from site.rs
    #[test]
    fn test_site_serde() {
        let site = Site {
            id: "site-1".into(),
            name: "My Server".into(),
            protocol: Protocol::Sftp,
            host: "example.com".into(),
            port: 22,
            username: "admin".into(),
            password: Some("secret".into()),
            key_path: None,
            folder: Some("/remote".into()),
            note: Some("my note".into()),
        };
        let json = serde_json::to_string(&site).unwrap();
        let deserialized: Site = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, site.id);
        assert_eq!(deserialized.name, site.name);
        assert_eq!(deserialized.protocol, site.protocol);
        assert_eq!(deserialized.host, site.host);
        assert_eq!(deserialized.port, site.port);
        assert_eq!(deserialized.username, site.username);
        assert_eq!(deserialized.password, site.password);
        assert_eq!(deserialized.folder, site.folder);
        assert_eq!(deserialized.note, site.note);
    }

    #[test]
    fn test_site_minimal() {
        let site = Site {
            id: "site-2".into(),
            name: "Minimal".into(),
            protocol: Protocol::Ftp,
            host: "ftp.example.com".into(),
            port: 21,
            username: "user".into(),
            password: None,
            key_path: None,
            folder: None,
            note: None,
        };
        let json = serde_json::to_string(&site).unwrap();
        let deserialized: Site = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.protocol, Protocol::Ftp);
        assert!(deserialized.password.is_none());
    }

    // from transfer.rs
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

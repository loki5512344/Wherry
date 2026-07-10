use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

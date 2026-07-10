use crate::domain::connection::Protocol;
use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::connection::Protocol;

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
}

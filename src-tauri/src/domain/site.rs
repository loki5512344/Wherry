use serde::{Deserialize, Serialize};
use crate::domain::connection::Protocol;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub key_path: Option<String>,
    pub folder: Option<String>,
    pub note: Option<String>,
}

pub mod ftp;
pub mod sftp;

use async_trait::async_trait;
use crate::domain::file_entry::FileEntry;
use anyhow::Result;

/// Единый контракт для всех протоколов.
/// Очередь передач не знает — FTP это или SFTP.
#[async_trait]
pub trait RemoteFs: Send + Sync {
    async fn list(&self, path: &str) -> Result<Vec<FileEntry>>;
    async fn upload(&self, local: &str, remote: &str) -> Result<()>;
    async fn download(&self, remote: &str, local: &str) -> Result<()>;
    async fn mkdir(&self, path: &str) -> Result<()>;
    async fn rename(&self, from: &str, to: &str) -> Result<()>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn stat(&self, path: &str) -> Result<FileEntry>;
}

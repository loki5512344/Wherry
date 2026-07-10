pub mod ftp;
pub mod sftp;

pub use ftp::{FtpClient, FtpsClient};
pub use sftp::SftpClient;

use crate::domain::FileEntry;
use anyhow::Result;
use async_trait::async_trait;

/// Результат выполнения колбэка прогресса: продолжить, отменить или приостановить.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressAction {
    Continue,
    Cancel,
    Pause,
}

/// Единый контракт для всех протоколов.
/// Очередь передач не знает — FTP это или SFTP.
#[async_trait]
pub trait RemoteFs: Send + Sync {
    async fn list(&self, path: &str) -> Result<Vec<FileEntry>>;

    async fn upload(&self, local: &str, remote: &str) -> Result<()> {
        self.upload_with_progress(local, remote, None).await
    }

    async fn download(&self, remote: &str, local: &str) -> Result<()> {
        self.download_with_progress(remote, local, None).await
    }

    async fn upload_with_progress(
        &self,
        local: &str,
        remote: &str,
        on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>>,
    ) -> Result<()>;

    async fn download_with_progress(
        &self,
        remote: &str,
        local: &str,
        on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>>,
    ) -> Result<()>;

    async fn mkdir(&self, path: &str) -> Result<()>;
    async fn rename(&self, from: &str, to: &str) -> Result<()>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn stat(&self, path: &str) -> Result<FileEntry>;
}

//! Реализация `RemoteFs` для SFTP: всё через `spawn_blocking` (ssh2 синхронный).
//! Чанкованные upload/download вынесены в [`super::transfer`].
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;

use super::{SftpClient, transfer};
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::protocols::{ProgressAction, RemoteFs};

#[async_trait]
impl RemoteFs for SftpClient {
    async fn list(&self, path: &str) -> Result<Vec<FileEntry>> {
        let session = self.session.clone();
        let path = path.to_string();
        tokio::task::spawn_blocking(move || {
            let sftp = session
                .lock()
                .unwrap()
                .sftp()
                .context("SFTP subsystem failed")?;
            let entries = sftp.readdir(Path::new(&path)).context("readdir failed")?;

            let result = entries
                .into_iter()
                .map(|(pb, stat)| {
                    let kind = if stat.is_dir() {
                        EntryKind::Dir
                    } else if stat.file_type().is_symlink() {
                        EntryKind::Symlink
                    } else {
                        EntryKind::File
                    };
                    FileEntry {
                        name: pb
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        path: pb.to_string_lossy().to_string(),
                        kind,
                        size: stat.size,
                        modified: stat.mtime.map(|t| t as i64),
                        permissions: None,
                    }
                })
                .collect();

            Ok(result)
        })
        .await
        .map_err(|e| anyhow::anyhow!("SFTP list task failed: {}", e))?
    }

    async fn upload_with_progress(
        &self,
        local: &str,
        remote: &str,
        on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>>,
    ) -> Result<()> {
        transfer::upload(
            self.session.clone(),
            local.to_string(),
            remote.to_string(),
            on_progress,
        )
        .await
    }

    async fn download_with_progress(
        &self,
        remote: &str,
        local: &str,
        on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>>,
    ) -> Result<()> {
        transfer::download(
            self.session.clone(),
            remote.to_string(),
            local.to_string(),
            on_progress,
        )
        .await
    }

    async fn mkdir(&self, path: &str) -> Result<()> {
        let session = self.session.clone();
        let path = path.to_string();
        tokio::task::spawn_blocking(move || {
            let sftp = session
                .lock()
                .unwrap()
                .sftp()
                .context("SFTP subsystem failed")?;
            sftp.mkdir(Path::new(&path), 0o755)
                .context("mkdir failed")?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("SFTP mkdir task failed: {}", e))?
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let session = self.session.clone();
        let from = from.to_string();
        let to = to.to_string();
        tokio::task::spawn_blocking(move || {
            let sftp = session
                .lock()
                .unwrap()
                .sftp()
                .context("SFTP subsystem failed")?;
            sftp.rename(Path::new(&from), Path::new(&to), None)
                .context("rename failed")?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("SFTP rename task failed: {}", e))?
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let session = self.session.clone();
        let path = path.to_string();
        tokio::task::spawn_blocking(move || {
            let sftp = session
                .lock()
                .unwrap()
                .sftp()
                .context("SFTP subsystem failed")?;
            // пробуем как файл, потом как директорию
            if sftp.unlink(Path::new(&path)).is_err() {
                sftp.rmdir(Path::new(&path)).context("delete failed")?;
            }
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("SFTP delete task failed: {}", e))?
    }

    async fn stat(&self, path: &str) -> Result<FileEntry> {
        let session = self.session.clone();
        let path = path.to_string();
        tokio::task::spawn_blocking(move || {
            let sftp = session
                .lock()
                .unwrap()
                .sftp()
                .context("SFTP subsystem failed")?;
            let stat = sftp.stat(Path::new(&path)).context("stat failed")?;
            Ok(FileEntry {
                name: Path::new(&path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: path.to_string(),
                kind: if stat.is_dir() {
                    EntryKind::Dir
                } else {
                    EntryKind::File
                },
                size: stat.size,
                modified: stat.mtime.map(|t| t as i64),
                permissions: None,
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!("SFTP stat task failed: {}", e))?
    }
}

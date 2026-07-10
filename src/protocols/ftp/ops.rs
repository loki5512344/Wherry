//! Реализация `RemoteFs` для FTP. Чанкованные upload/download — в [`super::transfer`].
use anyhow::Result;
use async_trait::async_trait;

use super::{FtpClient, FtpsClient, parse_list_entry, transfer};
use crate::domain::file_entry::{EntryKind, FileEntry};
use crate::protocols::{ProgressAction, RemoteFs};

macro_rules! impl_remote_fs {
    ($client:ty, $label:literal, $upload:ident, $download:ident) => {
        #[async_trait]
        impl RemoteFs for $client {
            async fn list(&self, path: &str) -> Result<Vec<FileEntry>> {
                let mut guard = self.conn.lock().await;
                let conn = guard
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("not connected"))?;
                let lines = conn
                    .list(Some(path))
                    .await
                    .map_err(|e| anyhow::anyhow!(concat!($label, " list failed: {}"), e))?;

                let mut entries: Vec<FileEntry> = lines
                    .iter()
                    .filter_map(|line| parse_list_entry(line))
                    .collect();

                entries.sort_by(|a, b| match (&a.kind, &b.kind) {
                    (EntryKind::Dir, EntryKind::Dir) => a.name.cmp(&b.name),
                    (EntryKind::Dir, _) => std::cmp::Ordering::Less,
                    (_, EntryKind::Dir) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                });

                Ok(entries)
            }

            async fn upload_with_progress(
                &self,
                local: &str,
                remote: &str,
                on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>>,
            ) -> Result<()> {
                let mut guard = self.conn.lock().await;
                let conn = guard
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("not connected"))?;
                transfer::$upload(conn, local, remote, on_progress).await
            }

            async fn download_with_progress(
                &self,
                remote: &str,
                local: &str,
                on_progress: Option<Box<dyn Fn(u64) -> ProgressAction + Send>>,
            ) -> Result<()> {
                let mut guard = self.conn.lock().await;
                let conn = guard
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("not connected"))?;
                transfer::$download(conn, remote, local, on_progress).await
            }

            async fn mkdir(&self, path: &str) -> Result<()> {
                let mut guard = self.conn.lock().await;
                let conn = guard
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("not connected"))?;
                conn.mkdir(path)
                    .await
                    .map_err(|e| anyhow::anyhow!(concat!($label, " mkdir failed: {}"), e))?;
                Ok(())
            }

            async fn rename(&self, from: &str, to: &str) -> Result<()> {
                let mut guard = self.conn.lock().await;
                let conn = guard
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("not connected"))?;
                conn.rename(from, to)
                    .await
                    .map_err(|e| anyhow::anyhow!(concat!($label, " rename failed: {}"), e))?;
                Ok(())
            }

            async fn delete(&self, path: &str) -> Result<()> {
                let mut guard = self.conn.lock().await;
                let conn = guard
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("not connected"))?;
                if conn.rm(path).await.is_err() {
                    conn.rmdir(path)
                        .await
                        .map_err(|e| anyhow::anyhow!(concat!($label, " delete failed: {}"), e))?;
                }
                Ok(())
            }

            async fn stat(&self, path: &str) -> Result<FileEntry> {
                let mut guard = self.conn.lock().await;
                let conn = guard
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("not connected"))?;
                let size = conn
                    .size(path)
                    .await
                    .map_err(|e| anyhow::anyhow!(concat!($label, " stat failed: {}"), e))?;
                let modified = conn
                    .mdtm(path)
                    .await
                    .map(|dt| dt.and_utc().timestamp())
                    .ok();

                let name = std::path::Path::new(path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());

                Ok(FileEntry {
                    name,
                    path: path.to_string(),
                    kind: EntryKind::File,
                    size: Some(size as u64),
                    modified,
                    permissions: None,
                })
            }
        }
    };
}

impl_remote_fs!(FtpClient, "FTP", upload_ftp, download_ftp);
impl_remote_fs!(FtpsClient, "FTPS", upload_ftps, download_ftps);

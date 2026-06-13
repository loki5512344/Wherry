use async_trait::async_trait;
use anyhow::Result;
use std::str::FromStr;
use std::time::SystemTime;
use tokio::sync::Mutex;
use async_std::io::{ReadExt, WriteExt};

use suppaftp::{AsyncFtpStream, list::File};

use crate::domain::file_entry::{FileEntry, EntryKind};
use super::RemoteFs;

pub struct FtpClient {
    pub conn: Mutex<Option<AsyncFtpStream>>,
}

impl FtpClient {
    pub async fn connect(host: &str, port: u16, user: &str, pass: &str) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        let mut stream = AsyncFtpStream::connect(&addr).await
            .map_err(|e| anyhow::anyhow!("FTP connect failed: {}", e))?;
        stream.login(user, pass).await
            .map_err(|e| anyhow::anyhow!("FTP login failed: {}", e))?;
        Ok(Self { conn: Mutex::new(Some(stream)) })
    }

    fn parse_list_entry(line: &str) -> Option<FileEntry> {
        let file = File::from_str(line).ok()?;
        let kind = if file.is_directory() {
            EntryKind::Dir
        } else if file.is_symlink() {
            EntryKind::Symlink
        } else {
            EntryKind::File
        };

        let modified = file.modified()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64);

        Some(FileEntry {
            name: file.name().to_string(),
            path: file.name().to_string(),
            kind,
            size: Some(file.size() as u64),
            modified,
            permissions: None,
        })
    }
}

#[async_trait]
impl RemoteFs for FtpClient {
    async fn list(&self, path: &str) -> Result<Vec<FileEntry>> {
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let lines = conn.list(Some(path)).await
            .map_err(|e| anyhow::anyhow!("FTP list failed: {}", e))?;

        let mut entries: Vec<FileEntry> = lines.iter()
            .filter_map(|line| Self::parse_list_entry(line))
            .collect();

        entries.sort_by(|a, b| {
            match (&a.kind, &b.kind) {
                (EntryKind::Dir, EntryKind::Dir) => a.name.cmp(&b.name),
                (EntryKind::Dir, _) => std::cmp::Ordering::Less,
                (_, EntryKind::Dir) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(entries)
    }

    async fn upload(&self, local: &str, remote: &str) -> Result<()> {
        let data = std::fs::read(local)
            .map_err(|e| anyhow::anyhow!("cannot read local file: {}", e))?;
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let mut stream = conn.put_with_stream(remote).await
            .map_err(|e| anyhow::anyhow!("FTP upload stream failed: {}", e))?;
        stream.write_all(&data).await
            .map_err(|e| anyhow::anyhow!("FTP upload write failed: {}", e))?;
        conn.finalize_put_stream(stream).await
            .map_err(|e| anyhow::anyhow!("FTP upload finalize failed: {}", e))?;
        Ok(())
    }

    async fn download(&self, remote: &str, local: &str) -> Result<()> {
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let mut stream = conn.retr_as_stream(remote).await
            .map_err(|e| anyhow::anyhow!("FTP download stream failed: {}", e))?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await
            .map_err(|e| anyhow::anyhow!("FTP download read failed: {}", e))?;
        conn.finalize_retr_stream(stream).await
            .map_err(|e| anyhow::anyhow!("FTP download finalize failed: {}", e))?;
        std::fs::write(local, &buf)
            .map_err(|e| anyhow::anyhow!("cannot write local file: {}", e))?;
        Ok(())
    }

    async fn mkdir(&self, path: &str) -> Result<()> {
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        conn.mkdir(path).await
            .map_err(|e| anyhow::anyhow!("FTP mkdir failed: {}", e))?;
        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        conn.rename(from, to).await
            .map_err(|e| anyhow::anyhow!("FTP rename failed: {}", e))?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        if conn.rm(path).await.is_err() {
            conn.rmdir(path).await
                .map_err(|e| anyhow::anyhow!("FTP delete failed: {}", e))?;
        }
        Ok(())
    }

    async fn stat(&self, path: &str) -> Result<FileEntry> {
        let mut guard = self.conn.lock().await;
        let conn = guard.as_mut().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let size = conn.size(path).await
            .map_err(|e| anyhow::anyhow!("FTP stat failed: {}", e))?;
        let modified = conn.mdtm(path).await
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

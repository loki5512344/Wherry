use anyhow::Result;
use async_trait::async_trait;
use futures_lite::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;
use std::str::FromStr;
use std::time::SystemTime;
use suppaftp::async_native_tls::TlsConnector;
use suppaftp::{
    AsyncFtpStream, AsyncNativeTlsConnector, AsyncNativeTlsFtpStream, list::File as FtpListFile,
};
use tokio::fs::File;
use tokio::io::{AsyncReadExt as TokioReadExt, AsyncWriteExt as TokioWriteExt};
use tokio::sync::Mutex;

use crate::domain::{EntryKind, FileEntry};
use crate::protocols::{ProgressAction, RemoteFs};

fn parse_list_entry(line: &str) -> Option<FileEntry> {
    let file = FtpListFile::from_str(line).ok()?;
    let kind = if file.is_directory() {
        EntryKind::Dir
    } else if file.is_symlink() {
        EntryKind::Symlink
    } else {
        EntryKind::File
    };

    let modified = file
        .modified()
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

pub struct FtpClient {
    pub conn: Mutex<Option<AsyncFtpStream>>,
}

pub struct FtpsClient {
    pub conn: Mutex<Option<AsyncNativeTlsFtpStream>>,
}

impl FtpClient {
    pub async fn connect(host: &str, port: u16, user: &str, pass: &str) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        let mut stream = AsyncFtpStream::connect(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("FTP connect failed: {}", e))?;
        stream
            .login(user, pass)
            .await
            .map_err(|e| anyhow::anyhow!("FTP login failed: {}", e))?;
        Ok(Self {
            conn: Mutex::new(Some(stream)),
        })
    }
}

impl FtpsClient {
    pub async fn connect(host: &str, port: u16, user: &str, pass: &str) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        let stream = AsyncNativeTlsFtpStream::connect(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("FTPS connect failed: {}", e))?;
        let mut stream = stream
            .into_secure(AsyncNativeTlsConnector::from(TlsConnector::new()), host)
            .await
            .map_err(|e| anyhow::anyhow!("FTPS TLS upgrade failed: {}", e))?;
        stream
            .login(user, pass)
            .await
            .map_err(|e| anyhow::anyhow!("FTPS login failed: {}", e))?;
        Ok(Self {
            conn: Mutex::new(Some(stream)),
        })
    }
}

const CHUNK_SIZE: usize = 64 * 1024;

type ProgressCb = Option<Box<dyn Fn(u64) -> ProgressAction + Send>>;

fn check(cb: &ProgressCb, total: u64) -> Result<()> {
    if let Some(cb) = cb {
        match cb(total) {
            ProgressAction::Continue => {}
            ProgressAction::Cancel => return Err(anyhow::anyhow!("cancelled")),
            ProgressAction::Pause => return Err(anyhow::anyhow!("paused")),
        }
    }
    Ok(())
}

macro_rules! gen_transfer {
    ($upload:ident, $download:ident, $stream:ty) => {
        async fn $upload(
            conn: &mut $stream,
            local: &str,
            remote: &str,
            on_progress: ProgressCb,
        ) -> Result<()> {
            let mut file = File::open(local)
                .await
                .map_err(|e| anyhow::anyhow!("cannot open local file: {}", e))?;

            let mut dstream = conn
                .put_with_stream(remote)
                .await
                .map_err(|e| anyhow::anyhow!("FTP upload stream failed: {}", e))?;

            let mut buf = vec![0u8; CHUNK_SIZE];
            let mut total = 0u64;
            loop {
                let n = file
                    .read(&mut buf)
                    .await
                    .map_err(|e| anyhow::anyhow!("FTP upload read failed: {}", e))?;
                if n == 0 {
                    break;
                }
                dstream
                    .write_all(&buf[..n])
                    .await
                    .map_err(|e| anyhow::anyhow!("FTP upload write failed: {}", e))?;
                total += n as u64;
                check(&on_progress, total)?;
            }

            conn.finalize_put_stream(dstream)
                .await
                .map_err(|e| anyhow::anyhow!("FTP upload finalize failed: {}", e))?;
            Ok(())
        }

        async fn $download(
            conn: &mut $stream,
            remote: &str,
            local: &str,
            on_progress: ProgressCb,
        ) -> Result<()> {
            let mut dstream = conn
                .retr_as_stream(remote)
                .await
                .map_err(|e| anyhow::anyhow!("FTP download stream failed: {}", e))?;

            let mut file = File::create(local)
                .await
                .map_err(|e| anyhow::anyhow!("cannot create local file: {}", e))?;

            let mut buf = vec![0u8; CHUNK_SIZE];
            let mut total = 0u64;
            loop {
                let n = dstream
                    .read(&mut buf)
                    .await
                    .map_err(|e| anyhow::anyhow!("FTP download read failed: {}", e))?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n])
                    .await
                    .map_err(|e| anyhow::anyhow!("FTP download write failed: {}", e))?;
                total += n as u64;
                check(&on_progress, total)?;
            }

            conn.finalize_retr_stream(dstream)
                .await
                .map_err(|e| anyhow::anyhow!("FTP download finalize failed: {}", e))?;
            Ok(())
        }
    };
}

gen_transfer!(upload_ftp, download_ftp, AsyncFtpStream);
gen_transfer!(upload_ftps, download_ftps, AsyncNativeTlsFtpStream);

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
                $upload(conn, local, remote, on_progress).await
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
                $download(conn, remote, local, on_progress).await
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

                let name = Path::new(path)
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

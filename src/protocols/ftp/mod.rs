use anyhow::Result;
use std::str::FromStr;
use std::time::SystemTime;
use tokio::sync::Mutex;

use suppaftp::async_native_tls::TlsConnector;
use suppaftp::{
    AsyncFtpStream, AsyncNativeTlsConnector, AsyncNativeTlsFtpStream, list::File as FtpListFile,
};

use crate::domain::file_entry::{EntryKind, FileEntry};

mod ops;
mod transfer;

pub struct FtpClient {
    pub conn: Mutex<Option<AsyncFtpStream>>,
}

pub struct FtpsClient {
    pub conn: Mutex<Option<AsyncNativeTlsFtpStream>>,
}

pub(super) fn parse_list_entry(line: &str) -> Option<FileEntry> {
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

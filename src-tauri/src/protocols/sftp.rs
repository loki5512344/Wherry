use async_trait::async_trait;
use anyhow::{Result, Context};
use ssh2::Session;
use std::net::TcpStream;
use std::path::Path;

use crate::domain::file_entry::{FileEntry, EntryKind};
use super::RemoteFs;

pub struct SftpClient {
    session: Session,
}

impl SftpClient {
    /// Подключение по паролю
    pub fn connect_password(host: &str, port: u16, user: &str, password: &str) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .context("TCP connect failed")?;
        let mut session = Session::new().context("SSH session init failed")?;
        session.set_tcp_stream(tcp);
        session.handshake().context("SSH handshake failed")?;
        session.userauth_password(user, password)
            .context("SSH password auth failed")?;
        Ok(Self { session })
    }

    /// Подключение по ключу
    pub fn connect_key(host: &str, port: u16, user: &str, key_path: &str) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .context("TCP connect failed")?;
        let mut session = Session::new().context("SSH session init failed")?;
        session.set_tcp_stream(tcp);
        session.handshake().context("SSH handshake failed")?;
        session.userauth_pubkey_file(user, None, Path::new(key_path), None)
            .context("SSH key auth failed")?;
        Ok(Self { session })
    }
}

#[async_trait]
impl RemoteFs for SftpClient {
    async fn list(&self, path: &str) -> Result<Vec<FileEntry>> {
        // TODO: перенести в spawn_blocking когда будет Arc<Session>
        let sftp = self.session.sftp().context("SFTP subsystem failed")?;
        let entries = sftp.readdir(Path::new(path)).context("readdir failed")?;

        let result = entries.into_iter().map(|(pb, stat)| {
            let kind = if stat.is_dir() {
                EntryKind::Dir
            } else if stat.file_type().is_symlink() {
                EntryKind::Symlink
            } else {
                EntryKind::File
            };
            FileEntry {
                name: pb.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: pb.to_string_lossy().to_string(),
                kind,
                size: stat.size,
                modified: stat.mtime.map(|t| t as i64),
                permissions: None,
            }
        }).collect();

        Ok(result)
    }

    async fn upload(&self, local: &str, remote: &str) -> Result<()> {
        // TODO: chunked upload с прогресс-событиями
        let sftp = self.session.sftp().context("SFTP subsystem failed")?;
        let mut local_file = std::fs::File::open(local)
            .context("open local file failed")?;
        let mut remote_file = sftp.create(Path::new(remote))
            .context("create remote file failed")?;
        std::io::copy(&mut local_file, &mut remote_file)?;
        Ok(())
    }

    async fn download(&self, remote: &str, local: &str) -> Result<()> {
        // TODO: chunked download с прогресс-событиями
        let sftp = self.session.sftp().context("SFTP subsystem failed")?;
        let mut remote_file = sftp.open(Path::new(remote))
            .context("open remote file failed")?;
        let mut local_file = std::fs::File::create(local)
            .context("create local file failed")?;
        std::io::copy(&mut remote_file, &mut local_file)?;
        Ok(())
    }

    async fn mkdir(&self, path: &str) -> Result<()> {
        let sftp = self.session.sftp().context("SFTP subsystem failed")?;
        sftp.mkdir(Path::new(path), 0o755).context("mkdir failed")?;
        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let sftp = self.session.sftp().context("SFTP subsystem failed")?;
        sftp.rename(Path::new(from), Path::new(to), None)
            .context("rename failed")?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let sftp = self.session.sftp().context("SFTP subsystem failed")?;
        // пробуем как файл, потом как директорию
        if sftp.unlink(Path::new(path)).is_err() {
            sftp.rmdir(Path::new(path)).context("delete failed")?;
        }
        Ok(())
    }

    async fn stat(&self, path: &str) -> Result<FileEntry> {
        let sftp = self.session.sftp().context("SFTP subsystem failed")?;
        let stat = sftp.stat(Path::new(path)).context("stat failed")?;
        Ok(FileEntry {
            name: Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.to_string(),
            kind: if stat.is_dir() { EntryKind::Dir } else { EntryKind::File },
            size: stat.size,
            modified: stat.mtime.map(|t| t as i64),
            permissions: None,
        })
    }
}

//! Чанкованная загрузка/выгрузка по SFTP с колбэком прогресса (pause/cancel).
use anyhow::{Context, Result};
use ssh2::Session;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::protocols::ProgressAction;

const CHUNK_SIZE: usize = 64 * 1024;

type ProgressCb = Option<Box<dyn Fn(u64) -> ProgressAction + Send>>;

/// Проверяет действие прогресса; возвращает Err для pause/cancel (прерывает цикл).
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

pub(super) async fn upload(
    session: Arc<Mutex<Session>>,
    local: String,
    remote: String,
    on_progress: ProgressCb,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let sftp = session
            .lock()
            .unwrap()
            .sftp()
            .context("SFTP subsystem failed")?;
        let mut local_file = std::fs::File::open(&local).context("open local file failed")?;
        let mut remote_file = sftp
            .create(Path::new(&remote))
            .context("create remote file failed")?;

        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut total = 0u64;
        loop {
            let n = local_file
                .read(&mut buf)
                .context("read local file failed")?;
            if n == 0 {
                break;
            }
            remote_file
                .write_all(&buf[..n])
                .context("write remote file failed")?;
            total += n as u64;
            check(&on_progress, total)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("SFTP upload task failed: {}", e))?
}

pub(super) async fn download(
    session: Arc<Mutex<Session>>,
    remote: String,
    local: String,
    on_progress: ProgressCb,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let sftp = session
            .lock()
            .unwrap()
            .sftp()
            .context("SFTP subsystem failed")?;
        let mut remote_file = sftp
            .open(Path::new(&remote))
            .context("open remote file failed")?;
        let mut local_file = std::fs::File::create(&local).context("create local file failed")?;

        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut total = 0u64;
        loop {
            let n = remote_file
                .read(&mut buf)
                .context("read remote file failed")?;
            if n == 0 {
                break;
            }
            local_file
                .write_all(&buf[..n])
                .context("write local file failed")?;
            total += n as u64;
            check(&on_progress, total)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("SFTP download task failed: {}", e))?
}

//! Чанкованная загрузка/выгрузка по FTP с колбэком прогресса (pause/cancel).
use anyhow::Result;
use futures_lite::io::{AsyncReadExt, AsyncWriteExt};
use suppaftp::AsyncFtpStream;
use tokio::fs::File;
use tokio::io::{AsyncReadExt as TokioReadExt, AsyncWriteExt as TokioWriteExt};

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
    conn: &mut AsyncFtpStream,
    local: &str,
    remote: &str,
    on_progress: ProgressCb,
) -> Result<()> {
    let mut file = File::open(local)
        .await
        .map_err(|e| anyhow::anyhow!("cannot open local file: {}", e))?;

    let mut stream = conn
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
        stream
            .write_all(&buf[..n])
            .await
            .map_err(|e| anyhow::anyhow!("FTP upload write failed: {}", e))?;
        total += n as u64;
        check(&on_progress, total)?;
    }

    conn.finalize_put_stream(stream)
        .await
        .map_err(|e| anyhow::anyhow!("FTP upload finalize failed: {}", e))?;
    Ok(())
}

pub(super) async fn download(
    conn: &mut AsyncFtpStream,
    remote: &str,
    local: &str,
    on_progress: ProgressCb,
) -> Result<()> {
    let mut stream = conn
        .retr_as_stream(remote)
        .await
        .map_err(|e| anyhow::anyhow!("FTP download stream failed: {}", e))?;

    let mut file = File::create(local)
        .await
        .map_err(|e| anyhow::anyhow!("cannot create local file: {}", e))?;

    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut total = 0u64;
    loop {
        let n = stream
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

    conn.finalize_retr_stream(stream)
        .await
        .map_err(|e| anyhow::anyhow!("FTP download finalize failed: {}", e))?;
    Ok(())
}

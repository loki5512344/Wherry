//! Проверка SSH host key через ~/.ssh/known_hosts (TOFU: неизвестный ключ добавляется).
use anyhow::{Context, Result, bail};
use ssh2::{CheckResult, HashType, KnownHostFileKind, Session};
use std::path::PathBuf;

fn known_hosts_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".ssh/known_hosts"))
}

fn fingerprint_hex(session: &Session) -> String {
    session
        .host_key_hash(HashType::Sha256)
        .map(|h| {
            h.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(":")
        })
        .unwrap_or_else(|| "unknown".into())
}

pub(super) fn verify_host_key(session: &Session, host: &str, port: u16) -> Result<()> {
    let mut known = session.known_hosts().context("known_hosts init failed")?;

    if let Some(ref path) = known_hosts_path() {
        let _ = known.read_file(path, KnownHostFileKind::OpenSSH);
    }

    let (key, key_type) = session
        .host_key()
        .context("no host key received from server")?;

    match known.check_port(host, port, key) {
        CheckResult::Match => {} // known and verified
        CheckResult::Mismatch => {
            bail!(
                "SSH host key mismatch for {}!\n\
                 The server's host key has changed since the last connection.\n\
                 This could mean someone is intercepting the connection (MITM attack).\n\
                 Fingerprint (SHA256): {}",
                host,
                fingerprint_hex(session)
            );
        }
        CheckResult::NotFound => {
            tracing::info!(
                "Unknown host key for {}, adding to known_hosts (SHA256: {})",
                host,
                fingerprint_hex(session)
            );
            known
                .add(host, key, "loflum", key_type.into())
                .context("failed to add host key to known_hosts")?;
            if let Some(ref path) = known_hosts_path()
                && let Err(e) = known.write_file(path, KnownHostFileKind::OpenSSH)
            {
                tracing::warn!("failed to write known_hosts: {}", e);
            }
        }
        CheckResult::Failure => {
            bail!("known_hosts check failed for {}", host);
        }
    }

    Ok(())
}

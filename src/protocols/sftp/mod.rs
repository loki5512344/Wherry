use anyhow::{Context, Result};
use ssh2::Session;
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};

mod hostkey;
mod ops;
mod transfer;

pub struct SftpClient {
    session: Arc<Mutex<Session>>,
}

impl SftpClient {
    /// Подключение по паролю
    pub fn connect_password(host: &str, port: u16, user: &str, password: &str) -> Result<Self> {
        let session = handshake(host, port)?;
        session
            .userauth_password(user, password)
            .context("SSH password auth failed")?;
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
        })
    }

    /// Подключение по явно выбранному ключу. `passphrase` — пароль из формы
    /// (если задан): им расшифровывается защищённый паролем ключ.
    pub fn connect_key(
        host: &str,
        port: u16,
        user: &str,
        key_path: &str,
        passphrase: Option<&str>,
    ) -> Result<Self> {
        let session = handshake(host, port)?;
        session
            .userauth_pubkey_file(user, None, Path::new(key_path), passphrase)
            .with_context(|| format!("SSH key auth failed ({})", key_path))?;
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
        })
    }

    /// Автоматический подбор SSH-аутентификации, когда пароль не задан и ключ
    /// не выбран: сперва ssh-agent, затем стандартные ключи из ~/.ssh.
    pub fn connect_auto(host: &str, port: u16, user: &str) -> Result<Self> {
        let session = handshake(host, port)?;

        // 1) ssh-agent — покрывает ключи с passphrase, добавленные в агент.
        if session.userauth_agent(user).is_ok() && session.authenticated() {
            return Ok(Self {
                session: Arc::new(Mutex::new(session)),
            });
        }

        // 2) стандартные ключи в ~/.ssh (без passphrase).
        let ssh_dir = dirs::home_dir()
            .context("Cannot resolve home directory")?
            .join(".ssh");
        let mut tried = vec!["ssh-agent".to_string()];
        for name in ["id_ed25519", "id_ecdsa", "id_rsa"] {
            let key = ssh_dir.join(name);
            if !key.is_file() {
                continue;
            }
            if session.userauth_pubkey_file(user, None, &key, None).is_ok()
                && session.authenticated()
            {
                return Ok(Self {
                    session: Arc::new(Mutex::new(session)),
                });
            }
            tried.push(format!("~/.ssh/{}", name));
        }

        anyhow::bail!(
            "SSH auth failed: no password given, and none of [{}] were accepted. \
             Pick a key file manually or enter a password.",
            tried.join(", ")
        )
    }
}

/// TCP + SSH handshake + проверка host key (общий пролог обоих способов аутентификации).
fn handshake(host: &str, port: u16) -> Result<Session> {
    let tcp = TcpStream::connect(format!("{}:{}", host, port)).context("TCP connect failed")?;
    let mut session = Session::new().context("SSH session init failed")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("SSH handshake failed")?;
    hostkey::verify_host_key(&session, host, port)?;
    Ok(session)
}

//! Логика подключения: сборка параметров и запуск async-коннекта.
use std::sync::Arc;

use crate::domain::connection::{ConnectionParams, Protocol};
use crate::domain::file_entry::FileEntry;
use crate::fs::remote::RemoteRegistry;
use crate::protocols::{RemoteFs, ftp::FtpClient, sftp::SftpClient};
use crate::storage::keychain;
use crate::ui::state::{AppState, PendingConnect};

pub(super) fn do_connect(
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
) {
    state.connect_error.clear();

    if state.connect_host.trim().is_empty() {
        state.connect_error = "Host is required.".into();
        return;
    }
    if state.connect_user.trim().is_empty() {
        state.connect_error = "Username is required.".into();
        return;
    }
    if !state.connect_port.trim().is_empty() && state.connect_port.trim().parse::<u16>().is_err() {
        state.connect_error = "Port must be a number between 1 and 65535.".into();
        return;
    }

    let protocol = match state.connect_protocol {
        0 => Protocol::Sftp,
        1 => Protocol::Ftp,
        _ => Protocol::Ftps,
    };

    let port: u16 = state.connect_port.parse().unwrap_or(match protocol {
        Protocol::Sftp => 22,
        Protocol::Ftp => 21,
        Protocol::Ftps => 990,
    });

    let params = ConnectionParams {
        id: uuid::Uuid::new_v4().to_string(),
        label: if state.connect_label.is_empty() {
            format!(
                "{} ({})",
                state.connect_host,
                match protocol {
                    Protocol::Sftp => "SFTP",
                    Protocol::Ftp => "FTP",
                    Protocol::Ftps => "FTPS",
                }
            )
        } else {
            state.connect_label.clone()
        },
        protocol,
        host: state.connect_host.clone(),
        port,
        username: state.connect_user.clone(),
        password: if state.connect_pass.is_empty() {
            None
        } else {
            Some(state.connect_pass.clone())
        },
        key_path: if state.connect_key_path.is_empty() {
            None
        } else {
            Some(state.connect_key_path.clone())
        },
    };

    spawn_connect(state, registry, rt_handle, params);
}

/// Общий код запуска подключения — используется и диалогом New Connection,
/// и переподключением из истории (см. `reconnect_from_history`).
pub fn spawn_connect(
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    params: ConnectionParams,
) {
    let registry = registry.clone();
    let params_clone = params.clone();
    let result = Arc::new(std::sync::Mutex::new(None));
    let result_clone = result.clone();

    state.connect_loading = true;
    rt_handle.spawn(async move {
        let r = connect_async(&registry, &params_clone).await;
        *result_clone.lock().unwrap() = Some(r);
    });

    state.pending_connect = Some(PendingConnect { result });
}

async fn connect_async(
    registry: &RemoteRegistry,
    params: &ConnectionParams,
) -> Result<(ConnectionParams, Vec<FileEntry>), String> {
    // Пароль резолвим лениво по месту: для ключевой/автоматической SSH-аутен-
    // тификации он не нужен, и обязательный поход в keychain раньше валил
    // подключение по ключу с пустым паролем ("нет пароля в keychain").
    let stored_password = || -> Option<String> {
        params
            .password
            .clone()
            .or_else(|| keychain::get_password(&params.id).ok())
    };

    let fs: Arc<dyn RemoteFs> = match params.protocol {
        Protocol::Sftp => {
            if let Some(key_path) = &params.key_path {
                // Явно выбранный ключ; пароль из формы (если есть) — passphrase.
                Arc::new(
                    SftpClient::connect_key(
                        &params.host,
                        params.port,
                        &params.username,
                        key_path,
                        params.password.as_deref(),
                    )
                    .map_err(|e| e.to_string())?,
                )
            } else if let Some(password) = stored_password() {
                Arc::new(
                    SftpClient::connect_password(
                        &params.host,
                        params.port,
                        &params.username,
                        &password,
                    )
                    .map_err(|e| e.to_string())?,
                )
            } else {
                // Пароль пуст и ключ не выбран — ищем ключ сами (агент, ~/.ssh).
                Arc::new(
                    SftpClient::connect_auto(&params.host, params.port, &params.username)
                        .map_err(|e| e.to_string())?,
                )
            }
        }
        Protocol::Ftp => {
            let password = stored_password().ok_or("Password is required for FTP connections.")?;
            Arc::new(
                FtpClient::connect(&params.host, params.port, &params.username, &password)
                    .await
                    .map_err(|e| e.to_string())?,
            )
        }
        Protocol::Ftps => return Err("FTPS not yet implemented".into()),
    };

    registry.insert(params.id.clone(), fs.clone());
    let entries = fs.list("/").await.map_err(|e| e.to_string())?;
    Ok((params.clone(), entries))
}

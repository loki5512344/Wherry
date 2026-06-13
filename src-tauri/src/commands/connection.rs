use tauri::State;
use std::sync::Arc;
use crate::domain::connection::{ConnectionParams, Protocol};
use crate::fs::remote::RemoteRegistry;
use crate::protocols::sftp::SftpClient;
use crate::protocols::ftp::FtpClient;
use crate::storage::keychain;

#[tauri::command]
pub async fn connect(
    registry: State<'_, Arc<RemoteRegistry>>,
    params: ConnectionParams,
) -> Result<(), String> {
    let password = if let Some(p) = &params.password {
        p.clone()
    } else {
        keychain::get_password(&params.id).map_err(|e| e.to_string())?
    };

    let fs: Arc<dyn crate::protocols::RemoteFs> = match params.protocol {
        Protocol::Sftp => {
            if let Some(key_path) = &params.key_path {
                Arc::new(
                    SftpClient::connect_key(&params.host, params.port, &params.username, key_path)
                        .map_err(|e| e.to_string())?
                )
            } else {
                Arc::new(
                    SftpClient::connect_password(&params.host, params.port, &params.username, &password)
                        .map_err(|e| e.to_string())?
                )
            }
        }
        Protocol::Ftp => {
            Arc::new(
                FtpClient::connect(&params.host, params.port, &params.username, &password)
                    .await
                    .map_err(|e| e.to_string())?
            )
        }
        Protocol::Ftps => {
            return Err("FTPS not yet implemented (suppaftp API compat)".to_string());
        }
    };

    registry.insert(params.id, fs);
    Ok(())
}

#[tauri::command]
pub async fn disconnect(
    registry: State<'_, Arc<RemoteRegistry>>,
    connection_id: String,
) -> Result<(), String> {
    registry.remove(&connection_id);
    Ok(())
}

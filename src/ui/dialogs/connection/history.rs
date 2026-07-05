//! Действия над записями истории: переподключение, «Изменить», «Сохранить» как Site.
use std::sync::Arc;

use super::actions::spawn_connect;
use crate::domain::connection::{ConnectionParams, Protocol};
use crate::fs::remote::RemoteRegistry;
use crate::storage::keychain;
use crate::ui::state::AppState;

fn protocol_index(protocol: &Protocol) -> usize {
    match protocol {
        Protocol::Sftp => 0,
        Protocol::Ftp => 1,
        Protocol::Ftps => 2,
    }
}

/// Переподключение по клику на запись истории — пароль (если есть) уже лежит
/// в keychain под `entry.conn_id`, поэтому диалог не нужен.
pub fn reconnect_from_history(
    state: &mut AppState,
    registry: &Arc<RemoteRegistry>,
    rt_handle: &tokio::runtime::Handle,
    entry: &crate::ui::state::HistoryEntry,
) {
    state.connect_error.clear();
    let params = ConnectionParams {
        id: entry.conn_id.clone(),
        label: format!("{}@{}", entry.user, entry.host),
        protocol: entry.protocol.clone(),
        host: entry.host.clone(),
        port: entry.port,
        username: entry.user.clone(),
        password: None,
        key_path: entry.key_path.clone(),
    };
    spawn_connect(state, registry, rt_handle, params);
}

/// «Изменить» в меню истории — открывает диалог New Connection, предзаполненный
/// этой записью (включая пароль из keychain, если он там есть).
pub fn edit_history_entry(state: &mut AppState, entry: &crate::ui::state::HistoryEntry) {
    state.connect_error.clear();
    state.connect_host = entry.host.clone();
    state.connect_user = entry.user.clone();
    state.connect_port = entry.port.to_string();
    state.connect_protocol = protocol_index(&entry.protocol);
    state.connect_key_path = entry.key_path.clone().unwrap_or_default();
    state.connect_pass = keychain::get_password(&entry.conn_id).unwrap_or_default();
    state.connect_label.clear();
    state.show_connect_dialog = true;
}

/// «Сохранить» в меню истории — превращает разовое подключение в постоянный Site.
pub fn save_history_as_site(
    db: &Arc<std::sync::Mutex<rusqlite::Connection>>,
    sites: &mut Vec<crate::domain::site::Site>,
    entry: &crate::ui::state::HistoryEntry,
) -> Result<(), String> {
    let site = crate::domain::site::Site {
        id: entry.conn_id.clone(),
        name: format!("{}@{}", entry.user, entry.host),
        protocol: entry.protocol.clone(),
        host: entry.host.clone(),
        port: entry.port,
        username: entry.user.clone(),
        key_path: entry.key_path.clone(),
        folder: None,
        note: None,
    };
    let conn = db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    crate::storage::db::save_site(&conn, &site).map_err(|e| e.to_string())?;
    drop(conn);
    if let Some(existing) = sites.iter_mut().find(|s| s.id == site.id) {
        *existing = site;
    } else {
        sites.push(site);
    }
    Ok(())
}

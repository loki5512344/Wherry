//! Постановка выбранного файла в очередь из тулбара (Upload/Download).
use crate::domain::transfer::{TransferKind, TransferTask};
use crate::transfer::queue::TransferQueue;
use crate::ui::state::AppState;

pub fn queue_selected_upload(state: &mut AppState, queue: &TransferQueue) {
    let Some(name) = state.local_selected.clone() else {
        return;
    };
    let Some(tab) = state.active_tab_ref() else {
        return;
    };
    let connection_id = tab.id.clone();
    let remote_dir = tab.remote_path.clone();
    let local_path = format!("{}/{}", state.local_path.trim_end_matches('/'), name);
    let remote_path = format!("{}/{}", remote_dir.trim_end_matches('/'), name);
    let task = TransferTask::new(
        TransferKind::Upload,
        connection_id,
        local_path,
        remote_path,
        name.clone(),
        0,
    );
    queue.push(task);
    state.status_message = crate::i18n::tf("toolbar.upload_queued", &[("{name}", &name)]);
}

pub fn queue_selected_download(state: &mut AppState, queue: &TransferQueue) {
    let Some(tab) = state.active_tab_ref() else {
        return;
    };
    let Some(name) = tab.remote_selected.clone() else {
        return;
    };
    let Some(entry) = tab.remote_entries.iter().find(|e| e.name == name) else {
        return;
    };
    let connection_id = tab.id.clone();
    let remote_path = entry.path.clone();
    let local_path = format!("{}/{}", state.local_path.trim_end_matches('/'), name);
    let task = TransferTask::new(
        TransferKind::Download,
        connection_id,
        local_path,
        remote_path,
        name.clone(),
        0,
    );
    queue.push(task);
    state.status_message = crate::i18n::tf("toolbar.download_queued", &[("{name}", &name)]);
}

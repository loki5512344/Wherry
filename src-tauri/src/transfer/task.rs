use crate::domain::transfer::{TransferTask, TransferKind};

pub fn new_task(
    kind: TransferKind,
    connection_id: impl Into<String>,
    local_path: impl Into<String>,
    remote_path: impl Into<String>,
    file_name: impl Into<String>,
    total_bytes: u64,
) -> TransferTask {
    TransferTask::new(
        kind,
        connection_id.into(),
        local_path.into(),
        remote_path.into(),
        file_name.into(),
        total_bytes,
    )
}

//! Integration-level smoke tests: verify module imports and command function
//! signatures compile correctly. Does not exercise Tauri runtime.
use wherry_lib::domain::{
    ConnectionParams, EntryKind, FileEntry, Protocol, Site, TaskState, TransferKind, TransferTask,
};
use wherry_lib::error::AppError;
use wherry_lib::settings::{
    add_bookmark, get_bookmarks, get_bool, get_setting, get_u32, remove_bookmark, set_bool,
    set_setting, set_u32,
};
use wherry_lib::storage::{
    HistoryRow, add_history_entry, clear_history, delete_site, find_history_conn_id, get_history,
    get_sites, init_tables, save_site,
};

#[test]
fn test_domain_types_compile() {
    let _ = ConnectionParams {
        id: "".into(),
        label: "".into(),
        protocol: Protocol::Sftp,
        host: "".into(),
        port: 22,
        username: "".into(),
        password: None,
        key_path: None,
    };
    let _ = FileEntry {
        name: "".into(),
        path: "".into(),
        kind: EntryKind::File,
        size: None,
        modified: None,
        permissions: None,
    };
    let _ = TransferTask::new(
        TransferKind::Download,
        "".into(),
        "".into(),
        "".into(),
        "".into(),
        0,
    );
    let _ = vec![
        AppError::NotFound("".into()),
        AppError::AuthFailed("".into()),
        AppError::Io("".into()),
        AppError::Protocol("".into()),
        AppError::InvalidInput("".into()),
        AppError::AlreadyExists("".into()),
        AppError::Internal("".into()),
    ];
    let _ = Site {
        id: "".into(),
        name: "".into(),
        protocol: Protocol::Ftp,
        host: "".into(),
        port: 21,
        username: "".into(),
        password: None,
        key_path: None,
        folder: None,
        note: None,
    };
    let _ = TaskState::Queued;
}

#[test]
fn test_db_functions_compile() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    init_tables(&conn).unwrap();

    let _: Vec<Site> = get_sites(&conn).unwrap();
    let _: Vec<(i64, String, String)> = get_bookmarks(&conn).unwrap();
    let _: Vec<HistoryRow> = get_history(&conn).unwrap();
    let _: Option<String> = find_history_conn_id(&conn, "host", 22, "user").unwrap();
    let _: Option<String> = get_setting(&conn, "key");
    let _: bool = get_bool(&conn, "key", false);
    let _: u32 = get_u32(&conn, "key", 0);

    add_bookmark(&conn, "name", "/path").unwrap();
    remove_bookmark(&conn, 1).unwrap();
    save_site(
        &conn,
        &Site {
            id: "s".into(),
            name: "n".into(),
            protocol: Protocol::Ftp,
            host: "h".into(),
            port: 21,
            username: "u".into(),
            password: None,
            key_path: None,
            folder: None,
            note: None,
        },
    )
    .unwrap();
    delete_site(&conn, "s").unwrap();
    add_history_entry(&conn, "h", 21, "u", "c", &Protocol::Ftp, None).unwrap();
    clear_history(&conn).unwrap();
    set_setting(&conn, "k", "v").unwrap();
    set_bool(&conn, "k", true);
    set_u32(&conn, "k", 42);
}

#[test]
fn test_stateless_commands_compile() {
    let _ = wherry_lib::commands::platform_info();
    let _ = wherry_lib::commands::app_data_dir();
    let _ = wherry_lib::commands::local_home_dir();
}

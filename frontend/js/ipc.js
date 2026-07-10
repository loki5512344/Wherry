// Thin, typed-by-convention wrapper around the Tauri command surface defined
// in src/commands.rs / registered in src/lib.rs. One function per command —
// nothing here should contain UI logic, only the invoke() call and the small
// amount of shape-normalization Rust's serde output needs (tuples -> objects).
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// --- Connections ------------------------------------------------------------

export const connect = (params) => invoke("connect", { params });
export const disconnect = (connectionId) => invoke("disconnect", { connectionId });

// --- Remote file ops ---------------------------------------------------------

export const remoteList = (connectionId, path) => invoke("remote_list", { connectionId, path });
export const remoteStat = (connectionId, path) => invoke("remote_stat", { connectionId, path });
export const remoteMkdir = (connectionId, path) => invoke("remote_mkdir", { connectionId, path });
export const remoteRename = (connectionId, from, to) =>
  invoke("remote_rename", { connectionId, from, to });
export const remoteDelete = (connectionId, path) => invoke("remote_delete", { connectionId, path });

// --- Local file ops -----------------------------------------------------------

export const localList = (path) => invoke("local_list", { path });
export const localHomeDir = () => invoke("local_home_dir");
export const localMkdir = (path) => invoke("local_mkdir", { path });
export const localRename = (from, to) => invoke("local_rename", { from, to });
export const localDelete = (path) => invoke("local_delete", { path });
export const localOpen = (path) => invoke("local_open", { path });
export const localMoveInto = (srcPath, destDir) =>
  invoke("local_move_into", { srcPath, destDir });

// --- Transfers ----------------------------------------------------------------

export const enqueueTransfer = (kind, connectionId, localPath, remotePath, fileName, totalBytes) =>
  invoke("enqueue_transfer", {
    kind,
    connectionId,
    localPath,
    remotePath,
    fileName,
    totalBytes,
  });
export const listTasks = () => invoke("list_tasks");
export const pauseTask = (id) => invoke("pause_task", { id });
export const resumeTask = (id) => invoke("resume_task", { id });
export const cancelTask = (id) => invoke("cancel_task", { id });
export const removeTask = (id) => invoke("remove_task", { id });
export const setMaxConcurrent = (n) => invoke("set_max_concurrent", { n });

// --- Sites / bookmarks / history / prefs --------------------------------------

export const listSites = () => invoke("list_sites");
export const saveSite = (site) => invoke("save_site", { site });
export const deleteSite = (id) => invoke("delete_site", { id });

export const listBookmarks = () =>
  invoke("list_bookmarks").then((rows) => rows.map(([id, name, path]) => ({ id, name, path })));
export const addBookmark = (name, path) => invoke("add_bookmark", { name, path });
export const removeBookmark = (id) => invoke("remove_bookmark", { id });

// HistoryRow is a Rust tuple, serialized as a positional JSON array:
// (host, port, username, connectedAt, connId, protocol, keyPath)
export const listHistory = () =>
  invoke("list_history").then((rows) =>
    rows.map(([host, port, username, connectedAt, connId, protocol, keyPath]) => ({
      host,
      port,
      username,
      connectedAt,
      connId,
      protocol,
      keyPath,
    })),
  );
export const clearHistory = () => invoke("clear_history");
export const findHistoryConnId = (host, port, username) =>
  invoke("find_history_conn_id", { host, port, username });

export const getPref = (key) => invoke("get_pref", { key });
export const setPref = (key, value) => invoke("set_pref", { key, value });

// Read-back of passwords is deliberately absent: the backend resolves them
// from the system keychain at connect() time and never hands them to the UI.
export const deletePassword = (siteId) => invoke("delete_password", { siteId });
export const appDataDir = () => invoke("app_data_dir");
export const platformInfo = () => invoke("platform_info");

// --- Events -------------------------------------------------------------------
// Payloads here are snake_case on the wire (worker.rs's ProgressPayload/
// StatePayload aren't #[serde(rename_all = "camelCase")]), unlike every other
// struct crossing the IPC boundary -- normalize to camelCase right here so
// nothing downstream has to remember the exception.

export function onTransferProgress(fn) {
  return listen("transfer-progress", (e) => {
    const p = e.payload;
    fn({ id: p.id, transferredBytes: p.transferred_bytes, speed: p.speed, etaSecs: p.eta_secs });
  });
}

export function onTransferStateChanged(fn) {
  return listen("transfer-state-changed", (e) => {
    fn({ id: e.payload.id, state: e.payload.state });
  });
}

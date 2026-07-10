# Wherry Tauri backend API (design)

Goal: keep everything under `src/domain`, `src/fs`, `src/protocols`, `src/storage`, `src/transfer` as the Tauri backend. All of it already speaks in `serde` (camelCase) structs, so this is mostly a thin `#[tauri::command]` wrapper layer plus an event bus to replace the current per-frame egui polling.

State the current egui frontend polls every frame that the new frontend instead gets pushed to it via Tauri events (see "Events" below): `state.queue_tasks` (`TransferQueue::all()`), `state.tabs[].status` (`ConnectionStatus`).

## Managed state (`tauri::State`)

Same objects `lib.rs::run()` already builds, handed to Tauri via `.manage(...)` instead of into `FileManagerApp`:

- `Arc<Mutex<rusqlite::Connection>>` — sqlite db (sites/bookmarks/history/settings)
- `Arc<RemoteRegistry>` — live `connection_id -> Arc<dyn RemoteFs>` map
- `TransferQueue` — shared transfer task list
- `Arc<AtomicU32>` — max_concurrent, mutated live by Settings → Transfers

`transfer::worker::spawn_worker` keeps running as-is inside `tauri::Builder::setup`, using the app handle to `emit` progress instead of the queue being polled.

## Commands

### Connections (`src/protocols`, `src/fs/remote.rs`)

- `connect(params: ConnectionParams) -> Result<String, String>` — dispatches to `FtpClient::connect` / `SftpClient::connect_password|connect_key|connect_auto` based on `params.protocol`/`password`/`key_path`; on success `RemoteRegistry::insert(id, fs)`, `db::add_history_entry(...)`, returns `connection_id`. Emits `connection-status` (`Connecting` then `Connected`/`Error`) — see Events.
- `disconnect(connection_id: String)` — `RemoteRegistry::remove`.

### Remote file ops (`RemoteFs` trait, per `connection_id`)

- `remote_list(connection_id: String, path: String) -> Result<Vec<FileEntry>, String>`
- `remote_stat(connection_id: String, path: String) -> Result<FileEntry, String>`
- `remote_mkdir(connection_id: String, path: String) -> Result<(), String>`
- `remote_rename(connection_id: String, from: String, to: String) -> Result<(), String>`
- `remote_delete(connection_id: String, path: String) -> Result<(), String>`

All resolve `Arc<dyn RemoteFs>` from `RemoteRegistry::get`, return `"connection not found"` error if missing (mirrors `worker.rs` behavior).

### Local file ops (`src/fs/local.rs` — already sync, no connection_id)

- `local_list(path: String) -> Result<Vec<FileEntry>, String>`
- `local_home_dir() -> String`
- `local_mkdir(path: String) -> Result<(), String>`
- `local_rename(from: String, to: String) -> Result<(), String>`
- `local_delete(path: String) -> Result<(), String>`
- `local_open(path: String) -> Result<(), String>` — reveal/open in system app
- `local_move_into(src_path: String, dest_dir: String) -> Result<(), String>` — drag&drop move

### Transfers (`src/transfer`)

- `enqueue_transfer(kind: TransferKind, connection_id: String, local_path: String, remote_path: String, file_name: String, total_bytes: u64) -> String` — builds `TransferTask::new(...)`, `queue.push(task)`, returns task id. Worker picks it up same as today.
- `list_tasks() -> Vec<TransferTask>` — for initial load / reconnect after frontend refresh; live updates come from events, not polling.
- `pause_task(id: String)` / `resume_task(id: String)` — `queue.update_state(id, TaskState::Paused/Queued)` (worker's progress callback already checks for `Paused`/`Cancelled` each callback tick).
- `cancel_task(id: String)` — `queue.update_state(id, TaskState::Cancelled)`.
- `remove_task(id: String)` — `queue.remove(id)` (for clearing completed/failed rows).
- `set_max_concurrent(n: u32)` — stores atomically + persists via `db::set_u32`.

### Sites / bookmarks / history / settings (`src/storage/db`)

- `list_sites`, `save_site(site: Site)`, `delete_site(id: String)`
- `list_bookmarks() -> Vec<(i64, String, String)>` (id, name, path), `add_bookmark(name, path) -> i64`, `remove_bookmark(id: i64)`
- `list_history() -> Vec<HistoryRow>`, `clear_history()`, `find_history_conn_id(host, port, username) -> Option<String>`
- `get_pref_bool/str/u32(key)`, `set_pref_bool/str/u32(key, value)` — thin wrapper over `storage::db::settings`, used for confirm-before-delete, default local folder, language, transfer concurrency, panel visibility persistence if we decide to persist that (currently session-only in egui, per `state/mod.rs` comment) — confirm with FRONTEND_SPEC.md once written.

Credentials: `keyring` crate stays as-is; password storage keyed by `ConnectionParams.id` (stable id from history/sites), unchanged from current behavior — no new command needed beyond what `connect` already resolves internally.

## Events (app -> frontend, `app_handle.emit(...)`)

Replaces per-frame polling of `queue.all()` / `tabs[].status` in the egui loop:

- `transfer-progress` payload `{ id, transferredBytes, speed, etaSecs }` — emitted from the same throttled callback in `worker.rs::run_transfer` (already rate-limited to ~10/s via `ProgressThrottle`), just add `app_handle.emit(...)` next to `queue.update_progress(...)`.
- `transfer-state-changed` payload `{ id, state: TaskState }` — emitted next to every `queue.update_state(...)` call (queued->running->completed/failed/cancelled/paused).
- `connection-status-changed` payload `{ connectionId, status: ConnectionStatus }` — emitted from `connect`/`disconnect` and on unexpected drop (if/when we add liveness detection — not present today, egui doesn't detect mid-session drops either, so parity-only for v1).

Frontend subscribes once via `@tauri-apps/api/event.listen`, keeps a local task/connection map updated from these — no polling loop needed like `queue.all()` was in egui.

## Serialization note

Every domain type already derives `Serialize`/`Deserialize` with `#[serde(rename_all = "camelCase")]` (`ConnectionParams`, `FileEntry`, `Site`, `TransferTask`, `TaskState`, `Protocol`, `EntryKind`, `ConnectionStatus`) — these cross the Tauri IPC boundary unchanged, no adapter layer needed. `anyhow::Error` doesn't implement `Serialize`, so every command returns `Result<T, String>` (`.map_err(|e| e.to_string())`), matching the existing convention of stringly-typed errors already used in `TaskState::Failed(String)`.

## Open question for scaffolding phase (task #3)

Whether to keep `src/` as a single crate with a new `src-tauri/` binary depending on it (`wherry_lib` is already `staticlib/cdylib/rlib`, so this fits cleanly), or fold `src-tauri` in place of `main.rs`/`lib.rs::run()`. Recommend the former — cleanest separation, no changes to `domain`/`fs`/`protocols`/`storage`/`transfer` needed at all, only `lib.rs::run()` and `main.rs` get replaced by Tauri's builder.

// Local/remote filesystem operations shared by the toolbar, sidebar, file
// table context menus and drag&drop handlers. Mirrors the old
// src/ui/panels/{local_pane,remote_pane}/actions.rs + app/ops.rs.
import { state, notify, activeTab } from "./store.js";
import * as ipc from "./ipc.js";
import { t } from "./i18n.js";

function withParent(entries, path) {
  if (path === "/" || path === "") return entries;
  return [{ name: "..", path: parentPath(path), kind: "dir", size: null, modified: null, permissions: null }, ...entries];
}

export function parentPath(path) {
  const trimmed = path.replace(/\/+$/, "");
  if (!trimmed || trimmed === "") return "/";
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) return "/";
  return trimmed.slice(0, idx);
}

// --- Local -------------------------------------------------------------------

export async function refreshLocal() {
  try {
    const entries = await ipc.localList(state.local.path);
    state.local.entries = withParent(entries, state.local.path);
    notify();
  } catch (err) {
    state.statusMessage = `Local list error: ${err}`;
    notify();
  }
}

export function navigateLocal(path) {
  state.local.path = path;
  state.local.selected = [];
  state.local.selectAnchor = null;
  // Navigating local (sidebar quick-access/bookmarks especially) has to
  // bring the Local pane into view -- otherwise clicking it while a remote
  // tab is displayed silently updates state nobody's looking at and reads
  // as "nothing happened". notify() here (before the async list call even
  // starts) is what makes the tab switch itself feel instant.
  state.activeTabId = null;
  state.activePane = "local";
  notify();
  refreshLocal();
}

export async function initLocal() {
  const home = await ipc.localHomeDir();
  state.local.path = home;
  const qa = state.sidebar.quickAccess;
  const set = (name, suffix) => {
    const item = qa.find((q) => q.name === name);
    if (item) item.path = suffix ? `${home}/${suffix}` : home;
  };
  set("Home", "");
  set("Desktop", "Desktop");
  set("Documents", "Documents");
  set("Downloads", "Downloads");
  set("Pictures", "Pictures");
  set("Music", "Music");
  set("Videos", "Videos");
  await refreshLocal();
  try {
    state.sidebar.bookmarks = await ipc.listBookmarks();
  } catch {
    /* bookmarks table may be empty, fine */
  }
  notify();
}

export function openLocalEntry(name) {
  if (name === "..") {
    navigateLocal(parentPath(state.local.path));
    return;
  }
  const entry = state.local.entries.find((e) => e.name === name);
  if (!entry) return;
  if (entry.kind === "dir") {
    navigateLocal(entry.path);
  } else {
    ipc
      .localOpen(entry.path)
      .then(() => {
        state.statusMessage = t("panels.opening", { name: entry.name });
        notify();
      })
      .catch((err) => {
        state.statusMessage = `Open failed: ${err}`;
        notify();
      });
  }
}

export async function addBookmark(name, path) {
  if (state.sidebar.bookmarks.some((b) => b.path === path)) {
    state.statusMessage = t("panels.alreadyBookmarked");
    notify();
    return;
  }
  const id = await ipc.addBookmark(name, path);
  state.sidebar.bookmarks.push({ id, name, path });
  state.statusMessage = t("panels.bookmarkAdded");
  notify();
}

export async function removeBookmark(id) {
  await ipc.removeBookmark(id);
  state.sidebar.bookmarks = state.sidebar.bookmarks.filter((b) => b.id !== id);
  notify();
}

// --- Remote --------------------------------------------------------------------

export async function refreshRemote(tab) {
  tab.loading = true;
  notify();
  try {
    const entries = await ipc.remoteList(tab.id, tab.remotePath);
    tab.remoteEntries = withParent(entries, tab.remotePath);
  } catch (err) {
    state.statusMessage = `Remote list error: ${err}`;
  } finally {
    tab.loading = false;
    notify();
  }
}

export function navigateRemote(tab, path) {
  tab.remotePath = path;
  tab.remoteSelected = [];
  tab.remoteSelectAnchor = null;
  if (!tab.loading) refreshRemote(tab);
}

export function openRemoteEntry(tab, name) {
  if (name === "..") {
    navigateRemote(tab, parentPath(tab.remotePath));
    return;
  }
  const entry = tab.remoteEntries.find((e) => e.name === name);
  if (!entry) return;
  if (entry.kind === "dir") {
    navigateRemote(tab, entry.path);
  } else {
    queueDownload(tab, entry);
  }
}

export function closeTab(tabId) {
  ipc.disconnect(tabId);
  state.tabs = state.tabs.filter((t) => t.id !== tabId);
  // Clean up layout — remove panes referencing this tab, or reset to Local
  state.layout.panes = state.layout.panes
    .map((p) => (p.tabId === tabId ? { ...p, tabId: null } : p))
    // If we ended up with duplicate Local panes, merge them
    .filter((p, i, arr) => p.tabId !== null || arr.findIndex((x) => x.tabId === null) === i);
  // Keep at least one pane
  if (state.layout.panes.length === 0) {
    state.layout.panes = [{ id: "pane-0", tabId: null }];
  }
  if (state.activeTabId === tabId) {
    state.activeTabId = null;
    state.activePane = "local";
  }
  if (state.lastRemoteTabId === tabId) {
    state.lastRemoteTabId = state.tabs.at(-1)?.id ?? null;
  }
  notify();
}

// --- Transfers -------------------------------------------------------------------

// Optimistic insert so the queue panel/toolbar counter update the instant the
// user clicks Upload/Download, instead of waiting for the worker to pick the
// task up and emit its first transfer-state-changed event.
function addOptimisticTask(id, kind, connectionId, localPath, remotePath, fileName, totalBytes) {
  state.queueTasks.push({
    id,
    kind,
    connectionId,
    localPath,
    remotePath,
    fileName,
    totalBytes,
    transferredBytes: 0,
    state: "queued",
    speed: null,
    etaSecs: null,
  });
}

export async function queueUpload(tab, localEntry) {
  const remotePath = `${tab.remotePath.replace(/\/+$/, "")}/${localEntry.name}`;
  const totalBytes = localEntry.size ?? 0;
  const id = await ipc.enqueueTransfer("upload", tab.id, localEntry.path, remotePath, localEntry.name, totalBytes);
  addOptimisticTask(id, "upload", tab.id, localEntry.path, remotePath, localEntry.name, totalBytes);
  state.statusMessage = t("toolbar.uploadQueued", { name: localEntry.name });
  notify();
}

export async function queueDownload(tab, remoteEntry) {
  const localPath = `${state.local.path.replace(/\/+$/, "")}/${remoteEntry.name}`;
  const totalBytes = remoteEntry.size ?? 0;
  const id = await ipc.enqueueTransfer("download", tab.id, localPath, remoteEntry.path, remoteEntry.name, totalBytes);
  addOptimisticTask(id, "download", tab.id, localPath, remoteEntry.path, remoteEntry.name, totalBytes);
  state.statusMessage = t("toolbar.downloadQueued", { name: remoteEntry.name });
  notify();
}

export async function queueUploadMany(tab, localEntries) {
  if (localEntries.length === 1) return queueUpload(tab, localEntries[0]);
  for (const entry of localEntries) {
    const remotePath = `${tab.remotePath.replace(/\/+$/, "")}/${entry.name}`;
    const totalBytes = entry.size ?? 0;
    const id = await ipc.enqueueTransfer("upload", tab.id, entry.path, remotePath, entry.name, totalBytes);
    addOptimisticTask(id, "upload", tab.id, entry.path, remotePath, entry.name, totalBytes);
  }
  state.statusMessage = t("toolbar.uploadQueuedMulti", { n: localEntries.length });
  notify();
}

export async function queueDownloadMany(tab, remoteEntries) {
  if (remoteEntries.length === 1) return queueDownload(tab, remoteEntries[0]);
  for (const entry of remoteEntries) {
    const localPath = `${state.local.path.replace(/\/+$/, "")}/${entry.name}`;
    const totalBytes = entry.size ?? 0;
    const id = await ipc.enqueueTransfer("download", tab.id, localPath, entry.path, entry.name, totalBytes);
    addOptimisticTask(id, "download", tab.id, localPath, entry.path, entry.name, totalBytes);
  }
  state.statusMessage = t("toolbar.downloadQueuedMulti", { n: remoteEntries.length });
  notify();
}

// --- Drag & drop -----------------------------------------------------------------

// A folder can't be dropped into itself or into one of its own descendants —
// filters those out before a move is attempted so it fails silently instead
// of round-tripping an OS-level rename error to the status bar.
function isSelfOrDescendant(itemPath, destDir) {
  const src = itemPath.replace(/\/+$/, "");
  return destDir === src || destDir.startsWith(`${src}/`);
}

export async function handleDropOnLocal(payload, destDir) {
  const items = payload.items ?? [];
  if (payload.source === "remote") {
    const tab = state.tabs.find((tb) => tb.id === payload.connectionId);
    if (!tab) {
      state.statusMessage = t("panels.noActiveConnection");
      notify();
      return;
    }
    for (const item of items) {
      const localPath = `${destDir.replace(/\/+$/, "")}/${item.name}`;
      const id = await ipc.enqueueTransfer("download", tab.id, localPath, item.path, item.name, 0);
      addOptimisticTask(id, "download", tab.id, localPath, item.path, item.name, 0);
    }
    state.statusMessage =
      items.length === 1 ? t("toolbar.downloadQueued", { name: items[0].name }) : t("toolbar.downloadQueuedMulti", { n: items.length });
    notify();
  } else {
    const toMove = items.filter((item) => parentPath(item.path) !== destDir && !isSelfOrDescendant(item.path, destDir));
    if (toMove.length === 0) return;
    let failed = 0;
    for (const item of toMove) {
      try {
        await ipc.localMoveInto(item.path, destDir);
      } catch {
        failed++;
      }
    }
    await refreshLocal();
    state.statusMessage = failed > 0 ? t("panels.moveFailedCount", { n: failed }) : t("panels.moved", { n: toMove.length });
    notify();
  }
}

export async function handleDropOnRemote(tab, payload, destDir) {
  const items = payload.items ?? [];
  if (payload.source === "local") {
    for (const item of items) {
      const dest = `${destDir.replace(/\/+$/, "")}/${item.name}`;
      const id = await ipc.enqueueTransfer("upload", tab.id, item.path, dest, item.name, 0);
      addOptimisticTask(id, "upload", tab.id, item.path, dest, item.name, 0);
    }
    state.statusMessage =
      items.length === 1 ? t("toolbar.uploadQueued", { name: items[0].name }) : t("toolbar.uploadQueuedMulti", { n: items.length });
    notify();
    return;
  }
  if (payload.connectionId !== tab.id) {
    state.statusMessage = t("panels.cannotMoveCrossConnection");
    notify();
    return;
  }
  const toMove = items.filter((item) => parentPath(item.path) !== destDir && !isSelfOrDescendant(item.path, destDir));
  if (toMove.length === 0) return;
  state.statusMessage =
    toMove.length === 1 ? t("panels.movingFile", { name: toMove[0].name }) : t("panels.movingFiles", { n: toMove.length });
  notify();
  let failed = 0;
  for (const item of toMove) {
    const dest = `${destDir.replace(/\/+$/, "")}/${item.name}`;
    try {
      await ipc.remoteRename(tab.id, item.path, dest);
    } catch {
      failed++;
    }
  }
  await refreshRemote(tab);
  state.statusMessage = failed > 0 ? t("panels.moveFailedCount", { n: failed }) : t("panels.moved", { n: toMove.length });
  notify();
}

// --- New Folder / Rename / Delete -------------------------------------------------

export async function doMkdir(name) {
  if (state.opTarget === "local") {
    const path = `${state.local.path.replace(/\/+$/, "")}/${name}`;
    await ipc.localMkdir(path);
    state.statusMessage = "Folder created";
    await refreshLocal();
  } else {
    const tab = activeTab();
    if (!tab) return;
    const path = `${tab.remotePath.replace(/\/+$/, "")}/${name}`;
    await ipc.remoteMkdir(tab.id, path);
    state.statusMessage = "Folder created";
    await refreshRemote(tab);
  }
}

export async function doRename(oldName, newName) {
  if (state.opTarget === "local") {
    const entry = state.local.entries.find((e) => e.name === oldName);
    if (!entry) return;
    const to = `${parentPath(entry.path)}/${newName}`;
    await ipc.localRename(entry.path, to);
    state.statusMessage = "Renamed";
    await refreshLocal();
  } else {
    const tab = activeTab();
    if (!tab) return;
    const entry = tab.remoteEntries.find((e) => e.name === oldName);
    if (!entry) return;
    const to = `${parentPath(entry.path)}/${newName}`;
    await ipc.remoteRename(tab.id, entry.path, to);
    state.statusMessage = "Renamed";
    await refreshRemote(tab);
  }
}

export async function doDelete(names) {
  let failed = 0;
  if (state.opTarget === "local") {
    for (const name of names) {
      const entry = state.local.entries.find((e) => e.name === name);
      if (!entry) continue;
      try {
        await ipc.localDelete(entry.path);
      } catch {
        failed++;
      }
    }
    state.statusMessage = failed > 0 ? t("panels.deleteFailedCount", { n: failed }) : t("panels.deleted", { n: names.length });
    await refreshLocal();
  } else {
    const tab = activeTab();
    if (!tab) return;
    for (const name of names) {
      const entry = tab.remoteEntries.find((e) => e.name === name);
      if (!entry) continue;
      try {
        await ipc.remoteDelete(tab.id, entry.path);
      } catch {
        failed++;
      }
    }
    state.statusMessage = failed > 0 ? t("panels.deleteFailedCount", { n: failed }) : t("panels.deleted", { n: names.length });
    await refreshRemote(tab);
  }
}

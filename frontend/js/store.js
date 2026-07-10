// Single mutable state object + a plain pub/sub. No framework: components
// mutate `state` directly (or via the small helpers below) and then call
// notify() — every subscriber re-renders its own slice of the DOM from
// scratch. That's cheap at file-manager list sizes and keeps the mental
// model identical to the old egui immediate-mode AppState.
export const state = {
  // Local pane
  local: {
    path: "",
    entries: [],
    selected: [], // names of selected entries, in no particular order
    selectAnchor: null, // name shift-click range selection extends from
    sort: { col: "name", dir: "asc" },
  },

  // One entry per open remote connection tab.
  // { id, label, params, status, remotePath, remoteEntries, remoteSelected,
  //   remoteSelectAnchor, loading, sort: { col, dir } }
  tabs: [],
  activeTabId: null, // null = Local pane currently displayed, otherwise the displayed remote tab's id
  // The remote connection toolbar actions (Upload, refresh, New Folder, ...)
  // target. Unlike activeTabId this is NOT cleared when the Local pane is
  // displayed -- switching to Local to pick a file to upload shouldn't lose
  // track of which server it's going to, mirroring the old egui AppState
  // where `active_tab` persisted across the Local dock tab gaining focus.
  lastRemoteTabId: null,
  activePane: "local", // 'local' | 'remote' — who last received a click (toolbar targets this)

  // Split-pane layout. Each pane slot holds a tabId (null = Local, otherwise
  // a remote connection id). The user can rearrange panes by dragging tabs.
  layout: {
    dir: "row", // 'row' | 'column' — direction of the split
    panes: [
      { id: "pane-0", tabId: null }, // null = Local
    ],
  },

  sidebar: {
    quickAccess: [
      { name: "Home", path: "" },
      { name: "Desktop", path: "" },
      { name: "Documents", path: "" },
      { name: "Downloads", path: "" },
      { name: "Pictures", path: "" },
      { name: "Music", path: "" },
      { name: "Videos", path: "" },
    ],
    bookmarks: [],
  },

  history: [],
  sites: [],

  queueTasks: [],
  showQueue: false,

  statusMessage: "",

  showConnectDialog: false,
  connectLoading: false,
  connectError: "",

  showHistoryPopup: false,

  opTarget: "local", // which side New Folder / Rename / Delete apply to
  dialog: null, // { kind: 'mkdir'|'rename'|'delete', ... }

  showSettingsDialog: false,
  showSiteManager: false,
  // Persisted (via get_pref/set_pref) + session-only (panel visibility)
  // preferences -- see js/components/settingsDialog.js.
  settings: {
    language: "en",
    theme: "ember",
    confirmBeforeDelete: true,
    defaultLocalFolder: "",
    autoClearCompletedSecs: 0,
    maxConcurrent: 2,
    showToolbar: true,
    showSidebar: true,
    showStatusBar: true,
    showQueuePanel: true,
  },
};

const listeners = new Set();

export function notify() {
  listeners.forEach((fn) => fn(state));
}

export function subscribe(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function activeTab() {
  return state.tabs.find((t) => t.id === state.lastRemoteTabId) ?? null;
}

export function isWelcome() {
  return state.tabs.length === 0;
}

export function setStatus(message) {
  state.statusMessage = message;
  notify();
}

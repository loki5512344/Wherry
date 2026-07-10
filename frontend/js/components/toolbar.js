import { state, notify, activeTab } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { openNew } from "./connectDialog.js";
import { toggle as toggleHistory } from "./historyPopup.js";
import { openMkdir, openDelete, openRename } from "./opDialogs.js";
import { openSettings } from "./settingsDialog.js";
import { queueUploadMany, queueDownloadMany, refreshLocal, refreshRemote } from "../fsActions.js";
import { shortcutLabel, isMac } from "../platform.js";

let lastSig = null;

function pendingCount() {
  return state.queueTasks.filter((tk) => {
    const s = tk.state;
    const kind = typeof s === "string" ? s : Object.keys(s)[0];
    return kind === "running" || kind === "queued";
  }).length;
}

function currentSelection() {
  if (state.activePane === "remote") {
    return activeTab()?.remoteSelected ?? [];
  }
  return state.local.selected;
}

export function renderToolbar(container) {
  const tab = activeTab();
  const connected = tab?.status === "connected";

  container.innerHTML = `
    <button type="button" class="btn btn-accent-dim" data-action="new-connection" title="${t("toolbar.newConnection")} (${shortcutLabel("N")})">
      ${iconMarkup("addCircle", 15)}<span>${t("toolbar.newConnection")}</span>
    </button>
    <div class="btn-sep"></div>
    <button type="button" class="btn btn-ghost" data-action="upload" ${connected && state.local.selected.length > 0 ? "" : "disabled"}>
      ${iconMarkup("upload", 15)}<span>${t("common.upload")}</span>
    </button>
    <button type="button" class="btn btn-ghost" data-action="download" ${connected && tab?.remoteSelected.length > 0 ? "" : "disabled"}>
      ${iconMarkup("download", 15)}<span>${t("common.download")}</span>
    </button>
    <div class="btn-sep"></div>
    <button type="button" class="btn btn-toggle ${state.showQueue ? "active" : ""}" data-action="toggle-queue">
      ${iconMarkup("clock", 15)}<span>${t("toolbar.queueLabel")}${pendingCount() > 0 ? `  ${pendingCount()}` : ""}</span>
    </button>
    <div class="btn-sep"></div>
    <button type="button" class="btn icon-btn btn-ghost" data-action="refresh" title="${t("common.refresh")} (${shortcutLabel("R")})" ${state.activePane === "remote" && !connected ? "disabled" : ""}>${iconMarkup("refresh", 16)}</button>
    <button type="button" class="btn icon-btn btn-ghost" data-action="new-folder" title="${t("common.newFolder")} (${shortcutLabel("N", { shift: true })})" ${state.activePane === "remote" && !connected ? "disabled" : ""}>${iconMarkup("folderAdd", 16)}</button>
    <button type="button" class="btn icon-btn btn-ghost" data-action="delete" title="${t("common.delete")} (${isMac ? "⌘⌫" : "Del"})" ${currentSelection().length > 0 ? "" : "disabled"}>${iconMarkup("trash", 16)}</button>
    <button type="button" class="btn icon-btn btn-ghost" data-action="rename" title="${t("common.rename")} (${isMac ? "⏎" : "F2"})" ${currentSelection().length === 1 ? "" : "disabled"}>${iconMarkup("pen", 16)}</button>
    <div class="toolbar-spacer"></div>
    <button type="button" class="btn icon-btn btn-ghost" data-action="settings" title="${t("common.settings")} (${shortcutLabel(",")})">${iconMarkup("settings", 16)}</button>
    <button type="button" class="btn icon-btn btn-toggle ${state.showHistoryPopup ? "active" : ""}" data-action="history" data-history-toggle title="${t("common.history")}">${iconMarkup("history", 16)}</button>
  `;

  container.querySelector('[data-action="new-connection"]').addEventListener("click", openNew);
  container.querySelector('[data-action="settings"]').addEventListener("click", openSettings);
  container.querySelector('[data-action="upload"]').addEventListener("click", () => {
    const t2 = activeTab();
    const entries = state.local.entries.filter((e) => state.local.selected.includes(e.name));
    if (t2 && entries.length > 0) queueUploadMany(t2, entries);
  });
  container.querySelector('[data-action="download"]').addEventListener("click", () => {
    const t2 = activeTab();
    const entries = t2?.remoteEntries.filter((e) => t2.remoteSelected.includes(e.name)) ?? [];
    if (t2 && entries.length > 0) queueDownloadMany(t2, entries);
  });
  container.querySelector('[data-action="toggle-queue"]').addEventListener("click", () => {
    state.showQueue = !state.showQueue;
    notify();
  });
  container.querySelector('[data-action="refresh"]').addEventListener("click", () => {
    if (state.activePane === "remote" && tab) refreshRemote(tab);
    else refreshLocal();
  });
  container.querySelector('[data-action="new-folder"]').addEventListener("click", () => {
    state.opTarget = state.activePane;
    openMkdir();
  });
  container.querySelector('[data-action="delete"]').addEventListener("click", () => {
    const names = currentSelection();
    if (names.length === 0) return;
    state.opTarget = state.activePane;
    openDelete(names);
  });
  container.querySelector('[data-action="rename"]').addEventListener("click", () => {
    const names = currentSelection();
    if (names.length !== 1) return;
    state.opTarget = state.activePane;
    openRename(names[0]);
  });
  container.querySelector('[data-action="history"]').addEventListener("click", (e) => {
    toggleHistory(e.currentTarget);
  });

  lastSig = computeSig();
}

function computeSig() {
  const tab = activeTab();
  return JSON.stringify([
    state.activeTabId,
    state.activePane,
    tab?.status,
    tab?.remoteSelected,
    state.local.selected,
    state.showQueue,
    state.showHistoryPopup,
    pendingCount(),
  ]);
}

export function updateToolbarIfNeeded(container) {
  const sig = computeSig();
  if (sig === lastSig) return;
  renderToolbar(container);
}

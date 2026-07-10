import { state, notify } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { formatSize } from "../format.js";
import { renderFileTable } from "./fileTable.js";
import { renderBreadcrumbs } from "./breadcrumbs.js";
import { makeDropTarget } from "../dnd.js";
import { openRename, openDelete } from "./opDialogs.js";
import {
  navigateRemote,
  openRemoteEntry,
  handleDropOnRemote,
  refreshRemote,
  queueDownload,
  queueDownloadMany,
} from "../fsActions.js";

const STATUS_ICON = { connected: "●", connecting: "◐", error: "×", disconnected: "○" };
const STATUS_CLASS = { connected: "connected", connecting: "connecting", error: "error", disconnected: "disconnected" };

export function renderRemotePane(container, tab) {
  container.innerHTML = `
    <div class="path-bar">
      <span class="path-bar-label">${t("panels.remoteLabel", { host: tab.params.host })}</span>
      <span class="status-dot ${STATUS_CLASS[tab.status] ?? "disconnected"}">${STATUS_ICON[tab.status] ?? "○"}</span>
      <div class="breadcrumbs" data-role="breadcrumbs"></div>
      <div class="path-bar-right" data-role="loading"></div>
    </div>
    <div data-role="table" class="file-table-wrap"></div>
    <div class="pane-footer">
      <span data-role="count" class="text-dim" style="font-size:11px"></span>
      <span data-role="size" class="text-hint" style="font-size:11px"></span>
      <div class="pane-footer-right">
        <button type="button" class="btn icon-btn btn-ghost" data-action="refresh" title="${t("common.refresh")}">${iconMarkup("refresh", 13)}</button>
      </div>
    </div>
  `;

  container.querySelector('[data-action="refresh"]').addEventListener("click", () => {
    if (!tab.loading) refreshRemote(tab);
  });

  makeDropTarget(container, {
    onDrop: (payload) => handleDropOnRemote(tab, payload, tab.remotePath),
    onHoverChange: (hover) => container.classList.toggle("drag-over", hover),
  });

  updateRemotePane(container, tab);
}

export function updateRemotePane(container, tab) {
  renderBreadcrumbs(container.querySelector('[data-role="breadcrumbs"]'), tab.remotePath || "/", (path) =>
    navigateRemote(tab, path),
  );

  container.querySelector('[data-role="loading"]').innerHTML = tab.loading
    ? `<span>${t("panels.loading")}</span><span class="spinner"></span>`
    : "";

  renderFileTable(container.querySelector('[data-role="table"]'), {
    entries: tab.remoteEntries,
    selected: tab.remoteSelected,
    anchor: tab.remoteSelectAnchor,
    sort: tab.sort,
    onSort: (next) => {
      tab.sort = next;
      notify();
    },
    onSelect: (names, anchor) => {
      tab.remoteSelected = names;
      tab.remoteSelectAnchor = anchor;
      state.activePane = "remote";
      notify();
    },
    onOpen: (name) => openRemoteEntry(tab, name),
    onHoverOpen: (entry) => openRemoteEntry(tab, entry.name),
    dragPayload: (entry) => {
      if (entry.name === ".." || tab.status !== "connected") return null;
      const group =
        tab.remoteSelected.includes(entry.name) && tab.remoteSelected.length > 1
          ? tab.remoteEntries.filter((e) => tab.remoteSelected.includes(e.name))
          : [entry];
      return {
        source: "remote",
        connectionId: tab.id,
        items: group.map((e) => ({ path: e.path, name: e.name, kind: e.kind })),
      };
    },
    onDropOnEntry: (entry, payload) => handleDropOnRemote(tab, payload, entry.path),
    contextMenuItems: (entry, selectedEntries) => {
      if (selectedEntries.length > 1) {
        const files = selectedEntries.filter((e) => e.kind === "file");
        return [
          {
            label: t("common.delete") + ` (${selectedEntries.length})`,
            icon: "trash",
            danger: true,
            onClick: () => openDeleteRemote(selectedEntries),
          },
          ...(files.length > 0
            ? [
                "sep",
                {
                  label: t("panels.downloadToLocal") + ` (${files.length})`,
                  icon: "download",
                  onClick: () => queueDownloadMany(tab, files),
                },
              ]
            : []),
        ];
      }
      return [
        { label: t("common.open"), icon: "folder", onClick: () => openRemoteEntry(tab, entry.name) },
        { label: t("common.rename"), icon: "pen", onClick: () => openRenameRemote(entry) },
        { label: t("common.delete"), icon: "trash", danger: true, onClick: () => openDeleteRemote([entry]) },
        "sep",
        { label: t("common.copyPath"), icon: "copy", onClick: () => copyPath(entry.path) },
        ...(entry.kind === "file"
          ? [{ label: t("panels.downloadToLocal"), icon: "download", onClick: () => queueDownload(tab, entry) }]
          : []),
      ];
    },
  });

  const count = tab.remoteEntries.filter((e) => e.name !== "..").length;
  const totalSize = tab.remoteEntries.reduce((sum, e) => sum + (e.size ?? 0), 0);
  container.querySelector('[data-role="count"]').textContent = tab.loading
    ? t("panels.loading")
    : t("panels.countItems", { n: count });
  container.querySelector('[data-role="size"]').textContent = totalSize > 0 ? `(${formatSize(totalSize)})` : "";
}

function openRenameRemote(entry) {
  state.opTarget = "remote";
  openRename(entry.name);
}
function openDeleteRemote(entries) {
  state.opTarget = "remote";
  openDelete(entries.map((e) => e.name));
}
function copyPath(path) {
  navigator.clipboard?.writeText(path);
  state.statusMessage = t("panels.pathCopied");
  notify();
}

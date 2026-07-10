import { state, notify, activeTab } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { formatSize } from "../format.js";
import { renderFileTable } from "./fileTable.js";
import { renderBreadcrumbs } from "./breadcrumbs.js";
import { makeDropTarget } from "../dnd.js";
import { showContextMenu } from "./contextMenu.js";
import { openRename, openDelete } from "./opDialogs.js";
import {
  navigateLocal,
  openLocalEntry,
  addBookmark,
  handleDropOnLocal,
  queueUpload,
  queueUploadMany,
} from "../fsActions.js";

export function renderLocalPane(container) {
  container.innerHTML = `
    <div class="path-bar">
      <span class="path-bar-label">${t("panels.localLabel")}</span>
      <div class="breadcrumbs" data-role="breadcrumbs"></div>
    </div>
    <div data-role="table" class="file-table-wrap"></div>
    <div class="pane-footer">
      <span data-role="count" class="text-dim" style="font-size:11px"></span>
      <span data-role="size" class="text-hint" style="font-size:11px"></span>
      <div class="pane-footer-right">
        <button type="button" class="btn icon-btn btn-ghost" data-action="bookmark" title="${t("panels.bookmarkThisFolder")}">${iconMarkup("star", 13)}</button>
      </div>
    </div>
  `;

  container.querySelector('[data-action="bookmark"]').addEventListener("click", () => {
    const path = state.local.path;
    const name = path.split("/").filter(Boolean).pop() ?? "Bookmark";
    addBookmark(name, path);
  });

  makeDropTarget(container, {
    onDrop: (payload) => handleDropOnLocal(payload, state.local.path),
    onHoverChange: (hover) => container.classList.toggle("drag-over", hover),
  });

  updateLocalPane(container);
}

export function updateLocalPane(container) {
  renderBreadcrumbs(container.querySelector('[data-role="breadcrumbs"]'), state.local.path || "/", navigateLocal);

  const tab = activeTab();
  const connected = tab?.status === "connected";

  renderFileTable(container.querySelector('[data-role="table"]'), {
    entries: state.local.entries,
    selected: state.local.selected,
    anchor: state.local.selectAnchor,
    sort: state.local.sort,
    onSort: (next) => {
      state.local.sort = next;
      notify();
    },
    onSelect: (names, anchor) => {
      state.local.selected = names;
      state.local.selectAnchor = anchor;
      state.activePane = "local";
      notify();
    },
    onOpen: (name) => openLocalEntry(name),
    onHoverOpen: (entry) => openLocalEntry(entry.name),
    dragPayload: (entry) => {
      if (entry.name === "..") return null;
      const group =
        state.local.selected.includes(entry.name) && state.local.selected.length > 1
          ? state.local.entries.filter((e) => state.local.selected.includes(e.name))
          : [entry];
      return {
        source: "local",
        items: group.map((e) => ({ path: e.path, name: e.name, kind: e.kind })),
      };
    },
    onDropOnEntry: (entry, payload) => handleDropOnLocal(payload, entry.path),
    contextMenuItems: (entry, selectedEntries) => {
      if (selectedEntries.length > 1) {
        const files = selectedEntries.filter((e) => e.kind === "file");
        return [
          {
            label: t("common.delete") + ` (${selectedEntries.length})`,
            icon: "trash",
            danger: true,
            onClick: () => openDeleteLocal(selectedEntries),
          },
          ...(files.length > 0 && connected
            ? [
                "sep",
                {
                  label: t("panels.uploadToServer") + ` (${files.length})`,
                  icon: "upload",
                  onClick: () => queueUploadMany(tab, files),
                },
              ]
            : []),
        ];
      }
      return [
        { label: t("common.open"), icon: "folder", onClick: () => openLocalEntry(entry.name) },
        { label: t("common.rename"), icon: "pen", onClick: () => openRenameLocal(entry) },
        { label: t("common.delete"), icon: "trash", danger: true, onClick: () => openDeleteLocal([entry]) },
        "sep",
        ...(entry.kind === "dir"
          ? [{ label: t("panels.addToBookmarks"), icon: "star", onClick: () => addBookmark(entry.name, entry.path) }]
          : []),
        { label: t("common.copyPath"), icon: "copy", onClick: () => copyPath(entry.path) },
        ...(entry.kind === "file" && connected
          ? ["sep", { label: t("panels.uploadToServer"), icon: "upload", onClick: () => queueUpload(tab, entry) }]
          : []),
      ];
    },
  });

  const count = state.local.entries.filter((e) => e.name !== "..").length;
  const totalSize = state.local.entries.reduce((sum, e) => sum + (e.size ?? 0), 0);
  container.querySelector('[data-role="count"]').textContent = t("panels.countItems", { n: count });
  container.querySelector('[data-role="size"]').textContent = totalSize > 0 ? `(${formatSize(totalSize)})` : "";
}

function openRenameLocal(entry) {
  state.opTarget = "local";
  openRename(entry.name);
}
function openDeleteLocal(entries) {
  state.opTarget = "local";
  openDelete(entries.map((e) => e.name));
}
function copyPath(path) {
  navigator.clipboard?.writeText(path);
  state.statusMessage = t("panels.pathCopied");
  notify();
}

// Reusable file listing table shared by the Local and Remote panes — mirrors
// the old src/ui/panels/file_pane module (table.rs/row.rs/sort.rs collapsed
// into one file since there's no split-module pressure in JS).
import { t } from "../i18n.js";
import { iconMarkup, fileIconFor } from "../icons.js";
import { formatSize, formatTime } from "../format.js";
import { makeDraggable, makeDropTarget } from "../dnd.js";
import { showContextMenu } from "./contextMenu.js";
import { escapeHtml } from "../dom.js";

function sortEntries(entries, sort) {
  const dotdotIdx = entries.findIndex((e) => e.name === "..");
  const dotdot = dotdotIdx >= 0 ? entries[dotdotIdx] : null;
  const rest = entries.filter((e) => e.name !== "..");

  rest.sort((a, b) => {
    const aDir = a.kind === "dir";
    const bDir = b.kind === "dir";
    if (aDir !== bDir) return aDir ? -1 : 1;

    let cmp = 0;
    switch (sort.col) {
      case "size":
        cmp = (a.size ?? 0) - (b.size ?? 0);
        break;
      case "kind":
        cmp = a.kind.localeCompare(b.kind);
        break;
      case "modified":
        cmp = (a.modified ?? 0) - (b.modified ?? 0);
        break;
      default:
        cmp = a.name.toLowerCase().localeCompare(b.name.toLowerCase());
    }
    return sort.dir === "desc" ? -cmp : cmp;
  });

  return dotdot ? [dotdot, ...rest] : rest;
}

function rowIcon(entry) {
  if (entry.name === "..") return ["arrowUp", "icon--inactive"];
  if (entry.kind === "dir") return ["folder", "icon--important"];
  if (entry.kind === "symlink") return ["link", "icon--important"];
  return fileIconFor(entry.name);
}

function typeLabel(entry) {
  if (entry.kind === "dir") return t("panels.typeDir");
  if (entry.kind === "symlink") return t("panels.typeLink");
  return t("panels.typeFile");
}

function sortHeaderHtml(label, col, sort) {
  const active = sort.col === col;
  const arrow = active ? (sort.dir === "asc" ? " ↑" : " ↓") : "";
  return `<button type="button" class="sort-header ${active ? "active" : ""}" data-col="${col}">${label}${arrow}</button>`;
}

/**
 * opts:
 *  - entries: FileEntry[]
 *  - selected: string[]      (names of selected entries)
 *  - anchor: string|null     (name shift-click range selection extends from)
 *  - sort: { col, dir }
 *  - onSort(nextSort)
 *  - onSelect(names, anchor)
 *  - onOpen(name)
 *  - dragPayload(entry) => payload|null
 *  - onDropOnEntry(entry, payload)   only called for dir/".." rows
 *  - onHoverOpen(entry)   spring-loaded open: fires after a drag lingers on
 *                         a dir/".." row for HOVER_OPEN_DELAY, without
 *                         ending the drag -- only called for dir/".." rows
 *  - contextMenuItems(entry, selectedEntries) => menu items for showContextMenu, or null to skip
 */
export function renderFileTable(container, opts) {
  const scrollPos = container.querySelector(".file-table-body")?.scrollTop ?? 0;
  container.innerHTML = "";
  container.classList.add("file-table-wrap");

  const header = document.createElement("div");
  header.className = "file-table-header";
  header.innerHTML = `
    ${sortHeaderHtml(t("panels.colName"), "name", opts.sort)}
    ${sortHeaderHtml(t("panels.colModified"), "modified", opts.sort)}
    ${sortHeaderHtml(t("panels.colType"), "kind", opts.sort)}
    ${sortHeaderHtml(t("panels.colSize"), "size", opts.sort)}
  `;
  header.querySelectorAll(".sort-header").forEach((btn) => {
    btn.addEventListener("click", () => {
      const col = btn.dataset.col;
      const next =
        opts.sort.col === col
          ? { col, dir: opts.sort.dir === "asc" ? "desc" : "asc" }
          : { col, dir: "asc" };
      opts.onSort(next);
    });
  });
  container.appendChild(header);

  const body = document.createElement("div");
  body.className = "file-table-body";
  container.appendChild(body);

  const sorted = sortEntries(opts.entries, opts.sort);
  const selectableNames = sorted.filter((e) => e.name !== "..").map((e) => e.name);

  for (const entry of sorted) {
    const row = document.createElement("div");
    row.className = "file-row";
    if (opts.selected.includes(entry.name)) row.classList.add("selected");

    const [iconName, iconColor] = rowIcon(entry);
    const nameCol = entry.name === ".." ? "text-hint" : "text-primary";

    row.innerHTML = `
      <div class="file-row-name">
        <span class="icon ${iconColor}">${iconMarkup(iconName, 15)}</span>
        <span class="${nameCol}">${escapeHtml(entry.name)}</span>
      </div>
      <div class="file-row-modified mono">${formatTime(entry.modified)}</div>
      <div class="file-row-type">${typeLabel(entry)}</div>
      <div class="file-row-size mono">${formatSize(entry.size)}</div>
    `;

    row.addEventListener("click", (e) => {
      if (entry.name === "..") {
        opts.onSelect([], null);
        return;
      }
      const idx = selectableNames.indexOf(entry.name);
      if (e.shiftKey && opts.anchor && selectableNames.includes(opts.anchor)) {
        const anchorIdx = selectableNames.indexOf(opts.anchor);
        const [lo, hi] = anchorIdx < idx ? [anchorIdx, idx] : [idx, anchorIdx];
        opts.onSelect(selectableNames.slice(lo, hi + 1), opts.anchor);
      } else if (e.ctrlKey || e.metaKey) {
        const next = new Set(opts.selected);
        if (next.has(entry.name)) next.delete(entry.name);
        else next.add(entry.name);
        opts.onSelect(selectableNames.filter((n) => next.has(n)), entry.name);
      } else {
        opts.onSelect([entry.name], entry.name);
      }
    });
    row.addEventListener("dblclick", () => opts.onOpen(entry.name));

    if (entry.name !== "..") {
      row.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        let selection = opts.selected;
        if (!selection.includes(entry.name)) {
          selection = [entry.name];
          opts.onSelect(selection, entry.name);
        }
        const selectedEntries = sorted.filter((en) => selection.includes(en.name));
        const items = opts.contextMenuItems?.(entry, selectedEntries);
        if (items) showContextMenu(e.clientX, e.clientY, items);
      });

      const payload = opts.dragPayload?.(entry);
      if (payload) makeDraggable(row.querySelector(".file-row-name"), entry, payload);
    }

    if (entry.kind === "dir" || entry.name === "..") {
      makeDropTarget(row, {
        onDrop: (payload) => opts.onDropOnEntry?.(entry, payload),
        onHoverChange: (hover) => row.classList.toggle("drag-hover", hover),
        onHoverHold: () => opts.onHoverOpen?.(entry),
      });
    }

    body.appendChild(row);
  }

  body.scrollTop = scrollPos;
}

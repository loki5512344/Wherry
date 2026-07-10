import { t } from "../i18n.js";
import { iconMarkup, fileIconFor } from "../icons.js";
import { formatSize, formatTime } from "../format.js";
import { makeDraggable, makeDropTarget } from "../dnd.js";
import { showContextMenu } from "./contextMenu.js";
import { escapeHtml } from "../dom.js";

const ROW_HEIGHT = 32;
const OVERSCAN = 10;

let rafId = null;

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

function computeVisibleRange(scrollTop, viewportHeight, totalItems) {
  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const end = Math.min(totalItems, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN);
  return { start, end };
}

function getSignature(sorted, sort) {
  const len = sorted.length;
  if (len === 0) return `0:${sort.col}:${sort.dir}`;
  const mid = Math.min(len >> 1, len - 1);
  return `${len}:${sort.col}:${sort.dir}:${sorted[0].name}:${sorted[mid].name}:${sorted[len - 1].name}`;
}

function initStructure(container) {
  let cache = container._fileTableCache;
  if (cache) return cache;

  container.innerHTML = "";
  container.classList.add("file-table-wrap");

  const header = document.createElement("div");
  header.className = "file-table-header";
  container.appendChild(header);

  const body = document.createElement("div");
  body.className = "file-table-body";
  container.appendChild(body);

  const spacer = document.createElement("div");
  spacer.style.position = "relative";
  body.appendChild(spacer);

  cache = {
    header,
    body,
    spacer,
    lastSignature: null,
    sorted: [],
    selectableNames: [],
    opts: null,
  };
  container._fileTableCache = cache;

  body.addEventListener("scroll", () => {
    if (rafId) return;
    rafId = requestAnimationFrame(() => {
      rafId = null;
      const c = container._fileTableCache;
      if (c && c.sorted.length > 0) {
        updateVisibleRows(c);
      }
    });
  });

  return cache;
}

function updateHeader(header, opts) {
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
}

function createRowElement(entry, index, cache) {
  const row = document.createElement("div");
  row.className = "file-row";
  row.dataset.index = index;

  const current = cache.opts;
  if (current.selected.includes(entry.name)) row.classList.add("selected");

  row.style.position = "absolute";
  row.style.top = index * ROW_HEIGHT + "px";
  row.style.left = "0";
  row.style.right = "0";
  row.style.height = ROW_HEIGHT + "px";

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
    const opts = cache.opts;
    if (entry.name === "..") {
      opts.onSelect([], null);
      return;
    }
    const idx = cache.selectableNames.indexOf(entry.name);
    if (e.shiftKey && opts.anchor && cache.selectableNames.includes(opts.anchor)) {
      const anchorIdx = cache.selectableNames.indexOf(opts.anchor);
      const [lo, hi] = anchorIdx < idx ? [anchorIdx, idx] : [idx, anchorIdx];
      opts.onSelect(cache.selectableNames.slice(lo, hi + 1), opts.anchor);
    } else if (e.ctrlKey || e.metaKey) {
      const next = new Set(opts.selected);
      if (next.has(entry.name)) next.delete(entry.name);
      else next.add(entry.name);
      opts.onSelect(cache.selectableNames.filter((n) => next.has(n)), entry.name);
    } else {
      opts.onSelect([entry.name], entry.name);
    }
  });
  row.addEventListener("dblclick", () => cache.opts.onOpen(entry.name));

  if (entry.name !== "..") {
    row.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      const opts = cache.opts;
      let selection = opts.selected;
      if (!selection.includes(entry.name)) {
        selection = [entry.name];
        opts.onSelect(selection, entry.name);
      }
      const selectedEntries = cache.sorted.filter((en) => selection.includes(en.name));
      const items = opts.contextMenuItems?.(entry, selectedEntries);
      if (items) showContextMenu(e.clientX, e.clientY, items);
    });

    const payload = current.dragPayload?.(entry);
    if (payload) makeDraggable(row.querySelector(".file-row-name"), entry, payload);
  }

  if (entry.kind === "dir" || entry.name === "..") {
    makeDropTarget(row, {
      onDrop: (payload) => cache.opts.onDropOnEntry?.(entry, payload),
      onHoverChange: (hover) => row.classList.toggle("drag-hover", hover),
      onHoverHold: () => cache.opts.onHoverOpen?.(entry),
    });
  }

  return row;
}

function updateVisibleRows(cache) {
  const sorted = cache.sorted;
  if (sorted.length === 0) return;

  const { start, end } = computeVisibleRange(
    cache.body.scrollTop,
    cache.body.clientHeight,
    sorted.length,
  );

  const existing = cache.spacer.querySelectorAll(".file-row");
  for (const row of existing) {
    const idx = parseInt(row.dataset.index, 10);
    if (idx < start || idx >= end) {
      row.remove();
    }
  }

  for (let i = start; i < end; i++) {
    if (!cache.spacer.querySelector(`.file-row[data-index="${i}"]`)) {
      const row = createRowElement(sorted[i], i, cache);
      cache.spacer.appendChild(row);
    }
  }
}

function updateSelection(cache, selected) {
  const rows = cache.spacer.querySelectorAll(".file-row");
  for (const row of rows) {
    const idx = parseInt(row.dataset.index, 10);
    const entry = cache.sorted[idx];
    if (entry) {
      row.classList.toggle("selected", selected.includes(entry.name));
    }
  }
}

export function renderFileTable(container, opts) {
  const cache = initStructure(container);
  cache.opts = opts;

  updateHeader(cache.header, opts);

  const sorted = sortEntries(opts.entries, opts.sort);
  const sig = getSignature(sorted, opts.sort);

  if (sig !== cache.lastSignature) {
    cache.lastSignature = sig;
    cache.sorted = sorted;
    cache.selectableNames = sorted.filter((e) => e.name !== "..").map((e) => e.name);
    cache.spacer.style.height = sorted.length * ROW_HEIGHT + "px";
    cache.body.scrollTop = 0;
    cache.spacer.innerHTML = "";
    updateVisibleRows(cache);
  } else {
    updateSelection(cache, opts.selected);
  }
}

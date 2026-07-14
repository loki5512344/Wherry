// Split-pane layout — tabs dragged from tab bar into panes create splits.
// Direction-aware: drop left edge = split left, right edge = split right,
// center = replace. Panes can also be reordered by dragging headers.
import { state, notify } from "../store.js";
import { renderLocalPane, updateLocalPane } from "./localPane.js";
import { renderRemotePane, updateRemotePane } from "./remotePane.js";
import { iconMarkup } from "../icons.js";
import { HOVER_OPEN_DELAY, getDraggedTabId } from "../dnd.js";

const paneRefs = new Map();

function paneKind(p) { return p.tabId ?? "local"; }
function paneLabel(p) { return p.tabId === null ? "Local" : (state.tabs.find((t) => t.id === p.tabId)?.label ?? "?"); }

function ensureFlex(p) { if (p.flex == null) p.flex = 1; return p; }

/* Render the full pane grid */
export function renderPanes(container) {
  container.innerHTML = "";
  paneRefs.clear();

  const wrap = document.createElement("div");
  wrap.className = "pane-grid";
  applyGrid(wrap);
  wrap.addEventListener("dragover", (e) => { e.preventDefault(); });
  wrap.addEventListener("drop", (e) => {
    e.preventDefault();
    const tabId = e.dataTransfer.getData("tab-id");
    if (!tabId) return;
    stopDropHighlight();
    addPane(parsedTabId(tabId), state.layout.panes.length);
  });
  container.appendChild(wrap);

  for (let i = 0; i < state.layout.panes.length; i++) {
    if (i > 0) {
      const handle = createResizeHandle(i - 1);
      wrap.appendChild(handle);
    }
    const cell = createPaneCell(state.layout.panes[i]);
    wrap.appendChild(cell);
  }
}

function applyGrid(wrap) {
  const n = state.layout.panes.length;
  const totalFlex = state.layout.panes.reduce((s, p) => s + (p.flex || 1), 0);
  const cols = state.layout.panes.map((p) => `${((p.flex || 1) / totalFlex * 100).toFixed(2)}fr`).join(" ");
  wrap.style.cssText = `display:grid;grid-template-columns:${n > 1 ? cols : "1fr"};flex:1;min-height:0;`;
}

function createResizeHandle(index) {
  const div = document.createElement("div");
  div.className = "pane-resize-handle";
  div.dataset.index = index;
  div.addEventListener("mousedown", onResizeStart);
  return div;
}

let resizing = null;

function onResizeStart(e) {
  e.preventDefault();
  const handle = e.currentTarget;
  const index = parseInt(handle.dataset.index);
  const wrap = handle.closest(".pane-grid");
  const rect = wrap.getBoundingClientRect();
  const startX = e.clientX;

  const panes = state.layout.panes;
  const totalFlex = panes.reduce((s, p) => s + (p.flex || 1), 0);
  const currentPct = panes.map((p) => (p.flex || 1) / totalFlex);

  resizing = { index, startX, wrap, rect, panes, currentPct, totalFlex };

  document.addEventListener("mousemove", onResizeMove);
  document.addEventListener("mouseup", onResizeEnd);
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";
}

function onResizeMove(e) {
  if (!resizing) return;
  const { index, startX, rect, panes, currentPct } = resizing;
  const dx = e.clientX - startX;
  const totalWidth = rect.width;
  if (totalWidth <= 0) return;

  const leftPct = currentPct[index];
  const rightPct = currentPct[index + 1];
  const pctChange = dx / totalWidth;

  const newLeft = Math.max(0.1, Math.min(0.9, leftPct + pctChange));
  const newRight = Math.max(0.1, Math.min(0.9, rightPct - pctChange));
  if (newLeft + newRight < 0.2) return;

  const newTotal = newLeft + newRight + currentPct.slice(0, index).reduce((s, v) => s + v, 0) + currentPct.slice(index + 2).reduce((s, v) => s + v, 0);

  const scale = currentPct.reduce((a, b) => a + b, 0) / newTotal;
  for (let i = 0; i < panes.length; i++) {
    if (i === index) panes[i].flex = newLeft * scale;
    else if (i === index + 1) panes[i].flex = newRight * scale;
    else panes[i].flex = currentPct[i] * scale;
  }

  const wrap = resizing.wrap;
  applyGrid(wrap);
}

function onResizeEnd() {
  resizing = null;
  document.removeEventListener("mousemove", onResizeMove);
  document.removeEventListener("mouseup", onResizeEnd);
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
}

function createPaneCell(pane) {
  ensureFlex(pane);
  const cell = document.createElement("div");
  cell.className = "pane-cell";
  cell.dataset.paneId = pane.id;

  // Edge drop zones
  const zones = ["left","right","top","bottom","center","swap"].map((z) => {
    const d = document.createElement("div");
    d.className = `pane-zone pane-zone--${z}`;
    d.dataset.zone = z;
    return d;
  });

  // Header
  const header = document.createElement("div");
  header.className = "pane-header";
  header.draggable = true;
  header.dataset.paneId = pane.id;
  header.innerHTML = `<span class="pane-label">${paneLabel(pane)}</span>`;
  header.addEventListener("dragstart", onHeaderDragStart);
  header.addEventListener("dragend", onHeaderDragEnd);

  // Close btn
  const closeBtn = document.createElement("button");
  closeBtn.className = "pane-close-btn";
  closeBtn.title = "Close pane";
  closeBtn.innerHTML = iconMarkup("close", 13);
  closeBtn.addEventListener("click", (e) => { e.stopPropagation(); closePane(pane.id); });
  header.appendChild(closeBtn);

  // Body
  const body = document.createElement("div");
  body.className = "pane-body";

  cell.appendChild(header);
  cell.appendChild(body);
  zones.forEach((z) => cell.appendChild(z));

  // Drag events on the cell
  cell.addEventListener("dragover", onPaneDragOver);
  cell.addEventListener("dragleave", onPaneDragLeave);
  cell.addEventListener("drop", onPaneDrop);

  // Render file table
  if (pane.tabId === null) {
    renderLocalPane(body);
  } else {
    const tab = state.tabs.find((t) => t.id === pane.tabId);
    if (tab) renderRemotePane(body, tab);
  }
  paneRefs.set(pane.id, { kind: paneKind(pane), bodyEl: body });

  return cell;
}

/* In-place update (avoids full rebuild on every tick) */
export function updatePanes(container) {
  const wrap = container.querySelector(".pane-grid");
  if (!wrap) return renderPanes(container);
  const cells = wrap.querySelectorAll(".pane-cell");
  if (cells.length !== state.layout.panes.length) return renderPanes(container);
  const ids = Array.from(cells).map((c) => c.dataset.paneId);
  const expected = state.layout.panes.map((p) => p.id);
  if (ids.join(",") !== expected.join(",")) return renderPanes(container);

  applyGrid(wrap);

  for (const pane of state.layout.panes) {
    const ref = paneRefs.get(pane.id);
    if (!ref || ref.kind !== paneKind(pane)) return renderPanes(container);
    const cell = wrap.querySelector(`[data-pane-id="${pane.id}"]`);
    if (cell) {
      const label = cell.querySelector(".pane-label");
      if (label) label.textContent = paneLabel(pane);
    }
    if (pane.tabId === null) updateLocalPane(ref.bodyEl);
    else { const tab = state.tabs.find((t) => t.id === pane.tabId); if (tab) updateRemotePane(ref.bodyEl, tab); }
  }
}

// ── Drag & Drop: tab bar → pane ─────────────────────────────────────────

let activeZone = null;
let holdTimer = null;
let holdPaneId = null;
let openPaneId = null;

function computeZone(cell, clientX, clientY) {
  const rect = cell.getBoundingClientRect();
  const x = (clientX - rect.left) / rect.width;
  const y = (clientY - rect.top) / rect.height;
  if (x < 0.22) return "left";
  if (x > 0.78) return "right";
  if (y < 0.22) return "top";
  if (y > 0.78) return "bottom";
  return "center";
}

function showZone(cell, zone) {
  const key = `${cell.dataset.paneId}-${zone}`;
  if (activeZone === key) return;
  document.querySelectorAll(".pane-cell.drop-active").forEach((c) => {
    if (c !== cell) {
      c.classList.remove("drop-active");
      delete c.dataset.dropZone;
    }
  });
  activeZone = key;
  cell.classList.add("drop-active");
  cell.dataset.dropZone = zone;
}

function clearHold() {
  clearTimeout(holdTimer);
  holdTimer = null;
  holdPaneId = null;
}

function stopDropHighlight() {
  document.querySelectorAll(".pane-cell").forEach((c) => {
    c.classList.remove("drop-active", "drop-pending");
    delete c.dataset.dropZone;
  });
  activeZone = null;
}

function resetPaneDrag() {
  clearHold();
  openPaneId = null;
  stopDropHighlight();
}

function onPaneDragOver(e) {
  if (!e.dataTransfer.types.includes("tab-id")) return;
  const cell = e.currentTarget;
  const paneId = cell.dataset.paneId;
  const pane = state.layout.panes.find((p) => p.id === paneId);

  if (pane && pane.tabId === getDraggedTabId()) {
    e.stopPropagation();
    return;
  }

  e.preventDefault();
  e.stopPropagation();
  e.dataTransfer.dropEffect = "move";

  if (openPaneId === paneId) {
    showZone(cell, computeZone(cell, e.clientX, e.clientY));
    return;
  }

  if (holdPaneId !== paneId) {
    clearHold();
    holdPaneId = paneId;
    cell.classList.add("drop-pending");
    holdTimer = setTimeout(() => {
      holdTimer = null;
      cell.classList.remove("drop-pending");
      openPaneId = paneId;
      showZone(cell, computeZone(cell, e.clientX, e.clientY));
    }, HOVER_OPEN_DELAY);
  }
}

function onPaneDragLeave(e) {
  const cell = e.currentTarget;
  if (cell.contains(e.relatedTarget)) return;
  cell.classList.remove("drop-pending");
  if (holdPaneId === cell.dataset.paneId) clearHold();
  if (openPaneId === cell.dataset.paneId) {
    openPaneId = null;
    cell.classList.remove("drop-active");
    delete cell.dataset.dropZone;
    activeZone = null;
  }
}

function onPaneDrop(e) {
  e.preventDefault();
  e.stopPropagation();
  const tabId = e.dataTransfer.getData("tab-id");
  const cell = e.currentTarget;
  const paneId = cell.dataset.paneId;
  const wasOpen = openPaneId === paneId;
  const zone = cell.dataset.dropZone || "center";
  resetPaneDrag();
  if (!tabId || !wasOpen) return;

  const idx = state.layout.panes.findIndex((p) => p.id === paneId);
  if (idx === -1) return;
  const tid = parsedTabId(tabId);

  switch (zone) {
    case "left": addPane(tid, idx); break;
    case "right": addPane(tid, idx + 1); break;
    case "top": addPane(tid, idx + 1); break;
    case "bottom": addPane(tid, idx + 1); break;
    case "center":
      if (!state.layout.panes.some((p) => p.tabId === tid && p.id !== paneId)) {
        state.layout.panes[idx].tabId = tid;
      }
      break;
  }
  notify();
}

window.addEventListener("dragend", resetPaneDrag);

// ── Header drag (reorder) ───────────────────────────────────────────────

let headerDragId = null;
function onHeaderDragStart(e) {
  headerDragId = e.currentTarget.dataset.paneId;
  e.dataTransfer.effectAllowed = "move";
  e.dataTransfer.setData("text/plain", headerDragId);
  e.target.closest(".pane-cell")?.classList.add("pane-dragging");
}
function onHeaderDragEnd(e) {
  e.target.closest(".pane-cell")?.classList.remove("pane-dragging");
  stopDropHighlight();
  headerDragId = null;
}

// ── Helpers ─────────────────────────────────────────────────────────────

function parsedTabId(raw) {
  return raw === "local" ? null : raw;
}

function addPane(tabId, index) {
  const existing = state.layout.panes.findIndex((p) => p.tabId === tabId);
  if (existing !== -1) {
    state.layout.panes.splice(existing, 1);
    if (existing < index) index--;
  }
  state.layout.panes.splice(index, 0, { id: `pane-${Date.now()}`, tabId, flex: 1 });
}

function closePane(paneId) {
  const pane = state.layout.panes.find((p) => p.id === paneId);
  if (!pane) return;
  const isLastRemote = pane.tabId !== null && state.layout.panes.filter((p) => p.tabId !== null).length === 1;
  state.layout.panes = state.layout.panes.filter((p) => p.id !== paneId);
  if (state.layout.panes.length === 0) {
    state.layout.panes.push({ id: `pane-${Date.now()}`, tabId: null, flex: 1 });
  }
  notify();
}

// ── Public API: open a tab in a pane from outside ───────────────────────
export function openInPane(tabId) {
  if (state.layout.panes.some((p) => p.tabId === tabId)) return;
  state.layout.panes[0].tabId = tabId;
  notify();
}

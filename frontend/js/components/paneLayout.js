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

/* Render the full pane grid */
export function renderPanes(container) {
  container.innerHTML = "";
  paneRefs.clear();

  const wrap = document.createElement("div");
  wrap.className = "pane-grid";
  wrap.style.cssText = `display:grid;grid-template-columns:${buildGrid()};flex:1;min-height:0;`;
  wrap.addEventListener("dragover", (e) => { e.preventDefault(); });
  wrap.addEventListener("drop", (e) => {
    // Drop on empty area (after last pane) → append
    e.preventDefault();
    const tabId = e.dataTransfer.getData("tab-id");
    if (!tabId) return;
    stopDropHighlight();
    addPane(parsedTabId(tabId), state.layout.panes.length);
  });
  container.appendChild(wrap);

  for (const pane of state.layout.panes) {
    const cell = createPaneCell(pane);
    wrap.appendChild(cell);
  }
}

function createPaneCell(pane) {
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
  if (state.layout.panes.length > 1) {
    const closeBtn = document.createElement("button");
    closeBtn.className = "pane-close-btn";
    closeBtn.title = "Close";
    closeBtn.innerHTML = iconMarkup("close", 13);
    closeBtn.addEventListener("click", (e) => { e.stopPropagation(); closePane(pane.id); });
    header.appendChild(closeBtn);
  }

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

function buildGrid() {
  const n = state.layout.panes.length;
  if (n <= 1) return "1fr";
  return state.layout.panes.map(() => "1fr").join(" ");
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

  wrap.style.gridTemplateColumns = buildGrid();

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

/* Tab bar tabs become draggable — set data "tab-id" on dragstart. Pane cells
   get visual drop zones (left/right/top/bottom/center), but two rules keep
   it from feeling random:
     1. The pane already showing the dragged tab never offers itself as a
        target -- there's nowhere sensible to put a tab inside its own pane.
     2. Every other pane needs a beat of sustained hover (HOVER_OPEN_DELAY)
        before it "opens" as a drop target. Zones appearing the instant the
        pointer crosses into a pane read as jumpy; the pause makes hovering
        read as an intentional choice instead. */

let activeZone = null; // "<paneId>-<zone>" of the currently shown zone overlay
let holdTimer = null;  // pending "open" timer for the pane below
let holdPaneId = null; // pane the hold timer is counting for
let openPaneId = null;  // pane that passed the hold and is now a live drop target

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

  // Dragging a tab over the pane that already shows it -- never a valid
  // target. Stop the event here so it doesn't bubble to the grid wrap's
  // unconditional preventDefault(), which would otherwise still allow a
  // drop (routed to the "append at end" handler) at this pointer position.
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
  if (!tabId || !wasOpen) return; // released before this pane "opened" -- ignore

  const idx = state.layout.panes.findIndex((p) => p.id === paneId);
  if (idx === -1) return;
  const tid = parsedTabId(tabId);

  switch (zone) {
    case "left": addPane(tid, idx); break;
    case "right": addPane(tid, idx + 1); break;
    case "top": addPane(tid, idx + 1); break;     // in flat layout, left/right suffices
    case "bottom": addPane(tid, idx + 1); break;   // top/bottom = same as right
    case "center": // replace this pane (but don't add duplicate)
      if (!state.layout.panes.some((p) => p.tabId === tid && p.id !== paneId)) {
        state.layout.panes[idx].tabId = tid;
      }
      break;
  }
  notify();
}

// A drag can end (dropped, cancelled, Escaped) outside every pane cell --
// clean up hold/zone state unconditionally so it never gets stuck.
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
  // If this tab is already in another pane, remove it
  const existing = state.layout.panes.findIndex((p) => p.tabId === tabId);
  if (existing !== -1) {
    state.layout.panes.splice(existing, 1);
    if (existing < index) index--;
  }
  state.layout.panes.splice(index, 0, { id: `pane-${Date.now()}`, tabId });
}

function closePane(paneId) {
  if (state.layout.panes.length <= 1) return;
  state.layout.panes = state.layout.panes.filter((p) => p.id !== paneId);
  notify();
}

// ── Public API: open a tab in a pane from outside ───────────────────────
export function openInPane(tabId) {
  // If already visible, do nothing (it's already in some pane)
  if (state.layout.panes.some((p) => p.tabId === tabId)) return;

  // Replace the first pane (single-pane UX — click = switch, not split)
  // Splitting only happens via drag-and-drop into edge zones.
  state.layout.panes[0].tabId = tabId;
  notify();
}

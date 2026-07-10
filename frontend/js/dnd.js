// Native HTML5 drag & drop, standing in for egui's dnd_drag_source /
// dnd_drop_zone. Payload shape mirrors the old Rust `DragPayload` enum:
//   { source: 'local', path, name }
//   { source: 'remote', path, name, connectionId }
const MIME = "application/x-wherry-payload";

// "Spring-loaded folder" hover delay: you can't click a folder open while
// your mouse is busy carrying a payload, so a drag has to linger this long
// over a folder row (or another pane) before it auto-opens -- mirrors
// Finder/Explorer's spring-loaded folders.
export const HOVER_OPEN_DELAY = 900;

export function makeDraggable(el, entry, payload) {
  if (entry.name === "..") return; // parent row is never a drag source
  el.draggable = true;
  el.addEventListener("dragstart", (e) => {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData(MIME, JSON.stringify(payload));
    requestAnimationFrame(() => el.classList.add("dragging"));
  });
  el.addEventListener("dragend", () => el.classList.remove("dragging"));
}

/** opts.onHoverHold, if given, fires once after HOVER_OPEN_DELAY of
 *  uninterrupted dragover -- used to spring a folder row open without
 *  ending the drag. */
export function makeDropTarget(el, { onDrop, onHoverChange, onHoverHold } = {}) {
  let holdTimer = null;
  const clearHold = () => {
    clearTimeout(holdTimer);
    holdTimer = null;
  };

  el.addEventListener("dragover", (e) => {
    if (!e.dataTransfer.types.includes(MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    onHoverChange?.(true);
    if (onHoverHold && !holdTimer) {
      holdTimer = setTimeout(() => {
        holdTimer = null;
        onHoverHold();
      }, HOVER_OPEN_DELAY);
    }
  });
  el.addEventListener("dragleave", (e) => {
    if (el.contains(e.relatedTarget)) return; // moved onto a child, not actually left
    onHoverChange?.(false);
    clearHold();
  });
  el.addEventListener("drop", (e) => {
    if (!e.dataTransfer.types.includes(MIME)) return;
    e.preventDefault();
    e.stopPropagation(); // don't let the pane-level drop zone also fire for this same drop
    onHoverChange?.(false);
    clearHold();
    const raw = e.dataTransfer.getData(MIME);
    if (!raw) return;
    try {
      onDrop(JSON.parse(raw));
    } catch {
      /* malformed payload, ignore */
    }
  });
}

// ── Tab-bar → pane drag tracking ────────────────────────────────────────
// dataTransfer.getData() only returns data on dragstart/drop, not dragover
// (a browser security restriction) -- so paneLayout.js can't read which tab
// is mid-drag from the dragover event itself. Track it here instead; tabs.js
// sets it on dragstart/dragend. `undefined` means no tab drag is in flight
// (kept distinct from `null`, which is the Local tab's real id).
let draggedTabId;
export function setDraggedTabId(id) {
  draggedTabId = id;
}
export function getDraggedTabId() {
  return draggedTabId;
}

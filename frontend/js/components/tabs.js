import { state, notify } from "../store.js";
import { iconMarkup } from "../icons.js";
import { closeTab } from "../fsActions.js";
import { openInPane } from "./paneLayout.js";
import { escapeHtml } from "../dom.js";
import { setDraggedTabId } from "../dnd.js";

const STATUS_COLOR = { connected:"text-green", connecting:"text-yellow", error:"text-red", disconnected:"text-hint" };
const STATUS_DOT = { connected:"●", connecting:"◐", error:"×", disconnected:"○" };

export function renderTabBar(container) {
  container.innerHTML = "";

  // Local tab — always present, not closable
  const localTab = createTab("local", "Local", null, false);
  container.appendChild(localTab);

  for (const tab of state.tabs) {
    const el = createTab(tab.id, tab.label, tab.status, true);
    container.appendChild(el);
  }
}

function createTab(id, label, status, closable) {
  const el = document.createElement("button");
  el.type = "button";
  el.className = "tab";
  el.draggable = true;

  const inLayout = state.layout.panes.some((p) => p.tabId === (id === "local" ? null : id));
  if (inLayout) el.classList.add("active");

  if (id !== "local" && status) {
    el.innerHTML = `<span class="${STATUS_COLOR[status] ?? "text-hint"}">${STATUS_DOT[status] ?? "○"}</span>`;
  }
  el.innerHTML += `<span>${escapeHtml(label)}</span>`;

  // Drag — send tab-id
  el.addEventListener("dragstart", (e) => {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("tab-id", id);
    el.classList.add("tab-dragging");
    setDraggedTabId(id === "local" ? null : id);
  });
  el.addEventListener("dragend", () => {
    el.classList.remove("tab-dragging");
    // Cleanup drop highlights
    document.querySelectorAll(".pane-cell.drop-active").forEach((c) => c.classList.remove("drop-active"));
    setDraggedTabId(undefined);
  });

  // Click
  el.addEventListener("click", () => {
    openInPane(id === "local" ? null : id);
    if (id !== "local") {
      state.lastRemoteTabId = id;
      state.activePane = "remote";
    } else {
      state.activePane = "local";
    }
  });

  // Close
  if (closable) {
    const close = document.createElement("button");
    close.type = "button";
    close.className = "tab-close";
    close.innerHTML = iconMarkup("close", 13);
    close.title = "Close";
    close.addEventListener("click", (e) => {
      e.stopPropagation();
      closeTab(id);
    });
    el.appendChild(close);
  }

  return el;
}

export function renderActivePane() {}

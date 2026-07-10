import { state, notify } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { renderHistoryRows } from "./historyList.js";

let popoverEl = null;
let pendingAnchorRect = null;

function close() {
  state.showHistoryPopup = false;
  notify();
}

export function toggle(anchorEl) {
  // Measure the anchor before notify() -- the toolbar re-renders itself
  // synchronously inside notify() (its computeSig() includes
  // showHistoryPopup), which discards and recreates this exact button. A
  // rect read after that point, or against a stale element reference, would
  // measure a detached node and yield an all-zero rect.
  pendingAnchorRect = anchorEl.getBoundingClientRect();
  state.showHistoryPopup = !state.showHistoryPopup;
  notify();
  if (state.showHistoryPopup) positionNear(pendingAnchorRect);
}

function positionNear(rect) {
  if (!popoverEl || !rect) return;
  popoverEl.style.top = `${rect.bottom + 6}px`;
  popoverEl.style.right = `${window.innerWidth - rect.right}px`;
}

function onOutsideClick(e) {
  if (popoverEl && !popoverEl.contains(e.target) && !e.target.closest("[data-history-toggle]")) {
    close();
  }
}

export function renderHistoryPopup() {
  if (!state.showHistoryPopup) {
    if (popoverEl) {
      document.removeEventListener("mousedown", onOutsideClick, true);
      popoverEl.remove();
      popoverEl = null;
    }
    return;
  }
  if (!popoverEl) {
    popoverEl = document.createElement("div");
    popoverEl.className = "popover";
    document.body.appendChild(popoverEl);
    setTimeout(() => document.addEventListener("mousedown", onOutsideClick, true), 0);
  }

  popoverEl.innerHTML = `
    <div class="popover-header">
      <span class="icon">${iconMarkup("serverSquare", 12)}</span>
      <span>${t("history.recentConnections")}</span>
    </div>
    ${state.history.length === 0 ? `<div class="popover-empty">${t("history.noHistory")}</div>` : '<div class="popover-list" data-role="list"></div>'}
  `;

  if (state.history.length > 0) {
    renderHistoryRows(popoverEl.querySelector('[data-role="list"]'), state.history, {
      rowClass: "popover-row",
      onAfterClick: close,
    });
  }
}

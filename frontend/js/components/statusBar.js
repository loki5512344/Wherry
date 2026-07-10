import { state } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { formatSize, taskStateKind } from "../format.js";
import { escapeHtml } from "../dom.js";

let lastSig = null;

export function renderStatusBar(container) {
  const aggSpeed = state.queueTasks
    .filter((tk) => taskStateKind(tk.state) === "running")
    .reduce((sum, tk) => sum + (tk.speed ?? 0), 0);

  const connected = state.tabs.filter((tb) => tb.status === "connected").length;
  const total = state.tabs.length;

  container.innerHTML = `
    <span data-role="status">${escapeHtml(state.statusMessage)}</span>
    <div class="status-bar-right">
      ${aggSpeed > 0 ? `<span class="text-green mono">${formatSize(aggSpeed)}/s</span>` : ""}
      ${
        total > 0
          ? `<span class="${connected > 0 ? "text-green" : "text-hint"}">${connected > 0 ? iconMarkup("lockPassword", 11) : ""} ${connected}/${total}</span>`
          : `<span class="text-hint">${t("status.notConnected")}</span>`
      }
    </div>
  `;

  lastSig = computeSig();
}

function computeSig() {
  return JSON.stringify([
    state.statusMessage,
    state.tabs.map((tb) => tb.status),
    state.queueTasks.map((tk) => tk.speed),
  ]);
}

export function updateStatusBarIfNeeded(container) {
  if (computeSig() === lastSig) return;
  renderStatusBar(container);
}

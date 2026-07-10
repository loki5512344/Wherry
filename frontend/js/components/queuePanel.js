import { state, notify } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { formatSize, taskStateKind, taskStateDetail, progressPct } from "../format.js";
import * as ipc from "../ipc.js";
import { escapeHtml } from "../dom.js";

const STATE_LABEL_KEY = {
  queued: "queue.stateQueued",
  completed: "queue.stateComplete",
  failed: "queue.stateFailed",
  paused: "queue.statePaused",
  cancelled: "queue.stateCancelled",
  retrying: "queue.stateRetrying",
};

function stateBadge(task) {
  const kind = taskStateKind(task.state);
  if (kind === "running") {
    if (task.speed) return { text: `${formatSize(task.speed)}/s`, cls: "text-green" };
    const verb = task.kind === "upload" ? "queue.stateUploading" : "queue.stateDownloading";
    return { text: t(verb), cls: "text-accent" };
  }
  const cls = { completed: "text-green", failed: "text-red", paused: "text-yellow" }[kind] ?? "text-hint";
  return { text: t(STATE_LABEL_KEY[kind] ?? "queue.stateQueued"), cls };
}

function taskRow(task) {
  const row = document.createElement("div");
  row.className = "queue-task";
  const kind = taskStateKind(task.state);
  const pct = progressPct(task);
  const badge = stateBadge(task);
  const err = kind === "failed" ? taskStateDetail(task.state) : "";

  row.innerHTML = `
    <span class="icon icon--important">${iconMarkup(task.kind === "upload" ? "upload" : "download", 15)}</span>
    <span class="queue-task-name" ${err ? `title="${escapeAttr(err)}"` : ""}>${escapeHtml(task.fileName)}</span>
    <div class="progress"><div class="progress-fill ${kind}" style="width:${pct}%"></div></div>
    <span class="queue-task-pct mono">${pct.toFixed(0)}%</span>
    <span class="queue-task-state ${badge.cls}">${badge.text}</span>
    <div class="queue-task-actions" data-role="actions"></div>
  `;

  const actions = row.querySelector('[data-role="actions"]');
  if (kind === "running") {
    actions.appendChild(iconActionBtn("pauseCircle", () => ipc.pauseTask(task.id)));
    actions.appendChild(iconActionBtn("closeCircle", () => ipc.cancelTask(task.id)));
  } else if (kind === "paused" || kind === "queued") {
    actions.appendChild(iconActionBtn("playCircle", () => ipc.resumeTask(task.id)));
    actions.appendChild(iconActionBtn("closeCircle", () => ipc.cancelTask(task.id)));
  } else {
    actions.appendChild(iconActionBtn("closeCircle", () => ipc.removeTask(task.id)));
  }

  return row;
}

function iconActionBtn(icon, onClick) {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "btn icon-btn btn-ghost";
  btn.style.width = "22px";
  btn.style.height = "22px";
  btn.innerHTML = iconMarkup(icon, 13);
  btn.addEventListener("click", onClick);
  return btn;
}

let lastSig = null;

export function renderQueuePanel(container) {
  const tasks = state.queueTasks;
  const pending = tasks.filter((tk) => ["running", "queued"].includes(taskStateKind(tk.state)));
  const n = pending.length > 0 ? pending.length : tasks.length;
  const expanded = state.showQueue && tasks.length > 0;

  container.classList.toggle("expanded", expanded);

  const aggSpeed = tasks
    .filter((tk) => taskStateKind(tk.state) === "running")
    .reduce((sum, tk) => sum + (tk.speed ?? 0), 0);
  const done = tasks.filter((tk) => taskStateKind(tk.state) === "completed").length;
  const failed = tasks.filter((tk) => taskStateKind(tk.state) === "failed").length;

  let rightHtml = "";
  if (aggSpeed > 0) {
    rightHtml = `<span class="text-green mono">${formatSize(aggSpeed)}/s</span>`;
  } else if (tasks.length > 0) {
    rightHtml = `<span class="text-dim">${t("queue.doneSuffix", { done, total: tasks.length })}</span>`;
    if (failed > 0) rightHtml += `<span class="text-red" style="margin-left:8px">${t("queue.failedSuffix", { n: failed })}</span>`;
  }

  container.innerHTML = `
    <div class="queue-header" data-role="header">
      <span class="icon">${iconMarkup(state.showQueue ? "altArrowDown" : "altArrowUp", 13)}</span>
      <span class="queue-header-title">${t("queue.title", { n })}</span>
      <div class="queue-header-right">${rightHtml}</div>
    </div>
    <div class="queue-list" data-role="list"></div>
  `;

  container.querySelector('[data-role="header"]').addEventListener("click", () => {
    state.showQueue = !state.showQueue;
    notify();
  });

  const list = container.querySelector('[data-role="list"]');
  if (tasks.length === 0) {
    list.innerHTML = `<div class="popover-empty">${t("queue.empty")}</div>`;
  } else {
    for (const task of tasks) list.appendChild(taskRow(task));
  }

  lastSig = computeSig();
}

function computeSig() {
  return JSON.stringify([
    state.showQueue,
    state.queueTasks.map((tk) => [tk.id, tk.transferredBytes, tk.speed, JSON.stringify(tk.state)]),
  ]);
}

export function updateQueuePanelIfNeeded(container) {
  if (computeSig() === lastSig) return;
  renderQueuePanel(container);
}

const escapeAttr = escapeHtml;

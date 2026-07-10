import { state, notify } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { escapeHtml } from "../dom.js";
import { connectFromSite, PROTOCOL_LABELS } from "../connectActions.js";
import * as ipc from "../ipc.js";

const PROTOCOLS = ["sftp", "ftp", "ftps"];
const DEFAULT_PORTS = { sftp: 22, ftp: 21, ftps: 990 };

let overlayEl = null;
let viewMode = "list";
let form = emptyForm();
let editingId = null;
let saving = false;
let deleteConfirmId = null;

function emptyForm() {
  return {
    name: "",
    protocol: "sftp",
    host: "",
    port: "",
    username: "",
    password: "",
    keyPath: "",
    folder: "",
    note: "",
  };
}

export function openSiteManager(siteId) {
  const site = state.sites.find((s) => s.id === siteId);
  if (!site) return;
  editingId = siteId;
  form = {
    name: site.name,
    protocol: site.protocol,
    host: site.host,
    port: String(site.port),
    username: site.username,
    password: site.password ?? "",
    keyPath: site.keyPath ?? "",
    folder: site.folder ?? "",
    note: site.note ?? "",
  };
  viewMode = "form";
  saving = false;
  deleteConfirmId = null;
  state.showSettingsDialog = false;
  state.showSiteManager = true;
  notify();
}

export function openSiteManagerNew() {
  editingId = null;
  form = emptyForm();
  viewMode = "list";
  saving = false;
  deleteConfirmId = null;
  state.showSiteManager = true;
  notify();
}

function close() {
  state.showSiteManager = false;
  overlayEl?.remove();
  overlayEl = null;
  notify();
}

function switchToList() {
  viewMode = "list";
  deleteConfirmId = null;
  notify();
}

async function saveCurrent() {
  if (saving) return;
  const name = form.name.trim();
  const host = form.host.trim();
  const username = form.username.trim();
  if (!name || !host || !username) return;

  let port = DEFAULT_PORTS[form.protocol];
  if (form.port.trim()) {
    const parsed = Number(form.port);
    if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) return;
    port = parsed;
  }

  saving = true;
  notify();

  const site = {
    id: editingId || crypto.randomUUID(),
    name,
    protocol: form.protocol,
    host,
    port,
    username,
    password: form.password || null,
    keyPath: form.keyPath || null,
    folder: form.folder || null,
    note: form.note || null,
  };

  try {
    await ipc.saveSite(site);
    state.sites = await ipc.listSites();
    state.statusMessage = t("siteManager.saved");
    viewMode = "list";
    saving = false;
    notify();
  } catch {
    saving = false;
    notify();
  }
}

async function deleteSiteAction(id) {
  const site = state.sites.find((s) => s.id === id);
  await ipc.deleteSite(id);
  await ipc.deletePassword(id);
  state.sites = state.sites.filter((s) => s.id !== id);
  if (site) state.statusMessage = `Removed ${site.name}`;
  notify();
}

async function browseKeyFile() {
  try {
    const dialog = window.__TAURI__.dialog;
    const path = await dialog.open({ title: "Choose SSH key", multiple: false, directory: false });
    if (typeof path === "string") {
      form.keyPath = path;
      const input = overlayEl?.querySelector("[data-field=keyPath]");
      if (input) input.value = path;
    }
  } catch {
    /* dialog plugin unavailable / user cancelled */
  }
}

function build() {
  const overlay = document.createElement("div");
  overlay.className = "overlay";
  overlay.addEventListener("mousedown", (e) => {
    if (e.target === overlay) close();
  });

  const dialog = document.createElement("div");
  dialog.className = "dialog";
  dialog.style.width = "440px";
  dialog.addEventListener("mousedown", (e) => e.stopPropagation());
  dialog.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && viewMode === "form") saveCurrent();
  });

  dialog.innerHTML = `
    <div class="dialog-header">
      <span class="icon icon--important">${iconMarkup("serverSquare", 16)}</span>
      <span class="dialog-title" data-role="title">${t("siteManager.title")}</span>
      <button type="button" class="dialog-close" data-action="close">${iconMarkup("close", 16)}</button>
    </div>
    <div class="dialog-body" data-role="body" style="max-height:60vh;overflow-y:auto"></div>
  `;

  dialog.querySelector('[data-action="close"]').addEventListener("click", close);

  overlay.appendChild(dialog);
  return overlay;
}

function renderList() {
  const title = overlayEl.querySelector('[data-role="title"]');
  title.textContent = t("siteManager.title");

  const body = overlayEl.querySelector('[data-role="body"]');

  let html = "";
  if (!state.sites.length) {
    html += `<div class="settings-empty">${t("siteManager.noSites")}</div>`;
  } else {
    html += state.sites.map(
      (s) => `
      <div class="settings-list-row">
        <div class="settings-list-row-main">
          <div class="settings-list-row-title">
            <span class="proto-badge" style="margin-right:6px">${PROTOCOL_LABELS[s.protocol] ?? s.protocol}</span>
            ${escapeHtml(s.name)}
          </div>
          <div class="settings-list-row-sub">${escapeHtml(s.username)}@${escapeHtml(s.host)}:${s.port}</div>
        </div>
        <div class="settings-row" style="flex-shrink:0;gap:4px">
          <button type="button" class="btn icon-btn btn-ghost" data-action="sm-connect" data-id="${s.id}" title="${t("connect.connect")}">${iconMarkup("playCircle", 14)}</button>
          <button type="button" class="btn icon-btn btn-ghost" data-action="sm-edit" data-id="${s.id}" title="${t("siteManager.editSiteHover")}">${iconMarkup("pen", 13)}</button>
          <button type="button" class="btn icon-btn btn-ghost" data-action="sm-delete" data-id="${s.id}" title="${t("settings.deleteSiteHover")}">${iconMarkup("trash", 14)}</button>
        </div>
      </div>`,
    ).join("");
  }

  html += `
    <div style="margin-top:12px">
      <button type="button" class="btn btn-accent" data-action="sm-add" style="width:100%">
        ${iconMarkup("addCircle", 14)}<span>${t("siteManager.addSite")}</span>
      </button>
    </div>
  `;

  body.innerHTML = html;

  body.querySelectorAll("[data-action='sm-connect']").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const site = state.sites.find((s) => s.id === btn.dataset.id);
      if (site) {
        close();
        await connectFromSite(site);
      }
    });
  });

  body.querySelectorAll("[data-action='sm-edit']").forEach((btn) => {
    btn.addEventListener("click", () => openSiteManager(btn.dataset.id));
  });

  body.querySelectorAll("[data-action='sm-delete']").forEach((btn) => {
    btn.addEventListener("click", async () => {
      await deleteSiteAction(btn.dataset.id);
      if (overlayEl) renderList();
    });
  });

  body.querySelector("[data-action='sm-add']").addEventListener("click", () => {
    editingId = null;
    form = emptyForm();
    viewMode = "form";
    notify();
  });
}

function renderForm() {
  const title = overlayEl.querySelector('[data-role="title"]');
  title.textContent = editingId ? t("siteManager.editSite") : t("siteManager.addSite");

  const body = overlayEl.querySelector('[data-role="body"]');

  body.innerHTML = `
    <div class="field">
      <span class="field-label">${t("siteManager.name")}</span>
      <input class="field-input" data-field="name" type="text" value="${escapeHtml(form.name)}" />
    </div>

    <div class="field">
      <span class="field-label">${t("siteManager.protocol")}</span>
      <div class="segmented" data-role="protocol">
        ${PROTOCOLS.map((p) => `<button type="button" class="segmented-item ${form.protocol === p ? "active" : ""}" data-protocol="${p}">${PROTOCOL_LABELS[p]}</button>`).join("")}
      </div>
    </div>

    <div class="field">
      <span class="field-label">${t("connect.host")}</span>
      <input class="field-input" data-field="host" type="text" value="${escapeHtml(form.host)}" placeholder="example.com" />
    </div>

    <div class="field-row">
      <div class="field" style="flex:1">
        <span class="field-label">${t("connect.username")}</span>
        <input class="field-input" data-field="username" type="text" value="${escapeHtml(form.username)}" />
      </div>
      <div class="field" style="width:80px">
        <span class="field-label">${t("connect.port")}</span>
        <input class="field-input" data-field="port" type="text" value="${escapeHtml(form.port)}" placeholder="${DEFAULT_PORTS[form.protocol]}" />
      </div>
    </div>

    <div class="field">
      <span class="field-label">${t("connect.password")}</span>
      <div class="field-with-icon">
        <input class="field-input" data-field="password" type="password" value="${escapeHtml(form.password)}" />
        <button type="button" class="field-icon-btn" data-action="toggle-pass" title="Show/hide">${iconMarkup("eye", 15)}</button>
      </div>
    </div>

    <div data-role="key-section" style="display:${form.protocol === "sftp" ? "block" : "none"}">
      <p class="field-hint">${t("siteManager.keyHint")}</p>
      <div class="field">
        <span class="field-label">${t("connect.keyFile")}</span>
        <div class="field-with-btn">
          <input class="field-input" data-field="keyPath" type="text" value="${escapeHtml(form.keyPath)}" />
          <button type="button" class="btn btn-ghost" data-action="browse" style="flex-shrink:0">${t("connect.browse")}</button>
        </div>
      </div>
    </div>

    <div class="field">
      <span class="field-label">${t("siteManager.folder")}</span>
      <input class="field-input" data-field="folder" type="text" value="${escapeHtml(form.folder)}" />
    </div>

    <div class="field">
      <span class="field-label">${t("siteManager.note")}</span>
      <textarea class="field-input" data-field="note" rows="3" style="resize:vertical;padding:6px 10px;height:auto;line-height:1.4;font-family:inherit">${escapeHtml(form.note)}</textarea>
    </div>

    <div class="dialog-footer" style="margin-top:16px">
      <button type="button" class="btn btn-ghost" data-action="cancel">${t("common.cancel")}</button>
      <div class="spacer"></div>
      <button type="button" class="btn btn-accent" data-action="save" ${saving ? "disabled" : ""}>
        ${saving ? t("siteManager.saving") : t("common.save")}
      </button>
      ${saving ? '<span class="spinner" style="margin-left:8px"></span>' : ""}
    </div>
  `;

  body.querySelector('[data-action="cancel"]').addEventListener("click", switchToList);
  body.querySelector('[data-action="save"]').addEventListener("click", saveCurrent);

  body.querySelectorAll("[data-field]").forEach((input) => {
    input.addEventListener("input", () => {
      form[input.dataset.field] = input.value;
    });
  });

  const passInput = body.querySelector('[data-field="password"]');
  const toggleBtn = body.querySelector('[data-action="toggle-pass"]');
  if (toggleBtn && passInput) {
    toggleBtn.addEventListener("click", () => {
      const showing = passInput.type === "text";
      passInput.type = showing ? "password" : "text";
      toggleBtn.innerHTML = iconMarkup(showing ? "eye" : "eyeClosed", 15);
    });
  }

  const browseBtn = body.querySelector('[data-action="browse"]');
  if (browseBtn) browseBtn.addEventListener("click", browseKeyFile);

  body.querySelectorAll("[data-protocol]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const proto = btn.dataset.protocol;
      form.protocol = proto;
      const portInput = body.querySelector('[data-field="port"]');
      if (!form.port) {
        portInput.placeholder = String(DEFAULT_PORTS[proto]);
      }
      body.querySelectorAll("[data-protocol]").forEach((b) => b.classList.toggle("active", b === btn));
      const keySection = body.querySelector('[data-role="key-section"]');
      if (keySection) keySection.style.display = proto === "sftp" ? "block" : "none";
    });
  });

  setTimeout(() => body.querySelector('[data-field="name"]')?.focus(), 0);
}

export function renderSiteManager() {
  if (!state.showSiteManager) {
    if (overlayEl) {
      overlayEl.remove();
      overlayEl = null;
    }
    return;
  }

  if (!overlayEl) {
    overlayEl = build();
    document.body.appendChild(overlayEl);
  }

  if (viewMode === "list") {
    renderList();
  } else {
    renderForm();
  }
}

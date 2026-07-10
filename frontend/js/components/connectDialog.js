// "New Connection" modal — src/ui/dialogs/connection/{view,form}.rs equivalent.
import { state, notify } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { buildParamsFromForm, spawnConnect, DEFAULT_PORTS } from "../connectActions.js";
import { escapeHtml as escapeAttr } from "../dom.js";

const PROTOCOLS = ["sftp", "ftp", "ftps"];
const PROTOCOL_LABELS = { sftp: "SFTP", ftp: "FTP", ftps: "FTPS" };

let form = emptyForm();
let overlayEl = null;
let lastOpenSig = null;

function emptyForm() {
  return {
    protocol: "sftp",
    host: "",
    port: "",
    username: "",
    password: "",
    keyPath: "",
    label: "",
  };
}

/** Called by the toolbar "New Connection" button and the welcome screen CTA. */
export function openNew() {
  form = emptyForm();
  state.connectError = "";
  state.showConnectDialog = true;
  notify();
}

/** Called from history "Edit" — prefills everything. */
export function openForEdit(entry) {
  form = {
    protocol: entry.protocol,
    host: entry.host,
    port: String(entry.port),
    username: entry.username,
    password: "",
    keyPath: entry.keyPath ?? "",
    label: "",
  };
  state.connectError = "";
  state.showConnectDialog = true;
  notify();
}

function close() {
  if (state.connectLoading) return;
  state.showConnectDialog = false;
  notify();
}

async function submit() {
  if (state.connectLoading) return;
  const result = buildParamsFromForm(form);
  if (!result.ok) {
    state.connectError = t(result.error);
    notify();
    return;
  }
  await spawnConnect(result.params);
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
  dialog.addEventListener("mousedown", (e) => e.stopPropagation());

  dialog.innerHTML = `
    <div class="dialog-header">
      <span class="icon icon--important">${iconMarkup("serverSquare", 16)}</span>
      <span class="dialog-title">${t("connect.title")}</span>
      <button type="button" class="dialog-close" data-action="close">${iconMarkup("close", 16)}</button>
    </div>
    <div class="dialog-body">
      <div class="segmented" data-role="protocol">
        ${PROTOCOLS.map((p) => `<button type="button" class="segmented-item ${form.protocol === p ? "active" : ""}" data-protocol="${p}">${PROTOCOL_LABELS[p]}</button>`).join("")}
      </div>

      <div class="field">
        <span class="field-label">${t("connect.host")}</span>
        <input class="field-input" data-field="host" type="text" value="${escapeAttr(form.host)}" placeholder="example.com" />
      </div>

      <div class="field-row">
        <div class="field" style="flex:1">
          <span class="field-label">${t("connect.username")}</span>
          <input class="field-input" data-field="username" type="text" value="${escapeAttr(form.username)}" />
        </div>
        <div class="field" style="width:80px">
          <span class="field-label">${t("connect.port")}</span>
          <input class="field-input" data-field="port" type="text" value="${escapeAttr(form.port)}" placeholder="${DEFAULT_PORTS[form.protocol]}" />
        </div>
      </div>

      <div class="field">
        <span class="field-label">${t("connect.password")}</span>
        <div class="field-with-icon">
          <input class="field-input" data-field="password" type="password" value="${escapeAttr(form.password)}" placeholder="" />
          <button type="button" class="field-icon-btn" data-action="toggle-pass" title="Show/hide">${iconMarkup("eye", 15)}</button>
        </div>
      </div>

      <div data-role="key-section" style="display:${form.protocol === "sftp" ? "block" : "none"}">
        <p class="field-hint">${t("connect.keyHint")}</p>
        <div class="field">
          <span class="field-label">${t("connect.keyFile")}</span>
          <div class="field-with-btn">
            <input class="field-input" data-field="keyPath" type="text" value="${escapeAttr(form.keyPath)}" />
            <button type="button" class="btn btn-ghost" data-action="browse" style="flex-shrink:0">${t("connect.browse")}</button>
          </div>
        </div>
      </div>

      <div class="field">
        <span class="field-label">${t("connect.label")}</span>
        <input class="field-input" data-field="label" type="text" value="${escapeAttr(form.label)}" />
      </div>

      <div data-role="error"></div>
    </div>
    <div class="dialog-footer">
      <button type="button" class="btn btn-ghost" data-action="close">${t("common.cancel")}</button>
      <div class="spacer"></div>
      <button type="button" class="btn btn-accent" data-action="submit">${t("connect.connect")}</button>
      <span data-role="spinner" style="margin-left:8px"></span>
    </div>
  `;

  overlay.appendChild(dialog);

  dialog.querySelectorAll('[data-action="close"]').forEach((el) =>
    el.addEventListener("click", close),
  );
  dialog.querySelector('[data-action="submit"]').addEventListener("click", submit);
  dialog.querySelector('[data-action="browse"]').addEventListener("click", browseKeyFile);

  dialog.querySelectorAll("[data-field]").forEach((input) => {
    input.addEventListener("input", () => {
      form[input.dataset.field] = input.value;
    });
  });

  const passInput = dialog.querySelector('[data-field="password"]');
  dialog.querySelector('[data-action="toggle-pass"]').addEventListener("click", (e) => {
    const showing = passInput.type === "text";
    passInput.type = showing ? "password" : "text";
    e.currentTarget.innerHTML = iconMarkup(showing ? "eye" : "eyeClosed", 15);
  });

  dialog.querySelectorAll("[data-protocol]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const proto = btn.dataset.protocol;
      form.protocol = proto;
      if (!form.port) {
        dialog.querySelector('[data-field="port"]').placeholder = String(DEFAULT_PORTS[proto]);
      }
      dialog.querySelectorAll("[data-protocol]").forEach((b) => b.classList.toggle("active", b === btn));
      dialog.querySelector('[data-role="key-section"]').style.display = proto === "sftp" ? "block" : "none";
    });
  });

  dialog.addEventListener("keydown", (e) => {
    if (e.key === "Enter") submit();
  });

  return { overlay, dialog };
}

function updateDynamic(dialog) {
  const errRole = dialog.querySelector('[data-role="error"]');
  errRole.innerHTML = state.connectError
    ? `<div class="error-box">${iconMarkup("dangerTriangle", 14)}<span>${escapeAttr(state.connectError)}</span></div>`
    : "";

  const submitBtn = dialog.querySelector('[data-action="submit"]');
  submitBtn.disabled = state.connectLoading;
  submitBtn.textContent = state.connectLoading ? t("connect.connecting") : t("connect.connect");
  const cancelBtn = dialog.querySelector('[data-action="close"]');
  cancelBtn.disabled = state.connectLoading;

  dialog.querySelector('[data-role="spinner"]').innerHTML = state.connectLoading
    ? '<span class="spinner"></span>'
    : "";
}

export function renderConnectDialog() {
  const sig = JSON.stringify([state.showConnectDialog]);
  if (!state.showConnectDialog) {
    if (overlayEl) {
      overlayEl.remove();
      overlayEl = null;
    }
    lastOpenSig = sig;
    return;
  }

  if (sig !== lastOpenSig || !overlayEl) {
    if (overlayEl) overlayEl.remove();
    const { overlay, dialog } = build();
    document.body.appendChild(overlay);
    overlayEl = overlay;
    overlayEl._dialog = dialog;
    setTimeout(() => dialog.querySelector('[data-field="host"]').focus(), 0);
    lastOpenSig = sig;
  }

  updateDynamic(overlayEl._dialog);
}

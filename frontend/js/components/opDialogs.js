// New Folder / Rename / Delete-confirm modals — shared by local and remote
// panes, applied to whichever side is `state.opTarget`.
import { state, notify } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { doMkdir, doRename, doDelete } from "../fsActions.js";
import { escapeHtml as escapeAttr } from "../dom.js";

let overlayEl = null;
let lastSig = null;

export function openMkdir() {
  state.dialog = { kind: "mkdir", value: "" };
  notify();
}

export function openRename(name) {
  state.dialog = { kind: "rename", oldName: name, value: name };
  notify();
}

export function openDelete(names) {
  // Settings → General → "Confirm before delete" off skips the modal
  // entirely instead of just pre-answering it.
  if (!state.settings.confirmBeforeDelete) {
    doDelete(names);
    return;
  }
  state.dialog = { kind: "delete", names };
  notify();
}

export function closeDialog() {
  state.dialog = null;
  notify();
}

async function submit() {
  const d = state.dialog;
  if (!d) return;
  if (d.kind === "mkdir") {
    const name = d.value.trim();
    if (!name) return;
    closeDialog();
    await doMkdir(name);
  } else if (d.kind === "rename") {
    const name = d.value.trim();
    if (!name || name === d.oldName) {
      closeDialog();
      return;
    }
    closeDialog();
    await doRename(d.oldName, name);
  } else if (d.kind === "delete") {
    closeDialog();
    await doDelete(d.names);
  }
  notify();
}

function buildMkdirOrRename(d) {
  const isRename = d.kind === "rename";
  const overlay = document.createElement("div");
  overlay.className = "overlay";
  overlay.addEventListener("mousedown", (e) => {
    if (e.target === overlay) closeDialog();
  });
  const dialog = document.createElement("div");
  dialog.className = "dialog dialog-sm";
  dialog.addEventListener("mousedown", (e) => e.stopPropagation());
  dialog.innerHTML = `
    <div class="dialog-header">
      <span class="icon icon--important">${iconMarkup(isRename ? "pen" : "folderAdd", 15)}</span>
      <span class="dialog-title">${isRename ? t("dialogs.renameTitle") : t("dialogs.newFolderTitle")}</span>
    </div>
    <div class="dialog-body">
      <input class="field-input" data-field="value" type="text" value="${escapeAttr(d.value)}"
        placeholder="${isRename ? t("dialogs.renamePlaceholder") : t("dialogs.newFolderPlaceholder")}" />
    </div>
    <div class="dialog-footer">
      <button type="button" class="btn btn-ghost" data-action="close">${t("common.cancel")}</button>
      <div class="spacer"></div>
      <button type="button" class="btn btn-accent" data-action="submit">${t("common.ok")}</button>
    </div>
  `;
  overlay.appendChild(dialog);
  dialog.querySelector('[data-action="close"]').addEventListener("click", closeDialog);
  dialog.querySelector('[data-action="submit"]').addEventListener("click", submit);
  const input = dialog.querySelector('[data-field="value"]');
  input.addEventListener("input", () => {
    state.dialog.value = input.value;
  });
  dialog.addEventListener("keydown", (e) => {
    if (e.key === "Enter") submit();
  });
  document.body.appendChild(overlay);
  setTimeout(() => {
    input.focus();
    if (isRename) {
      const dot = input.value.lastIndexOf(".");
      input.setSelectionRange(0, dot > 0 ? dot : input.value.length);
    } else {
      input.select();
    }
  }, 0);
  return overlay;
}

function buildDelete(d) {
  const overlay = document.createElement("div");
  overlay.className = "overlay";
  overlay.addEventListener("mousedown", (e) => {
    if (e.target === overlay) closeDialog();
  });
  const dialog = document.createElement("div");
  dialog.className = "dialog dialog-sm";
  dialog.addEventListener("mousedown", (e) => e.stopPropagation());
  const title =
    d.names.length === 1
      ? t("dialogs.deleteTitle", { name: escapeAttr(d.names[0]) })
      : t("dialogs.deleteTitleMulti", { n: d.names.length });
  dialog.innerHTML = `
    <div class="dialog-header">
      <span class="icon text-red">${iconMarkup("dangerTriangle", 15)}</span>
      <span class="dialog-title">${title}</span>
    </div>
    <div class="dialog-body">
      <p class="dialog-text">${t("dialogs.deleteBody")}</p>
    </div>
    <div class="dialog-footer">
      <button type="button" class="btn btn-ghost" data-action="close">${t("common.cancel")}</button>
      <div class="spacer"></div>
      <button type="button" class="btn btn-danger" data-action="submit">${t("common.delete")}</button>
    </div>
  `;
  overlay.appendChild(dialog);
  dialog.querySelector('[data-action="close"]').addEventListener("click", closeDialog);
  dialog.querySelector('[data-action="submit"]').addEventListener("click", submit);
  document.body.appendChild(overlay);
  return overlay;
}

export function renderOpDialogs() {
  const sig = JSON.stringify(
    state.dialog ? { kind: state.dialog.kind, names: state.dialog.names, oldName: state.dialog.oldName } : null,
  );
  if (sig === lastSig) return;
  lastSig = sig;

  if (overlayEl) {
    overlayEl.remove();
    overlayEl = null;
  }
  if (!state.dialog) return;

  overlayEl =
    state.dialog.kind === "delete" ? buildDelete(state.dialog) : buildMkdirOrRename(state.dialog);
}

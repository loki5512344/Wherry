// Settings dialog — nav list left, section content right. Mirrors the old
// src/ui/app/settings/{general,connections,transfers,about}.rs sections
// (Security folded in here too). Unlike connectDialog/opDialogs there are no
// free-text inputs in here, so the whole content pane can just be rebuilt
// from scratch on every render -- no focus/caret state to preserve.
import { state, notify } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { escapeHtml } from "../dom.js";
import {
  setConfirmBeforeDelete,
  useCurrentFolderAsDefault,
  resetDefaultFolder,
  setMaxConcurrent,
  setAutoClearCompletedSecs,
  togglePanel,
  clearHistory,
  deleteSite,
  forgetPassword,
  forgetAllPasswords,
  revealDatabase,
  setLanguage,
  setTheme,
} from "../settingsActions.js";

const LANG_OPTIONS = [
  ["en", "English"],
  ["ru", "Русский"],
  ["es", "Español"],
  ["fr", "Français"],
  ["de", "Deutsch"],
  ["it", "Italiano"],
  ["pt", "Português"],
  ["pl", "Polski"],
  ["zh", "简体中文"],
  ["ja", "日本語"],
  ["ko", "한국어"],
  ["tr", "Türkçe"],
];

const SECTIONS = [
  { id: "general", icon: "settings", labelKey: "settings.navGeneral" },
  { id: "appearance", icon: "monitor", labelKey: "settings.navAppearance" },
  { id: "connections", icon: "serverSquare", labelKey: "settings.navConnections" },
  { id: "history", icon: "history", labelKey: "settings.navHistory" },
  { id: "security", icon: "shieldKeyhole", labelKey: "settings.navSecurity" },
  { id: "transfers", icon: "clock", labelKey: "settings.navTransfers" },
  { id: "about", icon: "documentText", labelKey: "settings.navAbout" },
];

let overlayEl = null;
let activeSection = "general";
let historyClearConfirm = false;

export function openSettings() {
  activeSection = "general";
  historyClearConfirm = false;
  state.showSettingsDialog = true;
  notify();
}

function close() {
  state.showSettingsDialog = false;
  notify();
}

function switchSection(id) {
  activeSection = id;
  historyClearConfirm = false;
  renderContent();
  renderNav();
}

// A plain <span>, not a <button> -- a labelable descendant (button, input)
// inside a clickable wrapper gets a second, forwarded click event from the
// browser itself on top of the one the wrapper's own listener already
// handles, which silently double-toggles state right back to where it
// started. Since the wrapper handles the click, the indicator only needs to
// be visual.
function switchToggle(active) {
  return `<span class="switch ${active ? "on" : ""}" data-role="switch"></span>`;
}

// --- section renderers -------------------------------------------------------

function sectionGeneral() {
  const folder = state.settings.defaultLocalFolder || t("settings.homeDirectory");
  return `
    <div class="settings-section-title">${t("settings.navGeneral")}</div>
    <div class="settings-block">
      <div class="settings-check-row" data-action="toggle-confirm-delete">
        ${switchToggle(state.settings.confirmBeforeDelete)}
        <div>
          <div class="settings-label">${t("settings.confirmDelete")}</div>
          <div class="settings-hint">${t("settings.confirmDeleteHint")}</div>
        </div>
      </div>
    </div>
    <div class="settings-block">
      <div class="settings-label">${t("settings.language")}</div>
      <div class="settings-hint">${t("settings.languageHint")}</div>
      <div class="settings-row" style="margin-top:8px">
        <select class="settings-select" data-action="set-language">
          ${LANG_OPTIONS.map(([code, native]) =>
            `<option value="${code}" ${code === state.settings.language ? "selected" : ""}>${native}</option>`
          ).join("")}
        </select>
      </div>
    </div>
    <div class="settings-block">
      <div class="settings-label">${t("settings.startupFolder")}</div>
      <div class="settings-hint">${t("settings.startupFolderHint")}</div>
      <div class="settings-hint mono" style="margin-top:8px;color:var(--text-dim)">${escapeHtml(folder)}</div>
      <div class="settings-row" style="margin-top:10px">
        <button type="button" class="btn btn-ghost" data-action="use-current-folder">${t("settings.useCurrentFolder")}</button>
        <button type="button" class="btn btn-ghost" data-action="reset-folder">${t("settings.resetToHome")}</button>
      </div>
    </div>
  `;
}

function sectionAppearance() {
  const currentTheme = state.settings.theme;
  const themes = [
    ["ember", "settings.themeEmber"],
    ["frost", "settings.themeFrost"],
    ["midnight", "settings.themeMidnight"],
    ["forest", "settings.themeForest"],
    ["dawn", "settings.themeDawn"],
    ["ash", "settings.themeAsh"],
  ];

  const toggles = [
    ["showToolbar", "settings.toolbar"],
    ["showSidebar", "settings.sidebar"],
    ["showStatusBar", "settings.statusBar"],
    ["showQueuePanel", "settings.transferQueue"],
  ];
  return `
    <div class="settings-section-title">${t("settings.navAppearance")}</div>
    <div class="settings-block">
      <div class="settings-label">${t("settings.theme")}</div>
      <div class="settings-hint" style="margin-bottom:10px">${t("settings.themeHint")}</div>
      <div class="theme-grid" data-action="set-theme">
        ${themes
          .map(
            ([id, labelKey]) => `
          <button type="button" class="theme-card ${id === currentTheme ? "active" : ""}" data-theme-id="${id}">
            <div class="theme-swatch theme-swatch--${id}"></div>
            <span class="theme-card-label">${t(labelKey)}</span>
          </button>`,
          )
          .join("")}
      </div>
    </div>
    <div class="settings-block">
      ${toggles
        .map(
          ([key, labelKey]) => `
        <div class="settings-check-row" style="margin-bottom:10px" data-panel-toggle="${key}">
          ${switchToggle(state.settings[key])}
          <div class="settings-label">${t(labelKey)}</div>
        </div>`,
        )
        .join("")}
    </div>
  `;
}

function sectionConnections() {
  const sites = state.sites
    .map(
      (s) => `
    <div class="settings-list-row">
      <div class="settings-list-row-main">
        <div class="settings-list-row-title">${escapeHtml(s.name)}</div>
        <div class="settings-list-row-sub">${escapeHtml(s.username)}@${escapeHtml(s.host)}:${s.port}</div>
      </div>
      <button type="button" class="btn icon-btn btn-ghost" data-action="delete-site" data-id="${s.id}" title="${t("settings.deleteSiteHover")}">${iconMarkup("trash", 14)}</button>
    </div>`,
    )
    .join("");

  const bookmarks = state.sidebar.bookmarks
    .map(
      (b) => `
    <div class="settings-list-row">
      <div class="settings-list-row-main">
        <div class="settings-list-row-title">${escapeHtml(b.name)}</div>
        <div class="settings-list-row-sub">${escapeHtml(b.path)}</div>
      </div>
      <button type="button" class="btn icon-btn btn-ghost" data-action="delete-bookmark" data-id="${b.id}" title="${t("panels.sidebarRemoveBookmark")}">${iconMarkup("trash", 14)}</button>
    </div>`,
    )
    .join("");

  return `
    <div class="settings-section-title">${t("settings.sites")}</div>
    <div class="settings-block">
      ${state.sites.length ? sites : `<div class="settings-empty">${t("settings.noSites")}</div>`}
    </div>
    <div class="settings-block">
      <div class="settings-label">${t("settings.bookmarksTitle")}</div>
      ${state.sidebar.bookmarks.length ? bookmarks : `<div class="settings-empty">${t("settings.noBookmarks")}</div>`}
    </div>
  `;
}

function sectionHistory() {
  const rows = state.history
    .map(
      (h) => `
    <div class="settings-list-row">
      <div class="settings-list-row-main">
        <div class="settings-list-row-title">${escapeHtml(h.username)}@${escapeHtml(h.host)}:${h.port}</div>
      </div>
      <div class="settings-list-row-sub">${escapeHtml(h.connectedAt)}</div>
    </div>`,
    )
    .join("");

  const footer = state.history.length
    ? historyClearConfirm
      ? `
      <div class="settings-hint" style="margin-top:12px">${t("settings.confirmClear")}</div>
      <div class="settings-row" style="margin-top:8px">
        <button type="button" class="btn btn-danger" data-action="confirm-clear-history">${t("settings.deleteAll")}</button>
        <button type="button" class="btn btn-ghost" data-action="cancel-clear-history">${t("common.cancel")}</button>
      </div>`
      : `<button type="button" class="btn btn-ghost" style="margin-top:12px;color:var(--red)" data-action="clear-history">${t("settings.clearAll")}</button>`
    : "";

  return `
    <div class="settings-section-title">${t("settings.historyTitle")}</div>
    <div class="settings-block">
      ${state.history.length ? rows : `<div class="settings-empty">${t("settings.historyEmpty")}</div>`}
      ${footer}
    </div>
  `;
}

function sectionSecurity() {
  const rows = state.sites
    .map(
      (s) => `
    <div class="settings-list-row">
      <div class="settings-list-row-main">
        <div class="settings-list-row-title">${escapeHtml(s.name)}</div>
      </div>
      <button type="button" class="btn icon-btn btn-ghost" data-action="forget-password" data-id="${s.id}" title="${t("settings.forgetPasswordHover")}">${iconMarkup("lockPassword", 13)}</button>
    </div>`,
    )
    .join("");

  return `
    <div class="settings-section-title">${t("settings.navSecurity")}</div>
    <div class="settings-hint">${t("settings.securityHint")}</div>
    <div class="settings-block">
      ${state.sites.length ? rows : `<div class="settings-empty">${t("settings.noConnections")}</div>`}
    </div>
    <div class="settings-block">
      <button type="button" class="btn btn-ghost" style="color:var(--red)" data-action="forget-all">${t("settings.forgetAll")}</button>
    </div>
  `;
}

function segmented(options, current, action) {
  return `
    <div class="segmented" data-action="${action}">
      ${options
        .map(
          ([label, value]) =>
            `<button type="button" class="segmented-item ${value === current ? "active" : ""}" data-value="${value}">${label}</button>`,
        )
        .join("")}
    </div>
  `;
}

function sectionTransfers() {
  return `
    <div class="settings-section-title">${t("settings.navTransfers")}</div>
    <div class="settings-block">
      <div class="settings-label">${t("settings.concurrentTitle")}</div>
      <div class="settings-hint">${t("settings.concurrentHint")}</div>
      <div style="margin-top:8px;width:220px">
        ${segmented(
          [1, 2, 3, 4, 5, 6].map((n) => [String(n), n]),
          state.settings.maxConcurrent,
          "set-max-concurrent",
        )}
      </div>
    </div>
    <div class="settings-block">
      <div class="settings-label">${t("settings.autoRemoveTitle")}</div>
      <div class="settings-hint">${t("settings.autoRemoveHint")}</div>
      <div style="margin-top:8px;width:260px">
        ${segmented(
          [
            [t("settings.never"), 0],
            [t("settings.tenSec"), 10],
            [t("settings.thirtySec"), 30],
            [t("settings.oneMin"), 60],
          ],
          state.settings.autoClearCompletedSecs,
          "set-auto-clear",
        )}
      </div>
    </div>
  `;
}

function sectionAbout() {
  return `
    <div class="settings-section-title">${t("settings.navAbout")}</div>
    <div class="settings-block">
      <div class="settings-value-row"><span class="k">${t("settings.version")}</span><span class="v">0.1.0</span></div>
      <div class="settings-value-row"><span class="k">${t("settings.license")}</span><span class="v">GPL-3.0-only</span></div>
      <div class="settings-value-row"><span class="k">${t("settings.database")}</span><span class="v" data-role="db-path">…</span></div>
      <button type="button" class="btn btn-ghost" style="margin-top:10px" data-action="reveal-db">${t("settings.revealInFolder")}</button>
    </div>
  `;
}

const RENDERERS = {
  general: sectionGeneral,
  appearance: sectionAppearance,
  connections: sectionConnections,
  history: sectionHistory,
  security: sectionSecurity,
  transfers: sectionTransfers,
  about: sectionAbout,
};

// --- shell + wiring ------------------------------------------------------------

function build() {
  const overlay = document.createElement("div");
  overlay.className = "overlay";
  overlay.addEventListener("mousedown", (e) => {
    if (e.target === overlay) close();
  });

  const dialog = document.createElement("div");
  dialog.className = "dialog dialog-settings";
  dialog.addEventListener("mousedown", (e) => e.stopPropagation());
  dialog.innerHTML = `
    <div class="dialog-header">
      <span class="icon icon--important">${iconMarkup("settings", 16)}</span>
      <span class="dialog-title">${t("settings.title")}</span>
      <button type="button" class="dialog-close" data-action="close">${iconMarkup("close", 13)}</button>
    </div>
    <div class="settings-body">
      <div class="settings-nav" data-role="nav"></div>
      <div class="settings-content" data-role="content"></div>
    </div>
  `;
  dialog.querySelector('[data-action="close"]').addEventListener("click", close);
  overlay.appendChild(dialog);
  document.body.appendChild(overlay);
  return overlay;
}

function renderNav() {
  const nav = overlayEl.querySelector('[data-role="nav"]');
  nav.innerHTML = SECTIONS.map(
    (s) => `
    <button type="button" class="settings-nav-item ${s.id === activeSection ? "active" : ""}" data-section="${s.id}">
      ${iconMarkup(s.icon, 14)}<span>${t(s.labelKey)}</span>
    </button>`,
  ).join("");
  nav.querySelectorAll("[data-section]").forEach((btn) => {
    btn.addEventListener("click", () => switchSection(btn.dataset.section));
  });
}

function renderContent() {
  const content = overlayEl.querySelector('[data-role="content"]');
  content.innerHTML = RENDERERS[activeSection]();
  wireContentActions(content);
}

function wireContentActions(root) {
  root.querySelector("[data-action='toggle-confirm-delete']")?.addEventListener("click", () => {
    setConfirmBeforeDelete(!state.settings.confirmBeforeDelete);
    renderContent();
  });
  root.querySelector("[data-action='use-current-folder']")?.addEventListener("click", async () => {
    await useCurrentFolderAsDefault();
    renderContent();
  });
  root.querySelector("[data-action='reset-folder']")?.addEventListener("click", async () => {
    await resetDefaultFolder();
    renderContent();
  });

  root.querySelectorAll("[data-action='set-theme'] .theme-card").forEach((el) => {
    el.addEventListener("click", () => {
      setTheme(el.dataset.themeId);
      renderContent();
    });
  });

  root.querySelectorAll("[data-panel-toggle]").forEach((el) => {
    el.addEventListener("click", () => {
      togglePanel(el.dataset.panelToggle);
      renderContent();
    });
  });

  root.querySelectorAll("[data-action='delete-site']").forEach((btn) => {
    btn.addEventListener("click", async () => {
      await deleteSite(btn.dataset.id);
      renderContent();
    });
  });
  root.querySelectorAll("[data-action='delete-bookmark']").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const { removeBookmark } = await import("../fsActions.js");
      await removeBookmark(Number(btn.dataset.id));
      renderContent();
    });
  });

  root.querySelector("[data-action='clear-history']")?.addEventListener("click", () => {
    historyClearConfirm = true;
    renderContent();
  });
  root.querySelector("[data-action='cancel-clear-history']")?.addEventListener("click", () => {
    historyClearConfirm = false;
    renderContent();
  });
  root.querySelector("[data-action='confirm-clear-history']")?.addEventListener("click", async () => {
    await clearHistory();
    historyClearConfirm = false;
    renderContent();
  });

  root.querySelectorAll("[data-action='forget-password']").forEach((btn) => {
    btn.addEventListener("click", () => forgetPassword(btn.dataset.id));
  });
  root.querySelector("[data-action='forget-all']")?.addEventListener("click", forgetAllPasswords);

  root.querySelectorAll("[data-action='set-max-concurrent'] .segmented-item").forEach((btn) => {
    btn.addEventListener("click", () => {
      setMaxConcurrent(Number(btn.dataset.value));
      renderContent();
    });
  });
  root.querySelectorAll("[data-action='set-auto-clear'] .segmented-item").forEach((btn) => {
    btn.addEventListener("click", () => {
      setAutoClearCompletedSecs(Number(btn.dataset.value));
      renderContent();
    });
  });

  root.querySelector("[data-action='reveal-db']")?.addEventListener("click", revealDatabase);

  root.querySelector("[data-action='set-language']")?.addEventListener("change", (e) => {
    setLanguage(e.target.value);
    // Re-render everything so all strings update immediately
    renderNav();
    renderContent();
    // After re-render, re-select the correct option
    const sel = overlayEl.querySelector("[data-action='set-language']");
    if (sel) sel.value = state.settings.language;
  });

  const dbPathEl = root.querySelector('[data-role="db-path"]');
  if (dbPathEl) {
    import("../ipc.js").then((ipc) =>
      ipc.appDataDir().then((dir) => {
        dbPathEl.textContent = `${dir}/wherry.db`;
      }),
    );
  }
}

let wasOpen = false;

export function renderSettingsDialog() {
  if (!state.showSettingsDialog) {
    if (overlayEl) {
      overlayEl.remove();
      overlayEl = null;
    }
    wasOpen = false;
    return;
  }
  if (!wasOpen) {
    overlayEl = build();
    renderNav();
    renderContent();
    wasOpen = true;
  }
}

import { state, subscribe, notify, isWelcome } from "./store.js";
import * as ipc from "./ipc.js";
import { initLocal } from "./fsActions.js";
import { loadSettings } from "./settingsActions.js";
import { initPlatform } from "./platform.js";
import { initShortcuts } from "./shortcuts.js";
import { renderWelcome, updateWelcome } from "./components/welcome.js";
import { renderToolbar, updateToolbarIfNeeded } from "./components/toolbar.js";
import { renderSidebar, updateSidebarIfNeeded } from "./components/sidebar.js";
import { renderTabBar } from "./components/tabs.js";
import { renderPanes, updatePanes } from "./components/paneLayout.js";
import { renderQueuePanel, updateQueuePanelIfNeeded } from "./components/queuePanel.js";
import { renderStatusBar, updateStatusBarIfNeeded } from "./components/statusBar.js";
import { renderConnectDialog } from "./components/connectDialog.js";
import { renderHistoryPopup } from "./components/historyPopup.js";
import { renderOpDialogs, closeDialog } from "./components/opDialogs.js";
import { renderSettingsDialog } from "./components/settingsDialog.js";
import { renderSiteManager } from "./components/siteManager.js";

const root = document.getElementById("screen-root");

let currentScreen = null; // 'welcome' | 'main'
let currentLocale = "en";
let welcomeEl = null;
let mainShell = null; // { el, refs }

function buildWelcomeScreen() {
  const el = document.createElement("div");
  el.className = "screen";
  root.appendChild(el);
  renderWelcome(el);
  return el;
}

function buildMainScreen() {
  const el = document.createElement("div");
  el.className = "screen app-shell";
  el.innerHTML = `
    <div class="toolbar" data-role="toolbar"></div>
    <div class="app-body">
      <div class="sidebar" data-role="sidebar"></div>
      <div class="main-content">
        <div class="tabbar" data-role="tabbar"></div>
        <div class="tab-panes" data-role="panes"></div>
        <div class="queue-panel" data-role="queue"></div>
        <div class="status-bar" data-role="statusbar"></div>
      </div>
    </div>
  `;
  root.appendChild(el);

  const refs = {
    toolbar: el.querySelector('[data-role="toolbar"]'),
    sidebar: el.querySelector('[data-role="sidebar"]'),
    tabbar: el.querySelector('[data-role="tabbar"]'),
    panes: el.querySelector('[data-role="panes"]'),
    queue: el.querySelector('[data-role="queue"]'),
    statusbar: el.querySelector('[data-role="statusbar"]'),
  };
  renderToolbar(refs.toolbar);
  renderSidebar(refs.sidebar);
  renderTabBar(refs.tabbar);
  renderPanes(refs.panes);
  renderQueuePanel(refs.queue);
  renderStatusBar(refs.statusbar);
  return { el, refs };
}

function render() {
  const welcome = isWelcome();
  const wantScreen = welcome ? "welcome" : "main";
  const localeChanged = state.settings.language !== currentLocale;

  if (wantScreen !== currentScreen || localeChanged) {
    if (welcomeEl) {
      welcomeEl.remove();
      welcomeEl = null;
    }
    if (mainShell) {
      mainShell.el.remove();
      mainShell = null;
    }
    if (welcome) welcomeEl = buildWelcomeScreen();
    else {
      mainShell = buildMainScreen();
      const { refs } = mainShell;
      applyPanelVisibility(refs);
    }
    currentScreen = wantScreen;
    currentLocale = state.settings.language;
  } else if (welcome) {
    updateWelcome(welcomeEl);
  } else {
    const { refs } = mainShell;
    updateToolbarIfNeeded(refs.toolbar);
    updateSidebarIfNeeded(refs.sidebar);
    renderTabBar(refs.tabbar);
    updatePanes(refs.panes);
    updateQueuePanelIfNeeded(refs.queue);
    updateStatusBarIfNeeded(refs.statusbar);
    applyPanelVisibility(refs);
  }

  renderConnectDialog();
  renderHistoryPopup();
  renderOpDialogs();
  renderSettingsDialog();
  renderSiteManager();
}

// Settings → Appearance toggles — animated via CSS transitions.
function applyPanelVisibility(refs) {
  refs.toolbar.classList.toggle("hidden", !state.settings.showToolbar);
  refs.sidebar.classList.toggle("hidden", !state.settings.showSidebar);
  refs.statusbar.classList.toggle("hidden", !state.settings.showStatusBar);
  refs.queue.classList.toggle("hidden", !state.settings.showQueuePanel);

  // Round main-content corners when adjacent panels are hidden
  const mc = refs.panes.closest(".main-content");
  if (mc) {
    mc.classList.toggle("no-sidebar", !state.settings.showSidebar);
    mc.classList.toggle("no-toolbar", !state.settings.showToolbar);
    mc.classList.toggle("no-status", !state.settings.showStatusBar && !state.settings.showQueuePanel);
  }
}

subscribe(render);

// Esc closes the topmost open overlay: op dialog > connect dialog > settings > history popover.
window.addEventListener("keydown", (e) => {
  if (e.key !== "Escape") return;
  if (state.dialog) {
    closeDialog();
  } else if (state.showSiteManager) {
    state.showSiteManager = false;
    notify();
  } else if (state.showConnectDialog && !state.connectLoading) {
    state.showConnectDialog = false;
    notify();
  } else if (state.showSettingsDialog) {
    state.showSettingsDialog = false;
    notify();
  } else if (state.showHistoryPopup) {
    state.showHistoryPopup = false;
    notify();
  }
});

async function boot() {
  initPlatform(); // stamps data-platform/-desktop on <html>; async part fills in later
  initShortcuts();
  render(); // paint the welcome screen immediately, don't wait on any IPC

  const [history, tasks, sites] = await Promise.allSettled([
    ipc.listHistory(),
    ipc.listTasks(),
    ipc.listSites(),
  ]);
  if (history.status === "fulfilled") state.history = history.value;
  if (tasks.status === "fulfilled") state.queueTasks = tasks.value;
  if (sites.status === "fulfilled") state.sites = sites.value;
  notify();

  await loadSettings();
  notify();

  await initLocal();

  ipc.onTransferProgress(({ id, transferredBytes, speed, etaSecs }) => {
    const task = state.queueTasks.find((tk) => tk.id === id);
    if (task) {
      task.transferredBytes = transferredBytes;
      task.speed = speed;
      task.etaSecs = etaSecs;
      notify();
    }
  });

  ipc.onTransferStateChanged(({ id, state: newState }) => {
    const task = state.queueTasks.find((tk) => tk.id === id);
    if (task) {
      task.state = newState;
      notify();
    }
  });
}

boot();

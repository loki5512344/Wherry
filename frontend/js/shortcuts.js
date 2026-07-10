// Global keyboard shortcuts, following each platform's conventions:
// the primary modifier is ⌘ on macOS and Ctrl elsewhere (see platform.js),
// and mac-specific idioms (⌘⌫ delete, Enter rename, ⌘↑ go up) are added on
// top of the cross-platform set (F2, F5, Delete, Alt+↑).
//
// Esc is NOT handled here — main.js owns the "close topmost overlay" logic.
import { state, notify, activeTab, isWelcome } from "./store.js";
import { modProp, isMac } from "./platform.js";
import { openNew } from "./components/connectDialog.js";
import { openSettings } from "./components/settingsDialog.js";
import { openMkdir, openDelete, openRename } from "./components/opDialogs.js";
import {
  refreshLocal,
  refreshRemote,
  navigateLocal,
  navigateRemote,
  parentPath,
  closeTab,
} from "./fsActions.js";

function isTyping(e) {
  const el = e.target;
  return el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);
}

function overlayOpen() {
  return Boolean(state.dialog || state.showConnectDialog || state.showSettingsDialog);
}

function currentSelection() {
  if (state.activePane === "remote") return activeTab()?.remoteSelected ?? [];
  return state.local.selected;
}

function withOpTarget(fn) {
  state.opTarget = state.activePane;
  fn();
}

export function initShortcuts() {
  window.addEventListener("keydown", (e) => {
    const mod = e[modProp];
    const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;

    // App-level — work everywhere, including the welcome screen.
    if (mod && !e.shiftKey && !e.altKey && !overlayOpen()) {
      if (key === "n") {
        e.preventDefault();
        openNew();
        return;
      }
      if (key === ",") {
        e.preventDefault();
        openSettings();
        return;
      }
    }

    if (isTyping(e) || overlayOpen() || isWelcome()) return;

    const tab = activeTab();
    const remote = state.activePane === "remote";

    // Mod+1…9 — switch tabs (1 = Local, 2… = remote tabs in order).
    if (mod && key >= "1" && key <= "9") {
      e.preventDefault();
      const n = Number(key);
      if (n === 1) {
        state.activeTabId = null;
        state.activePane = "local";
        notify();
      } else if (state.tabs[n - 2]) {
        state.activeTabId = state.tabs[n - 2].id;
        state.activePane = "remote";
        notify();
      }
      return;
    }

    // Mod+W — close the displayed remote tab (Local can't be closed).
    if (mod && key === "w" && state.activeTabId) {
      e.preventDefault();
      closeTab(state.activeTabId);
      return;
    }

    // Mod+Shift+N — new folder in the active pane.
    if (mod && e.shiftKey && key === "n") {
      e.preventDefault();
      withOpTarget(openMkdir);
      return;
    }

    // Mod+R / F5 — refresh the active pane.
    if ((mod && key === "r") || key === "F5") {
      e.preventDefault();
      if (remote && tab) refreshRemote(tab);
      else refreshLocal();
      return;
    }

    // Go up a directory: Alt+↑ everywhere, ⌘↑ on mac, bare Backspace.
    const goUp =
      (e.altKey && key === "ArrowUp") ||
      (isMac && mod && key === "ArrowUp") ||
      (key === "Backspace" && !mod && !e.altKey);
    if (goUp) {
      e.preventDefault();
      if (remote && tab) navigateRemote(tab, parentPath(tab.remotePath));
      else navigateLocal(parentPath(state.local.path));
      return;
    }

    // Rename: F2, plus Enter on mac (Finder convention).
    if (key === "F2" || (isMac && key === "Enter" && !mod)) {
      const names = currentSelection();
      if (names.length === 1) {
        e.preventDefault();
        withOpTarget(() => openRename(names[0]));
      }
      return;
    }

    // Delete: Delete key everywhere, ⌘⌫ on mac.
    if (key === "Delete" || (isMac && mod && key === "Backspace")) {
      const names = currentSelection();
      if (names.length > 0) {
        e.preventDefault();
        withOpTarget(() => openDelete(names));
      }
      return;
    }
  });
}

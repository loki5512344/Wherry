// Settings persistence + actions -- mirrors src/ui/app/settings/{prefs,
// connections}.rs. Prefs are stored through the generic get_pref/set_pref
// string k/v commands (booleans/numbers round-tripped as strings); panel
// visibility stays session-only, matching the old AppState (never had a
// KEY_ constant / db-backed default of its own).
import { state, notify } from "./store.js";
import * as ipc from "./ipc.js";
import { setLocale, availableLocales } from "./i18n.js";

const KEY_CONFIRM_DELETE = "confirm_before_delete";
const KEY_DEFAULT_LOCAL_FOLDER = "default_local_folder";
const KEY_AUTO_CLEAR_SECS = "auto_clear_completed_secs";
const KEY_MAX_CONCURRENT = "max_concurrent_transfers";
const KEY_THEME = "ui_theme";
const KEY_LANGUAGE = "language";

export async function loadSettings() {
  const [confirmDelete, defaultFolder, autoClear, maxConcurrent, theme, lang] = await Promise.all([
    ipc.getPref(KEY_CONFIRM_DELETE),
    ipc.getPref(KEY_DEFAULT_LOCAL_FOLDER),
    ipc.getPref(KEY_AUTO_CLEAR_SECS),
    ipc.getPref(KEY_MAX_CONCURRENT),
    ipc.getPref(KEY_THEME),
    ipc.getPref(KEY_LANGUAGE),
  ]);
  if (confirmDelete !== null) state.settings.confirmBeforeDelete = confirmDelete === "true";
  if (defaultFolder) state.settings.defaultLocalFolder = defaultFolder;
  if (autoClear !== null) state.settings.autoClearCompletedSecs = Number(autoClear);
  if (maxConcurrent !== null) {
    state.settings.maxConcurrent = Number(maxConcurrent);
    await ipc.setMaxConcurrent(state.settings.maxConcurrent);
  }
  if (theme) {
    state.settings.theme = theme;
    applyTheme(theme);
  }
  if (lang && availableLocales().includes(lang)) {
    state.settings.language = lang;
    setLocale(lang);
  }
}

const themeTransitionMs = 300;
let themeTimeout = null;

const DARK_THEMES = new Set(["ember", "midnight", "forest", "ash"]);

export function applyTheme(themeId) {
  const html = document.documentElement;
  // Add a class to enable CSS transitions during theme switch
  html.classList.add("theme-transitioning");
  html.setAttribute("data-theme", themeId);
  html.style.colorScheme = DARK_THEMES.has(themeId) ? "dark" : "light";
  clearTimeout(themeTimeout);
  themeTimeout = setTimeout(() => {
    html.classList.remove("theme-transitioning");
  }, themeTransitionMs);
}

export function setTheme(themeId) {
  state.settings.theme = themeId;
  applyTheme(themeId);
  ipc.setPref(KEY_THEME, themeId);
  notify();
}

export function setLanguage(locale) {
  state.settings.language = locale;
  setLocale(locale);
  ipc.setPref(KEY_LANGUAGE, locale);
  notify();
}

export function setConfirmBeforeDelete(value) {
  state.settings.confirmBeforeDelete = value;
  ipc.setPref(KEY_CONFIRM_DELETE, String(value));
  notify();
}

export async function useCurrentFolderAsDefault() {
  state.settings.defaultLocalFolder = state.local.path;
  await ipc.setPref(KEY_DEFAULT_LOCAL_FOLDER, state.local.path);
  notify();
}

export async function resetDefaultFolder() {
  state.settings.defaultLocalFolder = "";
  await ipc.setPref(KEY_DEFAULT_LOCAL_FOLDER, "");
  notify();
}

export function setMaxConcurrent(n) {
  state.settings.maxConcurrent = n;
  ipc.setMaxConcurrent(n);
  ipc.setPref(KEY_MAX_CONCURRENT, String(n));
  notify();
}

export function setAutoClearCompletedSecs(secs) {
  state.settings.autoClearCompletedSecs = secs;
  ipc.setPref(KEY_AUTO_CLEAR_SECS, String(secs));
  notify();
}

export function togglePanel(key) {
  state.settings[key] = !state.settings[key];
  notify();
}

export async function clearHistory() {
  await ipc.clearHistory();
  state.history = [];
  state.statusMessage = "History cleared";
  notify();
}

export async function deleteSite(id) {
  const site = state.sites.find((s) => s.id === id);
  await ipc.deleteSite(id);
  await ipc.deletePassword(id);
  state.sites = state.sites.filter((s) => s.id !== id);
  if (site) state.statusMessage = `Removed ${site.name}`;
  notify();
}

export async function forgetPassword(siteId) {
  await ipc.deletePassword(siteId);
  state.statusMessage = "Password forgotten";
  notify();
}

export async function forgetAllPasswords() {
  await Promise.all([
    ...state.sites.map((s) => ipc.deletePassword(s.id)),
    ...state.history.map((h) => ipc.deletePassword(h.connId)),
  ]);
  state.statusMessage = "All saved passwords forgotten";
  notify();
}

export async function revealDatabase() {
  const dir = await ipc.appDataDir();
  await ipc.localOpen(dir);
}

// Platform detection + per-OS conventions, applied once at boot.
//
// The webview UA is enough to tell mac/windows/linux apart, but on Linux the
// desktop environment (KDE vs GNOME vs Hyprland/sway) is only visible to the
// backend process — initPlatform() asks it via the platform_info command and
// stamps everything on <html> as data attributes so CSS can adapt:
//   <html data-platform="linux" data-desktop="kde" data-session="wayland">
import * as ipc from "./ipc.js";

const ua = navigator.userAgent;

export const isMac = /Mac/.test(ua);
export const isWindows = /Windows/.test(ua);
export const isLinux = !isMac && !isWindows;
export const platform = isMac ? "mac" : isWindows ? "windows" : "linux";

/** Which KeyboardEvent property is the primary shortcut modifier here. */
export const modProp = isMac ? "metaKey" : "ctrlKey";

/** Human-readable shortcut, following each platform's convention:
 * mac "⌘N" / "⇧⌘N", elsewhere "Ctrl+N" / "Ctrl+Shift+N". */
export function shortcutLabel(key, { shift = false, alt = false } = {}) {
  if (isMac) return `${alt ? "⌥" : ""}${shift ? "⇧" : ""}⌘${key}`;
  const parts = ["Ctrl"];
  if (shift) parts.push("Shift");
  if (alt) parts.push("Alt");
  parts.push(key);
  return parts.join("+");
}


/** True once initPlatform() resolves; Linux tiling compositors (Hyprland,
 * sway, …) get data-tiling="true" so CSS can drop window-chrome affordances
 * that assume floating windows. */
const TILING_DESKTOPS = ["hyprland", "sway", "i3", "river", "niri"];

export async function initPlatform() {
  const root = document.documentElement;
  root.dataset.platform = platform;

  try {
    const info = await ipc.platformInfo();
    if (info.desktop) {
      root.dataset.desktop = info.desktop;
      if (TILING_DESKTOPS.some((d) => info.desktop.includes(d))) {
        root.dataset.tiling = "true";
      }
    }
    if (info.session) root.dataset.session = info.session;
  } catch {
    /* backend command unavailable (e.g. plain-browser dev) — UA data is enough */
  }
}

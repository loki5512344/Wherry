import { state } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup, QUICK_ACCESS_ICONS } from "../icons.js";
import { showContextMenu } from "./contextMenu.js";
import { navigateLocal, removeBookmark } from "../fsActions.js";

let lastSig = null;

function item(icon, label, active, onClick) {
  const row = document.createElement("div");
  row.className = `row ${active ? "active" : ""}`;
  row.innerHTML = `<span class="icon">${iconMarkup(icon, 15)}</span><span style="margin-left:8px;font-size:12.5px">${label}</span>`;
  row.addEventListener("click", onClick);
  return row;
}

export function renderSidebar(container) {
  container.innerHTML = "";

  const logo = document.createElement("div");
  logo.className = "sidebar-logo";
  logo.innerHTML = `<span class="sidebar-logo-badge">W</span><span class="sidebar-logo-text">Wherry</span>`;
  container.appendChild(logo);

  const qaTitle = document.createElement("div");
  qaTitle.className = "sidebar-section-title";
  qaTitle.textContent = t("panels.localLabel");
  container.appendChild(qaTitle);

  for (const qa of state.sidebar.quickAccess) {
    if (!qa.path) continue;
    const active = state.local.path === qa.path;
    container.appendChild(
      item(QUICK_ACCESS_ICONS[qa.name] ?? "folder", qa.name, active, () => navigateLocal(qa.path)),
    );
  }

  const divider1 = document.createElement("div");
  divider1.className = "sidebar-divider";
  container.appendChild(divider1);

  const drivesTitle = document.createElement("div");
  drivesTitle.className = "sidebar-section-title";
  drivesTitle.textContent = t("panels.sidebarDrives");
  container.appendChild(drivesTitle);

  container.appendChild(
    item("ssd", t("panels.sidebarRoot"), state.local.path === "/", () => navigateLocal("/")),
  );

  if (state.sidebar.bookmarks.length > 0) {
    const divider2 = document.createElement("div");
    divider2.className = "sidebar-divider";
    container.appendChild(divider2);

    const bmTitle = document.createElement("div");
    bmTitle.className = "sidebar-section-title";
    bmTitle.textContent = t("panels.sidebarBookmarks");
    container.appendChild(bmTitle);

    for (const bm of state.sidebar.bookmarks) {
      const row = item("star", bm.name, state.local.path === bm.path, () => navigateLocal(bm.path));
      row.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        showContextMenu(e.clientX, e.clientY, [
          { label: t("panels.sidebarRemoveBookmark"), icon: "trash", danger: true, onClick: () => removeBookmark(bm.id) },
        ]);
      });
      container.appendChild(row);
    }
  }

  const spacer = document.createElement("div");
  spacer.className = "sidebar-spacer";
  container.appendChild(spacer);

  const footer = document.createElement("div");
  footer.className = "sidebar-footer";
  footer.textContent = "Wherry v0.1";
  container.appendChild(footer);

  lastSig = computeSig();
}

function computeSig() {
  return JSON.stringify([
    state.local.path,
    state.sidebar.bookmarks.map((b) => `${b.id}:${b.path}`),
    state.sidebar.quickAccess.map((q) => q.path),
  ]);
}

export function updateSidebarIfNeeded(container) {
  if (computeSig() === lastSig) return;
  renderSidebar(container);
}

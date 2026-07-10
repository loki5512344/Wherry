import { state } from "../store.js";
import { t } from "../i18n.js";
import { iconMarkup } from "../icons.js";
import { renderHistoryRows } from "./historyList.js";
import { openNew } from "./connectDialog.js";
import { openSettings } from "./settingsDialog.js";
import { openSiteManagerNew } from "./siteManager.js";
import { connectFromSite, PROTOCOL_LABELS } from "../connectActions.js";
import { escapeHtml } from "../dom.js";
import { shortcutLabel } from "../platform.js";

export function renderWelcome(container) {
  container.innerHTML = `
    <div class="welcome">
      <div class="welcome-hero">
        <div class="welcome-logo">${iconMarkup("folder", 34)}</div>
        <div class="welcome-title">Wherry</div>
        <div class="welcome-subtitle">${t("welcome.subtitle")}</div>
      </div>
      <div class="welcome-actions">
        <button type="button" class="btn btn-accent welcome-cta" data-action="new-connection">
          ${iconMarkup("addCircle", 15)}<span>${t("welcome.newConnection")}</span>
          <kbd class="kbd kbd-on-accent">${shortcutLabel("N")}</kbd>
        </button>
        <button type="button" class="btn btn-ghost" data-action="open-settings">
          ${iconMarkup("settings", 15)}<span>${t("common.settings")}</span>
          <kbd class="kbd">${shortcutLabel(",")}</kbd>
        </button>
      </div>
      <div class="welcome-status" data-role="status"></div>
      <div class="welcome-columns">
        <div class="welcome-card" data-role="sites" style="display:none">
          <div class="welcome-card-title">${iconMarkup("star", 12)}<span>${t("welcome.savedSites")}</span></div>
          <div data-role="sites-list"></div>
          <div style="margin-top:6px;text-align:center">
            <button type="button" class="btn btn-ghost" data-action="manage-sites" style="font-size:11px;height:28px">${t("siteManager.manageSites")}</button>
          </div>
        </div>
        <div class="welcome-card" data-role="recent" style="display:none">
          <div class="welcome-card-title">${iconMarkup("history", 12)}<span>${t("welcome.recentConnections")}</span></div>
          <div data-role="recent-list"></div>
        </div>
      </div>
    </div>
  `;

  container.querySelector('[data-action="new-connection"]').addEventListener("click", openNew);
  container.querySelector('[data-action="open-settings"]').addEventListener("click", openSettings);
  container.querySelector('[data-action="manage-sites"]')?.addEventListener("click", openSiteManagerNew);

  updateWelcome(container);
}

function renderSiteRows(container, sites) {
  container.innerHTML = "";
  for (const site of sites) {
    const row = document.createElement("div");
    row.className = "recent-row";
    row.innerHTML = `
      <span class="icon icon--important">${iconMarkup("serverSquare", 13)}</span>
      <span class="recent-label">${escapeHtml(site.name)}</span>
      <span class="proto-badge">${PROTOCOL_LABELS[site.protocol] ?? site.protocol}</span>
    `;
    row.addEventListener("click", () => connectFromSite(site));
    container.appendChild(row);
  }
}

export function updateWelcome(container) {
  const statusRole = container.querySelector('[data-role="status"]');
  if (!statusRole) return;

  if (state.connectLoading) {
    statusRole.innerHTML = '<span class="spinner"></span>';
  } else if (state.connectError) {
    statusRole.innerHTML = `<span class="text-red" style="font-size:11.5px">${escapeHtml(state.connectError)}</span>`;
  } else {
    statusRole.innerHTML = "";
  }

  const sitesWrap = container.querySelector('[data-role="sites"]');
  const sitesList = container.querySelector('[data-role="sites-list"]');
  const sites = state.sites.slice(0, 6);
  sitesWrap.style.display = sites.length ? "" : "none";
  if (sites.length) renderSiteRows(sitesList, sites);

  const recentWrap = container.querySelector('[data-role="recent"]');
  const recentList = container.querySelector('[data-role="recent-list"]');
  const rows = state.history.slice(0, 6);
  recentWrap.style.display = rows.length ? "" : "none";
  if (rows.length) renderHistoryRows(recentList, rows);
}

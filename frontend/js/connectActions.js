// Connection flow shared by the New Connection dialog, the welcome screen's
// recent-connections list and the toolbar History popover. Mirrors
// src/ui/dialogs/connection/{actions,history}.rs.
import { state, notify } from "./store.js";
import * as ipc from "./ipc.js";

export const DEFAULT_PORTS = { sftp: 22, ftp: 21, ftps: 990 };
export const PROTOCOL_LABELS = { sftp: "SFTP", ftp: "FTP", ftps: "FTPS" };

function uuid() {
  return crypto.randomUUID();
}

async function refreshHistory() {
  try {
    state.history = await ipc.listHistory();
  } catch {
    /* history is best-effort UI sugar, ignore failures */
  }
}

/** Runs a connect attempt for already-built ConnectionParams; used by the
 * dialog's Connect button and by history reconnect/edit. */
export async function spawnConnect(params) {
  state.connectError = "";
  state.connectLoading = true;
  notify();
  try {
    const connectionId = await ipc.connect(params);
    const entries = await ipc.remoteList(connectionId, "/");
    // The backend reuses the same connectionId for repeat connections to the
    // same host/port/username (see canonical_id in src/commands.rs::connect),
    // so reconnecting to an already-open server must update its existing tab
    // rather than push a second one with the same id.
    const existing = state.tabs.find((t) => t.id === connectionId);
    if (existing) {
      existing.label = params.label;
      existing.params = params;
      existing.status = "connected";
      existing.remotePath = "/";
      existing.remoteEntries = entries;
      existing.remoteSelected = [];
      existing.remoteSelectAnchor = null;
      existing.loading = false;
    } else {
      state.tabs.push({
        id: connectionId,
        label: params.label,
        params,
        status: "connected",
        remotePath: "/",
        remoteEntries: entries,
        remoteSelected: [],
        remoteSelectAnchor: null,
        loading: false,
        sort: { col: "name", dir: "asc" },
      });
    }
    state.activeTabId = connectionId;
    state.lastRemoteTabId = connectionId;
    state.activePane = "remote";
    // Replace first pane with new connection (click = switch, not split)
    state.layout.panes[0].tabId = connectionId;
    state.connectLoading = false;
    state.showConnectDialog = false;
    state.statusMessage = `Connected to ${params.host}`;
    notify();
    await refreshHistory();
    notify();
    return { ok: true };
  } catch (err) {
    state.connectLoading = false;
    state.connectError = String(err);
    state.statusMessage = `Connection failed: ${err}`;
    notify();
    return { ok: false, error: String(err) };
  }
}

/** Validates raw dialog form fields and turns them into ConnectionParams. */
export function buildParamsFromForm(form) {
  const host = form.host.trim();
  const username = form.username.trim();
  const portStr = form.port.trim();

  if (!host) return { ok: false, error: "connect.errorHost" };
  if (!username) return { ok: false, error: "connect.errorUser" };
  let port = DEFAULT_PORTS[form.protocol];
  if (portStr) {
    const parsed = Number(portStr);
    if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
      return { ok: false, error: "connect.errorPort" };
    }
    port = parsed;
  }

  const protocolLabel = PROTOCOL_LABELS[form.protocol];
  const params = {
    id: uuid(),
    label: form.label.trim() || `${host} (${protocolLabel})`,
    protocol: form.protocol,
    host,
    port,
    username,
    password: form.password ? form.password : null,
    keyPath: form.keyPath ? form.keyPath : null,
  };
  return { ok: true, params };
}

/** Click on a history row — reconnect straight away, no dialog. */
export function reconnectFromHistory(entry) {
  const params = {
    id: entry.connId,
    label: `${entry.username}@${entry.host}`,
    protocol: entry.protocol,
    host: entry.host,
    port: entry.port,
    username: entry.username,
    password: null, // history doesn't store passwords
    keyPath: entry.keyPath ?? null,
  };
  return spawnConnect(params);
}

/** Click on a saved-site row (welcome screen). */
export function connectFromSite(site) {
  const params = {
    id: site.id,
    label: site.name,
    protocol: site.protocol,
    host: site.host,
    port: site.port,
    username: site.username,
    password: site.password ?? null,
    keyPath: site.keyPath ?? null,
  };
  return spawnConnect(params);
}

/** "Save" on a history row — turns a one-off connection into a permanent Site. */
export async function saveHistoryAsSite(entry) {
  const site = {
    id: entry.connId,
    name: `${entry.username}@${entry.host}`,
    protocol: entry.protocol,
    host: entry.host,
    port: entry.port,
    username: entry.username,
    keyPath: entry.keyPath ?? null,
    folder: null,
    note: null,
  };
  await ipc.saveSite(site);
  state.statusMessage = `Saved ${site.name} as a site`;
  notify();
}

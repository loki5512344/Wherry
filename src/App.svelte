<script lang="ts">
  import "./app.css";
  import Tabs from "./lib/components/Tabs/Tabs.svelte";
  import Toolbar from "./lib/components/Toolbar/Toolbar.svelte";
  import LocalPane from "./lib/components/Panes/LocalPane.svelte";
  import RemotePane from "./lib/components/Panes/RemotePane.svelte";
  import TransferQueue from "./lib/components/Queue/TransferQueue.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import { activeConnection } from "./lib/stores/connection.svelte";
  import NewConnectionDialog from "./lib/components/Dialogs/NewConnectionDialog.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { FileEntry } from "./lib/types";

  let queueOpen = $state(true);
  let dialogOpen = $state(false);

  // Состояния для операций
  let localPath = $state("C:\\Users\\user");
  let remotePath = $state("/home/user");
  let localSelected = $state<FileEntry | null>(null);
  let remoteSelected = $state<FileEntry | null>(null);

  async function upload() {
    const conn = $activeConnection;
    if (!conn || conn.status !== "connected" || !localSelected) return;
    const target = remotePath.endsWith("/") ? remotePath + localSelected.name : remotePath + "/" + localSelected.name;
    try {
      await invoke("upload", {
        connectionId: conn.id,
        localPath: localSelected.path,
        remotePath: target,
      });
    } catch (e) {
      console.error("upload failed", e);
    }
  }

  async function download() {
    const conn = $activeConnection;
    if (!conn || conn.status !== "connected" || !remoteSelected) return;
    const target = localPath.endsWith("/") ? localPath + remoteSelected.name : localPath + "/" + remoteSelected.name;
    try {
      await invoke("download", {
        connectionId: conn.id,
        remotePath: remoteSelected.path,
        localPath: target,
      });
    } catch (e) {
      console.error("download failed", e);
    }
  }

  async function mkdir() {
    const conn = $activeConnection;
    if (!conn || conn.status !== "connected") return;
    const name = prompt("Имя новой папки:");
    if (!name) return;
    const target = remotePath.endsWith("/") ? remotePath + name : remotePath + "/" + name;
    try {
      await invoke("remote_mkdir", { connectionId: conn.id, path: target });
    } catch (e) {
      console.error("mkdir failed", e);
    }
  }

  async function deleteFile(entry: FileEntry) {
    const conn = $activeConnection;
    if (!conn || conn.status !== "connected") return;
    if (!confirm(`Удалить ${entry.kind === "dir" ? "папку" : "файл"} "${entry.name}"?`)) return;
    try {
      await invoke("remote_delete", { connectionId: conn.id, path: entry.path });
    } catch (e) {
      console.error("delete failed", e);
    }
  }

  async function rename(entry: FileEntry) {
    const conn = $activeConnection;
    if (!conn || conn.status !== "connected") return;
    const newName = prompt("Новое имя:", entry.name);
    if (!newName || newName === entry.name) return;
    const newPath = entry.path.replace(/[^/]+$/, newName);
    try {
      await invoke("remote_rename", { connectionId: conn.id, from: entry.path, to: newPath });
    } catch (e) {
      console.error("rename failed", e);
    }
  }

  function onLocalPathChange(p: string) { localPath = p; }
  function onRemotePathChange(p: string) { remotePath = p; }
</script>

<div class="app-shell">
  <!-- Вкладки соединений -->
  <Tabs />

  <!-- Тулбар -->
  <Toolbar
    onNewConnection={() => dialogOpen = true}
    {upload}
    {download}
    {mkdir}
  />

  <!-- Диалог нового соединения -->
  <NewConnectionDialog bind:open={dialogOpen} />

  <!-- Основная область: левая + правая панели -->
  <div class="panes">
    <LocalPane
      bind:selected={localSelected}
      onPathChange={onLocalPathChange}
    />
    <div class="pane-divider"></div>
    <RemotePane
      bind:selected={remoteSelected}
      onPathChange={onRemotePathChange}
      onDelete={(e) => deleteFile(e)}
      onRename={(e) => rename(e)}
    />
  </div>

  <!-- Очередь передач -->
  <TransferQueue bind:open={queueOpen} />

  <!-- Статусбар -->
  <StatusBar />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--bg-0);
  }

  .panes {
    display: flex;
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }

  .pane-divider {
    width: 1px;
    background: var(--border);
    flex-shrink: 0;
  }
</style>

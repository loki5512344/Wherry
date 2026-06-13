<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import FilePane from "./FilePane.svelte";
  import { activeConnection } from "../../stores/connection.svelte";
  import type { FileEntry } from "../../types";

  let {
    selected = $bindable<FileEntry | null>(null),
    onPathChange = (p: string) => {},
    onDelete,
    onRename,
  }: {
    selected?: FileEntry | null;
    onPathChange?: (p: string) => void;
    onDelete?: (entry: FileEntry) => void;
    onRename?: (entry: FileEntry) => void;
  } = $props();

  let path = $state("/home/user");
  let entries = $state<FileEntry[]>([]);
  let loading = $state(false);
  let remoteSelected = $state<FileEntry | null>(null);

  $effect(() => { selected = remoteSelected; });

  async function loadDir(p: string) {
    const conn = $activeConnection;
    if (!conn || conn.status !== "connected") return;
    loading = true;
    try {
      entries = await invoke<FileEntry[]>("list_remote", {
        connectionId: conn.id,
        path: p,
      });
      path = p;
      onPathChange(p);
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if ($activeConnection?.status === "connected") {
      loadDir(path);
    }
  });
</script>

<FilePane
  title="Удалённые файлы"
  icon="🖥"
  bind:path
  {entries}
  {loading}
  bind:selected={remoteSelected}
  {onDelete}
  {onRename}
  onNavigate={(e) => loadDir(e.path)}
  onPathChange={(p) => loadDir(p)}
/>

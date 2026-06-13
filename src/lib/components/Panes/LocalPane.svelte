<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import FilePane from "./FilePane.svelte";
  import type { FileEntry } from "../../types";

  let {
    selected = $bindable<FileEntry | null>(null),
    onPathChange = (p: string) => {},
  }: {
    selected?: FileEntry | null;
    onPathChange?: (p: string) => void;
  } = $props();

  let path = $state("C:\\Users\\user");
  let entries = $state<FileEntry[]>([]);
  let loading = $state(false);
  let localSelected = $state<FileEntry | null>(null);

  // Sync local selected with parent
  $effect(() => { selected = localSelected; });

  async function loadDir(p: string) {
    loading = true;
    try {
      entries = await invoke<FileEntry[]>("list_local", { path: p });
      path = p;
      onPathChange(p);
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadDir(path);
  });
</script>

<FilePane
  title="Локальные файлы"
  icon="💻"
  bind:path
  {entries}
  {loading}
  bind:selected={localSelected}
  onNavigate={(e) => loadDir(e.path)}
  onPathChange={(p) => loadDir(p)}
/>

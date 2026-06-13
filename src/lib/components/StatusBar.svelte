<script lang="ts">
  import { activeConnection } from "../stores/connection.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  let conn = $derived($activeConnection);
  let totalSpeed = $state(0);
  let unlisten: (() => void) | null = null;
  const speeds = new Map<string, number>();

  onMount(() => {
    listen<{ taskId: string; speed?: number | null }>("transfer-progress", (event) => {
      const s = event.payload.speed ?? 0;
      if (event.payload.taskId) {
        speeds.set(event.payload.taskId, s);
      }
      totalSpeed = [...speeds.values()].reduce((a, b) => a + b, 0);
    }).then((unsub) => {
      unlisten = unsub;
    });
  });

  onDestroy(() => {
    unlisten?.();
  });

  function formatSpeed(bps: number): string {
    if (bps <= 0) return "0 B/s";
    if (bps < 1024) return `${bps} B/s`;
    if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
    return `${(bps / 1024 / 1024).toFixed(1)} MB/s`;
  }
</script>

<div class="statusbar">
  <span class="status-dot" class:connected={conn?.status === "connected"}></span>
  <span class="status-text">
    {#if conn?.status === "connected"}
      Подключено к {conn.label} ({conn.protocol.toUpperCase()})
    {:else}
      Нет соединения
    {/if}
  </span>

  <div class="statusbar-spacer"></div>

  {#if totalSpeed > 0}
    <span class="speed-indicator">📊 {formatSpeed(totalSpeed)}</span>
  {/if}
</div>

<style>
  .statusbar {
    display: flex;
    align-items: center;
    gap: 6px;
    height: var(--statusbar-h);
    padding: 0 12px;
    background: var(--bg-1);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-muted);
    flex-shrink: 0;
  }
  .status-dot.connected { background: var(--green); }

  .statusbar-spacer { flex: 1; }

  .speed-indicator { font-family: var(--font-mono); font-size: 11px; }
</style>

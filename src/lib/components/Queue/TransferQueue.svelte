<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { TransferTask } from "../../types";
  import { onMount, onDestroy } from "svelte";

  let { open = $bindable(true) }: { open: boolean } = $props();

  let tasks = $state<TransferTask[]>([]);
  let unlisten: (() => void) | null = null;

  onMount(() => {
    invoke<TransferTask[]>("get_queue").then(t => { tasks = t; }).catch(e => console.error("get_queue failed", e));

    listen<{
      taskId: string;
      state: string;
      transferredBytes: number;
      totalBytes: number;
      speed?: number | null;
      etaSecs?: number | null;
      error?: string | null;
    }>("transfer-progress", (event) => {
      const { taskId, state, transferredBytes, totalBytes, speed, etaSecs, error } = event.payload;
      tasks = tasks.map(t => {
        if (t.id !== taskId) return t;
        const newState = parseState(state);
        if (newState === "completed" || newState === "failed" || newState === "cancelled") {
          return { ...t, state: newState, transferredBytes, speed: speed ?? null, etaSecs: etaSecs ?? null };
        }
        return { ...t, state: newState, transferredBytes, speed: speed ?? null, etaSecs: etaSecs ?? null };
      });
    }).then(unsub => { unlisten = unsub; });
  });

  onDestroy(() => {
    unlisten?.();
  });

  function parseState(s: string): TransferTask["state"] {
    if (s.startsWith("failed")) return "failed";
    if (s.startsWith("retrying")) return "queued";
    return s as TransferTask["state"];
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function formatSpeed(bps?: number | null): string {
    if (!bps) return "—";
    if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
    return `${(bps / 1024 / 1024).toFixed(1)} MB/s`;
  }

  function formatEta(secs?: number | null): string {
    if (!secs) return "—";
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    return [h, m, s].map(v => String(v).padStart(2, "0")).join(":");
  }

  function pct(task: TransferTask): number {
    if (!task.totalBytes) return 0;
    return Math.round(task.transferredBytes / task.totalBytes * 100);
  }

  function stateLabel(state: string): string {
    return ({ queued: "В очереди", running: "Передача", paused: "Пауза",
              completed: "Завершено", cancelled: "Отменено",
              failed: "Ошибка" } as any)[state] ?? state;
  }

  async function pause(id: string) {
    await invoke("pause_task", { taskId: id });
    tasks = tasks.map(t => t.id === id ? { ...t, state: "paused" as const } : t);
  }

  async function cancel(id: string) {
    await invoke("cancel_task", { taskId: id });
    tasks = tasks.filter(t => t.id !== id);
  }

  let totalSpeed = $derived(
    tasks.filter(t => t.state === "running").reduce((s, t) => s + (t.speed ?? 0), 0)
  );

  let activeTasks = $derived(tasks);
</script>

<div class="queue-wrap" class:collapsed={!open}>
  <div class="queue-header">
    <span class="queue-title">Очередь передач ({tasks.length})</span>
    <button class="collapse-btn" onclick={() => open = !open} title={open ? "Свернуть" : "Развернуть"}>
      {open ? "▼" : "▲"}
    </button>
    <button class="sort-btn" title="Сортировка">⇅</button>
  </div>

  {#if open}
    <div class="queue-body">
      <div class="queue-cols">
        <span>Имя</span>
        <span>Операция</span>
        <span>Источник</span>
        <span>Назначение</span>
        <span>Размер</span>
        <span>Прогресс</span>
        <span>Скорость</span>
        <span>Осталось</span>
        <span>Статус</span>
      </div>

      {#each activeTasks as task}
        <div class="queue-row" class:completed={task.state === "completed"}>
          <span class="q-name">
            <span class="q-file-icon">📄</span>
            {task.fileName}
          </span>
          <span class="q-op">{task.kind === "upload" ? "Загрузка" : "Скачать"}</span>
          <span class="q-path" title={task.localPath}>{task.localPath}</span>
          <span class="q-path" title={task.remotePath}>{task.remotePath}</span>
          <span class="q-size">{formatSize(task.totalBytes)}</span>
          <span class="q-progress">
            <div class="progress-bar">
              <div
                class="progress-fill"
                class:green={task.state === "completed"}
                class:blue={task.state === "running"}
                style="width: {pct(task)}%"
              ></div>
              <span class="progress-label">{pct(task)}%</span>
            </div>
          </span>
          <span class="q-speed">{formatSpeed(task.speed)}</span>
          <span class="q-eta">{formatEta(task.etaSecs)}</span>
          <span class="q-state">
            {#if task.state === "completed"}
              <span class="badge green">✓ {stateLabel(task.state)}</span>
            {:else if task.state === "running"}
              <span class="badge blue">{stateLabel(task.state)}</span>
              <button class="icon-btn" onclick={() => pause(task.id)} title="Пауза">⏸</button>
              <button class="icon-btn red" onclick={() => cancel(task.id)} title="Отмена">✕</button>
            {:else}
              <span class="badge">{stateLabel(task.state)}</span>
            {/if}
          </span>
        </div>
      {:else}
        <div class="queue-empty">Нет активных передач</div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .queue-wrap {
    flex-shrink: 0;
    background: var(--bg-1);
    border-top: 1px solid var(--border);
    max-height: var(--queue-h);
    display: flex;
    flex-direction: column;
    transition: max-height 0.2s;
  }
  .queue-wrap.collapsed { max-height: 36px; }

  .queue-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    height: 36px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .queue-title { font-size: 12px; font-weight: 500; color: var(--text-secondary); flex: 1; }

  .collapse-btn, .sort-btn {
    width: 24px; height: 24px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 11px;
  }
  .collapse-btn:hover, .sort-btn:hover { background: var(--bg-3); color: var(--text-primary); }

  .queue-body { overflow-y: auto; flex: 1; }

  .queue-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .queue-cols {
    display: grid;
    grid-template-columns: 140px 70px 200px 200px 70px 120px 80px 80px 140px;
    padding: 3px 12px;
    font-size: 11px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    position: sticky; top: 0;
    background: var(--bg-1);
  }

  .queue-row {
    display: grid;
    grid-template-columns: 140px 70px 200px 200px 70px 120px 80px 80px 140px;
    padding: 5px 12px;
    font-size: 12px;
    align-items: center;
    border-bottom: 1px solid var(--border);
  }
  .queue-row:hover { background: var(--bg-hover); }
  .queue-row.completed { opacity: 0.7; }

  .q-name { display: flex; align-items: center; gap: 5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .q-file-icon { font-size: 12px; flex-shrink: 0; }
  .q-op { color: var(--text-secondary); }
  .q-path { color: var(--text-muted); font-family: var(--font-mono); font-size: 10px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .q-size { color: var(--text-secondary); }
  .q-speed, .q-eta { color: var(--text-secondary); font-family: var(--font-mono); font-size: 11px; }

  .progress-bar {
    height: 16px;
    background: var(--bg-3);
    border-radius: 3px;
    position: relative;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s;
    background: var(--accent-dim);
  }
  .progress-fill.green { background: #2e7d32; }
  .progress-fill.blue { background: var(--accent-dim); }
  .progress-label {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .q-state { display: flex; align-items: center; gap: 4px; }

  .badge {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--bg-3);
    color: var(--text-secondary);
  }
  .badge.green { background: #1b3a1c; color: #66bb6a; }
  .badge.blue { background: #1a2e4a; color: var(--accent); }

  .icon-btn {
    width: 20px; height: 20px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 11px;
  }
  .icon-btn:hover { background: var(--bg-3); color: var(--text-primary); }
  .icon-btn.red:hover { color: var(--red); }
</style>

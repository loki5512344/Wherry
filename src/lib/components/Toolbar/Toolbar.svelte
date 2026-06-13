<script lang="ts">
  import { activeConnection } from "../../stores/connection.svelte";

  let {
    onNewConnection,
    upload,
    download,
    mkdir,
  }: {
    onNewConnection?: () => void;
    upload?: () => void;
    download?: () => void;
    mkdir?: () => void;
  } = $props();

  let connected = $derived($activeConnection?.status === "connected");
</script>

<div class="toolbar">
  <button class="new-conn-btn" onclick={onNewConnection}>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="10"/>
      <line x1="12" y1="8" x2="12" y2="16"/>
      <line x1="8" y1="12" x2="16" y2="12"/>
    </svg>
    Новое соединение
  </button>

  <div class="separator"></div>

  <button class="toolbar-btn" onclick={upload} disabled={!connected} title="Загрузить на сервер">
    <span class="toolbar-icon">↑</span>
    <span class="toolbar-label">Загрузить</span>
  </button>

  <button class="toolbar-btn" onclick={download} disabled={!connected} title="Скачать с сервера">
    <span class="toolbar-icon">↓</span>
    <span class="toolbar-label">Скачать</span>
  </button>

  <button class="toolbar-btn" onclick={mkdir} disabled={!connected} title="Создать папку">
    <span class="toolbar-icon">□</span>
    <span class="toolbar-label">Создать папку</span>
  </button>

  <div class="toolbar-spacer"></div>

  <div class="view-switcher">
    <button class="view-btn active" title="Детали">⊞</button>
    <button class="view-btn" title="Список">☰</button>
    <button class="view-btn" title="Иконки">⊟</button>
  </div>

  <button class="settings-btn" title="Настройки">⚙</button>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    height: var(--toolbar-h);
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    padding: 0 8px;
    gap: 2px;
    flex-shrink: 0;
  }

  .new-conn-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 5px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 12px;
    flex-shrink: 0;
    margin-right: 4px;
  }
  .new-conn-btn:hover { background: var(--bg-3); }

  .separator {
    width: 1px;
    height: 24px;
    background: var(--border);
    margin: 0 6px;
    flex-shrink: 0;
  }

  .toolbar-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 8px;
    border-radius: 4px;
    color: var(--text-secondary);
    font-size: 12px;
    white-space: nowrap;
  }
  .toolbar-btn:hover:not(:disabled) { background: var(--bg-hover); color: var(--text-primary); }
  .toolbar-btn:disabled { opacity: 0.4; cursor: default; }

  .toolbar-icon { font-size: 13px; }

  .toolbar-spacer { flex: 1; }

  .view-switcher {
    display: flex;
    gap: 2px;
    margin-right: 8px;
  }

  .view-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 15px;
  }
  .view-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .view-btn.active { background: var(--bg-3); color: var(--text-primary); }

  .settings-btn {
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 16px;
  }
  .settings-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
</style>

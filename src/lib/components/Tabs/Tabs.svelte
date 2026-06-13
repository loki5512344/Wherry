<script lang="ts">
  import { connections, activeId } from "../../stores/connection.svelte";

  function addConnection() {
    // TODO: открыть диалог нового соединения
  }
</script>

<div class="tabs-bar">
  <button class="menu-btn" title="Меню">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <line x1="3" y1="6" x2="21" y2="6"/>
      <line x1="3" y1="12" x2="21" y2="12"/>
      <line x1="3" y1="18" x2="21" y2="18"/>
    </svg>
  </button>

  <div class="tabs">
    {#each $connections as conn}
      <button
        class="tab"
        class:active={$activeId === conn.id}
        onclick={() => activeId.set(conn.id)}
      >
        <span class="tab-dot" class:connected={conn.status === "connected"}></span>
        <span class="tab-label">{conn.label} ({conn.protocol.toUpperCase()})</span>
        <span class="tab-close" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); connections.remove(conn.id); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); connections.remove(conn.id); } }}>×</span>
      </button>
    {/each}

    <button class="tab-add" onclick={addConnection} title="Новое соединение">+</button>
  </div>
</div>

<style>
  .tabs-bar {
    display: flex;
    align-items: center;
    height: var(--tab-h);
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 4px;
    padding: 0 4px;
  }

  .menu-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .menu-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    overflow-x: auto;
    flex: 1;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px 0 10px;
    height: 32px;
    border-radius: 4px;
    color: var(--text-secondary);
    font-size: 12px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tab:hover { background: var(--bg-hover); color: var(--text-primary); }
  .tab.active { background: var(--bg-2); color: var(--text-primary); }

  .tab-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-muted);
    flex-shrink: 0;
  }
  .tab-dot.connected { background: var(--green); }

  .tab-close {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    font-size: 14px;
    color: var(--text-muted);
    line-height: 1;
  }
  .tab-close:hover { background: var(--bg-3); color: var(--text-primary); }

  .tab-add {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 18px;
    flex-shrink: 0;
  }
  .tab-add:hover { background: var(--bg-hover); color: var(--text-primary); }
</style>

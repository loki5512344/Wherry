<script lang="ts">
  import type { FileEntry } from "../../types";

  let {
    title,
    icon,
    path = $bindable(""),
    entries = [],
    loading = false,
    selected = $bindable<string | null>(null),
    selectedEntry = $bindable<FileEntry | null>(null),
    onNavigate,
    onPathChange,
    onUpload,
    onDownload,
    onMkdir,
    onDelete,
    onRename,
  }: {
    title: string;
    icon: string;
    path: string;
    entries: FileEntry[];
    loading?: boolean;
    selected?: string | null;
    selectedEntry?: FileEntry | null;
    onNavigate?: (entry: FileEntry) => void;
    onPathChange?: (path: string) => void;
    onUpload?: () => void;
    onDownload?: () => void;
    onMkdir?: () => void;
    onDelete?: (entry: FileEntry) => void;
    onRename?: (entry: FileEntry) => void;
  } = $props();

  function formatSize(bytes?: number): string {
    if (bytes == null) return "";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function formatDate(ts?: number): string {
    if (!ts) return "";
    return new Date(ts * 1000).toLocaleString("ru-RU", {
      day: "2-digit", month: "2-digit", year: "numeric",
      hour: "2-digit", minute: "2-digit",
    });
  }

  function getIcon(entry: FileEntry): string {
    if (entry.kind === "dir") return "📁";
    const ext = entry.name.split(".").pop()?.toLowerCase();
    const icons: Record<string, string> = {
      rs: "🦀", ts: "📘", js: "📜", json: "📋",
      md: "📝", toml: "⚙", png: "🖼", jpg: "🖼",
      zip: "📦", gz: "📦", tar: "📦", php: "🐘",
      html: "🌐", css: "🎨", txt: "📄",
    };
    return icons[ext ?? ""] ?? "📄";
  }

  function handleClick(entry: FileEntry) {
    selected = entry.name;
    selectedEntry = entry;
  }

  function handleDblClick(entry: FileEntry) {
    if (entry.kind === "dir") {
      onNavigate?.(entry);
    }
  }

  function goUp() {
    const parent = path.includes("/")
      ? path.replace(/\/[^/]+\/?$/, "") || "/"
      : path.replace(/\\[^\\]+\\?$/, "") || "C:\\";
    onPathChange?.(parent);
  }
</script>

<div class="file-pane">
  <!-- Заголовок панели с кнопками операций -->
  <div class="pane-header">
    <span class="pane-icon">{icon}</span>
    <span class="pane-title">{title}</span>
    <div class="pane-actions">
      {#if onUpload}
        <button class="action-btn" onclick={onUpload} title="Загрузить">↑</button>
      {/if}
      {#if onDownload}
        <button class="action-btn" onclick={onDownload} title="Скачать">↓</button>
      {/if}
      {#if onMkdir}
        <button class="action-btn" onclick={onMkdir} title="Создать папку">+📁</button>
      {/if}
      {#if onDelete}
        <button class="action-btn red" onclick={() => selectedEntry && onDelete?.(selectedEntry)} title="Удалить" disabled={!selectedEntry}>🗑</button>
      {/if}
      {#if onRename}
        <button class="action-btn" onclick={() => selectedEntry && onRename?.(selectedEntry)} title="Переименовать" disabled={!selectedEntry}>✎</button>
      {/if}
    </div>
  </div>

  <!-- Адресная строка -->
  <div class="address-bar">
    <button class="nav-btn" onclick={goUp} title="Вверх">←</button>
    <input
      class="path-input"
      value={path}
      onchange={(e) => onPathChange?.((e.target as HTMLInputElement).value)}
      spellcheck="false"
    />
    <button class="nav-btn" title="Вперёд">→</button>
  </div>

  <!-- Заголовки колонок -->
  <div class="col-headers">
    <span class="col-name">Имя</span>
    <span class="col-size">Размер</span>
    <span class="col-type">Тип</span>
    <span class="col-date">Изменён ↕</span>
  </div>

  <!-- Список файлов -->
  <div class="file-list">
    {#if loading}
      <div class="empty-state">Загрузка...</div>
    {:else if entries.length === 0}
      <div class="empty-state">Пусто</div>
    {:else}
      {#each entries as entry}
        <button
          class="file-row"
          class:selected={selected === entry.name}
          onclick={() => handleClick(entry)}
          ondblclick={() => handleDblClick(entry)}
        >
          <span class="col-name">
            <span class="file-icon">{getIcon(entry)}</span>
            {entry.name}
          </span>
          <span class="col-size">{formatSize(entry.size)}</span>
          <span class="col-type">{entry.kind === "dir" ? "Папка" : entry.name.split(".").pop()?.toUpperCase() ?? "Файл"}</span>
          <span class="col-date">{formatDate(entry.modified)}</span>
        </button>
      {/each}
    {/if}
  </div>

  <!-- Футер -->
  <div class="pane-footer">
    {entries.length} элементов
  </div>
</div>

<style>
  .file-pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    background: var(--bg-0);
  }

  .pane-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .pane-icon { font-size: 14px; }
  .pane-title { font-weight: 500; color: var(--text-primary); flex-shrink: 0; }

  .pane-actions {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }

  .action-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 12px;
  }
  .action-btn:hover:not(:disabled) { background: var(--bg-3); color: var(--text-primary); }
  .action-btn.red:hover:not(:disabled) { color: var(--red); }
  .action-btn:disabled { opacity: 0.3; cursor: default; }

  .address-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .nav-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 14px;
    flex-shrink: 0;
  }
  .nav-btn:hover { background: var(--bg-3); color: var(--text-primary); }

  .path-input {
    flex: 1;
    height: 26px;
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--bg-2);
    border-color: var(--border);
  }

  .col-headers {
    display: grid;
    grid-template-columns: 1fr 80px 80px 140px;
    padding: 4px 12px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
    user-select: none;
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .file-row {
    display: grid;
    grid-template-columns: 1fr 80px 80px 140px;
    width: 100%;
    padding: 3px 12px;
    text-align: left;
    color: var(--text-primary);
    font-size: 12px;
    border-radius: 0;
    align-items: center;
  }
  .file-row:hover { background: var(--bg-hover); }
  .file-row.selected { background: var(--bg-selected); }

  .col-name {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .file-icon { font-size: 13px; flex-shrink: 0; }

  .col-size, .col-type { color: var(--text-secondary); font-size: 11px; }
  .col-date { color: var(--text-secondary); font-size: 11px; font-family: var(--font-mono); }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 13px;
  }

  .pane-footer {
    padding: 4px 12px;
    background: var(--bg-1);
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
  }
</style>

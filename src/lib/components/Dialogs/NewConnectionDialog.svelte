<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { connections, activeId } from "../../stores/connection.svelte";

  let { open = $bindable(false) }: { open: boolean } = $props();

  let protocol = $state<"sftp" | "ftp" | "ftps">("sftp");
  let host = $state("");
  let port = $state(22);
  let username = $state("");
  let password = $state("");
  let keyPath = $state("");
  let label = $state("");
  let useKey = $state(false);
  let error = $state("");
  let connecting = $state(false);

  $effect(() => {
    port = protocol === "sftp" ? 22 : 21;
  });

  async function connect() {
    if (!host || !username) { error = "Хост и логин обязательны"; return; }
    error = "";
    connecting = true;

    const id = crypto.randomUUID();
    const connLabel = label || host;

    try {
      await invoke("connect", {
        params: {
          id,
          label: connLabel,
          protocol,
          host,
          port,
          username,
          password: useKey ? null : password,
          keyPath: useKey ? keyPath : null,
        }
      });

      connections.add({ id, label: connLabel, protocol, host, status: "connected" });
      activeId.set(id);
      open = false;
      reset();
    } catch (e: any) {
      error = e?.toString() ?? "Ошибка соединения";
    } finally {
      connecting = false;
    }
  }

  function reset() {
    host = ""; port = 22; username = ""; password = "";
    keyPath = ""; label = ""; useKey = false; error = "";
  }

  function close() { open = false; reset(); }
</script>

{#if open}
  <div class="overlay" onclick={close}>
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <div class="dialog-header">
        <span>Новое соединение</span>
        <button class="close-btn" onclick={close}>✕</button>
      </div>

      <div class="dialog-body">
        <!-- Протокол -->
        <div class="field">
          <label>Протокол</label>
          <div class="radio-group">
            {#each ["sftp", "ftp", "ftps"] as p}
              <label class="radio-label">
                <input type="radio" bind:group={protocol} value={p} />
                {p.toUpperCase()}
              </label>
            {/each}
          </div>
        </div>

        <div class="row">
          <div class="field grow">
            <label>Хост</label>
            <input bind:value={host} placeholder="192.168.1.1" spellcheck="false" />
          </div>
          <div class="field w80">
            <label>Порт</label>
            <input type="number" bind:value={port} min="1" max="65535" />
          </div>
        </div>

        <div class="field">
          <label>Метка (необязательно)</label>
          <input bind:value={label} placeholder={host || "my-server"} spellcheck="false" />
        </div>

        <div class="field">
          <label>Пользователь</label>
          <input bind:value={username} placeholder="root" spellcheck="false" autocomplete="off" />
        </div>

        {#if protocol === "sftp"}
          <div class="field">
            <label class="checkbox-label">
              <input type="checkbox" bind:checked={useKey} />
              Использовать SSH ключ
            </label>
          </div>
        {/if}

        {#if useKey && protocol === "sftp"}
          <div class="field">
            <label>Путь к ключу</label>
            <input bind:value={keyPath} placeholder="~/.ssh/id_rsa" spellcheck="false" />
          </div>
        {:else}
          <div class="field">
            <label>Пароль</label>
            <input type="password" bind:value={password} autocomplete="off" />
          </div>
        {/if}

        {#if error}
          <div class="error">{error}</div>
        {/if}
      </div>

      <div class="dialog-footer">
        <button class="btn-cancel" onclick={close}>Отмена</button>
        <button class="btn-connect" onclick={connect} disabled={connecting}>
          {connecting ? "Подключение..." : "Подключиться"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 420px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 60px rgba(0,0,0,0.5);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    font-weight: 500;
  }

  .close-btn {
    width: 24px; height: 24px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
  }
  .close-btn:hover { background: var(--bg-3); color: var(--text-primary); }

  .dialog-body {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field label { font-size: 11px; color: var(--text-secondary); }
  .field input { width: 100%; }

  .row { display: flex; gap: 10px; align-items: flex-end; }
  .grow { flex: 1; }
  .w80 { width: 80px; flex-shrink: 0; }

  .radio-group { display: flex; gap: 12px; }
  .radio-label {
    display: flex; align-items: center; gap: 5px;
    font-size: 12px; color: var(--text-primary);
    cursor: pointer;
  }

  .checkbox-label {
    display: flex; align-items: center; gap: 6px;
    font-size: 12px; color: var(--text-primary);
    cursor: pointer;
    flex-direction: row !important;
  }

  .error {
    background: #3a1a1a;
    border: 1px solid #6a2a2a;
    border-radius: 4px;
    padding: 8px 10px;
    font-size: 12px;
    color: var(--red);
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }

  .btn-cancel {
    padding: 6px 14px;
    border-radius: 5px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
  }
  .btn-cancel:hover { background: var(--bg-3); color: var(--text-primary); }

  .btn-connect {
    padding: 6px 16px;
    border-radius: 5px;
    background: var(--accent-dim);
    border: 1px solid var(--accent);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 500;
  }
  .btn-connect:hover:not(:disabled) { background: var(--accent); }
  .btn-connect:disabled { opacity: 0.5; cursor: not-allowed; }
</style>

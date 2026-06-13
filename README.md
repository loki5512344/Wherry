# LoFlum

Современный FTP/SFTP клиент на Tauri 2 + Rust + Svelte 5.

## Стек

- **Tauri 2** — шелл
- **Rust** — протоколы, файловая система, очередь передач
- **Svelte 5** — UI
- **tokio** — async runtime

## Протоколы

| Версия | Протоколы |
|--------|-----------|
| v1 | SFTP, FTP, FTPS |
| v2 | S3, WebDAV |
| v3 | Google Drive, Dropbox |

## Разработка

```bash
# Зависимости
bun install

# Dev режим
bun run tauri dev

# Сборка
bun run tauri build
```

## Зависимости системы

- Rust + cargo
- Bun (или npm/pnpm)
- libssh2 (`pacman -S libssh2` на Arch)
- webkit2gtk (`pacman -S webkit2gtk-4.1` на Arch)

## Архитектура

```
src-tauri/src/
├── domain/       — модели данных (Connection, FileEntry, TransferTask, Site)
├── protocols/    — RemoteFs trait + sftp.rs, ftp.rs
├── transfer/     — очередь, воркеры, прогресс
├── storage/      — SQLite (сайты), keychain (пароли)
├── fs/           — локальная ФС + реестр удалённых соединений
└── commands/     — tauri commands (вызовы из фронта)
```

## TODO

- [ ] Chunked upload/download с прогресс-событиями
- [x] FTP/FTPS — FTP реализован (FTPS — stub)
- [ ] Site Manager UI
- [x] Диалог нового соединения
- [x] Transfer Queue с воркером и прогресс-событиями
- [ ] Drag & drop
- [ ] Горячие клавиши

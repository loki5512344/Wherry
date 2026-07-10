# Wherry

![Rust](https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white)
![egui](https://img.shields.io/badge/egui-FF5733?style=flat-square&logo=egui&logoColor=white)
![FTP](https://img.shields.io/badge/FTP/SFTP-0078D4?style=flat-square&logo=files&logoColor=white)
![status](https://img.shields.io/badge/status-development-yellow?style=flat-square)
![license](https://img.shields.io/badge/license-GPLv3-blue?style=flat-square)

FTP/SFTP клиент написанный на Rust с UI на egui/eframe.

## Стек

- **egui/eframe** — нативный GUI (чистый Rust, без веба)
- **Rust** — протоколы, файловая система, очередь передач
- **tokio** — async runtime

## Протоколы

| Версия | Протоколы         |
|--------|-------------------|
| v1     | SFTP, FTP, FTPS   |
| v2     | S3, WebDAV        |
| v3     | Google Drive, Dropbox |

## Разработка

```bash
cargo run
```

## Архитектура

```
src/
├── domain/       -- модели данных (Connection, FileEntry, TransferTask, Site)
├── protocols/    -- RemoteFs trait + sftp.rs, ftp.rs
├── transfer/     -- очередь, воркеры, прогресс
├── storage/      -- SQLite (сайты), keychain (пароли)
├── fs/           -- локальная ФС + реестр удалённых соединений
├── ui/           -- egui панели (в разработке)
└── main.rs       -- точка входа eframe
```


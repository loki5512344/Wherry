use crate::domain::file_entry::{EntryKind, FileEntry};
use anyhow::Result;
use std::fs;

pub fn list(path: &str) -> Result<Vec<FileEntry>> {
    let mut entries: Vec<FileEntry> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| {
            let meta = e.metadata().ok();
            let kind = meta
                .as_ref()
                .map(|m| {
                    if m.is_dir() {
                        EntryKind::Dir
                    } else if m.is_symlink() {
                        EntryKind::Symlink
                    } else {
                        EntryKind::File
                    }
                })
                .unwrap_or(EntryKind::File);

            let size = meta
                .as_ref()
                .and_then(|m| if m.is_file() { Some(m.len()) } else { None });

            let modified = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);

            FileEntry {
                name: e.file_name().to_string_lossy().to_string(),
                path: e.path().to_string_lossy().to_string(),
                kind,
                size,
                modified,
                permissions: None,
            }
        })
        .collect();

    // папки сначала, потом файлы, по имени
    entries.sort_by(|a, b| match (&a.kind, &b.kind) {
        (EntryKind::Dir, EntryKind::Dir) => a.name.cmp(&b.name),
        (EntryKind::Dir, _) => std::cmp::Ordering::Less,
        (_, EntryKind::Dir) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(entries)
}

pub fn home_dir() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

pub fn rename(from: &str, to: &str) -> Result<()> {
    fs::rename(from, to)?;
    Ok(())
}

pub fn delete(path: &str) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn mkdir(path: &str) -> Result<()> {
    fs::create_dir(path)?;
    Ok(())
}

/// Открывает файл/папку в системном приложении по умолчанию.
pub fn open(path: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(path).spawn()?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(path).spawn()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/C", "start", "", path])
        .spawn()?;
    Ok(())
}

/// Перемещает файл/папку в целевую директорию (для drag & drop).
pub fn move_into(src_path: &str, dest_dir: &str) -> Result<()> {
    let name = std::path::Path::new(src_path)
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("invalid source path"))?;
    let dest = std::path::Path::new(dest_dir).join(name);
    fs::rename(src_path, dest)?;
    Ok(())
}

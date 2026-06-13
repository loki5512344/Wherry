use anyhow::Result;
use std::fs;
use crate::domain::file_entry::{FileEntry, EntryKind};

pub fn list(path: &str) -> Result<Vec<FileEntry>> {
    let mut entries: Vec<FileEntry> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| {
            let meta = e.metadata().ok();
            let kind = meta.as_ref().map(|m| {
                if m.is_dir() { EntryKind::Dir }
                else if m.is_symlink() { EntryKind::Symlink }
                else { EntryKind::File }
            }).unwrap_or(EntryKind::File);

            let size = meta.as_ref().and_then(|m| {
                if m.is_file() { Some(m.len()) } else { None }
            });

            let modified = meta.as_ref()
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
    entries.sort_by(|a, b| {
        match (&a.kind, &b.kind) {
            (EntryKind::Dir, EntryKind::Dir) => a.name.cmp(&b.name),
            (EntryKind::Dir, _) => std::cmp::Ordering::Less,
            (_, EntryKind::Dir) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(entries)
}

pub fn home_dir() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

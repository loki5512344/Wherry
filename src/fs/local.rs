use crate::domain::{EntryKind, FileEntry};
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("wherry-test-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_list_empty_dir() {
        let dir = test_dir();
        let entries = list(&dir.to_string_lossy()).unwrap();
        assert!(entries.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_mkdir_and_list() {
        let dir = test_dir();
        let sub = dir.join("subdir");
        mkdir(&sub.to_string_lossy()).unwrap();

        let entries = list(&dir.to_string_lossy()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "subdir");
        assert_eq!(entries[0].kind, EntryKind::Dir);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rename_file() {
        let dir = test_dir();
        let old = dir.join("old.txt");
        let new = dir.join("new.txt");
        std::fs::write(&old, "hello").unwrap();

        rename(&old.to_string_lossy(), &new.to_string_lossy()).unwrap();
        assert!(!old.exists());
        assert!(new.exists());
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "hello");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_delete_file() {
        let dir = test_dir();
        let file = dir.join("delete_me.txt");
        std::fs::write(&file, "content").unwrap();

        delete(&file.to_string_lossy()).unwrap();
        assert!(!file.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_delete_dir() {
        let dir = test_dir();
        let sub = dir.join("subdir");
        std::fs::create_dir(&sub).unwrap();

        delete(&sub.to_string_lossy()).unwrap();
        assert!(!sub.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_home_dir() {
        let home = home_dir();
        assert!(!home.is_empty());
    }

    #[test]
    fn test_move_into() {
        let dir = test_dir();
        let src = dir.join("source.txt");
        let dest_dir = dir.join("dest");
        std::fs::create_dir(&dest_dir).unwrap();
        std::fs::write(&src, "move me").unwrap();

        move_into(&src.to_string_lossy(), &dest_dir.to_string_lossy()).unwrap();
        assert!(!src.exists());
        assert!(dest_dir.join("source.txt").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_sorting() {
        let dir = test_dir();
        std::fs::write(dir.join("a.txt"), "").unwrap();
        std::fs::create_dir(dir.join("b_dir")).unwrap();
        std::fs::write(dir.join("c.txt"), "").unwrap();

        let entries = list(&dir.to_string_lossy()).unwrap();
        // dirs first, then files alphabetically
        assert_eq!(entries[0].name, "b_dir");
        assert_eq!(entries[0].kind, EntryKind::Dir);
        assert_eq!(entries[1].name, "a.txt");
        assert_eq!(entries[2].name, "c.txt");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

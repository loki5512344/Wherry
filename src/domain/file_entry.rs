use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub modified: Option<i64>, // unix timestamp
    pub permissions: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_kind_serde() {
        assert_eq!(serde_json::to_string(&EntryKind::File).unwrap(), "\"file\"");
        assert_eq!(serde_json::to_string(&EntryKind::Dir).unwrap(), "\"dir\"");
        assert_eq!(
            serde_json::to_string(&EntryKind::Symlink).unwrap(),
            "\"symlink\""
        );
    }

    #[test]
    fn test_file_entry() {
        let entry = FileEntry {
            name: "test.txt".into(),
            path: "/tmp/test.txt".into(),
            kind: EntryKind::File,
            size: Some(1024),
            modified: Some(1234567890),
            permissions: Some("rw-r--r--".into()),
        };
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.size, Some(1024));
    }

    #[test]
    fn test_file_entry_serde() {
        let entry = FileEntry {
            name: "f".into(),
            path: "/f".into(),
            kind: EntryKind::Dir,
            size: None,
            modified: None,
            permissions: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: FileEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "f");
        assert_eq!(deserialized.kind, EntryKind::Dir);
    }
}

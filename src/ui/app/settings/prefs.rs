//! Ключи настроек в БД, значения по умолчанию и helpers сохранения —
//! общие для всех разделов Settings.
use super::FileManagerApp;

pub(super) const KEY_CONFIRM_DELETE: &str = "confirm_before_delete";
pub(super) const KEY_DEFAULT_LOCAL_FOLDER: &str = "default_local_folder";
pub(super) const KEY_AUTO_CLEAR_SECS: &str = "auto_clear_completed_secs";
pub(super) const KEY_MAX_CONCURRENT: &str = "max_concurrent_transfers";
pub(super) const KEY_LANGUAGE: &str = "language";
pub(super) const DEFAULT_MAX_CONCURRENT: u32 = 2;

/// Настройки, загруженные из БД при старте (см. `FileManagerApp::new`).
pub(in crate::ui::app) struct Prefs {
    pub confirm_before_delete: bool,
    pub default_local_folder: String,
    pub auto_clear_completed_secs: u32,
    pub max_concurrent: u32,
    /// `None` — язык ещё ни разу не выбирался явно, стартуем от локали ОС.
    pub language: Option<crate::i18n::Lang>,
}

pub(in crate::ui::app) fn load(
    db: &std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
) -> Prefs {
    let Ok(conn) = db.lock() else {
        return Prefs {
            confirm_before_delete: true,
            default_local_folder: String::new(),
            auto_clear_completed_secs: 0,
            max_concurrent: DEFAULT_MAX_CONCURRENT,
            language: None,
        };
    };
    Prefs {
        confirm_before_delete: crate::storage::db::get_bool(&conn, KEY_CONFIRM_DELETE, true),
        default_local_folder: crate::storage::db::get_setting(&conn, KEY_DEFAULT_LOCAL_FOLDER)
            .unwrap_or_default(),
        auto_clear_completed_secs: crate::storage::db::get_u32(&conn, KEY_AUTO_CLEAR_SECS, 0),
        max_concurrent: crate::storage::db::get_u32(
            &conn,
            KEY_MAX_CONCURRENT,
            DEFAULT_MAX_CONCURRENT,
        ),
        language: crate::storage::db::get_setting(&conn, KEY_LANGUAGE)
            .and_then(|c| crate::i18n::Lang::from_code(&c)),
    }
}

impl FileManagerApp {
    pub(super) fn save_pref_bool(&self, key: &str, value: bool) {
        if let Ok(conn) = self.db.lock() {
            crate::storage::db::set_bool(&conn, key, value);
        }
    }

    pub(super) fn save_pref_u32(&self, key: &str, value: u32) {
        if let Ok(conn) = self.db.lock() {
            crate::storage::db::set_u32(&conn, key, value);
        }
    }

    pub(super) fn save_pref_str(&self, key: &str, value: &str) {
        if let Ok(conn) = self.db.lock() {
            let _ = crate::storage::db::set_setting(&conn, key, value);
        }
    }
}

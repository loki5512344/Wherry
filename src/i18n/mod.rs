//! Локализация интерфейса. Иммедиэйт-мод UI перерисовывается целиком каждый
//! кадр на главном потоке, поэтому вместо протаскивания языка через сигнатуры
//! всех функций рендера — один глобальный атомик с текущим языком плюс
//! функция [`t`] для поиска перевода по ключу. Переводы лежат в `strings/*`,
//! по одному файлу на область интерфейса, и собираются в одну таблицу лениво.
mod strings;

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// Порядок вариантов — он же порядок колонок в каждом `[&str; N]` из `strings/*`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Ru,
    Es,
    Fr,
    De,
    It,
    Pt,
    Pl,
    Zh,
    Ja,
    Ko,
    Tr,
}

pub const COUNT: usize = 12;

impl Lang {
    pub const ALL: [Lang; COUNT] = [
        Lang::En,
        Lang::Ru,
        Lang::Es,
        Lang::Fr,
        Lang::De,
        Lang::It,
        Lang::Pt,
        Lang::Pl,
        Lang::Zh,
        Lang::Ja,
        Lang::Ko,
        Lang::Tr,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    fn from_index(i: u8) -> Lang {
        Self::ALL.get(i as usize).copied().unwrap_or(Lang::En)
    }

    /// Код языка — используется для хранения выбора в БД (Settings) и для
    /// определения стартового языка по локали ОС.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ru => "ru",
            Lang::Es => "es",
            Lang::Fr => "fr",
            Lang::De => "de",
            Lang::It => "it",
            Lang::Pt => "pt",
            Lang::Pl => "pl",
            Lang::Zh => "zh",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
            Lang::Tr => "tr",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        Self::ALL.into_iter().find(|l| l.code() == code)
    }

    /// Название языка на нём самом — так его узнают в списке выбора, даже не
    /// зная текущего языка интерфейса.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Ru => "Русский",
            Lang::Es => "Español",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
            Lang::It => "Italiano",
            Lang::Pt => "Português",
            Lang::Pl => "Polski",
            Lang::Zh => "简体中文",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
            Lang::Tr => "Türkçe",
        }
    }

    /// Есть ли у языка глифы вне Latin/Cyrillic, для которых системный SF Pro
    /// не подходит — тогда `visuals::system_fonts` подмешивает CJK-шрифт.
    pub fn needs_cjk_font(self) -> bool {
        matches!(self, Lang::Zh | Lang::Ja | Lang::Ko)
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0); // Lang::En

pub fn current() -> Lang {
    Lang::from_index(CURRENT.load(Ordering::Relaxed))
}

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang.index() as u8, Ordering::Relaxed);
}

/// Язык ОС на старте (до первой загрузки из БД) — грубое сопоставление по
/// первым двум буквам `LANG`/локали; неизвестное всегда откатывается на English.
pub fn detect_system_lang() -> Lang {
    let raw = std::env::var("LANG").unwrap_or_default();
    let code = raw.split(['_', '.', '-']).next().unwrap_or("");
    Lang::from_code(code).unwrap_or(Lang::En)
}

type Table = HashMap<&'static str, [&'static str; COUNT]>;
static TABLE: OnceLock<Table> = OnceLock::new();

fn table() -> &'static Table {
    TABLE.get_or_init(|| {
        let mut m = HashMap::with_capacity(1024);
        for &(key, values) in strings::all_entries() {
            if m.insert(key, values).is_some() {
                debug_assert!(false, "duplicate i18n key: {key}");
            }
        }
        m
    })
}

/// Перевод по ключу на текущий язык. Отсутствующий ключ возвращает сам ключ —
/// это специально заметно в UI (и в debug-сборке паникует), чтобы пропущенный
/// перевод не потерялся молча.
pub fn t(key: &'static str) -> &'static str {
    match table().get(key) {
        Some(values) => values[current().index()],
        None => {
            debug_assert!(false, "missing i18n key: {key}");
            key
        }
    }
}

/// [`t`] с подстановкой значений вместо плейсхолдеров вида `{name}` — для
/// строк с динамическим содержимым (имя файла, хост и т.п.), которое не может
/// быть частью статической таблицы переводов.
pub fn tf(key: &'static str, pairs: &[(&str, &str)]) -> String {
    let mut s = t(key).to_string();
    for (placeholder, value) in pairs {
        s = s.replace(placeholder, value);
    }
    s
}

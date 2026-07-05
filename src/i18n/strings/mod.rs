//! Таблицы переводов, разбитые по областям интерфейса — так проще найти и
//! проверить перевод конкретного экрана, чем в одном файле на тысячу строк.
//! Порядок колонок в каждом `[&str; 12]`: en, ru, es, fr, de, it, pt, pl, zh, ja, ko, tr.
mod common;
mod dialogs;
mod menu;
mod panels;
mod settings;
mod toolbar;
mod welcome;

type Entry = (&'static str, [&'static str; super::COUNT]);

pub fn all_entries() -> impl Iterator<Item = &'static Entry> {
    common::ENTRIES
        .iter()
        .chain(toolbar::ENTRIES.iter())
        .chain(panels::ENTRIES.iter())
        .chain(settings::ENTRIES.iter())
        .chain(dialogs::ENTRIES.iter())
        .chain(welcome::ENTRIES.iter())
        .chain(menu::ENTRIES.iter())
}

//! Диалог «New Connection» и связанные действия (подключение, история → Site).
mod actions;
mod form;
mod history;
mod view;

pub use actions::spawn_connect;
pub use history::{edit_history_entry, reconnect_from_history, save_history_as_site};
pub use view::render;

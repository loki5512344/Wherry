//! Нативное меню в верхней панели экрана (только macOS).
//! На остальных платформах кнопка настроек живёт в тулбаре — см. panels/toolbar.rs.
use crate::ui::state::AppState;

#[cfg(target_os = "macos")]
mod ids {
    pub const SETTINGS: &str = "loflum-settings";
    pub const NEW_CONNECTION: &str = "loflum-new-connection";
}

/// Строит нативное меню macOS: App-меню (с "Settings…"), File, Edit, Window.
/// Вызывать один раз при старте, на главном потоке, уже после инициализации NSApplication.
#[cfg(target_os = "macos")]
pub fn setup_native_menu() {
    use muda::{
        Menu, PredefinedMenuItem, Submenu,
        accelerator::{Accelerator, CMD_OR_CTRL, Code},
    };

    let menu = Menu::new();

    let app_menu = Submenu::new("LoFlum", true);
    let _ = app_menu.append_items(&[
        &PredefinedMenuItem::about(Some("About LoFlum"), None),
        &PredefinedMenuItem::separator(),
        &muda::MenuItem::with_id(
            ids::SETTINGS,
            "Settings…",
            true,
            Some(Accelerator::new(Some(CMD_OR_CTRL), Code::Comma)),
        ),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::hide(None),
        &PredefinedMenuItem::hide_others(None),
        &PredefinedMenuItem::show_all(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::quit(None),
    ]);

    let file_menu = Submenu::new("File", true);
    let _ = file_menu.append_items(&[
        &muda::MenuItem::with_id(
            ids::NEW_CONNECTION,
            "New Connection…",
            true,
            Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyN)),
        ),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::close_window(None),
    ]);

    let edit_menu = Submenu::new("Edit", true);
    let _ = edit_menu.append_items(&[
        &PredefinedMenuItem::undo(None),
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::select_all(None),
    ]);

    let window_menu = Submenu::new("Window", true);
    let _ = window_menu.append_items(&[
        &PredefinedMenuItem::minimize(None),
        &PredefinedMenuItem::maximize(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::bring_all_to_front(None),
    ]);

    let _ = menu.append_items(&[&app_menu, &file_menu, &edit_menu, &window_menu]);

    menu.init_for_nsapp();
    // Держим меню живым на весь процесс — NSApp хранит на него ссылку,
    // но Rust-сторона (Rc внутри muda) не должна быть уничтожена раньше.
    Box::leak(Box::new(menu));
}

#[cfg(not(target_os = "macos"))]
pub fn setup_native_menu() {}

/// Забирает события из нативного меню (клики по кастомным пунктам) и применяет к состоянию.
/// На платформах без нативного меню — no-op.
#[cfg(target_os = "macos")]
pub fn poll_menu_events(state: &mut AppState) {
    while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
        let id = event.id().0.as_str();
        if id == ids::SETTINGS {
            state.show_settings_dialog = true;
        } else if id == ids::NEW_CONNECTION {
            state.show_connect_dialog = true;
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn poll_menu_events(_state: &mut AppState) {}

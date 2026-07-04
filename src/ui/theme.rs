//! Цвета и константы темы LoFlum — "Clay"
use egui::Color32;

// Backgrounds — тёплый почти-чёрный, а не нейтрально-синий
pub const BG_BASE: Color32 = Color32::from_rgb(23, 20, 15); // #17140F — самый тёмный фон
pub const BG_PANEL: Color32 = Color32::from_rgb(25, 22, 17); // #19160F — тулбар/боковая панель
pub const BG_CONTENT: Color32 = Color32::from_rgb(32, 29, 25); // #201D19 — основная область
pub const BG_TOOLBAR: Color32 = Color32::from_rgb(25, 22, 17); // #19160F
pub const BG_TAB_ACTIVE: Color32 = Color32::from_rgb(36, 29, 20); // #241D14 — активная вкладка (clay-tint)
pub const BG_TAB_IDLE: Color32 = Color32::from_rgb(23, 20, 15);
pub const BG_ROW_HOVER: Color32 = Color32::from_rgb(38, 32, 25); // #262019
pub const BG_ROW_SEL: Color32 = Color32::from_rgb(42, 32, 24); // #2A2018 — тёплое выделение
pub const BG_QUEUE: Color32 = Color32::from_rgb(22, 19, 16); // #161310
pub const BG_HEADER: Color32 = Color32::from_rgb(23, 20, 15);

// Accents — clay/терракота вместо синего, один акцент на экран
pub const ACCENT: Color32 = Color32::from_rgb(193, 103, 60); // #C1673C — clay
pub const ACCENT_DIM: Color32 = Color32::from_rgb(139, 74, 44); // #8B4A2C — приглушённый clay
pub const ON_ACCENT: Color32 = Color32::from_rgb(26, 22, 17); // #1A1611 — текст на clay-заливке
pub const GREEN: Color32 = Color32::from_rgb(122, 148, 113); // #7A9471 — sage (успех/подключено)
pub const RED: Color32 = Color32::from_rgb(179, 82, 62); // #B3523E — brick (ошибка)
pub const YELLOW: Color32 = Color32::from_rgb(217, 164, 65); // #D9A441 — amber (папки/предупреждение)
pub const ORANGE: Color32 = Color32::from_rgb(193, 103, 60);

// Text
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(237, 232, 224); // #EDE8E0
pub const TEXT_DIM: Color32 = Color32::from_rgb(166, 156, 142); // #A69C8E
pub const TEXT_HINT: Color32 = Color32::from_rgb(110, 101, 90); // #6E655A

// Borders / separators
pub const BORDER: Color32 = Color32::from_rgb(46, 42, 36); // #2E2A24
pub const SEPARATOR: Color32 = Color32::from_rgb(40, 36, 30); // #28241E

// Size constants
pub const SIDEBAR_W: f32 = 200.0;
pub const TOOLBAR_H: f32 = 48.0;
pub const TABBAR_H: f32 = 36.0;
pub const QUEUE_COLLAPSED_H: f32 = 34.0;
pub const QUEUE_EXPANDED_H: f32 = 190.0;
pub const STATUS_H: f32 = 26.0;
pub const ROW_H: f32 = 28.0;

// Radius
pub const RADIUS_SM: f32 = 5.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 10.0;

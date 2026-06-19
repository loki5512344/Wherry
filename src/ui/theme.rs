//! Цвета и константы темы LoFlum
use egui::Color32;

// Backgrounds
pub const BG_BASE: Color32       = Color32::from_rgb(18, 18, 18);   // самый тёмный фон
pub const BG_PANEL: Color32      = Color32::from_rgb(24, 24, 24);   // боковая панель
pub const BG_CONTENT: Color32    = Color32::from_rgb(28, 28, 28);   // основная область
pub const BG_TOOLBAR: Color32    = Color32::from_rgb(22, 22, 22);   // тулбар
pub const BG_TAB_ACTIVE: Color32 = Color32::from_rgb(38, 38, 42);   // активная вкладка
pub const BG_TAB_IDLE: Color32   = Color32::from_rgb(28, 28, 30);   // неактивная
pub const BG_ROW_HOVER: Color32  = Color32::from_rgb(35, 35, 38);
pub const BG_ROW_SEL: Color32    = Color32::from_rgb(42, 64, 95);   // выделение — синеватое
pub const BG_QUEUE: Color32      = Color32::from_rgb(20, 20, 22);
pub const BG_HEADER: Color32     = Color32::from_rgb(22, 22, 24);

// Accents
pub const ACCENT: Color32        = Color32::from_rgb(64, 130, 220);  // синий акцент
pub const ACCENT_DIM: Color32    = Color32::from_rgb(45, 95, 165);
pub const GREEN: Color32         = Color32::from_rgb(60, 180, 100);
pub const RED: Color32           = Color32::from_rgb(210, 70, 70);
pub const YELLOW: Color32        = Color32::from_rgb(210, 160, 50);
pub const ORANGE: Color32        = Color32::from_rgb(210, 120, 50);

// Text
pub const TEXT_PRIMARY: Color32  = Color32::from_rgb(220, 220, 220);
pub const TEXT_DIM: Color32      = Color32::from_rgb(130, 130, 135);
pub const TEXT_HINT: Color32     = Color32::from_rgb(80, 80, 85);

// Borders / separators
pub const BORDER: Color32        = Color32::from_rgb(40, 40, 44);
pub const SEPARATOR: Color32     = Color32::from_rgb(35, 35, 38);

// Size constants
pub const SIDEBAR_W: f32         = 180.0;
pub const TOOLBAR_H: f32         = 40.0;
pub const TABBAR_H: f32          = 32.0;
pub const QUEUE_COLLAPSED_H: f32 = 32.0;
pub const QUEUE_EXPANDED_H: f32  = 160.0;
pub const STATUS_H: f32          = 24.0;
pub const ROW_H: f32             = 22.0;

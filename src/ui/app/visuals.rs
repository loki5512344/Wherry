//! Тема egui (цвета/отступы) и системный шрифт.
use egui::Visuals;

use super::FileManagerApp;
use crate::ui::theme::*;

impl FileManagerApp {
    pub(super) fn apply_visuals(&self, ctx: &egui::Context) {
        let mut vis = Visuals::dark();
        vis.window_fill = BG_PANEL;
        vis.panel_fill = BG_CONTENT;
        vis.extreme_bg_color = BG_BASE;
        vis.code_bg_color = BG_BASE;
        vis.override_text_color = Some(TEXT_PRIMARY);
        vis.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        vis.widgets.inactive.bg_fill = BG_CONTENT;
        vis.widgets.hovered.bg_fill = BG_ROW_HOVER;
        vis.widgets.active.bg_fill = ACCENT_DIM;
        vis.selection.bg_fill = BG_ROW_SEL;
        vis.selection.stroke = egui::Stroke::new(1.0, ACCENT);
        vis.window_rounding = egui::Rounding::same(8.0);
        ctx.set_visuals(vis);

        self.refresh_fonts(ctx);

        let mut style = ctx.style().as_ref().clone();
        style.spacing.item_spacing = egui::vec2(4.0, 2.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        ctx.set_style(style);
    }

    /// Перезагружает шрифты под текущий язык (см. [`crate::i18n::current`]) —
    /// вызывается при старте и заново при смене языка в Settings, потому что
    /// SF Pro не содержит глифов CJK (китайский/японский/корейский).
    pub(super) fn refresh_fonts(&self, ctx: &egui::Context) {
        ctx.set_fonts(system_fonts(crate::i18n::current()));
    }
}

/// Загружает системный шрифт (SF Pro / SF Mono на macOS) вместо egui-дефолта,
/// чтобы интерфейс выглядел нативно, плюс CJK-шрифты фолбэком (SF Pro не
/// содержит китайских/японских/корейских глифов — без фолбэка был бы tofu).
/// На других ОС файлы просто отсутствуют — тогда остаются встроенные шрифты egui.
fn system_fonts(lang: crate::i18n::Lang) -> egui::FontDefinitions {
    use crate::i18n::Lang;

    let mut fonts = egui::FontDefinitions::default();

    // Основной шрифт всегда первый в списке: даёт консистентную латиницу и
    // кириллицу во всём интерфейсе. CJK-фонты идут строго ПОСЛЕ — иначе их
    // собственное начертание латинских букв/цифр перекрасило бы весь текст,
    // а не только иероглифы/кану/хангыль, которых нет в SF Pro.
    let mut candidates: Vec<(&str, &str, egui::FontFamily)> = vec![
        (
            "system-sans",
            "/System/Library/Fonts/SFNS.ttf",
            egui::FontFamily::Proportional,
        ),
        (
            "system-mono",
            "/System/Library/Fonts/SFNSMono.ttf",
            egui::FontFamily::Monospace,
        ),
    ];

    // CJK Unified Ideographs пересекаются между китайским/японским/корейским
    // (ханьцзы/кандзи/ханча — одни кодпоинты), поэтому шрифт активного языка
    // ставим первым среди CJK-фолбэков: он и определит начертание общих иероглифов.
    let mut cjk: Vec<(&str, &str)> = vec![
        ("cjk-ja", "/System/Library/Fonts/ヒラギノ角ゴシック W4.ttc"),
        ("cjk-zh", "/System/Library/Fonts/Hiragino Sans GB.ttc"),
        ("cjk-ko", "/System/Library/Fonts/AppleSDGothicNeo.ttc"),
    ];
    let active = match lang {
        Lang::Ja => Some("cjk-ja"),
        Lang::Zh => Some("cjk-zh"),
        Lang::Ko => Some("cjk-ko"),
        _ => None,
    };
    if let Some(active) = active
        && let Some(pos) = cjk.iter().position(|(name, _)| *name == active)
    {
        cjk.swap(0, pos);
    }
    candidates.extend(cjk.into_iter().map(|(n, p)| (n, p, egui::FontFamily::Proportional)));

    for (name, path, family) in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .entry(name.to_owned())
                .or_insert_with(|| egui::FontData::from_owned(bytes));
            let list = fonts.families.entry(family).or_default();
            if !list.iter().any(|n| n == name) {
                list.push(name.to_owned());
            }
        }
    }

    fonts
}

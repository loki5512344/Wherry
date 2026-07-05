//! Левое меню диалога Settings: сгруппированные пункты разделов.
use egui::{RichText, Ui};

use crate::i18n::t;
use crate::ui::icons::{self, Icon};
use crate::ui::state::SettingsSection;
use crate::ui::theme::*;
use crate::ui::widgets::row::clickable_row;

struct NavItem {
    section: SettingsSection,
    label_key: &'static str,
    icon: Icon,
}

struct NavGroup {
    label_key: &'static str,
    items: &'static [NavItem],
}

const GENERAL: &[NavItem] = &[
    NavItem {
        section: SettingsSection::General,
        label_key: "settings.nav.general",
        icon: Icon::Settings,
    },
    NavItem {
        section: SettingsSection::Appearance,
        label_key: "settings.nav.appearance",
        icon: Icon::Monitor,
    },
];

const CONNECTIONS: &[NavItem] = &[
    NavItem {
        section: SettingsSection::Connections,
        label_key: "settings.nav.connections",
        icon: Icon::ServerSquare,
    },
    NavItem {
        section: SettingsSection::History,
        label_key: "settings.nav.history",
        icon: Icon::History,
    },
    NavItem {
        section: SettingsSection::Security,
        label_key: "settings.nav.security",
        icon: Icon::ShieldKeyhole,
    },
];

const TRANSFERS: &[NavItem] = &[NavItem {
    section: SettingsSection::Transfers,
    label_key: "settings.nav.transfers",
    icon: Icon::Upload,
}];

const ABOUT: &[NavItem] = &[NavItem {
    section: SettingsSection::About,
    label_key: "settings.nav.about",
    icon: Icon::Document,
}];

const GROUPS: &[NavGroup] = &[
    NavGroup {
        label_key: "settings.nav.group_general",
        items: GENERAL,
    },
    NavGroup {
        label_key: "settings.nav.group_connections",
        items: CONNECTIONS,
    },
    NavGroup {
        label_key: "settings.nav.group_transfers",
        items: TRANSFERS,
    },
    NavGroup {
        label_key: "settings.nav.group_about",
        items: ABOUT,
    },
];

pub(super) fn render(ui: &mut Ui, active: &mut SettingsSection) {
    ui.set_width(168.0);
    for group in GROUPS {
        ui.label(
            RichText::new(t(group.label_key))
                .color(TEXT_HINT)
                .size(10.0)
                .strong(),
        );
        ui.add_space(4.0);
        for item in group.items {
            let is_active = *active == item.section;
            let icon_col = if is_active { ACCENT } else { TEXT_DIM };
            let resp = clickable_row(ui, is_active, 30.0, |ui| {
                icons::icon(ui, item.icon, 14.0, icon_col);
                ui.add_space(8.0);
                ui.label(RichText::new(t(item.label_key)).color(TEXT_PRIMARY).size(12.5));
            });
            if resp.clicked() {
                *active = item.section;
            }
        }
        ui.add_space(10.0);
    }
}

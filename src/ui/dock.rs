//! Модульная система панелей: Local и каждое подключение — перетаскиваемые,
//! разбиваемые и закрываемые вкладки внутри одной рабочей области (egui_dock).
use crate::domain::connection::ConnectionStatus;
use crate::fs::remote::RemoteRegistry;
use crate::transfer::queue::TransferQueue;
use crate::ui::panels::{local_pane, remote_pane};
use crate::ui::state::AppState;
use crate::ui::theme::*;
use egui::{RichText, Ui, WidgetText};
use std::sync::{Arc, Mutex};

/// Одна вкладка рабочей области — локальная ФС или конкретное удалённое соединение
/// (по стабильному id, а не индексу — индексы сдвигаются при закрытии других вкладок).
#[derive(Clone, PartialEq, Eq)]
pub enum PaneTab {
    Local,
    Remote(String),
}

pub struct AppTabViewer<'a> {
    pub state: &'a mut AppState,
    pub queue: &'a TransferQueue,
    pub registry: &'a Arc<RemoteRegistry>,
    pub rt_handle: &'a tokio::runtime::Handle,
    pub db: &'a Arc<Mutex<rusqlite::Connection>>,
    /// Если пользователь закрыл вкладку с удалённым соединением — сюда кладём его id,
    /// вызывающий код (app.rs) разрывает соединение после `show_inside`.
    pub closed_remote: &'a mut Option<String>,
}

impl egui_dock::TabViewer for AppTabViewer<'_> {
    type Tab = PaneTab;

    fn title(&mut self, tab: &mut PaneTab) -> WidgetText {
        match tab {
            PaneTab::Local => RichText::new("Local").into(),
            PaneTab::Remote(id) => match self.state.tabs.iter().find(|t| &t.id == id) {
                Some(t) => {
                    let (dot, col) = match t.status {
                        ConnectionStatus::Connected => ("● ", GREEN),
                        ConnectionStatus::Connecting => ("◐ ", YELLOW),
                        ConnectionStatus::Error(_) => ("× ", RED),
                        ConnectionStatus::Disconnected => ("○ ", TEXT_HINT),
                    };
                    RichText::new(format!("{dot}{}", t.label)).color(col).into()
                }
                None => RichText::new("Remote").color(TEXT_HINT).into(),
            },
        }
    }

    fn id(&mut self, tab: &mut PaneTab) -> egui::Id {
        match tab {
            PaneTab::Local => egui::Id::new("pane_tab_local"),
            PaneTab::Remote(id) => egui::Id::new(("pane_tab_remote", id.clone())),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut PaneTab) {
        match tab {
            PaneTab::Local => {
                local_pane::render(
                    ui,
                    self.state,
                    self.queue,
                    self.registry,
                    self.rt_handle,
                    self.db,
                );
            }
            PaneTab::Remote(id) => {
                if let Some(idx) = self.state.tabs.iter().position(|t| &t.id == id) {
                    self.state.active_tab = idx;
                    remote_pane::render(
                        ui,
                        self.state,
                        idx,
                        self.registry,
                        self.rt_handle,
                        self.queue,
                    );
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);
                        ui.label(RichText::new("Disconnected").color(TEXT_HINT));
                    });
                }
            }
        }
    }

    fn closeable(&mut self, tab: &mut PaneTab) -> bool {
        matches!(tab, PaneTab::Remote(_))
    }

    fn on_close(&mut self, tab: &mut PaneTab) -> bool {
        if let PaneTab::Remote(id) = tab {
            *self.closed_remote = Some(id.clone());
        }
        true
    }
}

//! Tray icon and menu (WR-01), implementation A: Slint `SystemTrayIcon` (ADR-0010).
//!
//! This is implementation A on every OS. Slint's `SystemTrayIcon` covers Windows (notification
//! area), macOS (menu bar) and Linux (`StatusNotifierItem`). If it cannot meet
//! WR-01 on some OS, implementation B (`tray-icon` + `muda`, ADR-0010) replaces this module; the
//! `Tray` trait keeps the rest of the app unchanged.

use nepdate::Lang;

use crate::TrayComponent;
use crate::platform::taskbar::MenuLabels;

/// Events the tray delivers to the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    Open,
    CopyBs,
    CopyAd,
    Settings,
    Quit,
}

/// The tray surface the app updates once per tick.
pub trait Tray {
    fn set_title(&self, text: &str);
    fn set_icon_day(&self, day: &str);
    fn set_tooltip(&self, tooltip: &str);
    /// Picks the icon ink for a dark (`true`) or light taskbar.
    fn set_dark_taskbar(&self, dark: bool);
}

/// Implementation A: a Slint `SystemTrayIcon`. The two day-number image arrays live in
/// `ui/tray.slint` (pre-rendered by `cargo xtask icons`); `set_icon_day` only sets the index,
/// and the `is-ne` flag picks the digit set.
pub struct SlintTray {
    component: TrayComponent,
}

impl SlintTray {
    /// Creates the tray and wires the menu and click events to `on_event`. The language is fixed
    /// at construction; the settings UI updates `is_ne` when it changes.
    pub fn new(lang: Lang, on_event: impl FnMut(TrayEvent) + 'static) -> anyhow::Result<Self> {
        let component = TrayComponent::new()?;
        component.set_is_ne(lang == Lang::Ne);
        // The five callbacks share `on_event`, so it lives behind Rc<RefCell> and each borrow is
        // dropped before the next event.
        let events = std::rc::Rc::new(std::cell::RefCell::new(on_event));
        {
            let events = events.clone();
            component.on_open(move || (events.borrow_mut())(TrayEvent::Open));
        }
        {
            let events = events.clone();
            component.on_copy_bs(move || (events.borrow_mut())(TrayEvent::CopyBs));
        }
        {
            let events = events.clone();
            component.on_copy_ad(move || (events.borrow_mut())(TrayEvent::CopyAd));
        }
        {
            let events = events.clone();
            component.on_settings(move || (events.borrow_mut())(TrayEvent::Settings));
        }
        {
            let events = events.clone();
            component.on_quit(move || (events.borrow_mut())(TrayEvent::Quit));
        }
        Ok(SlintTray { component })
    }

    /// The translated WR-01 AC5 labels, for the taskbar text's right-click menu. The
    /// popup may not exist yet; the tray component always does.
    pub fn menu_labels(&self) -> MenuLabels {
        MenuLabels {
            open: self.component.get_menu_open().to_string(),
            copy_bs: self.component.get_menu_copy_bs().to_string(),
            copy_ad: self.component.get_menu_copy_ad().to_string(),
            settings: self.component.get_menu_settings().to_string(),
            quit: self.component.get_menu_quit().to_string(),
        }
    }
}

impl Tray for SlintTray {
    fn set_title(&self, text: &str) {
        self.component.set_title_text(text.into());
    }

    fn set_icon_day(&self, day: &str) {
        let Ok(day) = day.parse::<i32>() else {
            return;
        };
        if (1..=32).contains(&day) {
            self.component.set_day_index(day - 1);
        }
    }

    fn set_tooltip(&self, tooltip: &str) {
        self.component.set_tooltip_text(tooltip.into());
    }

    fn set_dark_taskbar(&self, dark: bool) {
        self.component.set_dark_taskbar(dark);
    }
}

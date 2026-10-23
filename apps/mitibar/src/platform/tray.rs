//! Tray icon and menu (WR-01), implementation A: Slint `SystemTrayIcon` (ADR-0010).
//!
//! This is implementation A on every OS. Slint's `SystemTrayIcon` covers Windows (notification
//! area), macOS (menu bar) and Linux (`StatusNotifierItem`). If it cannot meet
//! WR-01 on some OS, implementation B (`tray-icon` + `muda`, ADR-0010) replaces this module; the
//! `Tray` trait keeps the rest of the app unchanged.

use nepdate::Lang;

use crate::TrayComponent;
use crate::platform::bar::MenuLabels;

/// Events the tray delivers to the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    Open,
    CopyBs,
    CopyAd,
    /// The Show bar check item (Windows only, WR-01 AC5): toggles the bar.
    ShowBar,
    Settings,
    Quit,
}

/// The tray surface the app updates: once per tick, and when a setting changes (WR-06 AC5).
pub trait Tray {
    fn set_title(&self, text: &str);
    fn set_icon_day(&self, day: &str);
    fn set_tooltip(&self, tooltip: &str);
    /// Picks the icon ink for a dark (`true`) or light taskbar.
    fn set_dark_taskbar(&self, dark: bool);
    /// Picks the digits of the icon: Devanagari for Nepali, ASCII otherwise.
    fn set_language(&self, lang: Lang);
    /// Shows the Show bar check item, on a platform that has a bar (WR-01 AC5).
    fn set_bar_supported(&self, supported: bool);
    /// Checks or unchecks the Show bar item to match the `bar` setting.
    fn set_bar_shown(&self, shown: bool);
    /// The translated labels, for the bar's right-click menu (WR-08 AC6).
    fn menu_labels(&self) -> MenuLabels;
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
        // The six callbacks share `on_event`, so it lives behind Rc<RefCell> and each borrow is
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
            component.on_show_bar(move || (events.borrow_mut())(TrayEvent::ShowBar));
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

    fn set_language(&self, lang: Lang) {
        self.component.set_is_ne(lang == Lang::Ne);
    }

    fn set_bar_supported(&self, supported: bool) {
        self.component.set_bar_supported(supported);
    }

    fn set_bar_shown(&self, shown: bool) {
        self.component.set_bar_shown(shown);
    }

    /// The popup may not exist yet; the tray component always does.
    fn menu_labels(&self) -> MenuLabels {
        MenuLabels {
            open: self.component.get_menu_open().to_string(),
            copy_bs: self.component.get_menu_copy_bs().to_string(),
            copy_ad: self.component.get_menu_copy_ad().to_string(),
            customize: self.component.get_menu_customize_bar().to_string(),
            lock: self.component.get_menu_lock_position().to_string(),
            hide: self.component.get_menu_hide_bar().to_string(),
            settings: self.component.get_menu_settings().to_string(),
            quit: self.component.get_menu_quit().to_string(),
        }
    }
}

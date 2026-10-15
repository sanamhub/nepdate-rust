//! Tray icon and menu (WR-01), implementation A: Slint `SystemTrayIcon` (ADR-0010).
//!
//! W1 uses implementation A on every OS. Slint's `SystemTrayIcon` covers Windows (notification
//! area), macOS (menu bar) and Linux (`StatusNotifierItem`). If a later phase finds it cannot meet
//! WR-01 on some OS, implementation B (`tray-icon` + `muda`, ADR-0010) replaces this module; the
//! `Tray` trait keeps the rest of the app unchanged.

// Filled before main.rs wires it (W1-05); removed then.
#![allow(dead_code)]

use nepdate::Lang;

use crate::TrayComponent;

/// Events the tray delivers to the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    Open,
    CopyBs,
    CopyAd,
    Settings,
    Quit,
}

/// The tray surface the app updates once per tick (W1-05).
pub trait Tray {
    fn set_title(&self, text: &str);
    fn set_icon_day(&self, day: &str);
    fn set_tooltip(&self, tooltip: &str);
}

/// Implementation A: a Slint `SystemTrayIcon`. The two day-number image arrays live in
/// `ui/tray.slint` (pre-rendered by `cargo xtask icons`); `set_icon_day` only sets the index,
/// and the `is-ne` flag picks the digit set.
pub struct SlintTray {
    component: TrayComponent,
}

impl SlintTray {
    /// Creates the tray and wires the menu and click events to `on_event`. The language is fixed
    /// at construction in W1; W3's settings UI will update `is_ne` when it changes.
    pub fn new(lang: Lang, on_event: impl FnMut(TrayEvent) + 'static) -> anyhow::Result<Self> {
        let component = TrayComponent::new()?;
        component.set_is_ne(lang == Lang::Ne);
        // The five callbacks share `on_event`, so it lives behind Rc<RefCell> and each borrow is
        // dropped before the next event (learn/09 §3).
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
}

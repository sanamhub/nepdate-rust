//! The system-wide shortcut (WR-13, ADR-0019), behind a small port so the app and its tests never
//! touch the OS. The real one wraps `global-hotkey`: the OS delivers the key press to a handler,
//! so nothing polls (WR-13 AC5).

use std::cell::{Cell, RefCell};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::app::shortcut::{Combo, Key};

/// The OS refused the registration, usually because another app owns the combination (WR-13 AC3).
/// The reason is for the log; the field's note is the same whatever it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused(pub String);

/// The port the app registers its shortcut through.
pub trait Hotkeys {
    /// Whether this session can grab a global key at all. False on Wayland, which has no portable
    /// way to (WR-13 AC4).
    fn available(&self) -> bool;

    /// Makes `combo` the one shortcut, replacing the previous one; `None` removes it.
    ///
    /// # Errors
    ///
    /// `Refused` when the OS won't register it. The previous shortcut is gone then too.
    fn set(&self, combo: Option<Combo>) -> Result<(), Refused>;
}

/// Whether a Linux session is Wayland, from `XDG_SESSION_TYPE` and `WAYLAND_DISPLAY`. The session
/// type wins when it says either; otherwise a Wayland display with no X11 one counts.
pub fn is_wayland(
    session_type: Option<&str>,
    wayland_display: Option<&str>,
    x_display: Option<&str>,
) -> bool {
    let set = |value: Option<&str>| value.is_some_and(|text| !text.is_empty());
    match session_type {
        Some(kind) if kind.eq_ignore_ascii_case("wayland") => true,
        Some(kind) if kind.eq_ignore_ascii_case("x11") => false,
        _ => set(wayland_display) && !set(x_display),
    }
}

/// Whether this process runs under Wayland. Only Linux has the question.
fn running_on_wayland() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    let var = |name: &str| std::env::var(name).ok();
    is_wayland(
        var("XDG_SESSION_TYPE").as_deref(),
        var("WAYLAND_DISPLAY").as_deref(),
        var("DISPLAY").as_deref(),
    )
}

/// The real registration. It lives on the UI thread: on Windows and macOS the manager must be
/// created on the thread that runs the event loop.
pub struct SystemHotkeys {
    /// `None` on Wayland, or when the OS would not give a manager.
    manager: Option<GlobalHotKeyManager>,
    wayland: bool,
    current: Cell<Option<HotKey>>,
    /// Why there is no manager, for the first `set`.
    why_not: RefCell<Option<String>>,
}

impl SystemHotkeys {
    /// Starts listening. `on_pressed` runs for each press of the registered key, on whatever
    /// thread the OS delivers it on, so it must only wake the UI thread.
    pub fn new(on_pressed: impl Fn() + Send + Sync + 'static) -> Self {
        let wayland = running_on_wayland();
        if wayland {
            log::info!("global shortcut: Wayland has no portable way to grab a key; off");
            return Self::without_manager(true, None);
        }
        match GlobalHotKeyManager::new() {
            Ok(manager) => {
                GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
                    if event.state == HotKeyState::Pressed {
                        on_pressed();
                    }
                }));
                Self {
                    manager: Some(manager),
                    wayland: false,
                    current: Cell::new(None),
                    why_not: RefCell::new(None),
                }
            }
            Err(err) => {
                log::warn!("global shortcut: could not start: {err}");
                Self::without_manager(false, Some(err.to_string()))
            }
        }
    }

    fn without_manager(wayland: bool, why_not: Option<String>) -> Self {
        Self {
            manager: None,
            wayland,
            current: Cell::new(None),
            why_not: RefCell::new(why_not),
        }
    }
}

impl Hotkeys for SystemHotkeys {
    fn available(&self) -> bool {
        !self.wayland
    }

    fn set(&self, combo: Option<Combo>) -> Result<(), Refused> {
        let Some(manager) = &self.manager else {
            return match combo {
                None => Ok(()),
                Some(_) => {
                    Err(Refused(self.why_not.borrow().clone().unwrap_or_else(
                        || "no global shortcuts in this session".to_owned(),
                    )))
                }
            };
        };
        if let Some(old) = self.current.take()
            && let Err(err) = manager.unregister(old)
        {
            log::warn!("global shortcut: could not unregister {old}: {err}");
        }
        let Some(combo) = combo else {
            return Ok(());
        };
        let hotkey = to_hotkey(combo);
        match manager.register(hotkey) {
            Ok(()) => {
                self.current.set(Some(hotkey));
                Ok(())
            }
            Err(err) => {
                log::warn!("global shortcut: the OS refused {combo}: {err}");
                Err(Refused(err.to_string()))
            }
        }
    }
}

/// The `global-hotkey` form of `combo`.
pub fn to_hotkey(combo: Combo) -> HotKey {
    let mut mods = Modifiers::empty();
    for (held, flag) in [
        (combo.mods.meta, Modifiers::SUPER),
        (combo.mods.ctrl, Modifiers::CONTROL),
        (combo.mods.alt, Modifiers::ALT),
        (combo.mods.shift, Modifiers::SHIFT),
    ] {
        if held {
            mods |= flag;
        }
    }
    HotKey::new(Some(mods), code(combo.key))
}

/// The physical key of a combination's main key. The names are the W3C code names that
/// `keyboard-types` parses, so a mistake here is a test failure, not a wrong key.
fn code(key: Key) -> Code {
    let name = match key {
        Key::Letter(letter) => format!("Key{letter}"),
        Key::Digit(digit) => format!("Digit{digit}"),
        Key::Function(number) => format!("F{number}"),
        Key::Space => "Space".to_owned(),
        Key::Insert => "Insert".to_owned(),
        Key::Delete => "Delete".to_owned(),
        Key::Home => "Home".to_owned(),
        Key::End => "End".to_owned(),
        Key::PageUp => "PageUp".to_owned(),
        Key::PageDown => "PageDown".to_owned(),
        Key::Left => "ArrowLeft".to_owned(),
        Key::Right => "ArrowRight".to_owned(),
        Key::Up => "ArrowUp".to_owned(),
        Key::Down => "ArrowDown".to_owned(),
    };
    name.parse().unwrap_or(Code::Unidentified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wr13_ac4_wayland_is_detected_from_the_session() {
        // XDG_SESSION_TYPE, WAYLAND_DISPLAY, DISPLAY.
        for (session, wayland, x, expected) in [
            (Some("wayland"), None, None, true),
            (Some("Wayland"), Some("wayland-0"), Some(":0"), true),
            (Some("x11"), Some("wayland-0"), Some(":0"), false),
            (Some("x11"), None, Some(":0"), false),
            (None, Some("wayland-0"), None, true),
            (None, Some("wayland-0"), Some(":0"), false),
            (Some("tty"), Some("wayland-0"), None, true),
            (Some("tty"), None, None, false),
            (None, None, None, false),
            (None, Some(""), None, false),
        ] {
            assert_eq!(
                is_wayland(session, wayland, x),
                expected,
                "{session:?} {wayland:?} {x:?}"
            );
        }
    }

    fn combo(text: &str) -> Combo {
        Combo::parse(text).expect("a combination")
    }

    #[test]
    fn a_combination_maps_to_the_hotkey_it_names() {
        assert_eq!(
            to_hotkey(combo("Win+Shift+N")),
            HotKey::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyN)
        );
        assert_eq!(
            to_hotkey(combo("Ctrl+Shift+7")),
            HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit7)
        );
        assert_eq!(
            to_hotkey(combo("Alt+Shift+F12")),
            HotKey::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::F12)
        );
    }

    #[test]
    fn every_main_key_has_a_code() {
        let keys = (b'A'..=b'Z')
            .map(|letter| Key::Letter(char::from(letter)))
            .chain((0..=9).map(Key::Digit))
            .chain((1..=12).map(Key::Function))
            .chain([
                Key::Space,
                Key::Insert,
                Key::Delete,
                Key::Home,
                Key::End,
                Key::PageUp,
                Key::PageDown,
                Key::Left,
                Key::Right,
                Key::Up,
                Key::Down,
            ]);
        let mut seen = Vec::new();
        for key in keys {
            let code = code(key);
            assert_ne!(code, Code::Unidentified, "{key}");
            assert!(!seen.contains(&code), "{key} repeats a code");
            seen.push(code);
        }
    }
}

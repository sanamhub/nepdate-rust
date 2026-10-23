//! The global shortcut (WR-13): the combination type, its text form in `settings.toml`, the rules
//! of AC2 and what a recorded key press turns into. Pure: the OS registration is in
//! `platform::hotkey`, and the mapping from a Slint key event is in `ui_bridge`.

use std::fmt;

/// What the Win, Cmd or Super key is called on this platform. The settings file stores this word.
pub const META: &str = if cfg!(windows) {
    "Win"
} else if cfg!(target_os = "macos") {
    "Cmd"
} else {
    "Super"
};

/// The modifier keys of a combination: one flag for each key, as the keyboard has them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Modifiers {
    /// Win, Cmd or Super.
    pub meta: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Modifiers {
    /// How many of Ctrl, Alt and Shift are held.
    fn others(self) -> usize {
        usize::from(self.ctrl) + usize::from(self.alt) + usize::from(self.shift)
    }

    fn any(self) -> bool {
        self.meta || self.others() > 0
    }
}

/// The main key of a combination. Punctuation is left out on purpose: its position depends on the
/// keyboard layout, so the same text could be a different key on another one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// `A` to `Z`, stored upper case.
    Letter(char),
    /// `0` to `9`.
    Digit(u8),
    /// `F1` to `F12`.
    Function(u8),
    Space,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
}

impl Key {
    /// The names that are not one character or `F<n>`, with their spellings in the file.
    const NAMED: [(Key, &'static str); 11] = [
        (Key::Space, "Space"),
        (Key::Insert, "Insert"),
        (Key::Delete, "Delete"),
        (Key::Home, "Home"),
        (Key::End, "End"),
        (Key::PageUp, "PageUp"),
        (Key::PageDown, "PageDown"),
        (Key::Left, "Left"),
        (Key::Right, "Right"),
        (Key::Up, "Up"),
        (Key::Down, "Down"),
    ];

    fn parse(token: &str) -> Option<Key> {
        let mut chars = token.chars();
        if let (Some(only), None) = (chars.next(), chars.next()) {
            return Key::from_char(only);
        }
        if let Some(number) = token
            .strip_prefix(['F', 'f'])
            .and_then(|digits| digits.parse::<u8>().ok())
            && (1..=12).contains(&number)
        {
            return Some(Key::Function(number));
        }
        Key::NAMED
            .iter()
            .find(|(_, name)| name.eq_ignore_ascii_case(token))
            .map(|(key, _)| *key)
    }

    /// The key for one typed character, if it is a letter or a digit.
    pub fn from_char(c: char) -> Option<Key> {
        match c {
            'a'..='z' | 'A'..='Z' => Some(Key::Letter(c.to_ascii_uppercase())),
            '0'..='9' => u8::try_from(u32::from(c) - u32::from('0'))
                .ok()
                .map(Key::Digit),
            _ => None,
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Letter(letter) => write!(f, "{letter}"),
            Key::Digit(digit) => write!(f, "{digit}"),
            Key::Function(number) => write!(f, "F{number}"),
            other => {
                let name = Key::NAMED
                    .iter()
                    .find(|(key, _)| key == other)
                    .map_or("", |(_, name)| name);
                f.write_str(name)
            }
        }
    }
}

/// A shortcut: modifiers and one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Combo {
    pub mods: Modifiers,
    pub key: Key,
}

/// Why a combination is refused (WR-13 AC2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Ctrl and Alt together are `AltGr` on many keyboards.
    CtrlAlt,
    /// Needs Win, Cmd or Super, or at least two of Ctrl, Alt and Shift.
    TooFewModifiers,
}

impl Combo {
    /// The default shortcut of the platform (WR-06): Win+Shift+N, Cmd+Shift+N or Super+Shift+N.
    pub const DEFAULT: Combo = Combo {
        mods: Modifiers {
            meta: true,
            ctrl: false,
            alt: false,
            shift: true,
        },
        key: Key::Letter('N'),
    };

    /// Reads the text form, such as `Win+Shift+N`. Win, Cmd, Super and Meta are the same key, so
    /// a file copied from another platform still reads. Returns `None` for text that is not a
    /// combination, and does not apply the rules of `check`.
    pub fn parse(text: &str) -> Option<Combo> {
        let mut tokens: Vec<&str> = text.split('+').map(str::trim).collect();
        let key = Key::parse(tokens.pop()?)?;
        let mut mods = Modifiers::default();
        for token in tokens {
            let flag = match token.to_ascii_lowercase().as_str() {
                "win" | "cmd" | "command" | "super" | "meta" => &mut mods.meta,
                "ctrl" | "control" => &mut mods.ctrl,
                "alt" | "option" => &mut mods.alt,
                "shift" => &mut mods.shift,
                _ => return None,
            };
            if *flag {
                return None;
            }
            *flag = true;
        }
        Some(Combo { mods, key })
    }

    /// The rules of WR-13 AC2: Ctrl+Alt is refused, and the rest needs Win, Cmd or Super or at
    /// least two other modifiers.
    ///
    /// # Errors
    ///
    /// The reason the combination is refused.
    pub fn check(self) -> Result<(), Refusal> {
        if self.mods.ctrl && self.mods.alt {
            return Err(Refusal::CtrlAlt);
        }
        if self.mods.meta || self.mods.others() >= 2 {
            Ok(())
        } else {
            Err(Refusal::TooFewModifiers)
        }
    }
}

impl fmt::Display for Combo {
    /// The text form, `Win+Shift+N`: the platform's word for the meta key first, then Ctrl, Alt
    /// and Shift, then the key.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, name) in [
            (self.mods.meta, META),
            (self.mods.ctrl, "Ctrl"),
            (self.mods.alt, "Alt"),
            (self.mods.shift, "Shift"),
        ] {
            if held {
                write!(f, "{name}+")?;
            }
        }
        write!(f, "{}", self.key)
    }
}

/// A key press in the shortcut field, after `ui_bridge` has read the Slint event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressed {
    /// A key that can be the main key of a combination.
    Key(Key),
    /// A modifier by itself. The field keeps waiting for the key.
    Modifier,
    /// Tab, Escape, Enter or a key with no meaning here. These move focus or close the window as
    /// usual, so the field leaves them alone.
    Navigation,
}

/// What the field does with a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recorded {
    /// Not the field's: let the key do what it does anywhere else in the window.
    Ignored,
    /// A valid combination: it becomes the shortcut.
    Accepted(Combo),
    /// A combination the rules refuse, shown as a note; the shortcut stays as it was.
    Refused(Refusal),
}

/// Turns a key press with `mods` held into what the field does with it (WR-13 AC2). A key with no
/// modifier at all is refused for being too bare, so typing a letter doesn't go unanswered, while
/// Tab, Escape and Enter keep working for the keyboard.
pub fn record(mods: Modifiers, pressed: Pressed) -> Recorded {
    match pressed {
        Pressed::Navigation | Pressed::Modifier => Recorded::Ignored,
        Pressed::Key(key) => {
            let combo = Combo { mods, key };
            if !mods.any() {
                return Recorded::Refused(Refusal::TooFewModifiers);
            }
            match combo.check() {
                Ok(()) => Recorded::Accepted(combo),
                Err(refusal) => Recorded::Refused(refusal),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(text: &str) -> Combo {
        Combo::parse(text).expect("a combination")
    }

    #[test]
    fn the_default_is_win_shift_n_on_windows_and_follows_the_platform_word_elsewhere() {
        assert_eq!(
            Combo::DEFAULT.to_string(),
            format!("{META}+Shift+N"),
            "Win+Shift+N, Cmd+Shift+N or Super+Shift+N"
        );
        assert_eq!(Combo::DEFAULT.check(), Ok(()));
        assert_eq!(
            Combo::parse(&Combo::DEFAULT.to_string()),
            Some(Combo::DEFAULT)
        );
    }

    #[test]
    fn a_combination_round_trips_through_its_text_form() {
        for text in [
            "Win+Shift+N",
            "Ctrl+Shift+7",
            "Alt+Shift+F5",
            "Win+Space",
            "Ctrl+Shift+PageDown",
            "Win+Down",
            "Win+Ctrl+Alt+Shift+Home",
        ] {
            let parsed = combo(text);
            let written = parsed.to_string();
            // The meta key is written as this platform's word, whichever word the text used.
            assert_eq!(written, text.replace("Win", META), "{text}");
            assert_eq!(Combo::parse(&written), Some(parsed));
        }
    }

    #[test]
    fn parse_is_forgiving_about_case_spaces_and_the_name_of_the_meta_key() {
        let expected = combo("Win+Shift+N");
        for text in [
            "win+shift+n",
            "Super + Shift + N",
            "CMD+SHIFT+N",
            "Meta+Shift+n",
            "Shift+Win+N",
        ] {
            assert_eq!(Combo::parse(text), Some(expected), "{text}");
        }
    }

    #[test]
    fn parse_rejects_text_that_is_not_a_combination() {
        for text in [
            "",
            "Win+",
            "+N",
            "Shift",
            "Win+Shift",
            "Win+Shift+NN",
            "Win+Win+N",
            "Hyper+N",
            "Win+F13",
            "Win+F0",
            "Win+;",
            "Win+Shift+N+M",
        ] {
            assert_eq!(Combo::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn wr13_ac2_a_combination_needs_the_meta_key_or_two_other_modifiers() {
        for (text, allowed) in [
            ("Win+N", true),
            ("Win+Shift+N", true),
            ("Ctrl+Shift+N", true),
            ("Alt+Shift+N", true),
            ("Win+Ctrl+N", true),
            ("Ctrl+N", false),
            ("Alt+N", false),
            ("Shift+N", false),
            ("F5", false),
        ] {
            let wanted = if allowed {
                Ok(())
            } else {
                Err(Refusal::TooFewModifiers)
            };
            assert_eq!(combo(text).check(), wanted, "{text}");
        }
    }

    #[test]
    fn wr13_ac2_ctrl_alt_is_refused_with_or_without_company() {
        for text in ["Ctrl+Alt+N", "Ctrl+Alt+Shift+N", "Win+Ctrl+Alt+N"] {
            assert_eq!(combo(text).check(), Err(Refusal::CtrlAlt), "{text}");
        }
    }

    #[test]
    fn recording_accepts_a_valid_combination_and_names_the_refusal_of_the_others() {
        let held = |meta, ctrl, alt, shift| Modifiers {
            meta,
            ctrl,
            alt,
            shift,
        };
        let key = Pressed::Key(Key::Letter('K'));
        assert_eq!(
            record(held(true, false, false, true), key),
            Recorded::Accepted(combo("Win+Shift+K"))
        );
        assert_eq!(
            record(held(false, true, true, false), key),
            Recorded::Refused(Refusal::CtrlAlt)
        );
        assert_eq!(
            record(held(false, true, false, false), key),
            Recorded::Refused(Refusal::TooFewModifiers)
        );
        assert_eq!(
            record(Modifiers::default(), key),
            Recorded::Refused(Refusal::TooFewModifiers),
            "a bare letter is answered, not ignored"
        );
    }

    #[test]
    fn recording_leaves_tab_escape_enter_and_a_lone_modifier_alone() {
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        for pressed in [Pressed::Navigation, Pressed::Modifier] {
            assert_eq!(record(shift, pressed), Recorded::Ignored);
            assert_eq!(record(Modifiers::default(), pressed), Recorded::Ignored);
        }
    }

    #[test]
    fn keys_come_from_letters_digits_and_nothing_else() {
        assert_eq!(Key::from_char('n'), Some(Key::Letter('N')));
        assert_eq!(Key::from_char('N'), Some(Key::Letter('N')));
        assert_eq!(Key::from_char('7'), Some(Key::Digit(7)));
        for c in [';', ',', ' ', 'न', '\u{e}'] {
            assert_eq!(Key::from_char(c), None, "{c:?}");
        }
    }
}

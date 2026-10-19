//! Clipboard access for the tray menu and the day detail (WR-01 AC5, WR-04 AC2). The trait is
//! the port the interaction tests inject a fake through; the shipped app uses the system
//! clipboard (arboard).

/// Writes plain text to a clipboard. The app never reads back.
pub trait Clipboard {
    /// Writes `text`.
    ///
    /// # Errors
    ///
    /// Returns the platform error when the clipboard cannot be opened or written.
    fn set_text(&self, text: &str) -> anyhow::Result<()>;
}

/// The system clipboard (arboard). One short-lived handle per write, which is what arboard
/// wants on Windows and X11.
pub struct SystemClipboard;

impl Clipboard for SystemClipboard {
    fn set_text(&self, text: &str) -> anyhow::Result<()> {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(text.to_owned())?;
        Ok(())
    }
}

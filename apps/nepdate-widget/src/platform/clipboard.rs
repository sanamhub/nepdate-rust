//! Clipboard access for the tray menu's Copy items (WR-01 AC5). One small function keeps the
//! tray event handling free of clipboard plumbing.

/// Writes `text` to the system clipboard (plain text).
pub fn set_text(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text.to_owned())?;
    Ok(())
}

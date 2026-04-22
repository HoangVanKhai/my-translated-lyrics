//! HTML-style escape for characters that would otherwise terminate
//! a WebVTT or SRT tag (`<`, `>`) or an HTML entity reference (`&`).
//!
//! WebVTT formally specifies `&lt;`, `&gt;`, and `&amp;` (plus a few
//! named entities such as `&nbsp;`) as the only escape sequences that
//! appear inside cue text. SRT has no formal spec, but every mainstream
//! player accepts the same HTML entity references in practice, so one
//! helper covers both renderers.

use core::fmt::{self, Write};

/// Wraps a string slice so its `Display` impl emits an HTML-escaped
/// form. Use this wherever source text flows into a `<c.…>…</c>`,
/// `<v …>…</v>`, `<font …>…</font>`, or similar tag body: raw `<`,
/// `>`, and `&` characters would otherwise terminate the tag early or
/// open an entity reference that was never intended.
pub struct Escaped<'a>(pub &'a str);

impl fmt::Display for Escaped<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ch in self.0.chars() {
            match ch {
                '<' => f.write_str("&lt;")?,
                '>' => f.write_str("&gt;")?,
                '&' => f.write_str("&amp;")?,
                _ => f.write_char(ch)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

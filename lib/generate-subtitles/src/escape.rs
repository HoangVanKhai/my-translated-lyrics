//! Small text-to-tag-body helpers shared by the SRT and WebVTT
//! renderers.
//!
//! [`Escaped`] is an HTML-style escape for characters that would
//! otherwise terminate a WebVTT or SRT tag (`<`, `>`) or open an HTML
//! entity reference (`&`). WebVTT formally specifies `&lt;`, `&gt;`,
//! and `&amp;` as the only escape sequences that appear inside cue
//! text. SRT has no formal spec, but every mainstream player accepts
//! the same HTML entity references in practice, so one helper covers
//! both renderers.
//!
//! [`append_separator_for_output`] reproduces a colon-free between-tag
//! separator captured from a credit source line. ASCII space/tab runs
//! round-trip verbatim so a multi-space gutter survives; every other
//! separator shape collapses to a single ASCII space. Separators that
//! carry a colon are handled by the renderers themselves, which place
//! the colon according to [`super::credits_parse::SeparatorStyle`].

use core::fmt::{self, Write};

/// Wraps a string slice so its `Display` impl emits an HTML-escaped
/// form. Use this wherever source text flows into a `<c....>...</c>`,
/// `<v ...>...</v>`, `<font ...>...</font>`, or similar tag body: raw `<`,
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

/// Appends a colon-free separator run from a credit source line into
/// the renderer's output buffer. ASCII space/tab runs pass through
/// verbatim so a multi-space gutter survives round-tripping; any
/// other separator shape (`\u{3000}` or mixed whitespace) collapses
/// to a single ASCII space on output. The renderers route only
/// [`super::credits_parse::SeparatorStyle::Spaces`] separators here;
/// colon-bearing separators never reach this function.
pub fn append_separator_for_output(output: &mut String, raw: &str) {
    if !raw.is_empty() && raw.chars().all(|ch| ch == ' ' || ch == '\t') {
        output.push_str(raw);
    } else {
        output.push(' ');
    }
}

#[cfg(test)]
mod tests;

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
//! Three helpers reproduce the role-to-name separator of a credit
//! line, selected by [`super::credits_parse::SeparatorStyle`].
//! [`role_span_suffix`] returns the colon that a Latin-script layout
//! tucks inside the role span; [`append_role_name_separator`] writes
//! the separator that sits between the role and name spans; and
//! [`append_separator_for_output`], which the latter delegates to for
//! the colon-free case, round-trips an ASCII space/tab gutter verbatim
//! and collapses any other whitespace shape to a single ASCII space.

use super::credits_parse::SeparatorStyle;
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

/// The colon, if any, that belongs inside the role's styled span. A
/// Latin-script credit line ([`SeparatorStyle::AsciiColon`]) tucks an
/// ASCII colon inside the role color; the CJK and colon-free layouts
/// contribute nothing here and place their separator between the spans
/// instead (see [`append_role_name_separator`]).
pub fn role_span_suffix(style: SeparatorStyle<'_>) -> &'static str {
    match style {
        SeparatorStyle::AsciiColon => ":",
        SeparatorStyle::FullWidthColon | SeparatorStyle::Spaces(_) => "",
    }
}

/// Appends the separator that sits between the role span and the name
/// span: one ASCII space after a Latin colon, a full-width colon for
/// the CJK layout, or the colon-free gutter reproduced verbatim by
/// [`append_separator_for_output`].
pub fn append_role_name_separator(output: &mut String, style: SeparatorStyle<'_>) {
    match style {
        SeparatorStyle::AsciiColon => output.push(' '),
        SeparatorStyle::FullWidthColon => output.push('：'),
        SeparatorStyle::Spaces(raw) => append_separator_for_output(output, raw),
    }
}

/// Appends a colon-free separator run from a credit source line into
/// the renderer's output buffer. ASCII space/tab runs pass through
/// verbatim so a multi-space gutter survives round-tripping; any
/// other separator shape (`\u{3000}` or mixed whitespace) collapses
/// to a single ASCII space on output. Only the colon-free
/// [`SeparatorStyle::Spaces`] case reaches this function, routed here
/// by [`append_role_name_separator`].
pub fn append_separator_for_output(output: &mut String, raw: &str) {
    if !raw.is_empty() && raw.chars().all(|ch| ch == ' ' || ch == '\t') {
        output.push_str(raw);
    } else {
        output.push(' ');
    }
}

#[cfg(test)]
mod tests;

//! `Display` wrapper for the CSS attribute selector that targets
//! a [`VoiceName`].
//!
//! [`VoiceName`] does not implement `Display` on its own because
//! the type is consumed in two contexts whose quoting rules
//! disagree (the WebVTT cue tag and the CSS attribute selector),
//! and a single `Display` impl could only be correct in one.
//! [`VoiceSelector`] is the CSS-side helper: it produces the
//! `v[voice="{name}"]` shape that goes inside `::cue(...)` in the
//! STYLE block. The cue-tag side is emitted directly by the
//! renderer, which writes `<v {name}>...</v>` into the per-cue
//! output buffer rather than constructing an intermediate value.
//!
//! The [`VoiceName`] reject list covers `"`, `\`, and line
//! terminators, which are exactly the characters that would break
//! the CSS double-quoted attribute-value string, so the selector
//! needs no additional escape.

use crate::line_markers_descriptor::VoiceName;
use core::fmt;

/// Renders the CSS attribute selector `v[voice="{name}"]` that
/// targets every voice span in the current cue scope whose voice
/// attribute matches `name`.
///
/// The caller wraps the result in `::cue(...)` to form the full
/// WebVTT STYLE-block selector.
pub struct VoiceSelector<'a>(pub &'a VoiceName);

impl fmt::Display for VoiceSelector<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v[voice=\"{name}\"]", name = self.0.as_str())
    }
}

#[cfg(test)]
mod tests;

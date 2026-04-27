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
//! [`VoiceName::new`] rejects `<`, `>`, `"`, `\`, `U+2028`,
//! `U+2029`, and any control character. Those characters are
//! exactly the set that would break either the WebVTT cue tag or
//! the CSS double-quoted attribute-value string, so neither side
//! needs an escape pass on top of the reject list.
//!
//! [`VoiceName::new`]: crate::line_markers_descriptor::VoiceName::new

use crate::line_markers_descriptor::VoiceName;
use derive_more::Display;

/// Renders the CSS attribute selector `v[voice="{name}"]` that
/// targets every voice span in the current cue scope whose voice
/// attribute matches `name`.
///
/// The caller wraps the result in `::cue(...)` to form the full
/// WebVTT STYLE-block selector.
#[derive(Display)]
#[display("v[voice=\"{name}\"]", name = _0.as_str())]
pub struct VoiceSelector<'a>(pub &'a VoiceName);

#[cfg(test)]
mod tests;

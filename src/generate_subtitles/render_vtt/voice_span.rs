//! Context-specific `Display` wrappers for [`VoiceName`].
//!
//! [`VoiceName`] does not implement `Display` on its own, so every
//! call site has to state whether it is building a WebVTT cue tag
//! or a CSS attribute selector. The two contexts have incompatible
//! quoting rules, and a single `Display` impl could only be
//! correct in one of them.
//!
//! - [`VoiceSpan`] renders `<v {name}>{inner}</v>` for cue-text
//!   output. The WebVTT cue-tag grammar reads the annotation up to
//!   the first `>`, so the [`VoiceName`] invariant keeps the tag
//!   well-formed. The inner body is expected to be already
//!   HTML-entity-escaped by the caller, because the cue-text parser
//!   resolves entity references inside the `<v>` body.
//! - [`VoiceSelector`] renders `v[voice="{name}"]` for the STYLE
//!   block. The [`VoiceName`] reject list covers `"`, `\`, and line
//!   terminators, which are exactly the characters that would break
//!   the CSS double-quoted attribute-value string, so no additional
//!   escape is needed here either.

use crate::line_markers_descriptor::VoiceName;
use core::fmt;

/// Renders a cue body with its surrounding `<v {name}>…</v>` voice
/// span. The spec name for this construct is "WebVTT cue voice
/// span".
///
/// The inner text must already be in cue-text form: plain prose
/// passed through the HTML-entity escape, or an already-wrapped
/// `<c.class>…</c>` fragment. This type does not escape the inner
/// text because the caller may have composed it from several
/// already-escaped pieces.
pub struct VoiceSpan<'a> {
    pub voice_name: &'a VoiceName,
    pub inner: &'a str,
}

impl fmt::Display for VoiceSpan<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<v {name}>{inner}</v>",
            name = self.voice_name.as_str(),
            inner = self.inner,
        )
    }
}

/// Renders the CSS attribute selector `v[voice="{name}"]` that
/// targets every voice span in the current cue scope whose voice
/// attribute matches `name`.
///
/// The caller wraps the result in `::cue(…)` to form the full
/// WebVTT STYLE-block selector.
pub struct VoiceSelector<'a>(pub &'a VoiceName);

impl fmt::Display for VoiceSelector<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v[voice=\"{name}\"]", name = self.0.as_str())
    }
}

#[cfg(test)]
mod tests;

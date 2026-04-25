//! WebVTT renderer.
//!
//! Each song is rendered as a `WEBVTT` header, followed by a single
//! `STYLE` block, followed by the cues themselves. The style block
//! emits a rule for every voice and named class declared in the
//! line-markers descriptor, in the order they appear in the `markers`
//! list. The three built-in credit classes, `creditRole`,
//! `creditName`, and `creditSpecial`, are emitted conditionally based
//! on what the cue bodies actually reference: songs without a credits
//! marker omit the role and name rules, and songs whose credits do
//! not use any bracketed highlight omit the `creditSpecial` rule.
//!
//! Each cue's body is wrapped according to the role its marker plays
//! in the descriptor:
//!
//! * Markers in [`LineMarkersDesc::voices`] wrap the line in
//!   `<v voice-name>...</v>`, with the voice name looked up per
//!   language.
//! * Markers in [`LineMarkersDesc::classes`] wrap the line in
//!   `<c.className>...</c>`, with the class name read from the map.
//! * Markers in [`LineMarkersDesc::credits`] go through the credit
//!   parser in [`super::credits_parse`] and emit one
//!   `<c.creditRole>role</c> <c.creditName>name</c>` pair per
//!   recognized cell.
//! * Any other marker emits the cue text unwrapped.
//!
//! [`LineMarkersDesc`]: crate::line_markers_descriptor::LineMarkersDesc
//! [`LineMarkersDesc::voices`]: crate::line_markers_descriptor::LineMarkersDesc::voices
//! [`LineMarkersDesc::classes`]: crate::line_markers_descriptor::LineMarkersDesc::classes
//! [`LineMarkersDesc::credits`]: crate::line_markers_descriptor::LineMarkersDesc::credits

use super::credits_parse::{
    CreditPair, CreditsVocabulary, NameSegment, ParseCreditError, parse_credit_line,
};
use super::escape::{Escaped, append_separator_for_output};
use super::parse::{CuePart, SubtitleCue};
use super::styles::{Style, class_style, voice_style};
use crate::credits_descriptor::CreditsDesc;
use crate::line_markers_descriptor::{LineMarkersDesc, VoiceName};
use crate::timestamp::{Timestamp, VttTime};
use crate::video_descriptor::Language;
use core::fmt::Write;
use derive_more::{Display, Error};
use text_block_macros::text_block_fnl;
use voice_span::{VoiceSelector, VoiceSpan};

mod voice_span;

/// Built-in class name for the role cell of a credit line.
const CLASS_CREDIT_ROLE: &str = "creditRole";
/// Built-in class name for the name cell of a credit line.
const CLASS_CREDIT_NAME: &str = "creditName";
/// Built-in class name for a bracketed highlight (`【...】`, `[...]`,
/// or `(...)`) inside a credit name.
const CLASS_CREDIT_SPECIAL: &str = "creditSpecial";

/// Fixed style for the credit role class.
const CREDIT_ROLE_STYLE: Style = Style {
    color: Some("#AAAA22"),
    italic: false,
    bold: false,
};
/// Fixed style for the credit name class.
const CREDIT_NAME_STYLE: Style = Style {
    color: Some("#AAAAAA"),
    italic: false,
    bold: false,
};
/// Fixed style for the credit highlight class.
const CREDIT_SPECIAL_STYLE: Style = Style {
    color: Some("#55ABCD"),
    italic: false,
    bold: false,
};

/// Renders all cues for a single language into a complete `.vtt` file.
pub fn render_vtt(
    cues: &[SubtitleCue],
    markers: &LineMarkersDesc,
    credits: &CreditsDesc,
    language: &Language,
) -> Result<String, RenderVttError> {
    let vocabulary = CreditsVocabulary::from_descriptor(credits, language);

    let mut cue_renderings: Vec<CueRendering> = Vec::with_capacity(cues.len());
    let mut features = Features::default();
    for cue in cues {
        let rendering = render_cue(cue, markers, &vocabulary, language)?;
        features.record(&rendering);
        cue_renderings.push(rendering);
    }

    let mut output = String::new();
    write!(output, "WEBVTT\nLanguage: {language}\n\n").unwrap();
    write_style_block(&mut output, markers, &features, language);
    output.push('\n');
    for rendering in &cue_renderings {
        writeln!(
            output,
            "{start} --> {end}",
            start = VttTime::from(rendering.start),
            end = VttTime::from(rendering.end),
        )
        .unwrap();
        output.push_str(&rendering.content);
        output.push_str("\n\n");
    }
    output.truncate(output.trim_end().len());
    output.push('\n');
    Ok(output)
}

/// Aggregated flags used to decide which built-in credit style rules
/// to emit. Voice and class rules are always emitted for every entry
/// in the line-markers descriptor; the credit styles are emitted
/// conditionally because the `creditSpecial` class, in particular,
/// appears only when a song's credits list includes a bracketed
/// highlight (`【...】`, `[...]`, or `(...)`).
#[derive(Debug, Default)]
struct Features {
    used_credit_role: bool,
    used_credit_name: bool,
    used_credit_special: bool,
}

impl Features {
    fn record(&mut self, rendering: &CueRendering) {
        if rendering.used_credit_role {
            self.used_credit_role = true;
        }
        if rendering.used_credit_name {
            self.used_credit_name = true;
        }
        if rendering.used_credit_special {
            self.used_credit_special = true;
        }
    }
}

struct CueRendering {
    start: Timestamp,
    end: Timestamp,
    content: String,
    used_credit_role: bool,
    used_credit_name: bool,
    used_credit_special: bool,
}

fn render_cue(
    cue: &SubtitleCue,
    markers: &LineMarkersDesc,
    vocabulary: &CreditsVocabulary,
    language: &Language,
) -> Result<CueRendering, RenderVttError> {
    let mut used_credit_role = false;
    let mut used_credit_name = false;
    let mut used_credit_special = false;

    let mut rendered_parts: Vec<String> = Vec::with_capacity(cue.parts.len());
    for part in &cue.parts {
        rendered_parts.push(render_cue_part(
            cue.start,
            part,
            markers,
            vocabulary,
            language,
            &mut used_credit_role,
            &mut used_credit_name,
            &mut used_credit_special,
        )?);
    }

    Ok(CueRendering {
        start: cue.start,
        end: cue.end,
        content: rendered_parts.join("\n"),
        used_credit_role,
        used_credit_name,
        used_credit_special,
    })
}

#[allow(clippy::too_many_arguments)]
fn render_cue_part(
    cue_start: Timestamp,
    part: &CuePart,
    markers: &LineMarkersDesc,
    vocabulary: &CreditsVocabulary,
    language: &Language,
    used_credit_role: &mut bool,
    used_credit_name: &mut bool,
    used_credit_special: &mut bool,
) -> Result<String, RenderVttError> {
    let marker = part.marker.as_str();

    let inner = if markers.credits.iter().any(|entry| entry == marker) {
        let mut rendered_lines: Vec<String> = Vec::new();
        for line in part.text.lines() {
            let pairs = parse_credit_line(line.trim_start(), vocabulary).map_err(|cause| {
                RenderVttError::Credits(Credits {
                    start: cue_start,
                    cause,
                })
            })?;
            rendered_lines.push(render_credit_line(
                &pairs,
                used_credit_role,
                used_credit_name,
                used_credit_special,
            ));
        }
        rendered_lines.join("\n")
    } else if let Some(class_name) = markers.classes.get(marker) {
        format!("<c.{class_name}>{text}</c>", text = Escaped(&part.text))
    } else {
        Escaped(&part.text).to_string()
    };

    let voice_name = markers
        .voices
        .get(marker)
        .and_then(|by_language| by_language.get(language));

    Ok(match voice_name {
        Some(voice_name) => VoiceSpan {
            voice_name,
            inner: &inner,
        }
        .to_string(),
        None => inner,
    })
}

fn render_credit_line(
    pairs: &[CreditPair],
    used_role: &mut bool,
    used_name: &mut bool,
    used_special: &mut bool,
) -> String {
    let mut output = String::new();
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        render_credit_pair(&mut output, pair, used_role, used_name, used_special);
    }
    output
}

fn render_credit_pair(
    output: &mut String,
    pair: &CreditPair,
    used_role: &mut bool,
    used_name: &mut bool,
    used_special: &mut bool,
) {
    *used_role = true;
    *used_name = true;
    write!(
        output,
        "<c.{CLASS_CREDIT_ROLE}>{role}</c>",
        role = Escaped(&pair.role),
    )
    .unwrap();
    append_separator_for_output(output, &pair.separator);
    write!(output, "<c.{CLASS_CREDIT_NAME}>").unwrap();
    write_name_segments(output, &pair.name_segments, used_special);
    output.push_str("</c>");
}

fn write_name_segments(output: &mut String, segments: &[NameSegment], used_special: &mut bool) {
    for segment in segments {
        match segment {
            NameSegment::Plain(text) => {
                write!(output, "{}", Escaped(text)).unwrap();
            }
            NameSegment::Special(text) => {
                *used_special = true;
                write!(
                    output,
                    "<c.{CLASS_CREDIT_SPECIAL}>{text}</c>",
                    text = Escaped(text.as_str()),
                )
                .unwrap();
            }
        }
    }
}

fn write_style_block(
    output: &mut String,
    markers: &LineMarkersDesc,
    features: &Features,
    language: &Language,
) {
    output.push_str(text_block_fnl! {
        "STYLE"
        "::cue {"
        "  background-color: transparent;"
        "  text-shadow: 2px 2px 2px black;"
        "}"
    });

    for marker_name in &markers.markers {
        let Some(voice_name) = markers
            .voices
            .get(marker_name)
            .and_then(|by_language| by_language.get(language))
        else {
            continue;
        };
        let style = voice_style(marker_name);
        write_voice_rule(output, voice_name, style.as_ref());
    }

    if features.used_credit_role {
        write_class_rule(output, CLASS_CREDIT_ROLE, &CREDIT_ROLE_STYLE);
    }
    if features.used_credit_name {
        write_class_rule(output, CLASS_CREDIT_NAME, &CREDIT_NAME_STYLE);
    }
    if features.used_credit_special {
        write_class_rule(output, CLASS_CREDIT_SPECIAL, &CREDIT_SPECIAL_STYLE);
    }

    for marker_name in &markers.markers {
        let Some(class_name) = markers.classes.get(marker_name) else {
            continue;
        };
        let Some(style) = class_style(class_name.as_str()) else {
            continue;
        };
        write_class_rule(output, class_name.as_str(), &style);
    }
}

fn write_voice_rule(output: &mut String, voice_name: &VoiceName, style: Option<&Style>) {
    writeln!(
        output,
        "::cue({selector}) {{",
        selector = VoiceSelector(voice_name),
    )
    .unwrap();
    if let Some(style) = style {
        write_style_body(output, style);
    }
    output.push_str("}\n");
}

fn write_class_rule(output: &mut String, class_name: &str, style: &Style) {
    writeln!(output, "::cue(c.{class_name}) {{").unwrap();
    write_style_body(output, style);
    output.push_str("}\n");
}

fn write_style_body(output: &mut String, style: &Style) {
    if let Some(color) = style.color {
        writeln!(output, "  color: {color};").unwrap();
    }
    if style.italic {
        output.push_str("  font-style: italic;\n");
    }
    if style.bold {
        output.push_str("  font-weight: bold;\n");
    }
}

/// Payload for [`RenderVttError::Credits`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("cue at {start} failed to render as a credit line: {cause}")]
pub struct Credits {
    pub start: Timestamp,
    pub cause: ParseCreditError,
}

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderVttError {
    Credits(#[error(not(source))] Credits),
}

#[cfg(test)]
mod tests;

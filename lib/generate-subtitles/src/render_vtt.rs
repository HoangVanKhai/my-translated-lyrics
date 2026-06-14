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
//!   `<c.creditRole>role</c><sep><c.creditName>name</c>` pair per
//!   recognized cell, where `<sep>` follows
//!   [`CreditPair::separator_style`]: a full-width colon between the
//!   spans, an ASCII colon inside the role class before a single
//!   space, or a verbatim ASCII space gutter.
//! * Any other marker emits the cue text unwrapped.
//!
//! [`LineMarkersDesc`]: lyrics_core::line_markers_descriptor::LineMarkersDesc
//! [`LineMarkersDesc::voices`]: lyrics_core::line_markers_descriptor::LineMarkersDesc::voices
//! [`LineMarkersDesc::classes`]: lyrics_core::line_markers_descriptor::LineMarkersDesc::classes
//! [`LineMarkersDesc::credits`]: lyrics_core::line_markers_descriptor::LineMarkersDesc::credits

use super::credits_parse::{
    CreditLead, CreditPair, CreditRoles, NameSegment, ParseCreditError, parse_credit_line,
};
use super::escape::Escaped;
use super::parse::{CuePart, SubtitleCue};
use super::styles::{MissingStyle, Style, StylePalette};
use core::fmt::Write;
use derive_more::{BitOrAssign, Display};
use lyrics_core::credits_descriptor::CreditsDesc;
use lyrics_core::line_markers_descriptor::{LineMarkersDesc, VoiceName};
use lyrics_core::timestamp::{Timestamp, VttTime};
use lyrics_core::video_descriptor::Language;
use text_block_macros::text_block_fnl;

/// Built-in class name for the role cell of a credit line.
const CLASS_CREDIT_ROLE: &str = "creditRole";
/// Built-in class name for the name cell of a credit line.
const CLASS_CREDIT_NAME: &str = "creditName";
/// Built-in class name for a bracketed highlight (`【...】`, `[...]`,
/// `(...)`, or `（...）`) inside a credit name.
const CLASS_CREDIT_SPECIAL: &str = "creditSpecial";

/// Renders all cues for a single language into a complete `.vtt` file.
pub fn render_vtt(
    cues: &[SubtitleCue],
    markers: &LineMarkersDesc,
    credits: &CreditsDesc,
    palette: &StylePalette,
    language: &Language,
) -> Result<String, RenderVttError> {
    let roles = CreditRoles::from_descriptor(credits, language);

    let mut cue_renderings = Vec::<CueRendering>::with_capacity(cues.len());
    let mut features = Features::default();
    for cue in cues {
        let rendering = render_cue(cue, markers, &roles, language)?;
        features |= rendering.features;
        cue_renderings.push(rendering);
    }

    let mut output = String::new();
    write!(output, "WEBVTT\nLanguage: {language}\n\n").unwrap();
    write_style_block(&mut output, markers, palette, &features, language)?;
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

/// Flags that record which built-in credit classes a cue (or, when
/// merged across cues, a whole song) actually used. Voice and class
/// rules are always emitted for every entry in the line-markers
/// descriptor; the credit styles are emitted conditionally because
/// the `creditSpecial` class, in particular, appears only when a
/// song's credits list includes a bracketed highlight (`【...】`,
/// `[...]`, `(...)`, or `（...）`).
///
/// The same shape is used at two levels: each `CueRendering` carries
/// the per-cue flags, and `render_vtt` keeps a song-level
/// accumulator that folds the per-cue flags in via `|=`.
#[derive(Debug, Default, Clone, Copy, BitOrAssign)]
struct Features {
    used_credit_role: bool,
    used_credit_name: bool,
    used_credit_special: bool,
}

struct CueRendering {
    start: Timestamp,
    end: Timestamp,
    content: String,
    features: Features,
}

fn render_cue(
    cue: &SubtitleCue,
    markers: &LineMarkersDesc,
    roles: &CreditRoles,
    language: &Language,
) -> Result<CueRendering, RenderVttError> {
    let mut content = String::new();
    let mut features = Features::default();

    for (index, part) in cue.parts.iter().enumerate() {
        if index > 0 {
            content.push('\n');
        }
        render_cue_part(
            &mut content,
            &mut features,
            cue.start,
            part,
            markers,
            roles,
            language,
        )?;
    }

    Ok(CueRendering {
        start: cue.start,
        end: cue.end,
        content,
        features,
    })
}

fn render_cue_part(
    output: &mut String,
    features: &mut Features,
    cue_start: Timestamp,
    part: &CuePart,
    markers: &LineMarkersDesc,
    roles: &CreditRoles,
    language: &Language,
) -> Result<(), RenderVttError> {
    let marker = &part.marker;
    let voice_name = markers
        .voices
        .get(marker)
        .and_then(|by_language| by_language.get(language));

    // `VoiceName::new` rejects `<`, `>`, `"`, `\`, `U+2028`,
    // `U+2029`, and any control character at the data boundary, so
    // splatting the name directly into the cue tag is safe without
    // an HTML-entity escape pass: none of the rejected characters
    // can break the tag, and `&` (which `VoiceName` allows) is left
    // verbatim because the WebVTT parser would decode `&amp;` back
    // to `&` and that would fall out of step with the CSS-side
    // selector.
    if let Some(voice_name) = voice_name {
        write!(output, "<v {}>", voice_name.as_str()).unwrap();
    }

    if markers.credits.contains(marker) {
        for (index, line) in part.text.lines().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            let pairs = parse_credit_line(line.trim_start(), roles).map_err(|cause| {
                RenderVttError::Credits(RenderVttErrorCreditsPayload {
                    start: cue_start,
                    cause,
                })
            })?;
            render_credit_line(output, features, &pairs);
        }
    } else if let Some(class_name) = markers.classes.get(marker) {
        write!(output, "<c.{class_name}>{}</c>", Escaped(&part.text)).unwrap();
    } else {
        write!(output, "{}", Escaped(&part.text)).unwrap();
    }

    if voice_name.is_some() {
        output.push_str("</v>");
    }

    Ok(())
}

fn render_credit_line(output: &mut String, features: &mut Features, pairs: &[CreditPair]) {
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        render_credit_pair(output, features, pair);
    }
}

fn render_credit_pair(output: &mut String, features: &mut Features, pair: &CreditPair) {
    let style = pair.separator_style();
    // The lead is a role (creditRole) or, on a role-less line, a
    // bracket highlight (creditSpecial); either carries any Latin
    // colon inside its own span.
    let (class, text) = match pair.lead {
        CreditLead::Role(role) => {
            features.used_credit_role = true;
            (CLASS_CREDIT_ROLE, role)
        }
        CreditLead::Special(bracket) => {
            features.used_credit_special = true;
            (CLASS_CREDIT_SPECIAL, bracket.as_str())
        }
    };
    write!(
        output,
        "<c.{class}>{text}{colon}</c>",
        text = Escaped(text),
        colon = style.role_span_suffix(),
    )
    .unwrap();
    // A role-only header line carries no name; emit just the lead.
    if pair.name_segments.is_empty() {
        return;
    }
    features.used_credit_name = true;
    style.append_between_spans(output);
    write!(output, "<c.{CLASS_CREDIT_NAME}>").unwrap();
    write_name_segments(output, features, &pair.name_segments);
    output.push_str("</c>");
}

fn write_name_segments(output: &mut String, features: &mut Features, segments: &[NameSegment]) {
    for segment in segments {
        match segment {
            NameSegment::Unbracketed(text) => {
                write!(output, "{}", Escaped(text.as_str())).unwrap();
            }
            NameSegment::Bracketed(text) => {
                features.used_credit_special = true;
                write!(
                    output,
                    "<c.{CLASS_CREDIT_SPECIAL}>{}</c>",
                    Escaped(text.as_str()),
                )
                .unwrap();
            }
        }
    }
}

fn write_style_block(
    output: &mut String,
    markers: &LineMarkersDesc,
    palette: &StylePalette,
    features: &Features,
    language: &Language,
) -> Result<(), RenderVttError> {
    output.push_str(text_block_fnl! {
        "STYLE"
        "::cue {"
        "  background-color: transparent;"
        "  text-shadow: 2px 2px 2px black;"
        "}"
    });

    for marker_name in &markers.markers {
        let Some(by_language) = markers.voices.get(marker_name) else {
            continue;
        };
        // Every declared voice marker must resolve to a palette style,
        // even when this particular language omits its voice name, so a
        // missing entry surfaces regardless of which language renders.
        let style = palette.voice_style(marker_name)?;
        let Some(voice_name) = by_language.get(language) else {
            continue;
        };
        write_voice_rule(output, voice_name, style);
    }

    if features.used_credit_role {
        write_class_rule(
            output,
            CLASS_CREDIT_ROLE,
            &Style::color_only(palette.credit.role.clone()),
        );
    }
    if features.used_credit_name {
        write_class_rule(
            output,
            CLASS_CREDIT_NAME,
            &Style::color_only(palette.credit.name.clone()),
        );
    }
    if features.used_credit_special {
        write_class_rule(
            output,
            CLASS_CREDIT_SPECIAL,
            &Style::color_only(palette.credit.special.clone()),
        );
    }

    for marker_name in &markers.markers {
        let Some(class_name) = markers.classes.get(marker_name) else {
            continue;
        };
        let style = palette.class_style(class_name)?;
        write_class_rule(output, class_name.as_str(), style);
    }

    Ok(())
}

/// `Display` wrapper that renders the CSS attribute selector
/// `v[voice="{name}"]` for a [`VoiceName`].
///
/// `VoiceName` does not implement `Display` on its own because the
/// type is consumed in two contexts whose quoting rules disagree
/// (the WebVTT cue tag and the CSS attribute selector), and a
/// single `Display` impl could only be correct in one. This wrapper
/// is the CSS-side helper: it produces the shape that goes inside
/// `::cue(...)` in the STYLE block. The cue-tag side is emitted
/// directly by [`render_cue_part`], which writes `<v {name}>...</v>`
/// into the per-cue output buffer rather than constructing an
/// intermediate value.
///
/// [`VoiceName::new`] rejects `<`, `>`, `"`, `\`, `U+2028`,
/// `U+2029`, and any control character. Those characters are
/// exactly the set that would break either the WebVTT cue tag or
/// the CSS double-quoted attribute-value string, so neither side
/// needs an escape pass on top of the reject list.
///
/// [`VoiceName::new`]: lyrics_core::line_markers_descriptor::VoiceName::new
#[derive(Display)]
#[display(r#"v[voice="{}"]"#, _0.as_str())]
struct VoiceSelector<'a>(&'a VoiceName);

fn write_voice_rule(output: &mut String, voice_name: &VoiceName, style: &Style) {
    writeln!(output, "::cue({}) {{", VoiceSelector(voice_name)).unwrap();
    write_style_body(output, style);
    output.push_str("}\n");
}

fn write_class_rule(output: &mut String, class_name: &str, style: &Style) {
    writeln!(output, "::cue(c.{class_name}) {{").unwrap();
    write_style_body(output, style);
    output.push_str("}\n");
}

fn write_style_body(output: &mut String, style: &Style) {
    if let Some(color) = &style.color {
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
pub struct RenderVttErrorCreditsPayload {
    pub start: Timestamp,
    pub cause: ParseCreditError,
}

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderVttError {
    Credits(RenderVttErrorCreditsPayload),
    Style(MissingStyle),
}

impl From<MissingStyle> for RenderVttError {
    fn from(error: MissingStyle) -> Self {
        RenderVttError::Style(error)
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod voice_selector_tests;

//! SubRip renderer.
//!
//! SRT has no concept of voices or classes, so presentation metadata
//! is expressed as inline HTML-like tags. For each cue this renderer
//! looks up the marker's style in the [`StylePalette`] and wraps the
//! text in `<font color="...">`, `<i>`, and/or `<b>` tags as required.
//! Credit lines go through the same role-driven parser as the VTT
//! renderer and emit each role and name with the palette's credit
//! colors, repeated inline because SRT has no central style definition.

use super::credits_parse::{
    CreditPair, CreditRoles, NameSegment, ParseCreditError, parse_credit_line,
};
use super::escape::{Escaped, append_separator_for_output};
use super::parse::{CuePart, SubtitleCue};
use super::styles::{MissingStyle, Style, StylePalette};
use core::fmt::Write;
use derive_more::Display;
use lyrics_core::credits_descriptor::CreditsDesc;
use lyrics_core::line_markers_descriptor::LineMarkersDesc;
use lyrics_core::timestamp::{SrtTime, Timestamp};
use lyrics_core::video_descriptor::Language;

/// Renders all cues for a single language into a complete `.srt` file.
pub fn render_srt(
    cues: &[SubtitleCue],
    markers: &LineMarkersDesc,
    credits: &CreditsDesc,
    palette: &StylePalette,
    language: &Language,
) -> Result<String, RenderSrtError> {
    let roles = CreditRoles::from_descriptor(credits, language);

    let mut output = String::new();
    for (cue_index, cue) in cues.iter().enumerate() {
        writeln!(output, "{}", cue_index + 1).unwrap();
        writeln!(
            output,
            "{start} --> {end}",
            start = SrtTime::from(cue.start),
            end = SrtTime::from(cue.end),
        )
        .unwrap();
        render_cue_body(&mut output, cue, markers, palette, &roles)?;
        output.push_str("\n\n");
    }
    output.truncate(output.trim_end().len());
    output.push('\n');
    Ok(output)
}

fn render_cue_body(
    output: &mut String,
    cue: &SubtitleCue,
    markers: &LineMarkersDesc,
    palette: &StylePalette,
    roles: &CreditRoles,
) -> Result<(), RenderSrtError> {
    for (index, part) in cue.parts.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        render_cue_part(output, cue.start, part, markers, palette, roles)?;
    }
    Ok(())
}

fn render_cue_part(
    output: &mut String,
    cue_start: Timestamp,
    part: &CuePart,
    markers: &LineMarkersDesc,
    palette: &StylePalette,
    roles: &CreditRoles,
) -> Result<(), RenderSrtError> {
    let marker = &part.marker;

    if markers.credits.contains(marker) {
        for (index, line) in part.text.lines().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            let pairs = parse_credit_line(line.trim_start(), roles).map_err(|cause| {
                RenderSrtError::Credits(RenderSrtErrorCreditsPayload {
                    start: cue_start,
                    cause,
                })
            })?;
            render_credit_line(output, palette, &pairs);
        }
        return Ok(());
    }

    let style = resolve_style(marker, markers, palette)?;
    wrap_with_style(output, &part.text, style);
    Ok(())
}

/// Looks up the SRT style for a marker by consulting the palette's voice
/// table first and then its class table. A marker declared as a voice or
/// a class but missing from the palette is a [`MissingStyle`] error;
/// markers that the descriptor registers as neither render as plain text.
fn resolve_style<'a>(
    marker_name: &str,
    markers: &LineMarkersDesc,
    palette: &'a StylePalette,
) -> Result<Option<&'a Style>, RenderSrtError> {
    if markers.voices.contains_key(marker_name) {
        return palette
            .voice_style(marker_name)
            .map(Some)
            .map_err(Into::into);
    }
    let Some(class_name) = markers.classes.get(marker_name) else {
        return Ok(None);
    };
    palette
        .class_style(class_name)
        .map(Some)
        .map_err(Into::into)
}

fn wrap_with_style(output: &mut String, text: &str, style: Option<&Style>) {
    let Some(style) = style else {
        write!(output, "{}", Escaped(text)).unwrap();
        return;
    };

    if style.bold {
        output.push_str("<b>");
    }
    if style.italic {
        output.push_str("<i>");
    }
    if let Some(color) = &style.color {
        write!(output, r#"<font color="{color}">"#).unwrap();
    }
    write!(output, "{}", Escaped(text)).unwrap();
    if style.color.is_some() {
        output.push_str("</font>");
    }
    if style.italic {
        output.push_str("</i>");
    }
    if style.bold {
        output.push_str("</b>");
    }
}

fn render_credit_line(output: &mut String, palette: &StylePalette, pairs: &[CreditPair]) {
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        render_credit_pair(output, palette, pair);
    }
}

fn render_credit_pair(output: &mut String, palette: &StylePalette, pair: &CreditPair) {
    write!(
        output,
        r#"<font color="{role}">{name}</font>"#,
        role = palette.credit.role,
        name = Escaped(pair.role),
    )
    .unwrap();
    append_separator_for_output(output, pair.separator);
    write!(output, r#"<font color="{}">"#, palette.credit.name).unwrap();
    write_name_segments(output, palette, &pair.name_segments);
    output.push_str("</font>");
}

fn write_name_segments(output: &mut String, palette: &StylePalette, segments: &[NameSegment]) {
    for segment in segments {
        match segment {
            NameSegment::Unbracketed(text) => {
                write!(output, "{}", Escaped(text.as_str())).unwrap();
            }
            NameSegment::Bracketed(text) => {
                write!(
                    output,
                    r#"<font color="{special}">{name}</font>"#,
                    special = palette.credit.special,
                    name = Escaped(text.as_str()),
                )
                .unwrap();
            }
        }
    }
}

/// Payload for [`RenderSrtError::Credits`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("cue at {start} failed to render as a credit line: {cause}")]
pub struct RenderSrtErrorCreditsPayload {
    pub start: Timestamp,
    pub cause: ParseCreditError,
}

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderSrtError {
    Credits(RenderSrtErrorCreditsPayload),
    Style(MissingStyle),
}

impl From<MissingStyle> for RenderSrtError {
    fn from(error: MissingStyle) -> Self {
        RenderSrtError::Style(error)
    }
}

#[cfg(test)]
mod tests;

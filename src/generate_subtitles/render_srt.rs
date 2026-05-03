//! SubRip renderer.
//!
//! SRT has no concept of voices or classes, so presentation metadata
//! is expressed as inline HTML-like tags. For each cue this renderer
//! looks up the marker's style in the central [`super::styles`] table
//! and wraps the text in `<font color="...">`, `<i>`, and/or `<b>`
//! tags as required. Credit lines go through the same vocabulary
//! parser as the VTT renderer and emit each role and name with a
//! hardcoded palette because SRT has no central style definition.

use super::credits_parse::{
    CreditPair, CreditsVocabulary, NameSegment, ParseCreditError, parse_credit_line,
};
use super::escape::{Escaped, append_separator_for_output};
use super::parse::{CuePart, SubtitleCue};
use super::styles::{
    CREDIT_NAME_COLOR, CREDIT_ROLE_COLOR, CREDIT_SPECIAL_COLOR, Style, class_style, voice_style,
};
use crate::credits_descriptor::CreditsDesc;
use crate::line_markers_descriptor::LineMarkersDesc;
use crate::timestamp::{SrtTime, Timestamp};
use crate::video_descriptor::Language;
use core::fmt::Write;
use derive_more::Display;

/// Renders all cues for a single language into a complete `.srt` file.
pub fn render_srt(
    cues: &[SubtitleCue],
    markers: &LineMarkersDesc,
    credits: &CreditsDesc,
    language: &Language,
) -> Result<String, RenderSrtError> {
    let vocabulary = CreditsVocabulary::from_descriptor(credits, language);

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
        render_cue_body(&mut output, cue, markers, &vocabulary)?;
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
    vocabulary: &CreditsVocabulary,
) -> Result<(), RenderSrtError> {
    for (index, part) in cue.parts.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        render_cue_part(output, cue.start, part, markers, vocabulary)?;
    }
    Ok(())
}

fn render_cue_part(
    output: &mut String,
    cue_start: Timestamp,
    part: &CuePart,
    markers: &LineMarkersDesc,
    vocabulary: &CreditsVocabulary,
) -> Result<(), RenderSrtError> {
    let marker = &part.marker;

    if markers.credits.contains(marker) {
        for (index, line) in part.text.lines().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            let pairs = parse_credit_line(line.trim_start(), vocabulary).map_err(|cause| {
                RenderSrtError::Credits(RenderSrtErrorCreditsPayload {
                    start: cue_start,
                    cause,
                })
            })?;
            render_credit_line(output, &pairs);
        }
        return Ok(());
    }

    let style = resolve_style(marker, markers);
    wrap_with_style(output, &part.text, style.as_ref());
    Ok(())
}

/// Looks up the SRT style for a marker by consulting the hardcoded
/// voice table first and then the class table. Markers that are not
/// registered in either table render as plain text.
fn resolve_style(marker_name: &str, markers: &LineMarkersDesc) -> Option<Style> {
    if let Some(style) = voice_style(marker_name) {
        return Some(style);
    }
    let class_name = markers.classes.get(marker_name)?;
    class_style(class_name.as_str())
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
    if let Some(color) = style.color {
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

fn render_credit_line(output: &mut String, pairs: &[CreditPair]) {
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        render_credit_pair(output, pair);
    }
}

fn render_credit_pair(output: &mut String, pair: &CreditPair) {
    write!(
        output,
        r#"<font color="{CREDIT_ROLE_COLOR}">{}</font>"#,
        Escaped(pair.role),
    )
    .unwrap();
    append_separator_for_output(output, pair.separator);
    write!(output, r#"<font color="{CREDIT_NAME_COLOR}">"#).unwrap();
    write_name_segments(output, &pair.name_segments);
    output.push_str("</font>");
}

fn write_name_segments(output: &mut String, segments: &[NameSegment]) {
    for segment in segments {
        match segment {
            NameSegment::Plain(text) => {
                write!(output, "{}", Escaped(text)).unwrap();
            }
            NameSegment::Special(text) => {
                write!(
                    output,
                    r#"<font color="{CREDIT_SPECIAL_COLOR}">{}</font>"#,
                    Escaped(text.as_str()),
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
}

#[cfg(test)]
mod tests;

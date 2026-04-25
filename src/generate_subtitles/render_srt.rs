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
use super::styles::{Style, class_style, voice_style};
use crate::credits_descriptor::CreditsDesc;
use crate::line_markers_descriptor::LineMarkersDesc;
use crate::timestamp::{SrtTime, Timestamp};
use crate::video_descriptor::Language;
use core::fmt::Write;
use derive_more::{Display, Error};

const CREDIT_ROLE_COLOR: &str = "#AAAA22";
const CREDIT_NAME_COLOR: &str = "#AAAAAA";
const CREDIT_SPECIAL_COLOR: &str = "#55ABCD";

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
        let body = render_cue_body(cue, markers, &vocabulary)?;
        output.push_str(&body);
        output.push_str("\n\n");
    }
    output.truncate(output.trim_end().len());
    output.push('\n');
    Ok(output)
}

fn render_cue_body(
    cue: &SubtitleCue,
    markers: &LineMarkersDesc,
    vocabulary: &CreditsVocabulary,
) -> Result<String, RenderSrtError> {
    let mut rendered_parts: Vec<String> = Vec::with_capacity(cue.parts.len());
    for part in &cue.parts {
        rendered_parts.push(render_cue_part(cue.start, part, markers, vocabulary)?);
    }
    Ok(rendered_parts.join("\n"))
}

fn render_cue_part(
    cue_start: Timestamp,
    part: &CuePart,
    markers: &LineMarkersDesc,
    vocabulary: &CreditsVocabulary,
) -> Result<String, RenderSrtError> {
    let marker = part.marker.as_str();

    if markers.credits.iter().any(|entry| entry == marker) {
        let mut rendered_lines: Vec<String> = Vec::new();
        for line in part.text.lines() {
            let pairs = parse_credit_line(line.trim_start(), vocabulary).map_err(|cause| {
                RenderSrtError::Credits(Credits {
                    start: cue_start,
                    cause,
                })
            })?;
            rendered_lines.push(render_credit_line(&pairs));
        }
        return Ok(rendered_lines.join("\n"));
    }

    let style = resolve_style(marker, markers);
    Ok(wrap_with_style(&part.text, style.as_ref()))
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

fn wrap_with_style(text: &str, style: Option<&Style>) -> String {
    let Some(style) = style else {
        return Escaped(text).to_string();
    };

    let mut output = String::new();
    if style.bold {
        output.push_str("<b>");
    }
    if style.italic {
        output.push_str("<i>");
    }
    if let Some(color) = style.color {
        write!(output, "<font color=\"{color}\">").unwrap();
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
    output
}

fn render_credit_line(pairs: &[CreditPair]) -> String {
    let mut output = String::new();
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        render_credit_pair(&mut output, pair);
    }
    output
}

fn render_credit_pair(output: &mut String, pair: &CreditPair) {
    write!(
        output,
        "<font color=\"{CREDIT_ROLE_COLOR}\">{role}</font>",
        role = Escaped(&pair.role),
    )
    .unwrap();
    append_separator_for_output(output, &pair.separator);
    write!(output, "<font color=\"{CREDIT_NAME_COLOR}\">").unwrap();
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
                    "<font color=\"{CREDIT_SPECIAL_COLOR}\">{text}</font>",
                    text = Escaped(text.as_str()),
                )
                .unwrap();
            }
        }
    }
}

/// Payload for [`RenderSrtError::Credits`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("cue at {start} failed to render as a credit line: {cause}")]
pub struct Credits {
    pub start: Timestamp,
    pub cause: ParseCreditError,
}

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderSrtError {
    Credits(#[error(not(source))] Credits),
}

#[cfg(test)]
mod tests;

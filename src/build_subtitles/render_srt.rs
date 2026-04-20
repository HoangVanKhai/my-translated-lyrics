//! SubRip renderer.
//!
//! SRT has no concept of voices or classes, so presentation metadata
//! is expressed as inline HTML-like tags. For each cue this renderer
//! looks up the marker's style in the central [`super::styles`] table
//! and wraps the text in `<font color="...">`, `<i>`, and/or `<b>`
//! tags as required. Credit lines follow the same structural parsing
//! as in the VTT renderer and render each role and name with a
//! hard-coded palette because SRT has no central style definition.

use super::credits_parse::{
    CreditPair, CreditsVocabulary, NameSegment, ParseCreditError, ParsedCreditLine,
};
use super::parse::SubtitleCue;
use super::styles::{Style, class_style, voice_style};
use crate::credits_descriptor::CreditsDesc;
use crate::line_markers_descriptor::LineMarkersDesc;
use crate::timestamp::{Milliseconds, SrtTime};
use crate::video_descriptor::Language;
use core::fmt::Write;
use derive_more::{Display, Error};

const CREDIT_ROLE_COLOR: &str = "#AAAA22";
const CREDIT_NAME_COLOR: &str = "#AAAAAA";
const CREDIT_SPECIAL_COLOR: &str = "#55ABCD";

/// Renders all cues for a single language into a complete `.srt` file.
pub fn render_file(
    cues: &[SubtitleCue],
    markers: &LineMarkersDesc,
    credits: &CreditsDesc,
    language: &Language,
) -> Result<String, RenderSrtError> {
    let vocabulary = CreditsVocabulary::from_descriptor(credits, language);

    let mut output = String::new();
    for (cue_index, cue) in cues.iter().enumerate() {
        writeln!(output, "{}", cue_index + 1).expect("writing to String is infallible");
        writeln!(
            output,
            "{start} --> {end}",
            start = SrtTime(cue.start),
            end = SrtTime(cue.end),
        )
        .expect("writing to String is infallible");
        let body = render_cue_body(cue, markers, &vocabulary)?;
        output.push_str(&body);
        output.push_str("\n\n");
    }
    let trimmed = output.trim_end().to_string();
    Ok(format!("{trimmed}\n"))
}

fn render_cue_body(
    cue: &SubtitleCue,
    markers: &LineMarkersDesc,
    vocabulary: &CreditsVocabulary,
) -> Result<String, RenderSrtError> {
    let marker = cue.marker.as_deref();

    if let Some(marker) = marker
        && markers.credits.iter().any(|entry| entry == marker)
    {
        let mut rendered_lines: Vec<String> = Vec::new();
        for line in cue.text.lines() {
            let trimmed = line.trim_start();
            let parsed =
                vocabulary
                    .parse_line(trimmed)
                    .map_err(|source| RenderSrtError::Credits {
                        start: cue.start,
                        source,
                    })?;
            rendered_lines.push(render_credit_line(&parsed));
        }
        return Ok(rendered_lines.join("\n"));
    }

    let text = cue.text.clone();
    let style = marker.and_then(|marker_name| resolve_style(marker_name, markers));
    Ok(wrap_with_style(&text, style.as_ref()))
}

/// Looks up the SRT style for a marker by consulting the hardcoded
/// voice table first and then the class table. Markers that are not
/// registered in either table render as plain text.
fn resolve_style(marker_name: &str, markers: &LineMarkersDesc) -> Option<Style> {
    if let Some(style) = voice_style(marker_name) {
        return Some(style);
    }
    let class_name = markers.classes.get(marker_name)?;
    class_style(class_name)
}

fn wrap_with_style(text: &str, style: Option<&Style>) -> String {
    let Some(style) = style else {
        return text.to_string();
    };

    let mut wrapped = text.to_string();
    if let Some(color) = style.color {
        wrapped = format!("<font color=\"{color}\">{wrapped}</font>");
    }
    if style.italic {
        wrapped = format!("<i>{wrapped}</i>");
    }
    if style.bold {
        wrapped = format!("<b>{wrapped}</b>");
    }
    wrapped
}

fn render_credit_line(parsed: &ParsedCreditLine) -> String {
    let mut output = String::new();
    for (index, pair) in parsed.pairs.iter().enumerate() {
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
        role = pair.role,
    )
    .expect("writing to String is infallible");

    output.push_str(&render_separator_for_output(&pair.separator));

    write!(output, "<font color=\"{CREDIT_NAME_COLOR}\">")
        .expect("writing to String is infallible");
    for segment in &pair.name_segments {
        match segment {
            NameSegment::Plain(text) => output.push_str(text),
            NameSegment::Name(name) => output.push_str(name),
            NameSegment::Special(text) => {
                write!(
                    output,
                    "<font color=\"{CREDIT_SPECIAL_COLOR}\">{text}</font>"
                )
                .expect("writing to String is infallible");
            }
        }
    }
    output.push_str("</font>");
}

fn render_separator_for_output(raw: &str) -> String {
    if !raw.is_empty() && raw.chars().all(|ch| ch == ' ' || ch == '\t') {
        raw.to_string()
    } else {
        " ".to_string()
    }
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum RenderSrtError {
    #[display("cue at {start} failed to render as a credit line: {source}")]
    Credits {
        #[error(not(source))]
        start: Milliseconds,
        source: ParseCreditError,
    },
}

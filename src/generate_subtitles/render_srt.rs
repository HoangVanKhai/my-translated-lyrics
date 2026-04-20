//! SubRip renderer.
//!
//! SRT has no concept of voices or classes, so presentation metadata
//! is expressed as inline HTML-like tags. For each cue this renderer
//! looks up the marker's style in the central [`super::styles`] table
//! and wraps the text in `<font color="...">`, `<i>`, and/or `<b>`
//! tags as required. Credit lines follow the same structural parsing
//! as in the VTT renderer and render each role and name with a
//! hard-coded palette because SRT has no central style definition.

use super::credits_parse::{CreditPair, NameSegment, parse_credit_line};
use super::parse::SubtitleCue;
use super::styles::{Style, class_style, voice_style};
use crate::line_markers_descriptor::LineMarkersDesc;
use crate::timestamp::SrtTime;
use crate::video_descriptor::Language;
use core::fmt::Write;

const CREDIT_ROLE_COLOR: &str = "#AAAA22";
const CREDIT_NAME_COLOR: &str = "#AAAAAA";
const CREDIT_SPECIAL_COLOR: &str = "#55ABCD";

/// Renders all cues for a single language into a complete `.srt` file.
pub fn render_file(cues: &[SubtitleCue], markers: &LineMarkersDesc, language: &Language) -> String {
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
        let body = render_cue_body(cue, markers, language);
        output.push_str(&body);
        output.push_str("\n\n");
    }
    let trimmed = output.trim_end().to_string();
    format!("{trimmed}\n")
}

fn render_cue_body(cue: &SubtitleCue, markers: &LineMarkersDesc, language: &Language) -> String {
    let marker = cue.marker.as_str();

    if markers.credits.iter().any(|entry| entry == marker) {
        let mut rendered_lines: Vec<String> = Vec::new();
        for line in cue.text.lines() {
            let pairs = parse_credit_line(line.trim_start(), language);
            rendered_lines.push(render_credit_line(&pairs));
        }
        return rendered_lines.join("\n");
    }

    let style = resolve_style(marker, markers);
    wrap_with_style(&cue.text, style.as_ref())
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
    if let Some(role) = &pair.role {
        write!(output, "<font color=\"{CREDIT_ROLE_COLOR}\">{role}</font>")
            .expect("writing to String is infallible");
        output.push_str(&pair.separator);
        write!(output, "<font color=\"{CREDIT_NAME_COLOR}\">")
            .expect("writing to String is infallible");
        write_name_segments(output, &pair.name_segments);
        output.push_str("</font>");
    } else {
        write_name_segments(output, &pair.name_segments);
    }
}

fn write_name_segments(output: &mut String, segments: &[NameSegment]) {
    for segment in segments {
        match segment {
            NameSegment::Plain(text) => output.push_str(text),
            NameSegment::Special(text) => {
                write!(
                    output,
                    "<font color=\"{CREDIT_SPECIAL_COLOR}\">{text}</font>"
                )
                .expect("writing to String is infallible");
            }
        }
    }
}

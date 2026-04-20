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
//! not use any `【...】` highlight omit the `creditSpecial` rule.

use super::credits_parse::{CreditPair, NameSegment, parse_credit_line};
use super::parse::SubtitleCue;
use super::styles::{Style, class_style, voice_style};
use crate::line_markers_descriptor::LineMarkersDesc;
use crate::timestamp::{Milliseconds, VttTime};
use crate::video_descriptor::Language;
use core::fmt::Write;

/// Built-in class name for the role cell of a credit line.
const CLASS_CREDIT_ROLE: &str = "creditRole";
/// Built-in class name for the name cell of a credit line.
const CLASS_CREDIT_NAME: &str = "creditName";
/// Built-in class name for a `【...】` highlight inside a credit name.
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
pub fn render_file(cues: &[SubtitleCue], markers: &LineMarkersDesc, language: &Language) -> String {
    let mut cue_renderings: Vec<CueRendering> = Vec::with_capacity(cues.len());
    let mut features = Features::default();
    for cue in cues {
        let rendering = render_cue(cue, markers, language);
        features.record(&rendering);
        cue_renderings.push(rendering);
    }

    let mut output = String::new();
    write!(output, "WEBVTT\nLanguage: {language}\n\n").expect("writing to String is infallible");
    write_style_block(&mut output, markers, &features, language);
    output.push('\n');
    for rendering in &cue_renderings {
        writeln!(
            output,
            "{start} --> {end}",
            start = VttTime(rendering.start),
            end = VttTime(rendering.end),
        )
        .expect("writing to String is infallible");
        output.push_str(&rendering.content);
        output.push_str("\n\n");
    }
    let trimmed = output.trim_end().to_string();
    format!("{trimmed}\n")
}

/// Aggregated flags used to decide which built-in credit style rules
/// to emit. Voice and class rules are always emitted for every entry
/// in the line-markers descriptor; the credit styles are emitted
/// conditionally because the `creditSpecial` class, in particular,
/// appears only when a song's credits list includes a `【...】`
/// highlight.
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
    start: Milliseconds,
    end: Milliseconds,
    content: String,
    used_credit_role: bool,
    used_credit_name: bool,
    used_credit_special: bool,
}

fn render_cue(cue: &SubtitleCue, markers: &LineMarkersDesc, language: &Language) -> CueRendering {
    let marker = cue.marker.as_str();
    let mut used_credit_role = false;
    let mut used_credit_name = false;
    let mut used_credit_special = false;

    let inner = if markers.credits.iter().any(|entry| entry == marker) {
        let mut rendered_lines: Vec<String> = Vec::new();
        for line in cue.text.lines() {
            let pairs = parse_credit_line(line.trim_start(), language);
            rendered_lines.push(render_credit_line(
                &pairs,
                &mut used_credit_role,
                &mut used_credit_name,
                &mut used_credit_special,
            ));
        }
        rendered_lines.join("\n")
    } else if let Some(class_name) = markers.classes.get(marker) {
        format!("<c.{class_name}>{text}</c>", text = cue.text)
    } else {
        cue.text.clone()
    };

    let voice_name = markers
        .voices
        .get(marker)
        .and_then(|by_language| by_language.get(language));

    let content = match voice_name {
        Some(voice_name) => format!("<v {voice_name}>{inner}</v>"),
        None => inner,
    };

    CueRendering {
        start: cue.start,
        end: cue.end,
        content,
        used_credit_role,
        used_credit_name,
        used_credit_special,
    }
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
    if let Some(role) = &pair.role {
        *used_role = true;
        *used_name = true;
        write!(output, "<c.{CLASS_CREDIT_ROLE}>{role}</c>")
            .expect("writing to String is infallible");
        output.push_str(&pair.separator);
        write!(output, "<c.{CLASS_CREDIT_NAME}>").expect("writing to String is infallible");
        write_name_segments(output, &pair.name_segments, used_special);
        output.push_str("</c>");
    } else {
        write_name_segments(output, &pair.name_segments, used_special);
    }
}

fn write_name_segments(output: &mut String, segments: &[NameSegment], used_special: &mut bool) {
    for segment in segments {
        match segment {
            NameSegment::Plain(text) => output.push_str(text),
            NameSegment::Special(text) => {
                *used_special = true;
                write!(output, "<c.{CLASS_CREDIT_SPECIAL}>{text}</c>")
                    .expect("writing to String is infallible");
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
    output.push_str("STYLE\n");
    output.push_str("::cue {\n");
    output.push_str("  background-color: transparent;\n");
    output.push_str("  text-shadow: 2px 2px 2px black;\n");
    output.push_str("}\n");

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
        let Some(style) = class_style(class_name) else {
            continue;
        };
        write_class_rule(output, class_name, &style);
    }
}

fn write_voice_rule(output: &mut String, voice_name: &str, style: Option<&Style>) {
    writeln!(output, "::cue(v[voice=\"{voice_name}\"]) {{")
        .expect("writing to String is infallible");
    if let Some(style) = style {
        write_style_body(output, style);
    }
    output.push_str("}\n");
}

fn write_class_rule(output: &mut String, class_name: &str, style: &Style) {
    writeln!(output, "::cue(c.{class_name}) {{").expect("writing to String is infallible");
    write_style_body(output, style);
    output.push_str("}\n");
}

fn write_style_body(output: &mut String, style: &Style) {
    if let Some(color) = style.color {
        writeln!(output, "  color: {color};").expect("writing to String is infallible");
    }
    if style.italic {
        output.push_str("  font-style: italic;\n");
    }
    if style.bold {
        output.push_str("  font-weight: bold;\n");
    }
}

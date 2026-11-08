//! Parser for `lyrics.{lang}.txt` cue files.
//!
//! Each file is a sequence of timestamped events. A line that starts
//! with `MM:SS.mmm` opens an event. If the event's first non-whitespace
//! token is [`ReservedMarker::Clear`], the currently open cue is closed
//! at that timestamp; if it is [`ReservedMarker::EndOfVideo`], the line
//! is ignored. Any other event opens a new cue; continuation lines that
//! lack a leading timestamp are appended to the most recently opened
//! cue.
//!
//! A line at the shorthand column whose marker is
//! [`ReservedMarker::Annotation`] attaches commentary to the cue part
//! above it. It carries no timestamp and opens no event. Each line
//! opens one annotation, which takes continuation lines of its own.
//!
//! Between a column-zero `<additive>` line and a `</additive>` line,
//! cues accumulate rather than replace: each renders the parts of
//! every cue above it in the region, then its own. Regions do not
//! nest, enclose at least one cue, and admit neither
//! [`ReservedMarker::Clear`] nor [`ReservedMarker::EndOfVideo`].
//!
//! Every parser here consumes a prefix of the unread lines and hands
//! back the tail.

pub mod error;

use crate::take::{take_leading_whitespace, take_non_whitespace};
use core::iter::once;
use error::{
    AdditiveRegionError, ControlMarkerInRegion, CueTextReservedCharacter, EmptyAnnotation,
    EmptyCueBody, EmptyRegion, ExtraTextAfterControlMarker, InvalidTimestamp, MalformedHeader,
    MalformedIndentation, MalformedTagLine, MissingMarker, MissingSeparatorAfterTimestamp,
    NestedRegion, OrphanedAnnotation, OrphanedShorthandMarker, OutOfOrder, ParseLyricsError,
    ParseLyricsErrorKind, RepeatedTimestamp, TabIndentation, UnclosedCue, UnclosedRegion,
    UnopenedRegion,
};
use lyrics_core::line_markers_descriptor::{MarkerName, ReservedMarker};
use lyrics_core::timestamp::{TIMESTAMP_STR_LEN, TakeTimestampError, Timestamp};
use pipe_trait::Pipe;
use strum::EnumString;

/// Indent width of a line that opens a new marker at the same start
/// time as the cue immediately above.
const TIMESTAMP_PREFIX_WIDTH: usize = TIMESTAMP_STR_LEN + 1;

/// A subtitle cue with a resolved end time, ready for rendering.
///
/// A cue groups one or more [`CuePart`]s that share a start time.
/// Each part carries its own marker and text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubtitleCue {
    /// Timestamp at which the cue begins to display. Read directly
    /// from the `MM:SS.mmm` prefix on the cue-opening line.
    pub start: Timestamp,
    /// Timestamp at which the cue stops displaying. Taken from the
    /// timestamp of the next event in the source file, whether that
    /// is the next cue or a `clr` sentinel.
    pub end: Timestamp,
    /// One or more parts that share this cue's start and end times.
    /// Each part carries its own marker and text.
    pub parts: Vec<CuePart>,
}

/// One marker-text pair within a [`SubtitleCue`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CuePart {
    /// The leading marker token the line declared, for example `ttl`
    /// in `ttl: 《Song》`.
    pub marker: String,
    /// Cue text, with line breaks preserved between the opening line
    /// and any continuation lines.
    pub text: String,
    /// Commentary attached to this part by `ann` lines beneath it,
    /// one entry per line.
    pub annotations: Vec<String>,
}

/// One line of a source file that takes part in the grammar, split
/// into the indent that positions it and the body that follows.
#[derive(Clone, Copy)]
struct Line<'a> {
    /// One-based number of the line within the file.
    number: usize,
    /// How many ASCII spaces the line is indented by.
    indent: usize,
    /// The line with its indent removed. Never blank and never a
    /// comment.
    body: &'a str,
}

/// The unread remainder of a source file.
#[derive(Clone, Copy)]
struct Input<'a> {
    /// The unread text, always positioned at the start of a line.
    text: &'a str,
    /// The number the next line carries.
    number: usize,
    /// Line number of the `<additive>` enclosing the unread text, or
    /// `None` outside a region.
    region: Option<usize>,
}

impl<'a> Input<'a> {
    /// The whole of `content`, read from its first line and outside
    /// any region.
    fn new(content: &'a str) -> Self {
        Input {
            text: content,
            number: 1,
            region: None,
        }
    }

    /// The same position, read as the inside of the region opened at
    /// `opened_at`.
    fn inside(self, opened_at: usize) -> Self {
        Input {
            region: Some(opened_at),
            ..self
        }
    }

    /// The same position, read as outside any region.
    fn outside(self) -> Self {
        Input {
            region: None,
            ..self
        }
    }

    /// Consumes the next line that carries content, passing over the
    /// blank and comment lines above it. Lines are split as
    /// [`str::lines`] splits them.
    fn take_content_line(self) -> Result<Option<(Line<'a>, Self)>, ParseLyricsError> {
        if self.text.is_empty() {
            return Ok(None);
        }
        let (raw, tail) = match self.text.split_once('\n') {
            Some((raw, tail)) => (raw.strip_suffix('\r').unwrap_or(raw), tail),
            None => (self.text, ""),
        };
        let number = self.number;
        let rest = Input {
            text: tail,
            number: number + 1,
            ..self
        };
        if raw.trim().is_empty() || raw.trim_start().starts_with('#') {
            return rest.take_content_line();
        }
        if raw.trim_start_matches(' ').starts_with('\t') {
            return TabIndentation
                .pipe(ParseLyricsErrorKind::TabIndentation)
                .pipe(at_line(number))
                .pipe(Err);
        }
        let indent = raw.bytes().take_while(|&byte| byte == b' ').count();
        let line = Line {
            number,
            indent,
            body: &raw[indent..],
        };
        Ok(Some((line, rest)))
    }

    /// Consumes the next line that any parser here reads.
    ///
    /// Outside a region an `eov` line is passed over too, since the
    /// format ignores it entirely. Inside a region the line is handed
    /// back.
    fn take_line(self) -> Result<Option<(Line<'a>, Self)>, ParseLyricsError> {
        let Some((line, rest)) = self.take_content_line()? else {
            return Ok(None);
        };
        if self.region.is_none() && line.indent == 0 && is_end_of_video(line) {
            return rest.take_line();
        }
        Ok(Some((line, rest)))
    }
}

/// A tag name.
#[derive(Clone, Copy, Debug, strum::Display, EnumString, Eq, PartialEq)]
enum TagName {
    /// Opens a region whose cues accumulate.
    #[strum(serialize = "additive")]
    Additive,
}

impl TagName {
    /// Consumes a leading tag name and returns it with the unconsumed
    /// tail.
    fn take(source: &str) -> Option<(Self, &str)> {
        let end = source
            .char_indices()
            .find(|&(_, char)| !is_tag_name_char(char))
            .map_or(source.len(), |(index, _)| index);
        Some((source[..end].parse().ok()?, &source[end..]))
    }
}

/// Whether `char` may continue a [`TagName`].
fn is_tag_name_char(char: char) -> bool {
    char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-'
}

/// An opening tag. Takes the form of `<tag>`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OpeningTag(TagName);

impl OpeningTag {
    /// Consumes a leading opening tag and returns it with the
    /// unconsumed tail.
    fn take(source: &str) -> Option<(Self, &str)> {
        let after_delimiter = source.strip_prefix('<')?;
        let (name, after_name) = TagName::take(after_delimiter)?;
        let tail = after_name.strip_prefix('>')?;
        Some((OpeningTag(name), tail))
    }

    /// The name between the delimiters.
    fn name(self) -> TagName {
        self.0
    }
}

/// A closing tag. Takes the form of `</tag>`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ClosingTag(TagName);

impl ClosingTag {
    /// Consumes a leading closing tag and returns it with the
    /// unconsumed tail.
    fn take(source: &str) -> Option<(Self, &str)> {
        let after_delimiter = source.strip_prefix("</")?;
        let (name, after_name) = TagName::take(after_delimiter)?;
        let tail = after_name.strip_prefix('>')?;
        Some((ClosingTag(name), tail))
    }

    /// The name between the delimiters.
    fn name(self) -> TagName {
        self.0
    }
}

/// Whether `body` is an `<additive>` line. Nothing but trailing
/// whitespace may follow the tag.
fn is_opening_additive_tag(body: &str) -> bool {
    matches!(
        OpeningTag::take(body),
        Some((tag, tail)) if tag.name() == TagName::Additive && tail.trim().is_empty(),
    )
}

/// Whether `body` is a `</additive>` line, under the same rule as
/// [`is_opening_additive_tag`].
fn is_closing_additive_tag(body: &str) -> bool {
    matches!(
        ClosingTag::take(body),
        Some((tag, tail)) if tag.name() == TagName::Additive && tail.trim().is_empty(),
    )
}

/// Payload of an [`Event::Cue`]: the start time declared in the
/// source file and the parts collected under it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct CueGroup {
    start: Timestamp,
    /// Line of the header that opened the group.
    opened_at: usize,
    parts: Vec<CuePart>,
}

/// One `<additive>` region, holding the cue groups written between
/// its two tags.
///
/// A region encloses at least one cue, which the first group states
/// in the type.
#[derive(Clone, Debug, Eq, PartialEq)]
struct AdditiveRegion {
    first: CueGroup,
    rest: Vec<CueGroup>,
}

impl AdditiveRegion {
    /// The region `region` with `group` written beneath it, or the
    /// region that `group` opens when it is the first one enclosed.
    fn extended(region: Option<Self>, group: CueGroup) -> Self {
        match region {
            None => AdditiveRegion {
                first: group,
                rest: Vec::new(),
            },
            Some(mut region) => {
                region.rest.push(group);
                region
            }
        }
    }

    /// The region's cue groups, in the order they were written.
    fn groups(&self) -> impl Iterator<Item = &CueGroup> {
        once(&self.first).chain(&self.rest)
    }

    /// The last group the region encloses.
    fn last(&self) -> &CueGroup {
        self.rest.last().unwrap_or(&self.first)
    }
}

/// An intermediate event extracted from a source file before end times
/// are resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Cue(CueGroup),
    Clear(Timestamp),
    Region(AdditiveRegion),
}

impl Event {
    /// The start time the event opens with.
    fn first_start(&self) -> Timestamp {
        match self {
            Event::Cue(group) => group.start,
            Event::Clear(start) => *start,
            Event::Region(region) => region.first.start,
        }
    }

    /// The start time the event ends with.
    fn last_start(&self) -> Timestamp {
        match self {
            Event::Cue(group) => group.start,
            Event::Clear(start) => *start,
            Event::Region(region) => region.last().start,
        }
    }
}

/// Parses `content` into a list of cues ordered by start time.
pub fn parse_lyrics(content: &str) -> Result<Vec<SubtitleCue>, ParseLyricsError> {
    let events = collect_events(content)?;
    resolve_cues(&events)
}

/// Locates a [`ParseLyricsErrorKind`] at `line_number`, forming the
/// [`ParseLyricsError`] the parser returns.
fn at_line(line_number: usize) -> impl Fn(ParseLyricsErrorKind) -> ParseLyricsError {
    move |kind| ParseLyricsError { line_number, kind }
}

/// Reads `content` as a sequence of top-level elements.
fn collect_events(content: &str) -> Result<Vec<Event>, ParseLyricsError> {
    let mut events = Vec::<Event>::new();
    let mut input = Input::new(content);

    while let Some((line, rest)) = input.take_line()? {
        let previous_start = events.last().map(Event::last_start);
        let (event, tail) = take_element(line, rest, previous_start)?;
        if let Some(event) = event {
            events.push(event);
        }
        input = tail;
    }

    Ok(events)
}

/// Consumes the element that `line` opens, with every line beneath it
/// that the element owns, and returns it with the unconsumed tail.
///
/// `rest` is the tail that follows `line`, and `previous_start` the
/// start time of the event most recently recorded. The element
/// contributes no event when the line is ignored entirely.
fn take_element<'a>(
    line: Line<'a>,
    rest: Input<'a>,
    previous_start: Option<Timestamp>,
) -> Result<(Option<Event>, Input<'a>), ParseLyricsError> {
    match parse_element_line(line)? {
        ElementLine::OpeningTag => {
            let (region, tail) = take_additive_region(rest, line.number, previous_start)?;
            Ok((Some(Event::Region(region)), tail))
        }
        // A closing tag is consumed by the parser that consumed its
        // opening tag, so one reaching the top level closes nothing.
        ElementLine::ClosingTag => UnopenedRegion
            .pipe(AdditiveRegionError::Unopened)
            .pipe(ParseLyricsErrorKind::AdditiveRegion)
            .pipe(at_line(line.number))
            .pipe(Err),
        ElementLine::Header(start, Header::Control(ReservedMarker::Clear)) => {
            check_event_order(start, previous_start).map_err(at_line(line.number))?;
            Ok((Some(Event::Clear(start)), rest))
        }
        // The only other control marker is `eov`, which no line
        // reaching here can carry. Ignoring it entirely is what the
        // format asks for.
        ElementLine::Header(_, Header::Control(_)) => Ok((None, rest)),
        ElementLine::Header(start, Header::Cue(body)) => {
            check_event_order(start, previous_start).map_err(at_line(line.number))?;
            let header = PartHeader::parse(body).map_err(at_line(line.number))?;
            let (group, tail) = take_cue_group(start, line.number, header, rest)?;
            Ok((Some(Event::Cue(group)), tail))
        }
    }
}

/// What a line standing where an element may begin declares. The
/// indent rules and the tag spellings are the same inside a region as
/// outside one.
enum ElementLine<'a> {
    /// An `<additive>` line.
    OpeningTag,
    /// A `</additive>` line.
    ClosingTag,
    /// A header line, carrying the timestamp it opens with and what
    /// the body after that timestamp declares.
    Header(Timestamp, Header<'a>),
}

/// Reads a line that stands where an element may begin.
fn parse_element_line(line: Line<'_>) -> Result<ElementLine<'_>, ParseLyricsError> {
    if line.indent == TIMESTAMP_PREFIX_WIDTH {
        return Err(orphaned_shorthand_line(line));
    }
    if line.indent != 0 {
        return Err(malformed_indentation(line, None));
    }
    // A tag line is the one column-zero shape that carries no
    // timestamp, so it is recognized before the header parser sees the
    // line. Nothing else in the format opens with `<`.
    if is_opening_additive_tag(line.body) {
        return Ok(ElementLine::OpeningTag);
    }
    if is_closing_additive_tag(line.body) {
        return Ok(ElementLine::ClosingTag);
    }
    if line.body.starts_with('<') {
        return Err(malformed_tag_line(line));
    }
    let (start, header) = parse_header(line)?;
    Ok(ElementLine::Header(start, header))
}

/// Consumes the cue groups an `<additive>` region encloses and the
/// `</additive>` that closes it, returning the region with the
/// unconsumed tail.
///
/// The opening tag has already been consumed; `opened_at` is the line
/// it stood on, and `input` the tail that follows it.
fn take_additive_region<'a>(
    input: Input<'a>,
    opened_at: usize,
    previous_start: Option<Timestamp>,
) -> Result<(AdditiveRegion, Input<'a>), ParseLyricsError> {
    // The region built so far, which is `None` until its first cue
    // group opens it.
    let mut region: Option<AdditiveRegion> = None;
    let mut input = input.inside(opened_at);

    loop {
        let Some((line, rest)) = input.take_line()? else {
            return UnclosedRegion
                .pipe(AdditiveRegionError::Unclosed)
                .pipe(ParseLyricsErrorKind::AdditiveRegion)
                .pipe(at_line(opened_at))
                .pipe(Err);
        };

        match parse_element_line(line)? {
            ElementLine::ClosingTag => {
                let Some(region) = region else {
                    return opened_at
                        .pipe(EmptyRegion)
                        .pipe(AdditiveRegionError::Empty)
                        .pipe(ParseLyricsErrorKind::AdditiveRegion)
                        .pipe(at_line(line.number))
                        .pipe(Err);
                };
                return Ok((region, rest.outside()));
            }
            // Regions do not nest, so an `<additive>` met here is
            // reported where it stands rather than opening a region.
            ElementLine::OpeningTag => {
                return opened_at
                    .pipe(NestedRegion)
                    .pipe(AdditiveRegionError::Nested)
                    .pipe(ParseLyricsErrorKind::AdditiveRegion)
                    .pipe(at_line(line.number))
                    .pipe(Err);
            }
            // A region admits neither control marker.
            ElementLine::Header(_, Header::Control(marker)) => {
                return ControlMarkerInRegion { marker, opened_at }
                    .pipe(AdditiveRegionError::ControlMarker)
                    .pipe(ParseLyricsErrorKind::AdditiveRegion)
                    .pipe(at_line(line.number))
                    .pipe(Err);
            }
            ElementLine::Header(start, Header::Cue(body)) => {
                let previous = region
                    .as_ref()
                    .map(|region| region.last().start)
                    .or(previous_start);
                check_event_order(start, previous).map_err(at_line(line.number))?;
                let header = PartHeader::parse(body).map_err(at_line(line.number))?;
                let (group, tail) = take_cue_group(start, line.number, header, rest)?;
                region = Some(AdditiveRegion::extended(region, group));
                input = tail;
            }
        }
    }
}

/// Consumes the lines beneath the header that opened a cue at `start`
/// with `header`, returning the group with the unconsumed tail.
///
/// The group owns every line written beneath that header: the
/// shorthand marker lines that add parts to it, the annotations that
/// attach notes to those parts, and the continuation lines each of
/// those takes. It ends at the first column-zero line, which opens
/// the next element.
fn take_cue_group<'a>(
    start: Timestamp,
    opened_at: usize,
    header: PartHeader<'a>,
    input: Input<'a>,
) -> Result<(CueGroup, Input<'a>), ParseLyricsError> {
    // The part the shorthand column is currently writing into, held
    // aside from the parts a later shorthand line has closed.
    let (mut open, mut input) = take_cue_part(header, input)?;
    // A group of one part is the common case, so the vector is sized
    // for it.
    let mut parts = Vec::<CuePart>::with_capacity(1);
    let annotation_indent = continuation_indent(ReservedMarker::Annotation.as_ref());

    while let Some((line, rest)) = input.take_line()? {
        // A part accepts every line indented at its continuation width
        // and rejects every other width, so the line reached here
        // stands at column zero or at the shorthand column.
        if line.indent != TIMESTAMP_PREFIX_WIDTH {
            break;
        }
        match parse_shorthand_line(line.body).map_err(at_line(line.number))? {
            ShorthandLine::Annotation(text) => {
                let (annotation, tail) = take_body(
                    text.to_string(),
                    rest,
                    annotation_indent,
                    Continued::AnnotationText,
                )?;
                open.annotations.push(annotation);
                input = tail;
            }
            ShorthandLine::Part(header) => {
                let (part, tail) = take_cue_part(header, rest)?;
                parts.push(open);
                open = part;
                input = tail;
            }
        }
    }

    parts.push(open);
    Ok((
        CueGroup {
            start,
            opened_at,
            parts,
        },
        input,
    ))
}

/// Consumes the continuation lines that extend the text of the part
/// `header` opens, returning the part with the unconsumed tail. The
/// annotations written beneath the part are the cue group's to read.
fn take_cue_part<'a>(
    header: PartHeader<'a>,
    input: Input<'a>,
) -> Result<(CuePart, Input<'a>), ParseLyricsError> {
    let PartHeader { marker, text } = header;
    let indent = continuation_indent(marker.as_str());
    let (text, input) = take_body(text.to_string(), input, indent, Continued::CueText)?;
    let part = CuePart {
        marker: marker.into(),
        text,
        annotations: Vec::new(),
    };
    Ok((part, input))
}

/// Which body a run of continuation lines extends. Only cue text is
/// checked for the reserved tag delimiters; `<` and `>` are ordinary
/// punctuation in annotation text.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Continued {
    CueText,
    AnnotationText,
}

/// Reads the body of a part or an annotation: the `opening` text, then
/// the continuation lines indented by `indent` beneath it, each
/// appended to the opening under a newline. Returns that text with the
/// unconsumed tail.
///
/// The first line carrying another indent opens something else, so it
/// must stand at column zero, where a header or a tag line does, or at
/// the shorthand column, where a new part or an annotation does. Any
/// other indent is reported against the two widths in force.
fn take_body<'a>(
    opening: String,
    input: Input<'a>,
    indent: usize,
    continued: Continued,
) -> Result<(String, Input<'a>), ParseLyricsError> {
    let mut text = opening;
    let mut input = input;

    while let Some((line, rest)) = input.take_line()? {
        if line.indent == indent {
            if continued == Continued::CueText {
                reject_reserved_cue_text_characters(line.body).map_err(at_line(line.number))?;
            }
            text.push('\n');
            text.push_str(line.body);
            input = rest;
            continue;
        }
        if line.indent != 0 && line.indent != TIMESTAMP_PREFIX_WIDTH {
            return Err(malformed_indentation(line, Some(indent)));
        }
        break;
    }

    Ok((text, input))
}

/// What a line at the shorthand column declares.
enum ShorthandLine<'a> {
    /// An `ann` line, carrying the note it attaches to the part above
    /// it.
    Annotation(&'a str),
    /// A marker line, opening a part of its own.
    Part(PartHeader<'a>),
}

/// Reads a whole line at the shorthand column. The marker alone
/// decides which of the two shapes the line is.
fn parse_shorthand_line(body: &str) -> Result<ShorthandLine<'_>, ParseLyricsErrorKind> {
    let Some((marker, text)) = split_marker(body) else {
        reject_reserved_cue_text_characters(body)?;
        return body
            .to_string()
            .pipe(MissingMarker)
            .pipe(ParseLyricsErrorKind::MissingMarker)
            .pipe(Err);
    };
    if matches!(marker.parse(), Ok(ReservedMarker::Annotation)) {
        if text.is_empty() {
            return EmptyAnnotation
                .pipe(ParseLyricsErrorKind::EmptyAnnotation)
                .pipe(Err);
        }
        return Ok(ShorthandLine::Annotation(text));
    }
    reject_reserved_cue_text_characters(body)?;
    PartHeader::from_parts(marker, text).map(ShorthandLine::Part)
}

/// The marker and the text that a `marker: text` line declares, which
/// together open one [`CuePart`].
struct PartHeader<'a> {
    marker: MarkerName,
    text: &'a str,
}

impl<'a> PartHeader<'a> {
    /// Reads a whole `marker: text` body, rejecting the bodies that
    /// no cue part may carry.
    fn parse(body: &'a str) -> Result<Self, ParseLyricsErrorKind> {
        reject_reserved_cue_text_characters(body)?;
        let (marker, text) = split_marker(body)
            .ok_or(body)
            .map_err(String::from)
            .map_err(MissingMarker)
            .map_err(ParseLyricsErrorKind::MissingMarker)?;
        PartHeader::from_parts(marker, text)
    }

    /// Checks the two halves of a body that has already been split.
    fn from_parts(marker: &str, text: &'a str) -> Result<Self, ParseLyricsErrorKind> {
        let marker = marker
            .to_string()
            .pipe(MarkerName::new)
            .map_err(ParseLyricsErrorKind::InvalidMarkerName)?;
        if text.is_empty() {
            return marker
                .pipe(String::from)
                .pipe(EmptyCueBody)
                .pipe(ParseLyricsErrorKind::EmptyCueBody)
                .pipe(Err);
        }
        Ok(PartHeader { marker, text })
    }
}

/// What the body of a column-zero header line declares.
enum Header<'a> {
    /// A control marker standing alone on the line.
    Control(ReservedMarker),
    /// The body of a line that opens a cue, handed on unparsed for
    /// the caller to read as a [`PartHeader`].
    Cue(&'a str),
}

/// Reads a whole header line: an `MM:SS.mmm` timestamp, the
/// whitespace that separates it from the body, and a body that either
/// names a control marker or opens a cue.
fn parse_header(line: Line<'_>) -> Result<(Timestamp, Header<'_>), ParseLyricsError> {
    let (start, after_timestamp) = Timestamp::take(line.body)
        .map_err(|cause| match cause {
            TakeTimestampError::ShapeMismatch => line
                .body
                .to_string()
                .pipe(MalformedHeader)
                .pipe(ParseLyricsErrorKind::MalformedHeader),
            cause => cause
                .pipe(InvalidTimestamp)
                .pipe(ParseLyricsErrorKind::InvalidTimestamp),
        })
        .map_err(at_line(line.number))?;

    let (separator, cue_body) = take_leading_whitespace(after_timestamp);
    if separator.is_empty() {
        return line
            .body
            .to_string()
            .pipe(MissingSeparatorAfterTimestamp)
            .pipe(ParseLyricsErrorKind::MissingSeparatorAfterTimestamp)
            .pipe(at_line(line.number))
            .pipe(Err);
    }

    let (first_token, after_token) = take_non_whitespace(cue_body);
    if let Ok(marker) = first_token.parse::<ReservedMarker>()
        && marker.is_control()
    {
        let trailing = after_token.trim();
        if !trailing.is_empty() {
            return ExtraTextAfterControlMarker {
                marker,
                trailing: trailing.to_string(),
            }
            .pipe(ParseLyricsErrorKind::ExtraTextAfterControlMarker)
            .pipe(at_line(line.number))
            .pipe(Err);
        }
        return Ok((start, Header::Control(marker)));
    }

    Ok((start, Header::Cue(cue_body)))
}

/// Whether `line` is an `eov` line, which the format ignores
/// entirely. A line the header parser rejects is not one.
fn is_end_of_video(line: Line<'_>) -> bool {
    matches!(
        parse_header(line),
        Ok((_, Header::Control(ReservedMarker::EndOfVideo))),
    )
}

/// The indent a continuation of a `marker: ` line must carry: the
/// timestamp column, plus the marker name, its colon and one ASCII
/// space.
fn continuation_indent(marker: &str) -> usize {
    TIMESTAMP_PREFIX_WIDTH + marker.len() + 2
}

/// Rejects a new event whose start time matches or precedes the most
/// recent recorded event, whose start time is `previous`.
fn check_event_order(
    start: Timestamp,
    previous: Option<Timestamp>,
) -> Result<(), ParseLyricsErrorKind> {
    let Some(previous) = previous else {
        return Ok(());
    };
    if previous == start {
        return start
            .pipe(RepeatedTimestamp)
            .pipe(ParseLyricsErrorKind::RepeatedTimestamp)
            .pipe(Err);
    }
    if start < previous {
        return OutOfOrder {
            previous,
            next: start,
        }
        .pipe(ParseLyricsErrorKind::OutOfOrder)
        .pipe(Err);
    }
    Ok(())
}

/// The diagnostic for a shorthand-column line that no cue is open
/// above. An annotation reports its own.
fn orphaned_shorthand_line(line: Line<'_>) -> ParseLyricsError {
    let kind = if annotation_body(line.body).is_some() {
        line.body
            .to_string()
            .pipe(OrphanedAnnotation)
            .pipe(ParseLyricsErrorKind::OrphanedAnnotation)
    } else {
        line.body
            .to_string()
            .pipe(OrphanedShorthandMarker)
            .pipe(ParseLyricsErrorKind::OrphanedShorthandMarker)
    };
    kind.pipe(at_line(line.number))
}

/// The diagnostic for a line whose indent matches no width the parser
/// accepts where it stands. `continuation` is the width a continuation
/// would have carried, or `None` when no body is open for one to
/// continue.
fn malformed_indentation(line: Line<'_>, continuation: Option<usize>) -> ParseLyricsError {
    MalformedIndentation {
        actual: line.indent,
        shorthand_indent: TIMESTAMP_PREFIX_WIDTH,
        continuation_indent: continuation,
    }
    .pipe(ParseLyricsErrorKind::MalformedIndentation)
    .pipe(at_line(line.number))
}

/// The diagnostic for a column-zero line that opens with `<` but
/// spells neither tag.
fn malformed_tag_line(line: Line<'_>) -> ParseLyricsError {
    line.body
        .to_string()
        .pipe(MalformedTagLine)
        .pipe(ParseLyricsErrorKind::MalformedTagLine)
        .pipe(at_line(line.number))
}

/// Resolves the end time of every cue in `events` and flattens them
/// into the rendered order.
fn resolve_cues(events: &[Event]) -> Result<Vec<SubtitleCue>, ParseLyricsError> {
    let mut cues = Vec::<SubtitleCue>::new();

    for (index, event) in events.iter().enumerate() {
        let following = events.get(index + 1).map(Event::first_start);
        match event {
            Event::Clear(_) => continue,
            Event::Cue(group) => cues.push(resolve_group(group, Vec::new(), following)?),
            Event::Region(region) => cues.extend(resolve_region(region, following)?),
        }
    }

    Ok(cues)
}

/// Resolves the cues of one additive region, which is the one place
/// where a cue renders the parts of the cues above it.
///
/// `following` is the start time of the event after the region, which
/// closes its last cue.
fn resolve_region(
    region: &AdditiveRegion,
    following: Option<Timestamp>,
) -> Result<Vec<SubtitleCue>, ParseLyricsError> {
    let mut cues = Vec::<SubtitleCue>::new();
    let mut carried = Vec::<CuePart>::new();
    let mut groups = region.groups().peekable();

    while let Some(group) = groups.next() {
        let next = groups.peek().map(|group| group.start).or(following);
        let cue = resolve_group(group, carried, next)?;
        carried = cue.parts.iter().map(carried_part).collect();
        cues.push(cue);
    }

    Ok(cues)
}

/// Builds the cue of one group, whose own parts render below
/// `carried`, and which ends at `following`.
fn resolve_group(
    group: &CueGroup,
    carried: Vec<CuePart>,
    following: Option<Timestamp>,
) -> Result<SubtitleCue, ParseLyricsError> {
    let end = following
        .ok_or(group.start)
        .map_err(UnclosedCue)
        .map_err(ParseLyricsErrorKind::UnclosedCue)
        .map_err(at_line(group.opened_at))?;
    let mut parts = carried;
    parts.extend(group.parts.iter().cloned());
    Ok(SubtitleCue {
        start: group.start,
        end,
        parts,
    })
}

/// A copy of `part` for the cues below it in the same additive
/// region.
///
/// The copy carries the marker and text of `part` but none of its
/// annotations.
fn carried_part(part: &CuePart) -> CuePart {
    CuePart {
        marker: part.marker.clone(),
        text: part.text.clone(),
        annotations: Vec::new(),
    }
}

/// The text of a well-formed annotation line, or `None` when the body
/// does not carry the marker followed by an ASCII `:`.
fn annotation_body(body: &str) -> Option<&str> {
    let (marker, text) = split_marker(body)?;
    matches!(marker.parse(), Ok(ReservedMarker::Annotation)).then_some(text)
}

/// Splits a line body like `marker: text` into its two halves. Returns
/// `None` when the line has no `:` separator or when the marker half
/// is empty.
fn split_marker(body: &str) -> Option<(&str, &str)> {
    let (head, tail) = body.split_once(':')?;
    let marker = head.trim();
    if marker.is_empty() {
        return None;
    }
    Some((marker, tail.trim()))
}

/// Rejects cue text containing `<` or `>`, which the WebVTT cue-tag
/// grammar reserves as tag delimiters. Reports the first offender
/// only.
fn reject_reserved_cue_text_characters(text: &str) -> Result<(), ParseLyricsErrorKind> {
    if let Some(character) = text.chars().find(|&c| matches!(c, '<' | '>')) {
        return character
            .pipe(CueTextReservedCharacter)
            .pipe(ParseLyricsErrorKind::CueTextReservedCharacter)
            .pipe(Err);
    }
    Ok(())
}

#[cfg(test)]
mod test_additive_regions;
#[cfg(test)]
mod test_annotations;
#[cfg(test)]
mod test_control_markers;
#[cfg(test)]
mod test_cues;
#[cfg(test)]
mod test_event_order;
#[cfg(test)]
mod test_line_shape;
#[cfg(test)]
mod test_region_diagnostics;
#[cfg(test)]
mod test_reserved_characters;
#[cfg(test)]
mod test_tags;

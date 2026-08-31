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
//! above it. It carries no timestamp, opens no event, and reaches
//! neither renderer. Each line opens one annotation, which takes
//! continuation lines of its own.
//!
//! Between a column-zero `<additive>` line and a `</additive>` line,
//! cues accumulate rather than replace: each renders the parts of
//! every cue above it in the region, then its own. Regions do not
//! nest, enclose at least one cue, and admit neither
//! [`ReservedMarker::Clear`] nor [`ReservedMarker::EndOfVideo`].
//!
//! Every parser here consumes a prefix of the unread lines and hands
//! back the tail, so a construct that encloses others is parsed as one
//! value by one parser.

pub mod error;

use crate::take::{take_leading_whitespace, take_non_whitespace};
use core::iter::once;
use error::{
    AdditiveRegionError, ControlMarkerInRegion, CueTextReservedCharacter, EmptyAnnotation,
    EmptyCueBody, EmptyRegion, ExtraTextAfterControlMarker, InvalidTimestamp, MalformedHeader,
    MalformedIndentation, MalformedTagLine, MissingMarker, MissingSeparatorAfterTimestamp,
    NestedRegion, OrphanedAnnotation, OrphanedShorthandMarker, OutOfOrder, ParseLyricsError,
    RepeatedTimestamp, ReservedControlMarker, TabIndentation, UnclosedCue, UnclosedRegion,
    UnopenedRegion,
};
use lyrics_core::line_markers_descriptor::ReservedMarker;
use lyrics_core::timestamp::{TIMESTAMP_STR_LEN, TakeTimestampError, Timestamp};
use pipe_trait::Pipe;
use strum::EnumString;

/// Indent width of a line that opens a new marker at the same start
/// time as the cue immediately above. Equals the byte length of an
/// `MM:SS.mmm` timestamp plus one ASCII space.
const TIMESTAMP_PREFIX_WIDTH: usize = TIMESTAMP_STR_LEN + 1;

/// A subtitle cue with a resolved end time, ready for rendering.
///
/// A cue groups one or more [`CuePart`]s that share a start time.
/// Each part carries its own marker and text; the renderer emits
/// the parts as a single subtitle block whose body has one line
/// per part.
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
    /// Each part carries its own marker and text and renders to a
    /// separate line within the resulting SRT or VTT cue block.
    pub parts: Vec<CuePart>,
}

/// One marker-text pair within a [`SubtitleCue`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CuePart {
    /// The leading marker token that the cue-opening line declared, for
    /// example `ttl` in `ttl: 《Song》`.
    pub marker: String,
    /// Cue text, with line breaks preserved between the opening line
    /// and any continuation lines.
    pub text: String,
    /// Commentary attached to this part by `ann` lines beneath it,
    /// one entry per line. Neither renderer reads it.
    pub annotations: Vec<String>,
}

/// One line of a source file that takes part in the grammar, split
/// into the indent that positions it and the body that follows.
#[derive(Clone, Copy)]
struct Line<'a> {
    /// One-based number of the line within the file.
    number: usize,
    /// How many ASCII spaces the line is indented by. The indent
    /// selects which construct the line opens or extends.
    indent: usize,
    /// The line with its indent removed. Never blank, and never a
    /// comment, because [`Cursor::take_content_line`] passes over both.
    body: &'a str,
}

/// The unread remainder of a source file: what is left to parse, the
/// line it stands at, and the region enclosing it.
#[derive(Clone, Copy)]
struct Cursor<'a> {
    /// The unread text, always positioned at the start of a line.
    text: &'a str,
    /// The number the next line carries.
    number: usize,
    /// Line number of the `<additive>` enclosing the unread text, or
    /// `None` outside a region.
    region: Option<usize>,
}

impl<'a> Cursor<'a> {
    /// A cursor over the whole of `content`, outside any region.
    fn new(content: &'a str) -> Self {
        Cursor {
            text: content,
            number: 1,
            region: None,
        }
    }

    /// The same position, read as the inside of the region opened at
    /// `opened_at`.
    fn inside(self, opened_at: usize) -> Self {
        Cursor {
            region: Some(opened_at),
            ..self
        }
    }

    /// The same position, read as outside any region. Regions do not
    /// nest, so what encloses the text after a `</additive>` is
    /// always nothing.
    fn outside(self) -> Self {
        Cursor {
            region: None,
            ..self
        }
    }

    /// Consumes the next line that carries content, passing over the
    /// blank and comment lines above it. Lines are split as
    /// [`str::lines`] splits them.
    fn take_content_line(self) -> Result<Option<(Line<'a>, Self)>, ParseLyricsError> {
        let mut rest = self;
        while !rest.text.is_empty() {
            let (raw, tail) = match rest.text.split_once('\n') {
                Some((raw, tail)) => (raw.strip_suffix('\r').unwrap_or(raw), tail),
                None => (rest.text, ""),
            };
            let number = rest.number;
            rest = Cursor {
                text: tail,
                number: number + 1,
                ..rest
            };
            if raw.trim().is_empty() || raw.trim_start().starts_with('#') {
                continue;
            }
            if raw.trim_start_matches(' ').starts_with('\t') {
                return TabIndentation {
                    line_number: number,
                }
                .pipe(ParseLyricsError::TabIndentation)
                .pipe(Err);
            }
            let indent = raw.bytes().take_while(|&byte| byte == b' ').count();
            let line = Line {
                number,
                indent,
                body: &raw[indent..],
            };
            return Ok(Some((line, rest)));
        }
        Ok(None)
    }

    /// Consumes the next line that any parser here reads.
    ///
    /// Outside a region an `eov` line is passed over too. The format
    /// ignores it entirely, so it leaves the cue above it open and a
    /// continuation line beneath one still extends that cue. Inside a
    /// region the line is handed back, for [`take_additive_region`] to
    /// reject.
    fn take_line(self) -> Result<Option<(Line<'a>, Self)>, ParseLyricsError> {
        let mut rest = self;
        while let Some((line, tail)) = rest.take_content_line()? {
            if rest.region.is_none() && line.indent == 0 && is_end_of_video(line) {
                rest = tail;
                continue;
            }
            return Ok(Some((line, tail)));
        }
        Ok(None)
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

/// Whether `body` is an `<additive>` line. A tag carries no
/// attributes, so nothing but trailing whitespace may follow it.
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

/// Payload of an [`Event::Cue`]. The start time is the one declared
/// in the source file; the end time is resolved later by looking at
/// the next event in the stream. The list of parts mirrors
/// [`SubtitleCue::parts`]: a fresh timestamped header line opens a
/// group with one part, a column-[`TIMESTAMP_PREFIX_WIDTH`] shorthand
/// line appends a new part to that group, and a continuation line
/// extends the most recent part's text.
#[derive(Clone, Debug, Eq, PartialEq)]
struct CueGroup {
    start: Timestamp,
    parts: Vec<CuePart>,
}

/// One `<additive>` region, holding the cue groups written between
/// its two tags.
///
/// A region exists to accumulate cues, so it encloses at least one.
/// Splitting the first group from the rest states that in the type:
/// the region can be built no other way, and the code that resolves
/// end times never has to ask whether a region is empty.
#[derive(Clone, Debug, Eq, PartialEq)]
struct AdditiveRegion {
    first: CueGroup,
    rest: Vec<CueGroup>,
}

impl AdditiveRegion {
    /// Builds a region from the groups collected between its tags.
    /// `closed_at` is the line of the `</additive>` that closes it,
    /// and `opened_at` the line of the `<additive>` that opened it.
    fn new(
        groups: Vec<CueGroup>,
        closed_at: usize,
        opened_at: usize,
    ) -> Result<Self, ParseLyricsError> {
        let mut groups = groups.into_iter();
        let Some(first) = groups.next() else {
            return EmptyRegion {
                line_number: closed_at,
                opened_at,
            }
            .pipe(AdditiveRegionError::Empty)
            .pipe(ParseLyricsError::AdditiveRegion)
            .pipe(Err);
        };
        Ok(AdditiveRegion {
            first,
            rest: groups.collect(),
        })
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
    /// The start time the event opens with, which closes the event
    /// before it.
    fn first_start(&self) -> Timestamp {
        match self {
            Event::Cue(group) => group.start,
            Event::Clear(start) => *start,
            Event::Region(region) => region.first.start,
        }
    }

    /// The start time the event ends with, which the event after it
    /// must follow.
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

/// Reads `content` as a sequence of top-level elements.
fn collect_events(content: &str) -> Result<Vec<Event>, ParseLyricsError> {
    let mut events = Vec::<Event>::new();
    let mut cursor = Cursor::new(content);

    while let Some((line, rest)) = cursor.take_line()? {
        let previous_start = events.last().map(Event::last_start);
        let (event, tail) = take_element(line, rest, previous_start)?;
        if let Some(event) = event {
            events.push(event);
        }
        cursor = tail;
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
    rest: Cursor<'a>,
    previous_start: Option<Timestamp>,
) -> Result<(Option<Event>, Cursor<'a>), ParseLyricsError> {
    match parse_element_line(line)? {
        ElementLine::OpeningTag => {
            let (region, tail) = take_additive_region(rest, line.number, previous_start)?;
            Ok((Some(Event::Region(region)), tail))
        }
        // A closing tag is consumed by the parser that consumed its
        // opening tag, so one reaching the top level closes nothing.
        ElementLine::ClosingTag => UnopenedRegion {
            line_number: line.number,
        }
        .pipe(AdditiveRegionError::Unopened)
        .pipe(ParseLyricsError::AdditiveRegion)
        .pipe(Err),
        ElementLine::Header(start, Header::Control(ReservedMarker::Clear)) => {
            check_event_order(start, line.number, previous_start)?;
            Ok((Some(Event::Clear(start)), rest))
        }
        // The only other control marker is `eov`, which the cursor
        // passes over outside a region and which a region rejects, so
        // no line reaches here. Were one to, ignoring it entirely is
        // what the format asks for.
        ElementLine::Header(_, Header::Control(_)) => Ok((None, rest)),
        ElementLine::Header(start, Header::Cue(body)) => {
            check_event_order(start, line.number, previous_start)?;
            let header = PartHeader::parse(body, line.number)?;
            let (group, tail) = take_cue_group(start, header, rest)?;
            Ok((Some(Event::Cue(group)), tail))
        }
    }
}

/// What a line standing where an element may begin declares. The
/// indent rules and the tag spellings are the same inside a region as
/// outside one, so both callers read such a line through here and
/// differ only in what they make of the answer.
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
    // line. Nothing else in the format opens with `<`, so any other
    // line that does is a misspelled tag rather than a header.
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
/// it stood on, and `cursor` the tail that follows it. Nesting is not
/// part of the grammar this parser reads, so the region it returns is
/// flat by construction and an `<additive>` met on the way is
/// reported where it stands.
fn take_additive_region<'a>(
    cursor: Cursor<'a>,
    opened_at: usize,
    previous_start: Option<Timestamp>,
) -> Result<(AdditiveRegion, Cursor<'a>), ParseLyricsError> {
    let mut groups = Vec::<CueGroup>::new();
    let mut cursor = cursor.inside(opened_at);

    loop {
        let Some((line, rest)) = cursor.take_line()? else {
            return UnclosedRegion {
                line_number: opened_at,
            }
            .pipe(AdditiveRegionError::Unclosed)
            .pipe(ParseLyricsError::AdditiveRegion)
            .pipe(Err);
        };

        match parse_element_line(line)? {
            ElementLine::ClosingTag => {
                let region = AdditiveRegion::new(groups, line.number, opened_at)?;
                return Ok((region, rest.outside()));
            }
            // The cues a region encloses are read by a parser that
            // admits no opening tag, so a nested region is not a state
            // this parser can reach; the tag is reported where it
            // stands instead.
            ElementLine::OpeningTag => {
                return NestedRegion {
                    line_number: line.number,
                    opened_at,
                }
                .pipe(AdditiveRegionError::Nested)
                .pipe(ParseLyricsError::AdditiveRegion)
                .pipe(Err);
            }
            // A region encloses cues, not the boundary events that end
            // them, so both control markers are rejected here rather
            // than acted on.
            ElementLine::Header(_, Header::Control(marker)) => {
                return ControlMarkerInRegion {
                    line_number: line.number,
                    marker,
                    opened_at,
                }
                .pipe(AdditiveRegionError::ControlMarker)
                .pipe(ParseLyricsError::AdditiveRegion)
                .pipe(Err);
            }
            ElementLine::Header(start, Header::Cue(body)) => {
                let previous = groups.last().map(|group| group.start).or(previous_start);
                check_event_order(start, line.number, previous)?;
                let header = PartHeader::parse(body, line.number)?;
                let (group, tail) = take_cue_group(start, header, rest)?;
                groups.push(group);
                cursor = tail;
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
    header: PartHeader<'a>,
    cursor: Cursor<'a>,
) -> Result<(CueGroup, Cursor<'a>), ParseLyricsError> {
    // The part the shorthand column is currently writing into is held
    // aside from the parts a later shorthand line has already closed.
    // That is what lets an annotation reach the open part without the
    // group having to prove that a part exists.
    let (mut open, mut cursor) = take_cue_part(header, cursor)?;
    let mut parts = Vec::<CuePart>::new();
    let annotation_indent = continuation_indent(ReservedMarker::Annotation.as_ref());

    while let Some((line, rest)) = cursor.take_line()? {
        // A part accepts every line indented at its continuation width
        // and rejects every other width, so the line reached here
        // stands at column zero or at the shorthand column.
        if line.indent != TIMESTAMP_PREFIX_WIDTH {
            break;
        }
        match parse_shorthand_line(line.body, line.number)? {
            ShorthandLine::Annotation(text) => {
                let (continuations, tail) =
                    take_continuation_lines(rest, annotation_indent, Continued::AnnotationText)?;
                open.annotations.push(join_lines(text, &continuations));
                cursor = tail;
            }
            ShorthandLine::Part(header) => {
                let (part, tail) = take_cue_part(header, rest)?;
                parts.push(open);
                open = part;
                cursor = tail;
            }
        }
    }

    parts.push(open);
    Ok((CueGroup { start, parts }, cursor))
}

/// Consumes the continuation lines that extend the text of the part
/// `header` opens, returning the part with the unconsumed tail. The
/// annotations written beneath the part are the cue group's to read,
/// since each attaches to whichever part is open when it appears.
fn take_cue_part<'a>(
    header: PartHeader<'a>,
    cursor: Cursor<'a>,
) -> Result<(CuePart, Cursor<'a>), ParseLyricsError> {
    let PartHeader { marker, text } = header;
    let indent = continuation_indent(marker);
    let (continuations, cursor) = take_continuation_lines(cursor, indent, Continued::CueText)?;
    let part = CuePart {
        marker: marker.to_string(),
        text: join_lines(text, &continuations),
        annotations: Vec::new(),
    };
    Ok((part, cursor))
}

/// Which body a run of continuation lines extends. Only cue text is
/// checked for the reserved tag delimiters: annotation text reaches no
/// renderer, so `<` and `>` carry no meaning there and are ordinary
/// punctuation.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Continued {
    CueText,
    AnnotationText,
}

/// Consumes the continuation lines of a body whose continuations are
/// indented by `indent`, returning them with the unconsumed tail.
///
/// The first line carrying another indent opens something else, so it
/// must stand at column zero, where a header or a tag line does, or at
/// the shorthand column, where a new part or an annotation does. Any
/// other indent names nothing the grammar admits here and is reported
/// against the two widths that were in force.
fn take_continuation_lines<'a>(
    cursor: Cursor<'a>,
    indent: usize,
    continued: Continued,
) -> Result<(Vec<Line<'a>>, Cursor<'a>), ParseLyricsError> {
    let mut continuations = Vec::<Line>::new();
    let mut cursor = cursor;

    while let Some((line, rest)) = cursor.take_line()? {
        if line.indent == indent {
            if continued == Continued::CueText {
                reject_reserved_cue_text_characters(line.body, line.number)?;
            }
            continuations.push(line);
            cursor = rest;
            continue;
        }
        if line.indent != 0 && line.indent != TIMESTAMP_PREFIX_WIDTH {
            return Err(malformed_indentation(line, Some(indent)));
        }
        break;
    }

    Ok((continuations, cursor))
}

/// What a line at the shorthand column declares.
enum ShorthandLine<'a> {
    /// An `ann` line, carrying the note it attaches to the part above
    /// it.
    Annotation(&'a str),
    /// A marker line, opening a part of its own.
    Part(PartHeader<'a>),
}

/// Reads a whole line at the shorthand column. The line is split into
/// a marker and its text once, and the marker alone decides which of
/// the two shapes the line is, so neither half is derived twice.
fn parse_shorthand_line(
    body: &str,
    line_number: usize,
) -> Result<ShorthandLine<'_>, ParseLyricsError> {
    let Some((marker, text)) = split_marker(body) else {
        reject_reserved_cue_text_characters(body, line_number)?;
        return MissingMarker {
            line_number,
            content: body.to_string(),
        }
        .pipe(ParseLyricsError::MissingMarker)
        .pipe(Err);
    };
    if matches!(marker.parse(), Ok(ReservedMarker::Annotation)) {
        if text.is_empty() {
            return EmptyAnnotation { line_number }
                .pipe(ParseLyricsError::EmptyAnnotation)
                .pipe(Err);
        }
        return Ok(ShorthandLine::Annotation(text));
    }
    reject_reserved_cue_text_characters(body, line_number)?;
    PartHeader::from_parts(marker, text, line_number).map(ShorthandLine::Part)
}

/// The marker and the text that a `marker: text` line declares, which
/// together open one [`CuePart`].
#[derive(Clone, Copy)]
struct PartHeader<'a> {
    marker: &'a str,
    text: &'a str,
}

impl<'a> PartHeader<'a> {
    /// Reads a whole `marker: text` body, rejecting the bodies that
    /// no cue part may carry.
    fn parse(body: &'a str, line_number: usize) -> Result<Self, ParseLyricsError> {
        reject_reserved_cue_text_characters(body, line_number)?;
        let (marker, text) = split_marker(body).ok_or_else(|| {
            ParseLyricsError::MissingMarker(MissingMarker {
                line_number,
                content: body.to_string(),
            })
        })?;
        PartHeader::from_parts(marker, text, line_number)
    }

    /// Checks the two halves of a body that has already been split.
    /// A caller that split the body to learn what kind of line it was
    /// reaches the same checks through here rather than splitting the
    /// same bytes a second time.
    fn from_parts(
        marker: &'a str,
        text: &'a str,
        line_number: usize,
    ) -> Result<Self, ParseLyricsError> {
        if let Ok(reserved) = marker.parse::<ReservedMarker>() {
            return Err(ParseLyricsError::ReservedControlMarker(
                ReservedControlMarker {
                    line_number,
                    marker: reserved,
                },
            ));
        }
        if text.is_empty() {
            return Err(ParseLyricsError::EmptyCueBody(EmptyCueBody {
                line_number,
                marker: marker.to_string(),
            }));
        }
        Ok(PartHeader { marker, text })
    }
}

/// What the body of a column-zero header line declares.
enum Header<'a> {
    /// A control marker standing alone on the line.
    Control(ReservedMarker),
    /// The body of a line that opens a cue, which the caller reads as
    /// a [`PartHeader`]. It is handed on unparsed so that the checks a
    /// cue faces apply in the order the caller wants them, rather than
    /// in the order this layer happens to run.
    Cue(&'a str),
}

/// Reads a whole header line: an `MM:SS.mmm` timestamp, the
/// whitespace that separates it from the body, and a body that either
/// names a control marker or opens a cue.
///
/// Each layer is consumed by a parser that hands back its tail, and
/// the next layer reads that tail rather than a position computed
/// from what the previous one matched.
fn parse_header(line: Line<'_>) -> Result<(Timestamp, Header<'_>), ParseLyricsError> {
    let (start, after_timestamp) = match Timestamp::take(line.body) {
        Ok(parsed) => parsed,
        Err(TakeTimestampError::ShapeMismatch) => {
            return Err(ParseLyricsError::MalformedHeader(MalformedHeader {
                line_number: line.number,
                content: line.body.to_string(),
            }));
        }
        Err(cause) => {
            return Err(ParseLyricsError::InvalidTimestamp(InvalidTimestamp {
                line_number: line.number,
                cause,
            }));
        }
    };

    let (separator, cue_body) = take_leading_whitespace(after_timestamp);
    if separator.is_empty() {
        return Err(ParseLyricsError::MissingSeparatorAfterTimestamp(
            MissingSeparatorAfterTimestamp {
                line_number: line.number,
                content: line.body.to_string(),
            },
        ));
    }

    let (first_token, after_token) = take_non_whitespace(cue_body);
    if let Ok(marker) = first_token.parse::<ReservedMarker>()
        && marker.is_control()
    {
        let trailing = after_token.trim();
        if !trailing.is_empty() {
            return Err(ParseLyricsError::ExtraTextAfterControlMarker(
                ExtraTextAfterControlMarker {
                    line_number: line.number,
                    marker,
                    trailing: trailing.to_string(),
                },
            ));
        }
        return Ok((start, Header::Control(marker)));
    }

    Ok((start, Header::Cue(cue_body)))
}

/// Whether `line` is an `eov` line, which the format ignores
/// entirely. A line the header parser rejects is not one; its
/// diagnostic belongs to the parser that owns the position rather
/// than to this test.
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

/// Joins the opening text of a body with the continuation lines that
/// extend it, one line break apiece.
fn join_lines(opening: &str, continuations: &[Line<'_>]) -> String {
    once(opening)
        .chain(continuations.iter().map(|line| line.body))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Rejects a new event whose start time matches or precedes the most
/// recent recorded event, whose start time is `previous`. An `eov`
/// line never reaches this check, because it pushes no event and
/// therefore must not compete for the same start-time slot as a real
/// cue or `clr`.
fn check_event_order(
    start: Timestamp,
    line_number: usize,
    previous: Option<Timestamp>,
) -> Result<(), ParseLyricsError> {
    let Some(previous) = previous else {
        return Ok(());
    };
    if previous == start {
        return Err(ParseLyricsError::RepeatedTimestamp(RepeatedTimestamp {
            line_number,
            start,
        }));
    }
    if start < previous {
        return Err(ParseLyricsError::OutOfOrder(OutOfOrder {
            previous,
            next: start,
        }));
    }
    Ok(())
}

/// The diagnostic for a shorthand-column line that no cue is open
/// above. An annotation reports its own, because the fix for it
/// differs from the fix for a marker line.
fn orphaned_shorthand_line(line: Line<'_>) -> ParseLyricsError {
    if annotation_body(line.body).is_some() {
        return OrphanedAnnotation {
            line_number: line.number,
            content: line.body.to_string(),
        }
        .pipe(ParseLyricsError::OrphanedAnnotation);
    }
    OrphanedShorthandMarker {
        line_number: line.number,
        content: line.body.to_string(),
    }
    .pipe(ParseLyricsError::OrphanedShorthandMarker)
}

/// The diagnostic for a line whose indent matches no width the parser
/// accepts where it stands. `continuation` is the width a continuation
/// would have carried, or `None` when no body is open for one to
/// continue.
fn malformed_indentation(line: Line<'_>, continuation: Option<usize>) -> ParseLyricsError {
    MalformedIndentation {
        line_number: line.number,
        actual: line.indent,
        shorthand_indent: TIMESTAMP_PREFIX_WIDTH,
        continuation_indent: continuation,
    }
    .pipe(ParseLyricsError::MalformedIndentation)
}

/// The diagnostic for a column-zero line that opens with `<` but
/// spells neither tag.
fn malformed_tag_line(line: Line<'_>) -> ParseLyricsError {
    MalformedTagLine {
        line_number: line.number,
        content: line.body.to_string(),
    }
    .pipe(ParseLyricsError::MalformedTagLine)
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
/// where a cue renders the parts of the cues above it. The carried
/// parts start empty and are dropped at the closing tag, so two
/// adjacent regions cannot bleed into each other.
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
    let end = following.ok_or(ParseLyricsError::UnclosedCue(UnclosedCue {
        start: group.start,
    }))?;
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
/// Only the rendered halves of a part repeat. An annotation documents
/// the line its author wrote it under, so carrying it forward would
/// attribute one note to every cue in the rest of the region; the copy
/// therefore starts with none. Neither renderer reads annotations, so
/// the choice is invisible in the output and matters only to a reader
/// of the parsed cues.
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
/// is empty; the caller reports this as [`ParseLyricsError::MissingMarker`]
/// because every cue-opening line in the source format is expected to
/// carry a marker.
fn split_marker(body: &str) -> Option<(&str, &str)> {
    let (head, tail) = body.split_once(':')?;
    let marker = head.trim();
    if marker.is_empty() {
        return None;
    }
    Some((marker, tail.trim()))
}

/// Rejects cue text containing `<` or `>`, which the WebVTT cue-tag
/// grammar reserves as tag delimiters. The renderer escapes the cue
/// body, so neither could reach the output as itself. Reports the
/// first offender only.
fn reject_reserved_cue_text_characters(
    text: &str,
    line_number: usize,
) -> Result<(), ParseLyricsError> {
    if let Some(character) = text.chars().find(|&c| matches!(c, '<' | '>')) {
        return Err(ParseLyricsError::CueTextReservedCharacter(
            CueTextReservedCharacter {
                line_number,
                character,
            },
        ));
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

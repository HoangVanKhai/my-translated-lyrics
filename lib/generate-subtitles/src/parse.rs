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

pub mod error;

use error::{
    AdditiveRegionError, ControlMarkerInRegion, CueTextReservedCharacter, EmptyAnnotation,
    EmptyCueBody, EmptyRegion, ExtraTextAfterControlMarker, InvalidTimestamp, MalformedHeader,
    MalformedIndentation, MalformedTagLine, MissingMarker, MissingSeparatorAfterTimestamp,
    NestedRegion, OrphanedAnnotation, OrphanedShorthandMarker, OutOfOrder, ParseLyricsError,
    ParseLyricsErrorKind, RepeatedTimestamp, ReservedControlMarker, TabIndentation, UnclosedCue,
    UnclosedRegion, UnopenedRegion,
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

/// Which text body a continuation line extends.
#[derive(Clone, Copy)]
enum ContinuationTarget {
    /// The text of the most recently opened cue part.
    PartText,
    /// The most recent annotation of that part.
    AnnotationText,
}

/// The marker line a continuation would extend, and the indent such
/// a continuation must carry.
#[derive(Clone, Copy)]
struct OpenMarkerLine {
    /// Byte width of the line's `marker: ` prefix. A continuation of
    /// it is indented by [`TIMESTAMP_PREFIX_WIDTH`] plus this width.
    marker_prefix_width: usize,
    /// Where that continuation's text is appended.
    target: ContinuationTarget,
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

/// Identifies one `<additive>` region within a source file.
///
/// The index counts regions in the order they open, which keeps two
/// adjacent regions distinct even though no event separates them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AdditiveRegionIndex(usize);

/// The `<additive>` region the parser is currently inside.
#[derive(Clone, Copy)]
struct OpenRegion {
    /// The index the region's cue groups carry.
    index: AdditiveRegionIndex,
    /// Line of the `<additive>` that opened the region. Every
    /// diagnostic that names the region points back at this line,
    /// because that is where the author has to act.
    opened_at: usize,
    /// How many cue groups the region has collected so far. A region
    /// that closes having collected none is rejected.
    cue_count: usize,
}

/// The `<additive>` regions a source file declares, as seen from the
/// line currently being read.
#[derive(Default)]
struct RegionState {
    /// The region currently open, if any.
    open: Option<OpenRegion>,
    /// How many regions have opened so far, which names the next one.
    opened: usize,
}

impl RegionState {
    /// Opens a region whose `<additive>` sits on line `opened_at`.
    fn open_region(&mut self, opened_at: usize) -> Result<(), AdditiveRegionError> {
        if let Some(open) = self.open {
            return NestedRegion {
                opened_at: open.opened_at,
            }
            .pipe(AdditiveRegionError::Nested)
            .pipe(Err);
        }
        self.open = Some(OpenRegion {
            index: AdditiveRegionIndex(self.opened),
            opened_at,
            cue_count: 0,
        });
        self.opened += 1;
        Ok(())
    }

    /// Closes the region currently open. Both rules are checked
    /// before the region is released, so a rejected tag leaves the
    /// state as it was rather than half closed.
    fn close_region(&mut self) -> Result<(), AdditiveRegionError> {
        let Some(open) = self.open else {
            return UnopenedRegion.pipe(AdditiveRegionError::Unopened).pipe(Err);
        };
        if open.cue_count == 0 {
            return EmptyRegion {
                opened_at: open.opened_at,
            }
            .pipe(AdditiveRegionError::Empty)
            .pipe(Err);
        }
        self.open = None;
        Ok(())
    }
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
    /// Line of the header that opened the group. A cue whose end time
    /// no later event supplies is only detected once the line loop is
    /// over, and the author still has to be pointed at the line that
    /// opened it.
    opened_at: usize,
    region: Option<AdditiveRegionIndex>,
    parts: Vec<CuePart>,
}

/// An intermediate event extracted from a source file before end times
/// are resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Cue(CueGroup),
    Clear(Timestamp),
}

impl Event {
    fn start(&self) -> Timestamp {
        match self {
            Event::Cue(group) => group.start,
            Event::Clear(start) => *start,
        }
    }
}

/// Parses `content` into a list of cues ordered by start time.
pub fn parse_lyrics(content: &str) -> Result<Vec<SubtitleCue>, ParseLyricsError> {
    let events = collect_events(content)?;
    resolve_cues(events)
}

/// Locates a [`ParseLyricsErrorKind`] at `line_number`.
fn at_line(line_number: usize) -> impl Fn(ParseLyricsErrorKind) -> ParseLyricsError {
    move |kind| ParseLyricsError { line_number, kind }
}

fn collect_events(content: &str) -> Result<Vec<Event>, ParseLyricsError> {
    let mut events = Vec::<Event>::new();
    let mut last_cue_index: Option<usize> = None;
    // The marker line a continuation would extend, which is the one
    // most recently opened, whether by a cue part or by an
    // annotation. A continuation is valid only when its indent equals
    // `TIMESTAMP_PREFIX_WIDTH` plus that line's `marker: ` width.
    let mut open_marker_line: Option<OpenMarkerLine> = None;
    let mut regions = RegionState::default();

    for (line_index, raw_line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        if raw_line.trim().is_empty() || raw_line.trim_start().starts_with('#') {
            continue;
        }
        let locate = at_line(line_number);

        if raw_line.trim_start_matches(' ').starts_with('\t') {
            return TabIndentation
                .pipe(ParseLyricsErrorKind::TabIndentation)
                .pipe(locate)
                .pipe(Err);
        }

        let indent = raw_line.bytes().take_while(|&b| b == b' ').count();
        let body = &raw_line[indent..];

        if indent == 0 {
            // A tag line is the one column-zero shape that carries no
            // timestamp, so it is parsed before the header parser sees
            // the line. Nothing else in the format opens with `<`, so
            // any other line that does is a misspelled tag rather than
            // a header.
            if let Some((tag, tail)) = OpeningTag::take(body)
                && tag.name() == TagName::Additive
                && tail.trim().is_empty()
            {
                handle_additive_opening_tag_line(
                    line_number,
                    &mut regions,
                    &mut last_cue_index,
                    &mut open_marker_line,
                )
                .map_err(locate)?;
            } else if let Some((tag, tail)) = ClosingTag::take(body)
                && tag.name() == TagName::Additive
                && tail.trim().is_empty()
            {
                handle_additive_closing_tag_line(
                    &mut regions,
                    &mut last_cue_index,
                    &mut open_marker_line,
                )
                .map_err(locate)?;
            } else if body.starts_with('<') {
                return MalformedTagLine {
                    content: body.to_string(),
                }
                .pipe(ParseLyricsErrorKind::MalformedTagLine)
                .pipe(locate)
                .pipe(Err);
            } else {
                handle_header_line(
                    body,
                    line_number,
                    &mut events,
                    &mut last_cue_index,
                    &mut open_marker_line,
                    &mut regions,
                )
                .map_err(locate)?;
            }
        } else if indent == TIMESTAMP_PREFIX_WIDTH {
            handle_shorthand_marker_line(body, &mut events, last_cue_index, &mut open_marker_line)
                .map_err(locate)?;
        } else if let Some(open) = open_marker_line
            && indent == TIMESTAMP_PREFIX_WIDTH + open.marker_prefix_width
        {
            handle_continuation_line(body, &mut events, last_cue_index, open.target)
                .map_err(locate)?;
        } else {
            return MalformedIndentation {
                actual: indent,
                shorthand_indent: TIMESTAMP_PREFIX_WIDTH,
                continuation_indent: open_marker_line
                    .map(|open| TIMESTAMP_PREFIX_WIDTH + open.marker_prefix_width),
            }
            .pipe(ParseLyricsErrorKind::MalformedIndentation)
            .pipe(locate)
            .pipe(Err);
        }
    }

    if let Some(OpenRegion { opened_at, .. }) = regions.open {
        return UnclosedRegion
            .pipe(AdditiveRegionError::Unclosed)
            .pipe(ParseLyricsErrorKind::AdditiveRegion)
            .pipe(at_line(opened_at))
            .pipe(Err);
    }

    Ok(events)
}

/// Opens an additive region at an `<additive>` line. `opened_at` is
/// the line that tag sits on; it travels down because [`OpenRegion`]
/// records it, not because a diagnostic raised here needs to name it.
fn handle_additive_opening_tag_line(
    opened_at: usize,
    regions: &mut RegionState,
    last_cue_index: &mut Option<usize>,
    open_marker_line: &mut Option<OpenMarkerLine>,
) -> Result<(), ParseLyricsErrorKind> {
    regions
        .open_region(opened_at)
        .map_err(ParseLyricsErrorKind::AdditiveRegion)?;
    end_cue_scope(last_cue_index, open_marker_line);
    Ok(())
}

/// Closes the additive region at a `</additive>` line.
fn handle_additive_closing_tag_line(
    regions: &mut RegionState,
    last_cue_index: &mut Option<usize>,
    open_marker_line: &mut Option<OpenMarkerLine>,
) -> Result<(), ParseLyricsErrorKind> {
    regions
        .close_region()
        .map_err(ParseLyricsErrorKind::AdditiveRegion)?;
    end_cue_scope(last_cue_index, open_marker_line);
    Ok(())
}

/// Ends the scope of the cue above a region boundary, exactly as
/// [`ReservedMarker::Clear`] does, so that no continuation or
/// shorthand marker line reaches across the tag.
fn end_cue_scope(
    last_cue_index: &mut Option<usize>,
    open_marker_line: &mut Option<OpenMarkerLine>,
) {
    *last_cue_index = None;
    *open_marker_line = None;
}

/// Reads a column-zero header line. `opened_at` is the line that
/// header sits on; a header that opens a cue records it in the
/// [`CueGroup`], so that a cue the file never closes can still name
/// the line that opened it. No diagnostic raised here reads it.
fn handle_header_line(
    body: &str,
    opened_at: usize,
    events: &mut Vec<Event>,
    last_cue_index: &mut Option<usize>,
    open_marker_line: &mut Option<OpenMarkerLine>,
    regions: &mut RegionState,
) -> Result<(), ParseLyricsErrorKind> {
    let (start, after_prefix) = match Timestamp::take(body) {
        Ok(parsed) => parsed,
        Err(TakeTimestampError::ShapeMismatch) => {
            return Err(ParseLyricsErrorKind::MalformedHeader(MalformedHeader {
                content: body.to_string(),
            }));
        }
        Err(cause) => {
            return Err(ParseLyricsErrorKind::InvalidTimestamp(InvalidTimestamp {
                cause,
            }));
        }
    };

    let cue_body = after_prefix.trim_start();
    if cue_body.len() == after_prefix.len() {
        return Err(ParseLyricsErrorKind::MissingSeparatorAfterTimestamp(
            MissingSeparatorAfterTimestamp {
                content: body.to_string(),
            },
        ));
    }

    let first_token = cue_body.split_whitespace().next().unwrap_or("");
    if let Ok(marker) = first_token.parse::<ReservedMarker>()
        && marker.is_control()
    {
        let trailing = cue_body[first_token.len()..].trim();
        if !trailing.is_empty() {
            return Err(ParseLyricsErrorKind::ExtraTextAfterControlMarker(
                ExtraTextAfterControlMarker {
                    marker,
                    trailing: trailing.to_string(),
                },
            ));
        }
        if let Some(open) = &regions.open {
            let payload = ControlMarkerInRegion {
                marker,
                opened_at: open.opened_at,
            };
            return payload
                .pipe(AdditiveRegionError::ControlMarker)
                .pipe(ParseLyricsErrorKind::AdditiveRegion)
                .pipe(Err);
        }
        if marker == ReservedMarker::Clear {
            check_event_order(start, events)?;
            events.push(Event::Clear(start));
            *last_cue_index = None;
            *open_marker_line = None;
        }
        // `eov` is documented as "ignored entirely"; it pushes no
        // event and so does not participate in the repeated- or
        // out-of-order checks. This lets a source file note both
        // `clr` and `eov` at the moment the video ends, since the
        // `eov` is a documentation sentinel rather than a competing
        // cue boundary.
        return Ok(());
    }

    check_event_order(start, events)?;
    let (marker, text) = parse_marker_part(cue_body)?;
    let region = regions.open.as_mut().map(|open| {
        open.cue_count += 1;
        open.index
    });
    events.push(Event::Cue(CueGroup {
        start,
        opened_at,
        region,
        parts: vec![CuePart {
            marker: marker.to_string(),
            text: text.to_string(),
            annotations: Vec::new(),
        }],
    }));
    *last_cue_index = Some(events.len() - 1);
    *open_marker_line = Some(OpenMarkerLine {
        marker_prefix_width: marker_prefix_width(marker),
        target: ContinuationTarget::PartText,
    });
    Ok(())
}

fn handle_shorthand_marker_line(
    body: &str,
    events: &mut [Event],
    last_cue_index: Option<usize>,
    open_marker_line: &mut Option<OpenMarkerLine>,
) -> Result<(), ParseLyricsErrorKind> {
    if let Some(annotation) = annotation_body(body) {
        let Some(cue_index) = last_cue_index else {
            return Err(ParseLyricsErrorKind::OrphanedAnnotation(
                OrphanedAnnotation {
                    content: body.to_string(),
                },
            ));
        };
        return handle_annotation_line(annotation, cue_index, events, open_marker_line);
    }

    let Some(cue_index) = last_cue_index else {
        return Err(ParseLyricsErrorKind::OrphanedShorthandMarker(
            OrphanedShorthandMarker {
                content: body.to_string(),
            },
        ));
    };
    let (marker, text) = parse_marker_part(body)?;
    cue_group_mut(events, cue_index).parts.push(CuePart {
        marker: marker.to_string(),
        text: text.to_string(),
        annotations: Vec::new(),
    });
    *open_marker_line = Some(OpenMarkerLine {
        marker_prefix_width: marker_prefix_width(marker),
        target: ContinuationTarget::PartText,
    });
    Ok(())
}

/// Attaches an annotation carrying `text` to the last part of the
/// cue group at `cue_index`.
fn handle_annotation_line(
    text: &str,
    cue_index: usize,
    events: &mut [Event],
    open_marker_line: &mut Option<OpenMarkerLine>,
) -> Result<(), ParseLyricsErrorKind> {
    if text.is_empty() {
        return Err(ParseLyricsErrorKind::EmptyAnnotation(EmptyAnnotation));
    }
    last_part_mut(events, cue_index)
        .annotations
        .push(text.to_string());
    *open_marker_line = Some(OpenMarkerLine {
        marker_prefix_width: marker_prefix_width(ReservedMarker::Annotation.as_ref()),
        target: ContinuationTarget::AnnotationText,
    });
    Ok(())
}

/// The cue group at `cue_index`, which the caller's `last_cue_index`
/// guarantees is an [`Event::Cue`] rather than an [`Event::Clear`].
fn cue_group_mut(events: &mut [Event], cue_index: usize) -> &mut CueGroup {
    let Event::Cue(group) = &mut events[cue_index] else {
        unreachable!("last_cue_index must point at a Cue event");
    };
    group
}

/// The part a continuation line or an annotation line attaches to:
/// the most recently opened part of the cue group at `cue_index`.
fn last_part_mut(events: &mut [Event], cue_index: usize) -> &mut CuePart {
    cue_group_mut(events, cue_index)
        .parts
        .last_mut()
        .expect("a cue group always has at least one part once it is opened")
}

fn handle_continuation_line(
    body: &str,
    events: &mut [Event],
    last_cue_index: Option<usize>,
    target: ContinuationTarget,
) -> Result<(), ParseLyricsErrorKind> {
    let cue_index =
        last_cue_index.expect("indent matched continuation width, so a prior cue must exist");
    // Only cue text is checked for the reserved tag delimiters.
    // Annotation text reaches no renderer, so `<` and `>` carry no
    // meaning there and are ordinary punctuation.
    if matches!(target, ContinuationTarget::PartText) {
        reject_reserved_cue_text_characters(body)?;
    }
    let part = last_part_mut(events, cue_index);
    let destination = match target {
        ContinuationTarget::PartText => &mut part.text,
        ContinuationTarget::AnnotationText => part
            .annotations
            .last_mut()
            .expect("an annotation is open, so the part carries at least one"),
    };
    destination.push('\n');
    destination.push_str(body);
    Ok(())
}

fn parse_marker_part(body: &str) -> Result<(&str, &str), ParseLyricsErrorKind> {
    reject_reserved_cue_text_characters(body)?;
    let (marker, text) = split_marker(body).ok_or_else(|| {
        ParseLyricsErrorKind::MissingMarker(MissingMarker {
            content: body.to_string(),
        })
    })?;
    if let Ok(reserved) = marker.parse::<ReservedMarker>() {
        return Err(ParseLyricsErrorKind::ReservedControlMarker(
            ReservedControlMarker { marker: reserved },
        ));
    }
    if text.is_empty() {
        return Err(ParseLyricsErrorKind::EmptyCueBody(EmptyCueBody {
            marker: marker.to_string(),
        }));
    }
    Ok((marker, text))
}

/// Byte width of `marker: ` (the marker name, a colon, and one
/// ASCII space). Used to compute the expected indent of a
/// continuation line under the part it continues.
fn marker_prefix_width(marker: &str) -> usize {
    marker.len() + 2
}

/// Rejects a new event whose start time matches or precedes the
/// most recent recorded event. Skipped for `eov` lines because
/// `eov` does not push an event and therefore should not compete
/// for the same start-time slot as a real cue or `clr`.
fn check_event_order(start: Timestamp, events: &[Event]) -> Result<(), ParseLyricsErrorKind> {
    let Some(previous_start) = events.last().map(Event::start) else {
        return Ok(());
    };
    if previous_start == start {
        return Err(ParseLyricsErrorKind::RepeatedTimestamp(RepeatedTimestamp {
            start,
        }));
    }
    if start < previous_start {
        return Err(ParseLyricsErrorKind::OutOfOrder(OutOfOrder {
            previous: previous_start,
            next: start,
        }));
    }
    Ok(())
}

fn resolve_cues(events: Vec<Event>) -> Result<Vec<SubtitleCue>, ParseLyricsError> {
    let mut cues = Vec::<SubtitleCue>::new();
    // The region whose parts `carried` holds, and those parts. A cue
    // group in that same region renders them above its own; a group
    // anywhere else resets both, which is what keeps two adjacent
    // regions from bleeding into each other.
    let mut carried_region: Option<AdditiveRegionIndex> = None;
    let mut carried = Vec::<CuePart>::new();

    for (index, event) in events.iter().enumerate() {
        let Event::Cue(group) = event else {
            continue;
        };

        let Some(end) = events.get(index + 1).map(Event::start) else {
            return UnclosedCue { start: group.start }
                .pipe(ParseLyricsErrorKind::UnclosedCue)
                .pipe(at_line(group.opened_at))
                .pipe(Err);
        };

        if group.region != carried_region {
            carried_region = group.region;
            carried.clear();
        }

        let mut parts = carried.clone();
        parts.extend(group.parts.iter().cloned());
        if group.region.is_some() {
            carried = parts.iter().map(carried_part).collect();
        }

        cues.push(SubtitleCue {
            start: group.start,
            end,
            parts,
        });
    }

    Ok(cues)
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
/// is empty; the caller reports this as [`ParseLyricsErrorKind::MissingMarker`]
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
fn reject_reserved_cue_text_characters(text: &str) -> Result<(), ParseLyricsErrorKind> {
    if let Some(character) = text.chars().find(|&c| matches!(c, '<' | '>')) {
        return Err(ParseLyricsErrorKind::CueTextReservedCharacter(
            CueTextReservedCharacter { character },
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

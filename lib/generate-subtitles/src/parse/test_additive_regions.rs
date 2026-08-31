//! Tests for the accumulation an `<additive>` region performs. Each cue
//! inside a region renders the parts of every cue above it in the same
//! region, while the cues outside the region and the cues of an adjacent
//! region take no part in it.

use crate::parse::parse_lyrics;
use lyrics_core::timestamp::Timestamp;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

/// The defining behavior of an additive region: a cue does not
/// replace the one above it but renders below it, so the region
/// builds up a line at a time. The final cue ends at the event that
/// follows the closing tag, which is the `clr` here.
#[test]
fn additive_region_accumulates_each_cue_onto_the_ones_above_it() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "07:33.333 LRC: third line"
        "07:44.444 LRC: fourth line"
        "</additive>"
        ""
        "07:55.555 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    let bodies: Vec<Vec<&str>> = cues
        .iter()
        .map(|cue| cue.parts.iter().map(|part| part.text.as_str()).collect())
        .collect();
    assert_eq!(
        bodies,
        vec![
            vec!["first line"],
            vec!["first line", "second line"],
            vec!["first line", "second line", "third line"],
            vec!["first line", "second line", "third line", "fourth line"],
        ],
    );
    assert_eq!(cues[0].start, Timestamp::new(7, 11, 111).unwrap());
    assert_eq!(cues[0].end, Timestamp::new(7, 22, 222).unwrap());
    assert_eq!(cues[3].start, Timestamp::new(7, 44, 444).unwrap());
    assert_eq!(cues[3].end, Timestamp::new(7, 55, 555).unwrap());
}

/// Every carried part keeps the marker of the line that wrote it,
/// so a region may mix markers and each accumulated line still
/// renders under its own presentation rules.
#[test]
fn carried_parts_keep_their_own_markers() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 ttl: title body"
        "07:22.222 LRC: lyric body"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    let markers: Vec<&str> = cues[1]
        .parts
        .iter()
        .map(|part| part.marker.as_str())
        .collect();
    assert_eq!(markers, ["ttl", "LRC"]);
}

/// A cue group inside a region may carry several parts of its own
/// through the shorthand column, and all of them carry forward
/// together.
#[test]
fn a_multi_part_cue_group_carries_all_of_its_parts_forward() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 ttl: title body"
        "          cre: credit body"
        "07:22.222 LRC: lyric body"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    let bodies: Vec<&str> = cues[1]
        .parts
        .iter()
        .map(|part| part.text.as_str())
        .collect();
    assert_eq!(bodies, ["title body", "credit body", "lyric body"]);
}

/// Continuation lines belong to the part that opened them, so a
/// multi-line part carries forward as one part with its line breaks
/// intact rather than as several.
#[test]
fn a_carried_part_keeps_its_continuation_lines() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 cre: first body line"
        "               second body line"
        "07:22.222 LRC: lyric body"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[1].parts.len(), 2);
    assert_eq!(cues[1].parts[0].text, "first body line\nsecond body line");
    assert_eq!(cues[1].parts[1].text, "lyric body");
}

/// A cue after the closing tag is an ordinary cue again: it replaces
/// what the region left on screen instead of extending it.
#[test]
fn a_cue_after_the_region_does_not_accumulate() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "</additive>"
        "07:33.333 LRC: after the region"
        "07:44.444 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 3);
    assert_eq!(cues[2].parts.len(), 1);
    assert_eq!(cues[2].parts[0].text, "after the region");
}

/// A cue before the opening tag is likewise untouched, and the
/// region starts its accumulation from nothing rather than from
/// whatever preceded it.
#[test]
fn a_cue_before_the_region_is_not_carried_into_it() {
    let input = text_block_fnl! {
        "07:11.111 LRC: before the region"
        "<additive>"
        "07:22.222 LRC: first line"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[0].parts[0].text, "before the region");
    assert_eq!(cues[1].parts.len(), 1);
    assert_eq!(cues[1].parts[0].text, "first line");
}

/// Two regions may sit back to back with no event between them. Each
/// accumulates on its own, so the second starts empty rather than
/// continuing the first.
#[test]
fn adjacent_regions_accumulate_independently() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "</additive>"
        "<additive>"
        "07:33.333 LRC: third line"
        "07:44.444 LRC: fourth line"
        "</additive>"
        "07:55.555 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    let bodies: Vec<Vec<&str>> = cues
        .iter()
        .map(|cue| cue.parts.iter().map(|part| part.text.as_str()).collect())
        .collect();
    assert_eq!(
        bodies,
        vec![
            vec!["first line"],
            vec!["first line", "second line"],
            vec!["third line"],
            vec!["third line", "fourth line"],
        ],
    );
}

/// A region of one cue is legal and renders exactly as the same cue
/// would without the tags, because there is nothing above it to
/// accumulate.
#[test]
fn a_region_of_one_cue_renders_that_cue_alone() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: only line"
        "</additive>"
        "07:22.222 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].parts.len(), 1);
    assert_eq!(cues[0].parts[0].text, "only line");
    assert_eq!(cues[0].end, Timestamp::new(7, 22, 222).unwrap());
}

/// An annotation documents the line its author wrote it under, so it
/// stays on that cue rather than repeating on every cue below it in
/// the region.
#[test]
fn annotations_do_not_carry_forward_within_a_region() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "          ann: a note about the first line"
        "07:22.222 LRC: second line"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(
        cues[0].parts[0].annotations,
        ["a note about the first line"],
    );
    assert_eq!(cues[1].parts.len(), 2);
    assert_eq!(cues[1].parts[0].annotations, Vec::<String>::new());
    assert_eq!(cues[1].parts[1].annotations, Vec::<String>::new());
}

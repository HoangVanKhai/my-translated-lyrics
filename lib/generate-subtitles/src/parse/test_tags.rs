//! Tests for the `<additive>` and `</additive>` tag lines. They cover the
//! spellings a column-zero line accepts, the near misses it rejects, the
//! cue scope a tag ends, and the `take` parsers that recognize a tag name
//! and each of the two tag forms.

use crate::parse::error::{MalformedTagLine, OrphanedShorthandMarker, ParseLyricsError};
use crate::parse::{ClosingTag, OpeningTag, TagName, parse_lyrics};
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

/// A tag carries no attributes, so the two spellings are matched
/// literally and every near miss draws the same diagnostic, which
/// names both spellings in full rather than describing a grammar.
/// Whitespace inside the delimiters is a near miss like any other:
/// `<additive >` would otherwise have to read as an empty attribute
/// list, and the format has no attributes to read.
#[test]
fn rejects_every_near_miss_of_a_tag_line() {
    let near_misses = [
        "<verse>",
        "<additive",
        "</additive",
        "<>",
        "</>",
        "< additive>",
        "<additive >",
        "</ additive>",
        "</additive >",
        "< /additive>",
        "<additive> and some trailing text",
        "<additive></additive>",
    ];
    for content in near_misses {
        eprintln!("CASE: {content:?}");
        let input = format!(
            "{content}\n\
             07:11.111 LRC: first line\n\
             07:22.222 clr\n",
        );
        assert_eq!(
            parse_lyrics(&input).unwrap_err(),
            ParseLyricsError::MalformedTagLine(MalformedTagLine {
                line_number: 1,
                content: content.to_string(),
            }),
        );
    }
}

/// A tag line accepts trailing whitespace, the same allowance the
/// `control_markers_accept_trailing_whitespace_only` test of
/// [`super::test_control_markers`] locks in for `clr` and `eov`.
/// Whitespace inside the delimiters stays rejected, since a tag has
/// no attributes for it to separate.
#[test]
fn tag_lines_accept_trailing_whitespace_only() {
    let input = text_block_fnl! {
        "<additive> \t "
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "</additive>\t"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[1].parts.len(), 2);
}

/// The other half of the literal match: both spellings are accepted
/// exactly as written.
#[test]
fn accepts_both_tag_spellings_exactly_as_written() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[1].parts.len(), 2);
}

/// A tag ends the scope of the cue above it, exactly as `clr` does,
/// so a shorthand marker line below the tag has no cue to attach to.
#[test]
fn a_tag_ends_the_scope_of_the_cue_above_it() {
    let after_opening_tag = text_block_fnl! {
        "07:11.111 ttl: title body"
        "<additive>"
        "          cre: credit body"
        "</additive>"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(after_opening_tag).unwrap_err(),
        ParseLyricsError::OrphanedShorthandMarker(OrphanedShorthandMarker {
            line_number: 3,
            content: "cre: credit body".to_string(),
        }),
    );

    let after_closing_tag = text_block_fnl! {
        "<additive>"
        "07:11.111 ttl: title body"
        "</additive>"
        "          cre: credit body"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(after_closing_tag).unwrap_err(),
        ParseLyricsError::OrphanedShorthandMarker(OrphanedShorthandMarker {
            line_number: 4,
            content: "cre: credit body".to_string(),
        }),
    );
}

/// A name stops at the first character it does not admit, and the
/// rest comes back for the next layer to interpret.
#[test]
fn tag_name_takes_a_name_and_returns_the_tail() {
    for (source, tail) in [
        ("additive", ""),
        ("additive>", ">"),
        ("additive> trailing", "> trailing"),
        ("additive >", " >"),
        ("additive/>", "/>"),
        ("additive<", "<"),
        ("additive.b>", ".b>"),
    ] {
        eprintln!("CASE: {source:?}");
        assert_eq!(TagName::take(source), Some((TagName::Additive, tail)));
    }
}

/// The format defines the names, so a run of name characters that
/// denotes none of them is not a name at all.
#[test]
fn tag_name_rejects_a_run_that_names_nothing() {
    for source in [
        "",
        "verse>",
        "additives>",
        "a>",
        "append-only>",
        "verse2>",
        ">",
        "/additive>",
        "-additive>",
        "2additive>",
        "Additive>",
        " additive>",
    ] {
        eprintln!("CASE: {source:?}");
        assert_eq!(TagName::take(source), None);
    }
}

#[test]
fn opening_tag_takes_a_tag_and_returns_the_tail() {
    for (source, tail) in [("<additive>", ""), ("<additive> trailing", " trailing")] {
        eprintln!("CASE: {source:?}");
        assert_eq!(
            OpeningTag::take(source),
            Some((OpeningTag(TagName::Additive), tail)),
        );
    }
}

/// The three components sit flush against each other. Whitespace
/// between any two of them is what makes an attribute list possible,
/// and the format has no attributes.
#[test]
fn opening_tag_rejects_anything_but_the_three_components_flush() {
    for source in [
        "</additive>",
        "< additive>",
        "<additive >",
        "<additive",
        "<>",
        "<verse>",
        "additive>",
        "",
    ] {
        eprintln!("CASE: {source:?}");
        assert_eq!(OpeningTag::take(source), None);
    }
}

#[test]
fn closing_tag_takes_a_tag_and_returns_the_tail() {
    for (source, tail) in [("</additive>", ""), ("</additive> trailing", " trailing")] {
        eprintln!("CASE: {source:?}");
        assert_eq!(
            ClosingTag::take(source),
            Some((ClosingTag(TagName::Additive), tail)),
        );
    }
}

/// An opening tag is not a closing one, so the two parsers cannot
/// both match a line and the caller may try them in either order.
#[test]
fn closing_tag_rejects_anything_but_the_three_components_flush() {
    for source in [
        "<additive>",
        "</ additive>",
        "</additive >",
        "< /additive>",
        "</additive",
        "</>",
        "</verse>",
        "",
    ] {
        eprintln!("CASE: {source:?}");
        assert_eq!(ClosingTag::take(source), None);
    }
}

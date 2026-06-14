use super::{
    Bracketed, CreditRoles, NameSegment, ParseBracketedError, ParseCreditError, SeparatorStyle,
    Unbracketed, UnknownRole, parse_credit_line,
};
use lyrics_core::credits_descriptor::CreditsDesc;
use lyrics_core::video_descriptor::Language;
use maplit::btreemap;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;

fn make_descriptor(roles: &[&str]) -> CreditsDesc {
    CreditsDesc {
        credit_roles: roles
            .iter()
            .map(|role| btreemap! { Language::Vietnamese => role.to_string() })
            .collect(),
        credit_names: Vec::new(),
    }
}

fn make_roles(descriptor: &CreditsDesc) -> CreditRoles<'_> {
    CreditRoles::from_descriptor(descriptor, &Language::Vietnamese)
}

#[test]
fn from_descriptor_deduplicates_non_adjacent_entries_and_orders_longest_first() {
    // The input exercises three contractual properties at once:
    //   * `作詞` and `long-role` each appear twice with other entries
    //     between them, so a refactor that relies on `Vec::dedup` or
    //     `Itertools::dedup` leaves the duplicates in.
    //   * `abc` and `mid` are both 3 bytes long, so the ascending
    //     lexicographic tiebreak is observable.
    //   * `作詞` is 6 bytes (two CJK code points) and ranks above the
    //     3-byte ASCII entries, making byte length, not character
    //     count, the load-bearing measure.
    let descriptor = make_descriptor(&[
        "作詞",
        "long-role",
        "mid",
        "very-long-role",
        "作詞",
        "long-role",
        "abc",
    ]);
    let roles = make_roles(&descriptor);
    assert_eq!(
        roles.0,
        ["very-long-role", "long-role", "作詞", "abc", "mid"],
    );
}

#[test]
fn colon_separated_line_yields_one_pair_per_cell() {
    let descriptor = make_descriptor(&["role-a", "role-b", "role-c"]);
    let roles = make_roles(&descriptor);
    let parsed = parse_credit_line(
        "role-a：name-a\u{3000}role-b：name-b\u{3000}role-c：name-c",
        &roles,
    )
    .unwrap();
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].role, "role-a");
    assert_eq!(parsed[0].separator, "：");
    assert_eq!(
        parsed[0].name_segments,
        [NameSegment::Unbracketed(Unbracketed("name-a"))],
    );
    assert_eq!(parsed[1].role, "role-b");
    assert_eq!(parsed[2].role, "role-c");
}

#[test]
fn two_space_separated_line_yields_one_pair_with_embedded_spaces() {
    let descriptor = make_descriptor(&["role-a"]);
    let roles = make_roles(&descriptor);
    let parsed = parse_credit_line("role-a  name-a  name-b", &roles).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].role, "role-a");
    assert_eq!(parsed[0].separator, "  ");
    assert_eq!(
        parsed[0].name_segments,
        [NameSegment::Unbracketed(Unbracketed("name-a  name-b"))],
    );
}

#[test]
fn tolerates_runs_wider_than_two_spaces() {
    let descriptor = make_descriptor(&["role-a"]);
    let roles = make_roles(&descriptor);
    let parsed = parse_credit_line("role-a   name-a", &roles).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].role, "role-a");
    assert_eq!(parsed[0].separator, "   ");
    assert_eq!(
        parsed[0].name_segments,
        [NameSegment::Unbracketed(Unbracketed("name-a"))],
    );
}

#[test]
fn longer_role_wins_over_shorter_prefix() {
    let descriptor = make_descriptor(&["role", "role-a"]);
    let roles = make_roles(&descriptor);
    let parsed = parse_credit_line("role-a  name-a", &roles).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].role, "role-a");
}

#[test]
fn unknown_leading_text_errors() {
    let descriptor = make_descriptor(&["role-a"]);
    let roles = make_roles(&descriptor);
    assert_eq!(
        parse_credit_line("unknown  name-a", &roles).unwrap_err(),
        ParseCreditError::UnknownRole(UnknownRole {
            line: "unknown  name-a".to_string(),
            offset: 0,
        }),
    );
}

#[test]
fn recognizes_lenticular_highlight() {
    let descriptor = make_descriptor(&["role-a"]);
    let roles = make_roles(&descriptor);
    let parsed = parse_credit_line("role-a  name-a【label-a】", &roles).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0].name_segments,
        [
            NameSegment::Unbracketed(Unbracketed("name-a")),
            NameSegment::Bracketed("【label-a】".pipe(Bracketed::try_from).unwrap()),
        ],
    );
}

#[test]
fn multiple_highlights_interleave_with_plain_text() {
    let descriptor = make_descriptor(&["role-a"]);
    let roles = make_roles(&descriptor);
    let parsed = parse_credit_line("role-a  【label-a】name-a 【label-b】name-b", &roles).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0].name_segments,
        [
            "【label-a】"
                .pipe(Bracketed::try_from)
                .unwrap()
                .pipe(NameSegment::Bracketed),
            NameSegment::Unbracketed(Unbracketed("name-a ")),
            "【label-b】"
                .pipe(Bracketed::try_from)
                .unwrap()
                .pipe(NameSegment::Bracketed),
            NameSegment::Unbracketed(Unbracketed("name-b")),
        ],
    );
}

#[test]
fn separator_style_follows_the_colon_glyph() {
    let descriptor = make_descriptor(&["role-a"]);
    let roles = make_roles(&descriptor);

    // A full-width colon selects the CJK layout, even when an ASCII
    // colon shares the run, because the full-width glyph takes
    // priority.
    let full_width = parse_credit_line("role-a：name-a", &roles).unwrap();
    assert_eq!(
        full_width[0].separator_style(),
        SeparatorStyle::FullWidthColon,
    );
    let mixed = parse_credit_line("role-a:：name-a", &roles).unwrap();
    assert_eq!(mixed[0].separator_style(), SeparatorStyle::FullWidthColon);

    // A lone ASCII colon selects the Latin layout.
    let ascii = parse_credit_line("role-a: name-a", &roles).unwrap();
    assert_eq!(ascii[0].separator_style(), SeparatorStyle::AsciiColon);

    // A colon-free separator carries its captured run through for the
    // renderer to reproduce verbatim.
    let spaces = parse_credit_line("role-a  name-a", &roles).unwrap();
    assert_eq!(spaces[0].separator_style(), SeparatorStyle::Spaces("  "));
}

#[test]
fn role_span_suffix_emits_a_colon_only_for_the_latin_layout() {
    assert_eq!(SeparatorStyle::AsciiColon.role_span_suffix(), ":");
    assert_eq!(SeparatorStyle::FullWidthColon.role_span_suffix(), "");
    assert_eq!(SeparatorStyle::Spaces("  ").role_span_suffix(), "");
}

#[test]
fn between_span_separator_follows_the_layout() {
    let emit = |style: SeparatorStyle<'_>| {
        let mut output = String::new();
        style.append_between_spans(&mut output);
        output
    };
    assert_eq!(emit(SeparatorStyle::AsciiColon), " ");
    assert_eq!(emit(SeparatorStyle::FullWidthColon), "：");
    // A colon-free ASCII gutter round-trips verbatim.
    assert_eq!(emit(SeparatorStyle::Spaces("  ")), "  ");
    // `\u{3000}` IDEOGRAPHIC SPACE is not an ASCII gutter, so it
    // collapses to a single ASCII space.
    assert_eq!(emit(SeparatorStyle::Spaces("\u{3000}")), " ");
}

#[test]
fn bracketed_accepts_four_pair_kinds() {
    let (lenticular, rest) = Bracketed::take("【gold】tail").unwrap();
    assert_eq!(lenticular.as_str(), "【gold】");
    assert_eq!(rest, "tail");

    let (square, rest) = Bracketed::take("[silver]tail").unwrap();
    assert_eq!(square.as_str(), "[silver]");
    assert_eq!(rest, "tail");

    let (round, rest) = Bracketed::take("(bronze)tail").unwrap();
    assert_eq!(round.as_str(), "(bronze)");
    assert_eq!(rest, "tail");

    let (full_width_round, rest) = Bracketed::take("（铜）tail").unwrap();
    assert_eq!(full_width_round.as_str(), "（铜）");
    assert_eq!(rest, "tail");
}

#[test]
fn bracketed_rejects_non_bracket_prefix_and_nested_or_unclosed_brackets() {
    assert!(Bracketed::take("no bracket").is_none());
    assert!(Bracketed::take("【open only").is_none());
    assert!(Bracketed::take("[open only").is_none());
    assert!(Bracketed::take("(open only").is_none());
    assert!(Bracketed::take("【a【b】c】").is_none());
    assert!(Bracketed::take("[mismatch】").is_none());
    assert!(Bracketed::take("(also [nested])").is_none());
}

#[test]
fn bracketed_from_str_accepts_a_single_span() {
    let parsed: Bracketed = "【label-a】".try_into().unwrap();
    assert_eq!(parsed.as_str(), "【label-a】");
}

#[test]
fn bracketed_from_str_rejects_shape_mismatch() {
    // Whatever `take` would report as `None` maps to `ShapeMismatch`
    // for `FromStr`: empty input, no opening bracket, nested
    // bracket before the matching close, or end of input before
    // the close.
    for input in ["", "no bracket", "[open only", "【a【b】c】"] {
        assert_eq!(
            input.pipe(Bracketed::try_from).unwrap_err(),
            ParseBracketedError::ShapeMismatch,
        );
    }
}

#[test]
fn bracketed_from_str_rejects_unexpected_character_after_span() {
    // The first character past the closing bracket is what the
    // diagnostic reports; nothing else is needed to describe the
    // failure.
    assert_eq!(
        "【label-a】trailing".pipe(Bracketed::try_from).unwrap_err(),
        ParseBracketedError::UnexpectedCharacter('t'),
    );
}

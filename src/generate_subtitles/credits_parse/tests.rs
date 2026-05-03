use super::{
    Bracketed, CreditsVocabulary, NameSegment, ParseBracketedError, ParseCreditError, Unbracketed,
    UnknownRole, parse_credit_line,
};
use crate::credits_descriptor::CreditsDesc;
use crate::video_descriptor::Language;
use maplit::btreemap;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;

fn vocabulary(roles: &[&str]) -> CreditsVocabulary {
    let descriptor = CreditsDesc {
        credit_roles: roles
            .iter()
            .map(|role| btreemap! { Language::Vietnamese => role.to_string() })
            .collect(),
        credit_names: Vec::new(),
    };
    CreditsVocabulary::from_descriptor(&descriptor, &Language::Vietnamese)
}

#[test]
fn colon_separated_line_yields_one_pair_per_cell() {
    let v = vocabulary(&["role-a", "role-b", "role-c"]);
    let parsed = parse_credit_line(
        "role-a：name-a\u{3000}role-b：name-b\u{3000}role-c：name-c",
        &v,
    )
    .unwrap();
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].role, "role-a");
    assert_eq!(parsed[0].separator, "：");
    assert_eq!(
        parsed[0].name_segments,
        vec![NameSegment::Unbracketed(Unbracketed("name-a"))]
    );
    assert_eq!(parsed[1].role, "role-b");
    assert_eq!(parsed[2].role, "role-c");
}

#[test]
fn two_space_separated_line_yields_one_pair_with_embedded_spaces() {
    let v = vocabulary(&["role-a"]);
    let parsed = parse_credit_line("role-a  name-a  name-b", &v).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].role, "role-a");
    assert_eq!(parsed[0].separator, "  ");
    assert_eq!(
        parsed[0].name_segments,
        vec![NameSegment::Unbracketed(Unbracketed("name-a  name-b"))],
    );
}

#[test]
fn tolerates_runs_wider_than_two_spaces() {
    let v = vocabulary(&["role-a"]);
    let parsed = parse_credit_line("role-a   name-a", &v).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].role, "role-a");
    assert_eq!(parsed[0].separator, "   ");
    assert_eq!(
        parsed[0].name_segments,
        vec![NameSegment::Unbracketed(Unbracketed("name-a"))]
    );
}

#[test]
fn longer_role_wins_over_shorter_prefix() {
    let v = vocabulary(&["role", "role-a"]);
    let parsed = parse_credit_line("role-a  name-a", &v).unwrap();
    assert_eq!(parsed[0].role, "role-a");
}

#[test]
fn unknown_leading_text_errors() {
    let v = vocabulary(&["role-a"]);
    assert_eq!(
        parse_credit_line("unknown  name-a", &v).unwrap_err(),
        ParseCreditError::UnknownRole(UnknownRole {
            line: "unknown  name-a".to_string(),
            offset: 0,
        }),
    );
}

#[test]
fn recognizes_lenticular_highlight() {
    let v = vocabulary(&["role-a"]);
    let parsed = parse_credit_line("role-a  name-a【label-a】", &v).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0].name_segments,
        vec![
            NameSegment::Unbracketed(Unbracketed("name-a")),
            NameSegment::Bracketed("【label-a】".pipe(Bracketed::try_from).unwrap()),
        ],
    );
}

#[test]
fn multiple_highlights_interleave_with_plain_text() {
    let v = vocabulary(&["role-a"]);
    let parsed = parse_credit_line("role-a  【label-a】name-a 【label-b】name-b", &v).unwrap();
    assert_eq!(
        parsed[0].name_segments,
        vec![
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
fn bracketed_accepts_three_pair_kinds() {
    let (lenticular, rest) = Bracketed::take("【gold】tail").unwrap();
    assert_eq!(lenticular.as_str(), "【gold】");
    assert_eq!(rest, "tail");

    let (square, rest) = Bracketed::take("[silver]tail").unwrap();
    assert_eq!(square.as_str(), "[silver]");
    assert_eq!(rest, "tail");

    let (round, rest) = Bracketed::take("(bronze)tail").unwrap();
    assert_eq!(round.as_str(), "(bronze)");
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

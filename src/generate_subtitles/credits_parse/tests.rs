use super::{
    Bracketed, CreditsVocabulary, NameSegment, ParseCreditError, UnknownRole, parse_credit_line,
};
use crate::credits_descriptor::CreditsDesc;
use crate::video_descriptor::Language;
use maplit::btreemap;
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

fn bracketed(source: &str) -> Bracketed {
    let (value, rest) =
        Bracketed::take(source).expect("test fixture must be a valid bracketed prefix");
    assert!(
        rest.is_empty(),
        "test fixture must not carry trailing bytes past the closing bracket",
    );
    value
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
        vec![NameSegment::Plain("name-a".into())],
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
        vec![NameSegment::Plain("name-a  name-b".into())],
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
        vec![NameSegment::Plain("name-a".into())],
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
            NameSegment::Plain("name-a".into()),
            NameSegment::Special(bracketed("【label-a】")),
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
            NameSegment::Special(bracketed("【label-a】")),
            NameSegment::Plain("name-a ".into()),
            NameSegment::Special(bracketed("【label-b】")),
            NameSegment::Plain("name-b".into()),
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

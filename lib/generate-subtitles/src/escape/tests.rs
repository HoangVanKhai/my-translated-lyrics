use super::{Escaped, append_role_name_separator, role_span_suffix};
use crate::credits_parse::SeparatorStyle;
use pretty_assertions::assert_eq;

#[test]
fn role_span_suffix_emits_a_colon_only_for_the_latin_layout() {
    assert_eq!(role_span_suffix(SeparatorStyle::AsciiColon), ":");
    assert_eq!(role_span_suffix(SeparatorStyle::FullWidthColon), "");
    assert_eq!(role_span_suffix(SeparatorStyle::Spaces("  ")), "");
}

#[test]
fn between_span_separator_follows_the_layout() {
    let emit = |style| {
        let mut output = String::new();
        append_role_name_separator(&mut output, style);
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
fn plain_text_passes_through() {
    assert_eq!(Escaped("hello world").to_string(), "hello world");
    assert_eq!(Escaped("名字一").to_string(), "名字一");
    assert_eq!(Escaped("").to_string(), "");
}

#[test]
fn escapes_angle_brackets_and_ampersand() {
    assert_eq!(Escaped("<a>").to_string(), "&lt;a&gt;");
    assert_eq!(
        Escaped("name-a & name-b").to_string(),
        "name-a &amp; name-b",
    );
    assert_eq!(Escaped("<<>>").to_string(), "&lt;&lt;&gt;&gt;");
}

#[test]
fn escapes_pre_existing_entity_references() {
    // A raw `&lt;` in the source must become `&amp;lt;`, because the
    // `&` is a literal ampersand that itself needs to be escaped;
    // otherwise the output would be indistinguishable from a cue that
    // intended a `<` character.
    assert_eq!(Escaped("&lt;").to_string(), "&amp;lt;");
    assert_eq!(Escaped("&amp;").to_string(), "&amp;amp;");
}

#[test]
fn preserves_cjk_and_full_width_punctuation() {
    assert_eq!(Escaped("role-a：name-a").to_string(), "role-a：name-a");
    assert_eq!(Escaped("【gold】").to_string(), "【gold】");
}

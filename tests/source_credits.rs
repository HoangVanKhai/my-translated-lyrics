use into_sorted::IntoSorted;
use itertools::Itertools;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use pretty_yaml::config::{FormatOptions, LanguageOptions, Quotes};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::credits_descriptor::{CREDITS_CONFIG_FILE_NAME, CreditsDesc};
use translated_lyrics::video_descriptor::Language;

/// Each `sources/*/credits.yaml` must be in canonical form.
#[test]
fn source_credits_descriptors() {
    let sources_dir = env!("CARGO_MANIFEST_DIR").pipe(Path::new).join("sources");
    assert!(
        sources_dir.is_dir(),
        "expected sources directory to exist: {}",
        sources_dir.display()
    );

    let entries = sources_dir
        .pipe(read_dir)
        .unwrap()
        .map(Result::unwrap)
        .sorted_by_key(DirEntry::file_name);

    let format_options = FormatOptions {
        language: LanguageOptions {
            quotes: Quotes::PreferSingle,
            ..LanguageOptions::default()
        },
        ..FormatOptions::default()
    };

    for entry in entries {
        let song_dir = entry.path();
        if !song_dir.is_dir() {
            continue;
        }

        let credits_path = song_dir.join(CREDITS_CONFIG_FILE_NAME);
        if !credits_path.exists() {
            continue;
        }

        let song_name = entry.file_name();
        eprintln!("CASE: {}", song_name.display());

        let original = credits_path.pipe(read_to_string).unwrap();
        let desc: CreditsDesc = original.pipe_as_ref(serde_saphyr::from_str).unwrap();

        assert_eq!(
            desc.credit_roles,
            desc.credit_roles.clone().into_sorted(),
            "credit-roles in {song_name:?} are not in sorted order",
        );

        assert_eq!(
            desc.credit_names,
            desc.credit_names.clone().into_sorted(),
            "credit-names in {song_name:?} are not in sorted order",
        );

        let canonical = canonical_credits(&desc);
        let formatted = pretty_yaml::format_text(&canonical, &format_options).unwrap();
        assert_eq!(
            original, formatted,
            "{song_name:?} is not in canonical form",
        );
    }
}

/// Builds the canonical YAML text for a [`CreditsDesc`]: each
/// `credit-roles` and `credit-names` entry is emitted as a single-line
/// flow mapping, with a blank line separating the two top-level blocks.
fn canonical_credits(desc: &CreditsDesc) -> String {
    let mut out = String::new();
    writeln!(&mut out, "credit-roles:").unwrap();
    for entry in &desc.credit_roles {
        writeln!(&mut out, "  - {}", flow_map(entry)).unwrap();
    }
    writeln!(&mut out).unwrap();
    writeln!(&mut out, "credit-names:").unwrap();
    for entry in &desc.credit_names {
        writeln!(&mut out, "  - {}", flow_map(entry)).unwrap();
    }
    out
}

fn flow_map(entry: &BTreeMap<Language, String>) -> String {
    let parts: Vec<String> = entry
        .iter()
        .map(|(lang, val)| format!("{lang}: {val}"))
        .collect();
    format!("{{ {} }}", parts.join(", "))
}

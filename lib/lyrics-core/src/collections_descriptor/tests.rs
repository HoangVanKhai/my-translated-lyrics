use super::{CollectionName, CollectionsDesc, ParseCollectionNameError};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;

/// Wraps a collection name that the fixtures know to be of a valid shape.
fn collection_name(name: impl Into<String>) -> CollectionName {
    name.into()
        .pipe(CollectionName::try_from)
        .expect("test fixture passes the collection-name validator")
}

/// A manifest in the shape of a real `collections.toml`, with a nested
/// separated collection alongside a plain one.
fn manifest() -> CollectionsDesc {
    include_str!("fixtures/collections.toml")
        .pipe(toml::from_str::<CollectionsDesc>)
        .unwrap()
}

#[test]
fn manifest_declares_every_collection() {
    let desc = manifest();
    let unified: Vec<&str> = desc.unified.iter().map(|name| &**name).collect();
    assert_eq!(unified, ["Example Unified Collection"]);
    let separated: Vec<&str> = desc.separated.iter().map(|name| &**name).collect();
    assert_eq!(
        separated,
        [
            "Example Collection",
            "Example Group/Another Example Collection"
        ],
    );
}

/// [`CollectionsDesc::names`] lists the separated collections in
/// declaration order and the unified collection last.
#[test]
fn names_lists_the_separated_collections_then_the_unified_one() {
    let desc = manifest();
    let names: Vec<&str> = desc.names().map(|name| &**name).collect();
    assert_eq!(
        names,
        [
            "Example Collection",
            "Example Group/Another Example Collection",
            "Example Unified Collection",
        ],
    );
}

/// An unknown key is a typo or a stale field rather than a value the
/// manifest carries, so parsing rejects it.
/// The `separated` field may be left out, which declares no separated
/// collection rather than failing the parse.
#[test]
fn manifest_defaults_separated_to_an_empty_list() {
    let desc = r#"unified = ["Example Unified Collection"]"#
        .pipe(toml::from_str::<CollectionsDesc>)
        .unwrap();
    assert!(desc.separated.is_empty());
    let names: Vec<&str> = desc.names().map(|name| &**name).collect();
    assert_eq!(names, ["Example Unified Collection"]);
}

/// The `unified` field may be left out too, which declares no unified
/// collection rather than failing the parse.
#[test]
fn manifest_defaults_unified_to_an_empty_list() {
    let desc =
        r#"separated = ["Example Collection"]"#.pipe(toml::from_str::<CollectionsDesc>).unwrap();
    assert!(desc.unified.is_empty());
    let names: Vec<&str> = desc.names().map(|name| &**name).collect();
    assert_eq!(names, ["Example Collection"]);
}

#[test]
fn manifest_rejects_an_unknown_field() {
    let source = r#"
unified = ["Example Unified Collection"]
separated = []
unified-collection = ["Example Unified Collection"]
"#;
    assert!(source.pipe(toml::from_str::<CollectionsDesc>).is_err());
}

#[test]
fn check_separated_accepts_a_declared_collection() {
    let desc = manifest();
    for name in [
        "Example Collection",
        "Example Group/Another Example Collection",
    ] {
        eprintln!("CASE: {name:?}");
        desc.check_separated(&collection_name(name)).unwrap();
    }
}

/// The unified collection is not a collection a video descriptor may
/// name, so it is rejected like any other undeclared name.
#[test]
fn check_separated_rejects_an_undeclared_collection() {
    let desc = manifest();
    for name in [
        "Undeclared Example Collection",
        "Example Unified Collection",
    ] {
        eprintln!("CASE: {name:?}");
        let error = desc.check_separated(&collection_name(name)).unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with(&format!("unknown collection: {name:?}")),
            "unexpected message: {error}",
        );
    }
}

/// A name that a declared one is a few edits away from reads as a typo,
/// so the message points at the declared name.
#[test]
fn check_separated_hints_the_closest_declared_collection() {
    let desc = manifest();
    let cases = [
        ("example collection", "Example Collection"),
        ("ExampleCollection", "Example Collection"),
        (
            "Example Group/Another Example Collections",
            "Example Group/Another Example Collection",
        ),
    ];
    for (name, closest) in cases {
        eprintln!("CASE: {name:?}");
        let error = desc.check_separated(&collection_name(name)).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("unknown collection: {name:?}, did you mean {closest:?}?"),
        );
    }
}

/// A name that resembles nothing declared gets no hint, because a
/// suggestion nobody asked for is worse than none.
#[test]
fn check_separated_omits_the_hint_when_nothing_is_close() {
    let desc = manifest();
    let error = desc
        .check_separated(&collection_name("Undeclared Example Collection"))
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        r#"unknown collection: "Undeclared Example Collection""#,
    );
}

#[test]
fn collection_name_accepts_plain_components() {
    let cases = [
        "Example Collection",
        "Example Group/Another Example Collection",
        "示例合集",
    ];
    for input in cases {
        eprintln!("CASE: {input:?}");
        let name = input.to_string().pipe(CollectionName::try_from).unwrap();
        assert_eq!(&*name, input);
    }
}

#[test]
fn collection_name_rejects_backslash() {
    assert!(matches!(
        r"Example\Collection"
            .to_string()
            .pipe(CollectionName::try_from),
        Err(ParseCollectionNameError::ContainsBackslash),
    ));
}

#[test]
fn collection_name_rejects_empty() {
    assert!(matches!(
        String::new().pipe(CollectionName::try_from),
        Err(ParseCollectionNameError::Empty),
    ));
}

/// A slash that separates nothing, whether it leads, trails, or follows
/// another slash, is the one way a component can come out empty.
#[test]
fn collection_name_rejects_stray_slash() {
    let cases = [
        "/Example Collection",
        "Example Collection/",
        "Example//Collection",
    ];
    for input in cases {
        eprintln!("CASE: {input:?}");
        assert!(matches!(
            input.to_string().pipe(CollectionName::try_from),
            Err(ParseCollectionNameError::StraySlash),
        ));
    }
}

#[test]
fn collection_name_rejects_relative_component() {
    let cases = ["..", ".", "../Example Collection", "Example/./Collection"];
    for input in cases {
        eprintln!("CASE: {input:?}");
        assert!(matches!(
            input.to_string().pipe(CollectionName::try_from),
            Err(ParseCollectionNameError::RelativeComponent),
        ));
    }
}

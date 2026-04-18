use super::WordsDesc;
use crate::video_descriptor::Language;
use maplit::hashmap;
use pipe_trait::Pipe;

#[test]
fn empty_words_desc_parses() {
    let toml = "";
    toml.pipe(toml::from_str::<WordsDesc>).unwrap();
}

#[test]
fn words_desc_parses_credit_roles() {
    let toml = r#"
[credit-roles]
zh = ["演唱", "作曲"]
vi = ["Trình bày", "Soạn nhạc"]
"#;
    let desc = toml.pipe(toml::from_str::<WordsDesc>).unwrap();
    assert_eq!(
        desc.credit_roles,
        hashmap! {
            Language::Chinese => vec!["演唱".to_string(), "作曲".to_string()],
            Language::Vietnamese => vec!["Trình bày".to_string(), "Soạn nhạc".to_string()],
        }
    );
}

#[test]
fn words_desc_parses_credit_names() {
    let toml = r#"
[credit-names]
zh = ["洛天依", "乐正绫"]
vi = ["Luo Tianyi", "Yuezheng Ling"]
"#;
    let desc = toml.pipe(toml::from_str::<WordsDesc>).unwrap();
    assert_eq!(
        desc.credit_names,
        hashmap! {
            Language::Chinese => vec!["洛天依".to_string(), "乐正绫".to_string()],
            Language::Vietnamese => vec!["Luo Tianyi".to_string(), "Yuezheng Ling".to_string()],
        }
    );
}

#[test]
fn words_desc_parses_songstress_names() {
    let toml = r#"
[songstress-names.zh]
LTY = "洛天依"
YZL = "乐正绫"
"Y+L" = "洛天依 & 乐正绫"

[songstress-names.vi]
LTY = "Lạc Thiên Y"
YZL = "Nhạc Chính Lăng"
"Y+L" = "Lạc Thiên Y & Nhạc Chính Lăng"
"#;
    let desc = toml.pipe(toml::from_str::<WordsDesc>).unwrap();
    assert_eq!(
        desc.songstress_names,
        hashmap! {
            Language::Chinese => hashmap! {
                "LTY".to_string() => "洛天依".to_string(),
                "YZL".to_string() => "乐正绫".to_string(),
                "Y+L".to_string() => "洛天依 & 乐正绫".to_string(),
            },
            Language::Vietnamese => hashmap! {
                "LTY".to_string() => "Lạc Thiên Y".to_string(),
                "YZL".to_string() => "Nhạc Chính Lăng".to_string(),
                "Y+L".to_string() => "Lạc Thiên Y & Nhạc Chính Lăng".to_string(),
            },
        }
    );
}

#[test]
fn words_desc_parses_song_titles() {
    let toml = r#"
[song-titles]
zh = "青檀記"
vi = "Thanh Đàn Ký"
"#;
    let desc = toml.pipe(toml::from_str::<WordsDesc>).unwrap();
    assert_eq!(
        desc.song_titles,
        hashmap! {
            Language::Chinese => "青檀記".to_string(),
            Language::Vietnamese => "Thanh Đàn Ký".to_string(),
        }
    );
}

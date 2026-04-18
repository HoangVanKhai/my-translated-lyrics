use itertools::Itertools;
use maplit::btreemap;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::subtitle_descriptor::{SUBTITLE_CONFIG_FILE_NAME, SubtitleDesc};
use translated_lyrics::video_descriptor::Language::{self, Chinese as Zh, Vietnamese as Vi};

fn load_subtitle_desc(rel_path: &str) -> SubtitleDesc {
    env!("CARGO_MANIFEST_DIR")
        .pipe(Path::new)
        .join(rel_path)
        .pipe(read_to_string)
        .unwrap()
        .pipe_as_ref(serde_saphyr::from_str::<SubtitleDesc>)
        .unwrap()
}

/// Every `sources/*/subtitle.yaml` must parse as a valid [`SubtitleDesc`].
#[test]
fn source_subtitle_descriptors_are_valid() {
    let sources_dir = env!("CARGO_MANIFEST_DIR").pipe(Path::new).join("sources");
    assert!(
        sources_dir.is_dir(),
        "expected sources directory to exist for subtitle descriptor validation: {}",
        sources_dir.display()
    );

    let entries = sources_dir
        .pipe(read_dir)
        .unwrap()
        .map(Result::unwrap)
        .sorted_by_key(DirEntry::file_name);

    for entry in entries {
        let song_dir = entry.path();
        if !song_dir.is_dir() {
            continue;
        }

        let subtitle_path = song_dir.join(SUBTITLE_CONFIG_FILE_NAME);
        if !subtitle_path.exists() {
            continue;
        }

        eprintln!("CASE: {}", entry.file_name().display());
        subtitle_path
            .pipe(read_to_string)
            .unwrap()
            .pipe_as_ref(serde_saphyr::from_str::<SubtitleDesc>)
            .unwrap()
            .pipe(drop::<SubtitleDesc>);
    }
}

fn chinese_only(value: &str) -> BTreeMap<Language, String> {
    btreemap! { Zh => value.to_string() }
}

fn chinese_vietnamese(zh_value: &str, vi_value: &str) -> BTreeMap<Language, String> {
    btreemap! { Zh => zh_value.to_string(), Vi => vi_value.to_string() }
}

#[test]
fn farewell_to_jianghu_credit_roles_order() {
    let desc = load_subtitle_desc("sources/FarewellToJianghu-ChangGeYiQuJianghuYuan/subtitle.yaml");
    assert_eq!(
        desc.credit_roles,
        vec![
            chinese_vietnamese("作词", "Tác từ"),
            chinese_vietnamese("编曲", "Biên khúc"),
            chinese_vietnamese("VSINGER", "VSINGER"),
            chinese_vietnamese("笛子", "Sáo"),
            chinese_vietnamese("作曲", "Tác khúc"),
            chinese_vietnamese("演唱", "Trình bày"),
            chinese_vietnamese("调校", "Điều giáo"),
            chinese_vietnamese("制作人", "Sản xuất"),
            chinese_vietnamese("Ｖ家曲绘", "Hình ảnh"),
            chinese_vietnamese("美术", "Mỹ thuật"),
            chinese_vietnamese("PV", "PV"),
            chinese_vietnamese("鸣谢素材", "Cảm ơn"),
        ]
    );
}

#[test]
fn farewell_to_jianghu_credit_names_order() {
    let desc = load_subtitle_desc("sources/FarewellToJianghu-ChangGeYiQuJianghuYuan/subtitle.yaml");
    assert_eq!(
        desc.credit_names,
        vec![
            chinese_vietnamese("雨観", "雨観"),
            chinese_vietnamese("雨观", "雨观"),
            chinese_vietnamese("洛天依X楽正绫", "Luo Tianyi X Yuezheng Ling"),
            chinese_vietnamese("再也不上课", "再也不上课"),
            chinese_vietnamese("雨観X溪里", "雨観X溪里"),
            chinese_vietnamese("鬼面P", "鬼面P"),
            chinese_vietnamese("Vsinger团队", "Vsinger团队"),
            chinese_vietnamese("碎夊", "碎夊"),
            chinese_vietnamese("超级水水", "超级水水"),
            chinese_vietnamese("一勺酸橙汁", "一勺酸橙汁"),
            chinese_vietnamese("Ａ影羌", "Ａ影羌"),
            chinese_vietnamese("璇玑坊Studio", "璇玑坊Studio"),
            chinese_vietnamese("废画", "废画"),
            chinese_vietnamese("良月十八", "良月十八"),
            chinese_vietnamese("无声诗", "无声诗"),
            chinese_vietnamese("山晚樵渔", "山晚樵渔"),
            chinese_vietnamese("今日晴", "今日晴"),
            chinese_vietnamese("九镜", "九镜"),
        ]
    );
}

#[test]
fn tale_of_qingtan_credit_roles_order() {
    let desc = load_subtitle_desc("sources/TaleOfQingtan-QingtanJi/subtitle.yaml");
    assert_eq!(
        desc.credit_roles,
        vec![
            chinese_vietnamese("演唱", "Trình bày"),
            chinese_vietnamese("作曲", "Soạn nhạc"),
            chinese_vietnamese("作词", "Viết lời"),
            chinese_vietnamese("编曲", "Phối khí"),
            chinese_vietnamese("企划", "Kế hoạch"),
            chinese_vietnamese("制作人", "Nhà sản xuất"),
            chinese_vietnamese("调校", "Căn chỉnh giọng hát"),
            chinese_vietnamese("和声编写", "Viết hòa thanh"),
            chinese_vietnamese("混音", "Hòa âm"),
            chinese_vietnamese("美术", "Mỹ thuật"),
            chinese_vietnamese("视频", "Video"),
            chinese_vietnamese("素材鸣谢", "Tư liệu tham chiếu"),
        ]
    );
}

#[test]
fn tale_of_qingtan_credit_names_order() {
    let desc = load_subtitle_desc("sources/TaleOfQingtan-QingtanJi/subtitle.yaml");
    assert_eq!(
        desc.credit_names,
        vec![
            chinese_vietnamese("洛天依", "Luo Tianyi"),
            chinese_vietnamese("乐正绫", "Yuezheng Ling"),
            chinese_only("雨观"),
            chinese_only("三世"),
            chinese_only("鬼面Ｐ"),
            chinese_only("一勺酸橙汁"),
            chinese_only("Ａ影羌"),
            chinese_only("璇玑坊Studio"),
            chinese_only("废画"),
            chinese_only("良月十八"),
            chinese_only("无声诗"),
            chinese_only("山晚樵渔"),
            chinese_only("今日晴"),
            chinese_only("九镜"),
        ]
    );
}

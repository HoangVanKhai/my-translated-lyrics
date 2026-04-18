use itertools::Itertools;
use maplit::btreemap;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::subtitle_descriptor::{SUBTITLE_CONFIG_FILE_NAME, SubtitleDesc};
use translated_lyrics::video_descriptor::Language::{self, Chinese as Zh, Vietnamese as Vi};

fn load_subtitle(rel_path: &str) -> SubtitleDesc {
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

fn zh(value: &str) -> BTreeMap<Language, String> {
    btreemap! { Zh => value.to_string() }
}

fn zh_vi(zh_value: &str, vi_value: &str) -> BTreeMap<Language, String> {
    btreemap! { Zh => zh_value.to_string(), Vi => vi_value.to_string() }
}

#[test]
fn farewell_to_jianghu_credit_roles_order() {
    let desc = load_subtitle("sources/FarewellToJianghu-ChangGeYiQuJianghuYuan/subtitle.yaml");
    assert_eq!(
        desc.credit_roles,
        vec![
            zh_vi("作词", "Tác từ"),
            zh_vi("编曲", "Biên khúc"),
            zh_vi("VSINGER", "VSINGER"),
            zh_vi("笛子", "Sáo"),
            zh_vi("作曲", "Tác khúc"),
            zh_vi("演唱", "Trình bày"),
            zh_vi("调校", "Điều giáo"),
            zh_vi("制作人", "Sản xuất"),
            zh_vi("Ｖ家曲绘", "Hình ảnh"),
            zh_vi("美术", "Mỹ thuật"),
            zh_vi("PV", "PV"),
            zh_vi("鸣谢素材", "Cảm ơn"),
        ]
    );
}

#[test]
fn farewell_to_jianghu_credit_names_order() {
    let desc = load_subtitle("sources/FarewellToJianghu-ChangGeYiQuJianghuYuan/subtitle.yaml");
    assert_eq!(
        desc.credit_names,
        vec![
            zh_vi("雨観", "雨観"),
            zh_vi("雨观", "雨观"),
            zh_vi("洛天依X楽正绫", "Luo Tianyi X Yuezheng Ling"),
            zh_vi("再也不上课", "再也不上课"),
            zh_vi("雨観X溪里", "雨観X溪里"),
            zh_vi("鬼面P", "鬼面P"),
            zh_vi("Vsinger团队", "Vsinger团队"),
            zh_vi("碎夊", "碎夊"),
            zh_vi("超级水水", "超级水水"),
            zh_vi("一勺酸橙汁", "一勺酸橙汁"),
            zh_vi("Ａ影羌", "Ａ影羌"),
            zh_vi("璇玑坊Studio", "璇玑坊Studio"),
            zh_vi("废画", "废画"),
            zh_vi("良月十八", "良月十八"),
            zh_vi("无声诗", "无声诗"),
            zh_vi("山晚樵渔", "山晚樵渔"),
            zh_vi("今日晴", "今日晴"),
            zh_vi("九镜", "九镜"),
        ]
    );
}

#[test]
fn tale_of_qingtan_credit_roles_order() {
    let desc = load_subtitle("sources/TaleOfQingtan-QingtanJi/subtitle.yaml");
    assert_eq!(
        desc.credit_roles,
        vec![
            zh_vi("演唱", "Trình bày"),
            zh_vi("作曲", "Soạn nhạc"),
            zh_vi("作词", "Viết lời"),
            zh_vi("编曲", "Phối khí"),
            zh_vi("企划", "Kế hoạch"),
            zh_vi("制作人", "Nhà sản xuất"),
            zh_vi("调校", "Căn chỉnh giọng hát"),
            zh_vi("和声编写", "Viết hòa thanh"),
            zh_vi("混音", "Hòa âm"),
            zh_vi("美术", "Mỹ thuật"),
            zh_vi("视频", "Video"),
            zh_vi("素材鸣谢", "Tư liệu tham chiếu"),
        ]
    );
}

#[test]
fn tale_of_qingtan_credit_names_order() {
    let desc = load_subtitle("sources/TaleOfQingtan-QingtanJi/subtitle.yaml");
    assert_eq!(
        desc.credit_names,
        vec![
            zh_vi("洛天依", "Luo Tianyi"),
            zh_vi("乐正绫", "Yuezheng Ling"),
            zh("雨观"),
            zh("三世"),
            zh("鬼面Ｐ"),
            zh("一勺酸橙汁"),
            zh("Ａ影羌"),
            zh("璇玑坊Studio"),
            zh("废画"),
            zh("良月十八"),
            zh("无声诗"),
            zh("山晚樵渔"),
            zh("今日晴"),
            zh("九镜"),
        ]
    );
}

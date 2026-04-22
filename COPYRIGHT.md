# Copyright Notice for Non-Software Material

The MIT License in [`LICENSE.md`](./LICENSE.md) covers the software in
this repository, but not the song lyrics or their translations. This
file documents the status of those non-software files. Three categories
are described below: the original Chinese-language lyrics (Section 1),
the Vietnamese translations of those lyrics (Section 2), and the
metadata files that mix material from both (Section 3).

The repository author, Hoàng Văn Khải, is not a copyright lawyer.
Nothing in this file constitutes legal advice. The notices below
describe the author's intent and best understanding; they cannot
override the rights of any other copyright holder.

## 1. Original Chinese-language lyrics

Files in this category contain the original Chinese-language lyrics of
songs that the repository author did not write. They are reproduced for
non-commercial study, archival, and translation reference purposes
only. The repository author claims no copyright in them.

The files in this category are:

- `dist/*/lyrics.zh.srt`
- `dist/*/lyrics.zh.vtt`
- `sources/*/lyrics.zh.txt`
- `sources/*/lyrics.zh.srt`
- `sources/*/lyrics.zh.vtt`
- `drafts/MillenniumOfFrostAndSnow-ShuangXueQianNian/luatube712.zh-TW.srt`
- `drafts/MillenniumOfFrostAndSnow-ShuangXueQianNian/luotube712.zh-CN.srt`

Each lyrics file begins with a `cre` (credits) block that names the
people involved in producing the song, including the lyricist, composer,
arranger, vocalists, illustrator, and video editor. That credits block
is the canonical record of who holds copyright in the underlying song,
and this notice does not attempt to duplicate it.

Anyone who wishes to use the original lyrics independently of this
repository should obtain permission from the original rightsholder
identified in the relevant credits block. If you are a rightsholder of
any of these works and would prefer that the material be removed or
that a different attribution be used, please open an issue on this
repository.

## 2. Vietnamese translations

Files in this category contain Vietnamese translations of the
corresponding Chinese-language songs. The translations were produced by
Hoàng Văn Khải.

The files in this category are:

- `dist/*/lyrics.vi.srt`
- `dist/*/lyrics.vi.vtt`
- `sources/*/lyrics.vi.txt`
- `sources/*/lyrics.vi.srt`
- `sources/*/lyrics.vi.vtt`

A translation is a derivative work. It carries its own copyright in the
translator, but the right to authorize others to reproduce, distribute,
or further adapt the translation depends on the copyright in the
underlying original. The translator has not, in general, secured a
license from the rightsholders of the original Chinese lyrics, and
makes no representation that downstream re-use is authorized by them.

To the extent that the translator's own contribution can be licensed
independently of the underlying work, Hoàng Văn Khải releases his
Vietnamese translations under the Creative Commons
Attribution-NonCommercial-ShareAlike 4.0 International License
(CC BY-NC-SA 4.0), available at
<https://creativecommons.org/licenses/by-nc-sa/4.0/>. Any downstream
use of a translated file must additionally clear the rights to the
underlying Chinese lyrics with the original rightsholder identified in
the relevant credits block. The translator's permission alone is not
sufficient.

## 3. Mixed metadata files

A small number of files combine Chinese names or titles with their
Vietnamese transliterations or translations. The Chinese-language
portions of these files are subject to Section 1, and the
Vietnamese-language portions are subject to Section 2. The structural
markup of the file (TOML keys, YAML keys, JSON keys, marker codes such
as `cre` or `ttl`, and similar scaffolding) is part of the software and
falls under [`LICENSE.md`](./LICENSE.md).

The files in this category are:

- `dist/*/video.toml`
- `sources/*/video.toml`
- `sources/*/credits.yaml`
- `sources/*/line-markers.toml`
- `drafts/FarewellToJianghu-ChangGeYiQuJianghuYuan/credits-and-titles.json`
- `drafts/FarewellToJianghu-ChangGeYiQuJianghuYuan/credits-and-titles.full.json`
- `drafts/MillenniumOfFrostAndSnow-ShuangXueQianNian/video.toml`

## Reporting concerns

If you are a rightsholder and have a concern about how your work is
reproduced here, or if you are a downstream user with a question about
re-use, please open an issue on this repository.

//! The colour kinds' How cards (`practice_demo.rs`), after V大's method in our own words (squint,
//! a colour's hue first, then its lightness and purity, pick once by eye and correct two or
//! three times; squint for the overall tone and cover the frame first, the big areas by colour,
//! each against its neighbours, weak contrasts kept weak), and the colour copy's, which has no
//! demonstration and names his four brushes, the toolbox's (S104). In his terms: 色相, 明度,
//! 纯度, 更亮/更暗, 更纯/更灰, 眯眼, 色调, 弱对比, 明度推移; the original's colour is 原图的颜色,
//! never 原色 (a primary colour).
//! A value scale's steps name the cells of the row in hand (S97's review): of five the 3rd, then
//! the 2nd and 4th; of nine the 5th, the 3rd and 7th, then the rest.

use openpaint_practice::halving;

use super::Language;
use super::practice_demo::{COMPARE_STEP, How};

/// A value scale's How steps before Compare for a row of `cells`, a step a round of halving: the
/// middle cell, the middle of each half, then the cells left.
pub(crate) fn scale_steps(language: Language, cells: usize) -> Vec<String> {
    let rounds = halving(cells);
    (rounds.iter().enumerate())
        .map(|(at, round)| {
            let named = cells_named(language, round);
            match (language, at) {
                (Language::English, 0) => {
                    format!("The middle cell first ({named}): halfway in lightness")
                }
                (Language::English, 1) => format!("Then the middle of each half ({named})"),
                (Language::English, _) => format!("Then the cells left ({named})"),
                (Language::Chinese, 0) => format!("先填正中的{named}：明度取两端的正中"),
                (Language::Chinese, 1) => format!("再填两半各自的正中：{named}"),
                (Language::Chinese, _) => format!("最后填剩下的{named}"),
            }
        })
        .collect()
}

/// Cells (indices from 0) named in order: "the 2nd and 4th", "第 2、4 格".
fn cells_named(language: Language, cells: &[usize]) -> String {
    let places: Vec<usize> = cells.iter().map(|cell| cell + 1).collect();
    match language {
        Language::Chinese => {
            let numbers: Vec<String> = places.iter().map(usize::to_string).collect();
            format!("第 {} 格", numbers.join("、"))
        }
        Language::English => {
            let ordinal = |n: &usize| match n % 10 {
                1 if n % 100 != 11 => format!("{n}st"),
                2 if n % 100 != 12 => format!("{n}nd"),
                3 if n % 100 != 13 => format!("{n}rd"),
                _ => format!("{n}th"),
            };
            let named: Vec<String> = places.iter().map(ordinal).collect();
            match named.split_last() {
                Some((last, rest)) if !rest.is_empty() => {
                    format!("the {} and {last}", rest.join(", "))
                }
                _ => format!("the {}", named.join("")),
            }
        }
    }
}

pub(crate) const COLOUR: How = How {
    heading: super::MATCH_A_COLOUR,
    trains: Some(COLOUR_TRAINS),
    steps: &[
        COLOUR_STEP_1,
        COLOUR_STEP_2,
        COLOUR_STEP_3,
        super::FILL_FRAME,
        COMPARE_STEP,
    ],
    says: &[COLOUR_SAYS, COLOUR_SUMMARY],
};
/// A combination's How names its pens in the order its hand uses them (S100).
pub(crate) const AREAS: How = How {
    heading: super::AREAS_EXERCISE,
    trains: Some(AREAS_TRAINS),
    steps: &[
        AREAS_STEP_1,
        AREAS_STEP_2,
        AREAS_STEP_3,
        AREAS_STEP_4,
        AREAS_STEP_5,
        COMPARE_STEP,
    ],
    says: &[AREAS_SAYS],
};
/// A value scale's How: its steps before Compare name the cells of the row in hand
/// (`scale_steps`).
pub(crate) const SCALE: How = How {
    heading: super::SCALE_EXERCISE,
    trains: Some(SCALE_TRAINS),
    steps: &[COMPARE_STEP],
    says: &[SCALE_SAYS],
};
/// A long colour copy's How names V大's four brushes, the toolbox's own (S104), and when to take
/// each, then his order of work with them.
pub(crate) const STUDY: How = How {
    heading: super::STUDY_EXERCISE,
    trains: None,
    steps: &[],
    says: &[STUDY_1, STUDY_BRUSHES, STUDY_2, STUDY_3, STUDY_4],
};

texts! { PRACTICE_DEMO_COLOUR;
    COLOUR_TRAINS = "Week 1 · a colour judged by its hue, lightness and purity, and corrected by \
        eye.", "第 1 周 · 从色相、明度、纯度判断颜色，凭眼睛修正。";
    COLOUR_STEP_1 = "Squint, the hue first: the family on the ring",
        "眯眼，先定色相：在色环上找到色系";
    COLOUR_STEP_2 = "Lighter or darker", "再比明度：更亮还是更暗";
    COLOUR_STEP_3 = "Purer or greyer", "再比纯度：更纯还是更灰";
    COLOUR_SAYS = "Compare: on the wheel yours is ringed in ink, the original in red; under it ΔE, \
        how different they look (about 2 just noticeable), and its lightness, chroma and hue, \
        each a bar from the original's red line to yours. Correct the longest first, two or three \
        times. The swatch can't be picked before.",
        "叠图对比：色环上墨线圈出的是你的颜色，红圈是原图的颜色；下面的 ΔE 表示两者看起来差多少（约 2 刚能看出），再分明度、纯度、色相三行，每行的横条从原图颜色的红线画到你的颜色。先改差得最多的一项，改两三次。叠图对比之前，不能吸原图的色块。";
    COLOUR_SUMMARY = "The summary's shape is your colour judgement: the rim exact, your earlier \
        sessions dashed.", "总结里的这个图形就是你的辨色能力：最外圈是完全准确，虚线是你以往的练习。";
    AREAS_TRAINS = "Week 2 · a corner of a rusted old thing or an abstract painting, a few colours \
        side by side: the overall tone first, each judged against its neighbours.",
        "第 2 周 · 生锈旧物或抽象画的一角，几块颜色挨在一起：先定整体色调，每块都和相邻的比。";
    AREAS_STEP_1 = "Squint: Fill frame with the overall tone", "眯眼看整体色调，用它填满方框";
    AREAS_STEP_2 = "Big: the largest area first", "用大笔先画最大的色块";
    AREAS_STEP_3 = "Big: each neighbour against it", "用大笔画相邻的色块，逐块和它比";
    AREAS_STEP_4 = "Small: the edges, the weak contrast kept weak", "用小笔收边，弱对比别画强";
    AREAS_STEP_5 = "Soft: only where the picture's edge is soft",
        "柔边笔只用在原图边缘柔和的地方";
    AREAS_SAYS = "Big and Small lay the colour you picked however lightly you press. The original \
        can't be picked before Compare; your copy can. Compare lays the original over your copy, \
        the slider setting how strongly; each chip, the original's colour over yours, shows its \
        pair on the wheel and in the table, its area outlined in ink; a weak contrast made strong \
        is red. Then the eyedropper compares any point.",
        "大笔、小笔不管压感轻重，画出的都是选中的颜色。叠图对比之前不能吸原图的颜色，可以吸你自己画的。叠图对比把原图叠在你的临摹上，浓淡由滑块调节；每个色卡上半是原图的颜色、下半是你的，点一个色卡，这一对颜色就显示在色环和表格里，它的色块用墨线描边；画强了的弱对比标成红色。之后可以用吸管点任意一处来比较。";
    SCALE_TRAINS = "Beside week 2's colour work · lightness as the eye sees it, in even steps.",
        "配合第 2 周的色彩练习 · 按眼睛看到的明度均匀推移。";
    SCALE_SAYS = "The numbers under the example's cells are the order. Keep the hue and purity: \
        move only up and down in the wheel's square; Fill cell fills the cell in hand, the ends \
        can be picked. Compare lays the even scale under your row, cell under cell; a cell of \
        yours that is uneven is outlined red with its even one. To correct it, touch it with the \
        pen to take it in hand and Fill cell again.",
        "示例格子下的数字就是填的顺序。色相和纯度保持不变：只在色环中间的方块里上下移动；“填满格子”填满选中的那一格，两端的颜色可以吸。叠图对比把均匀的推移排在你那一排下面，一格对一格；不均匀的格子和它下面的那格一起用红线描边。要修正，先用笔点一下那一格选中它，再点“填满格子”。";
    STUDY_1 = "Copy the picture in colour into the empty frame beside it, over as many sittings as \
        you like: about 10 to 20 hours in all. It never counts as a session.",
        "把这张图用颜色临摹到旁边的空白框里，分几次画都行，总共约 10 到 20 小时。不计入练习次数。";
    STUDY_BRUSHES = "Paint with V大's four brushes, in the Painting set. Hard Paint for blocks: \
        it lays the colour you picked however lightly you press. Hard Round for transitions: \
        pressed lightly it lays a thin layer, so two colours turn into each other. Soft Brush, \
        the airbrush, only where the picture's own edge or gradient is soft. Textured Round for \
        texture, later, once the colours are right.",
        "用“绘画”组里 V大的四支笔来画。硬边颜料画色块：不管压感轻重，画出的都是选中的颜色。硬圆笔画过渡：轻压时颜色薄，两种颜色就能自然过渡。柔边笔相当于喷枪，只用在原图边缘或渐变柔和的地方。纹理圆笔画质感，等颜色都对了再用。";
    STUDY_2 = "Squint for the overall tone and cover the frame first with Hard Paint, large; then \
        the middle tones, by colour, still with Hard Paint; then, smaller, the hierarchy and the \
        detail.",
        "眯眼看整体色调，先把硬边颜料调大，铺满整个框；再按颜色分出中间调，仍用硬边颜料；最后把笔调小，画色彩层级和细节。";
    STUDY_3 = "Bring every part to the same finish; zoom in and work a part as if it were the \
        whole picture. Keep weak contrasts weak.",
        "各处的完成度要一致；放大画局部时，把局部当成一张完整的画来画。弱对比别画强。";
    STUDY_4 = "The original can't be picked before Compare. Compare lays it over your copy in \
        place, as strongly as the slider sets; then the eyedropper on any point shows both \
        colours there on the wheel and in the table.",
        "叠图对比之前不能吸原图的颜色。叠图对比把原图原位叠在你的临摹上，浓淡由滑块调节；之后用吸管点任意一处，色环和表格里会显示原图和你在这一处的颜色。";
}

//! The colour kinds' How cards (`practice_demo.rs`), after V大's method in our own words (name a
//! colour by its hue, then its purity and lightness, pick once by eye and correct two or three
//! times; squint for the overall tone and cover the frame first, the big areas by colour, each
//! against its neighbours, weak contrasts kept weak), and the colour copy's, which has no
//! demonstration. In his terms: 色相, 纯度, 明度, 更纯/更灰, 更亮/更暗, 眯眼, 色调, 弱对比, 明度推移.

use super::practice_demo::{COMPARE_STEP, How};

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
pub(crate) const AREAS: How = How {
    heading: super::AREAS_EXERCISE,
    trains: Some(AREAS_TRAINS),
    steps: &[
        AREAS_STEP_1,
        AREAS_STEP_2,
        AREAS_STEP_3,
        AREAS_STEP_4,
        COMPARE_STEP,
    ],
    says: &[AREAS_SAYS],
};
pub(crate) const SCALE: How = How {
    heading: super::SCALE_EXERCISE,
    trains: Some(SCALE_TRAINS),
    steps: &[SCALE_STEP_1, SCALE_STEP_2, COMPARE_STEP],
    says: &[SCALE_SAYS],
};
pub(crate) const STUDY: How = How {
    heading: super::STUDY_EXERCISE,
    trains: None,
    steps: &[],
    says: &[STUDY_1, STUDY_2, STUDY_3, STUDY_4],
};

texts! { PRACTICE_DEMO_COLOUR;
    COLOUR_TRAINS = "Week 1 · a colour judged by its hue, purity and lightness, and corrected by \
        eye.", "第 1 周 · 按色相、纯度、明度判断颜色，凭眼睛修正。";
    COLOUR_STEP_1 = "Its hue first: the family on the ring", "先定色相：在色环上找到色系";
    COLOUR_STEP_2 = "Purer or greyer", "再比纯度：更纯还是更灰";
    COLOUR_STEP_3 = "Lighter or darker", "再比明度：更亮还是更暗";
    COLOUR_SAYS = "Compare: on the wheel yours is ringed in ink, the original in red; under it ΔE, \
        how different they look (about 2 just noticeable), and its lightness, chroma and hue, \
        each a bar from the original's red line to yours. Correct the longest first, two or three \
        times. The swatch can't be picked before.",
        "叠图对比：色轮上墨色圈是你的颜色，红色圈是原色；下面 ΔE 是看起来差多少（约 2 刚能看出），再分明度、纯度、色相，每行一条从原色红线到你的颜色的横条。先改最长的，改两三次。对比之前，吸管吸不到色块。";
    COLOUR_SUMMARY = "The summary's shape is your colour judgement: the rim exact, your earlier \
        sessions dashed.", "练习结束时的图形是你的颜色判断力：外圈为完全准确，虚线是你此前的练习。";
    AREAS_TRAINS = "Week 2 · colours side by side: the overall tone first, each judged against \
        its neighbours.", "第 2 周 · 几块颜色并置：先定整体色调，每块和旁边的比。";
    AREAS_STEP_1 = "Squint: cover the frame with the overall tone", "眯眼看整体色调，先铺满方框";
    AREAS_STEP_2 = "The largest area first, with Big", "大笔先画最大的色块";
    AREAS_STEP_3 = "Each neighbour against it", "相邻的色块逐一和它比";
    AREAS_STEP_4 = "Keep the weak contrast weak", "弱对比保持弱";
    AREAS_SAYS = "The original can't be picked before Compare; your copy can. Compare lays the \
        original over your copy, the slider setting how strongly; each chip, the original's \
        colour over yours, shows its pair on the wheel and in the table, its area outlined in \
        ink; a weak contrast made strong is red. Then the eyedropper compares any point.",
        "叠图对比之前不能吸原图，可以吸你自己的。叠图对比把原图叠在你的临摹上，滑块调节强弱；每个色片上为原色、下为你的颜色，点它在色轮和表里比较，它的色块用墨线勾出；画强了的弱对比标红。之后用吸管点任意位置比较。";
    SCALE_TRAINS = "Beside week 2's colour work · lightness as the eye sees it, in even steps.",
        "配合第 2 周的色彩练习 · 按眼睛看到的明度均匀推移。";
    SCALE_STEP_1 = "The middle cell: halfway in lightness", "先填中间一格：明度在两端正中";
    SCALE_STEP_2 = "The quarters: halfway again", "再填四分之一处：各取一半";
    SCALE_SAYS = "Keep the hue and purity: move only up and down in the wheel's square; Fill cell \
        fills the cell in hand, the ends can be picked. Compare shows the even scale in the \
        original's frame; under your row each cell's lightness as a bar, its even step a red \
        tick, an uneven cell's bar outlined red. Fill a cell again to correct it.",
        "保持色相和纯度：只在色轮的方块里上下移动；“填满格子”填满选中的格子，两端可以吸取。叠图对比时原图框里显示均匀的推移；你的格子下方每格的明度画成竖条，红线是均匀的那一级，不均匀的格子竖条描红。重填某一格来修正。";
    STUDY_1 = "Copy the picture in colour into the empty frame beside it, over as many sittings as \
        you like: about 10 to 20 hours in all. It never counts as a session.",
        "把这张图用颜色临摹到旁边的空白框里，分多次画完，总共约 10 到 20 小时。它不算在练习次数里。";
    STUDY_2 = "Squint for the overall tone and cover the frame first with a big brush; then the \
        middle tones, by colour; then the hierarchy and the detail, the brush smaller as you go.",
        "眯眼看整体色调，先用大笔铺满整个框；再按颜色分中间调；最后画层次和细节，笔越画越小。";
    STUDY_3 = "Bring every part to the same finish; zoom in and work a part as if it were the \
        whole picture. Keep weak contrasts weak.",
        "各处的完成度要一致；放大画局部时，把局部当成一张完整的画。弱对比要保持弱。";
    STUDY_4 = "The original can't be picked before Compare. Compare lays it over your copy in \
        place, as strongly as the slider sets; then the eyedropper on any point shows both \
        colours there on the wheel and in the table.",
        "叠图对比之前不能吸原图的颜色。叠图对比把原图叠在你的临摹上的原位，显示多少由滑块调节；之后用吸管点任意位置，两边该处的颜色会显示在色轮和表里。";
}

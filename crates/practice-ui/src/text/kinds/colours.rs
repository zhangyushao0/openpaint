//! A colour's words: its name and its How card, after V大's
//! method in our own words (squint, a colour's hue first, then its lightness and purity, pick once
//! by eye and correct two or three times). In his terms: 单色, 色相, 明度, 纯度, 更亮/更暗, 更纯/更灰,
//! 眯眼; the original's colour is 原图的颜色, never 原色 (a primary colour).

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("colour", COLOUR_EXERCISE, COLOUR)];

const COLOUR: How = How {
    heading: MATCH_A_COLOUR,
    trains: Some(COLOUR_TRAINS),
    steps: &[
        COLOUR_STEP_1,
        COLOUR_STEP_2,
        COLOUR_STEP_3,
        crate::text::FILL_FRAME,
        COMPARE_STEP,
    ],
    says: &[COLOUR_SAYS],
};

texts! { PRACTICE_COLOURS;
    // V大's name for the single-colour exercise, apart from the canvas's Colour (颜色).
    COLOUR_EXERCISE = "Colour", "单色";
    MATCH_A_COLOUR = "Match a colour", "临摹单色";
    COLOUR_TRAINS = "A colour judged by its hue, lightness and purity, and corrected by \
        eye.", "从色相、明度、纯度判断颜色，凭眼睛修正。";
    COLOUR_STEP_1 = "Squint, the hue first: the family on the ring",
        "眯眼，先定色相：在色环上找到色系";
    COLOUR_STEP_2 = "Lighter or darker", "再比明度：更亮还是更暗";
    COLOUR_STEP_3 = "Purer or greyer", "再比纯度：更纯还是更灰";
    COLOUR_SAYS = "Compare: on the wheel yours is ringed in ink, the original in red; under it ΔE, \
        how different they look (about 2 just noticeable), and its lightness, chroma and hue, \
        each a bar from the original's red line to yours. Correct the longest first, two or three \
        times. The swatch can't be picked before.",
        "叠图对比：色环上墨线圈出的是你的颜色，红圈是原图的颜色；下面的 ΔE 表示两者看起来差多少（约 2 刚能看出），再分明度、纯度、色相三行，每行的横条从原图颜色的红线画到你的颜色。先改差得最多的一项，改两三次。叠图对比之前，不能吸原图的色块。";
}

//! A cold-warm copy's words (S154, V大's 整图变调 with W5-6's rules of a shift, in our own words):
//! its name, its How card (no demonstration: how to shift a whole picture toward two tendencies,
//! the method only, D118), the names of its two swatches beside the frame and the path page's
//! card. In his terms: 冷暖变调临摹, 倾向色, 亮部, 暗部, 固有色, 大作业.

use super::{How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("cold-warm", COLD_WARM_EXERCISE, COLD_WARM)];

/// How to shift a whole picture toward a warm and a cool at once, then the order of work.
const COLD_WARM: How = How {
    heading: COLD_WARM_EXERCISE,
    trains: None,
    steps: &[],
    says: &[
        COLD_WARM_TWO,
        COLD_WARM_VALUE,
        COLD_WARM_APART,
        COLD_WARM_ORDER,
    ],
};

texts! { PRACTICE_COLD_WARM;
    COLD_WARM_EXERCISE = "Cold-warm copy", "冷暖变调临摹";
    BIG_HOMEWORK = "Big homework", "大作业";
    LIGHTS_TENDENCY = "Lights", "亮部";
    SHADOWS_TENDENCY = "Shadows", "暗部";
    COLD_WARM_TWO = "Copy the picture into the frame beside it as if its light had changed: the \
        lights lean toward the warm colour beside the frame, the shadows toward the cool one.",
        "把这张图临摹到旁边的框里，像是光变了：亮部往框边的暖色偏，暗部往冷色偏。";
    COLD_WARM_VALUE = "Move hue and chroma, not value: each area stays about as light as in the \
        picture. Greyish areas can travel far toward their tendency and gain colour; a strong \
        colour facing it moves only a little, and greys; the lightest areas barely change.",
        "动色相和纯度，别动深浅：每块颜色的明暗跟原图差不多。偏灰的颜色可以朝倾向色走远一些，也更鲜；跟倾向色相对的鲜艳颜色只挪一点，还会变灰；最亮的地方几乎不变。";
    COLD_WARM_APART = "After the shift the picture's colours must still read apart: a red cloth \
        and an orange never merge into one colour.",
        "偏色之后，原来不同的颜色还得看得出不同：红布和橙子别混成一个色。";
    COLD_WARM_ORDER = "Block in the big colours first, quickly, with Hard Paint: worked out \
        slowly, a shift turns stiff. Then transitions and detail, as in any colour copy.",
        "先用硬边颜料快快铺大色块，慢慢算出来的变调会发僵；再画过渡和细节，和色彩临摹一样。";
}

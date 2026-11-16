//! A contrast thumbnail's words (S154, V大's analysis run backwards for one's own picture, in our
//! own words, W7-8): its name, its How card (decide the focus, choose the contrast that makes it,
//! a strong, middle and weak rhythm round it, several accents of different strength and means;
//! never what Compare draws, D118) and the lines its rules say on the card. In his terms: 对比小稿,
//! 重点, 节奏, 强调; the pens are value masses' 黑, 灰, 白.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] =
    [words("contrast-thumbnail", THUMBNAIL_EXERCISE, THUMBNAIL)];

const THUMBNAIL: How = How {
    heading: THUMBNAIL_HEADING,
    trains: Some(THUMBNAIL_TRAINS),
    steps: &[
        THUMBNAIL_STEP_1,
        THUMBNAIL_STEP_2,
        THUMBNAIL_STEP_3,
        COMPARE_STEP,
    ],
    says: &[THUMBNAIL_FOCUS, THUMBNAIL_RHYTHM, THUMBNAIL_TWO],
};

texts! { PRACTICE_THUMBNAILS;
    THUMBNAIL_EXERCISE = "Contrast thumbnail", "对比小稿";
    THUMBNAIL_HEADING = "Plan a picture by contrast", "对比小稿";
    THUMBNAIL_TRAINS = "Your own picture planned by contrast: decide where the eye goes first, \
        then make it go there.",
        "用对比安排自己的画面：先定眼睛先看哪儿，再让它看过去。";
    THUMBNAIL_STEP_1 = "Fill frame with Middle", "先用灰填满方框";
    THUMBNAIL_STEP_2 = "Dark at the ring only: the focus comes first",
        "黑只放在圈里：先定重点";
    THUMBNAIL_STEP_3 = "Light against it there; elsewhere a value apart at most",
        "在那里让白挨着黑；别处最多差一级";
    THUMBNAIL_FOCUS = "Decide the focus before you paint, then the contrast that makes it: here \
        the biggest step of value, light right against dark.",
        "动笔前先定重点，再定靠什么对比造出它：这里是明度上最大的一步，亮紧挨着暗。";
    THUMBNAIL_RHYTHM = "Beyond the focus, never all alike: a few places a little strong, most of \
        the picture weak, so it has a rhythm.",
        "重点之外不能都一样：几处稍强，大部分弱，画面才有节奏。";
    THUMBNAIL_TWO = "With two rings, make the second clearly weaker than the first and still \
        stronger than the rest, and give it another means: a smaller shape, or a step of one \
        value.",
        "有两个圈时，第二个要明显弱于第一个，又强于其余地方，手段也换一种：形小一些，或者只差一级明度。";
    FOCUS_OUTSIDE = "The strongest contrast lies outside the ring", "最强的对比在圈外";
    FOCUS_RIVAL = "A place outside is as strong as the ring", "圈外有一处和圈里一样强";
    SECOND_STRONG = "The second ring is as strong as the first", "第二个圈和第一个一样强";
    SECOND_WEAK = "A place outside is as strong as the second ring",
        "圈外有一处和第二个圈一样强";
    THUMBNAIL_BARE = "Part of the frame is still bare", "方框里还有地方空着";
}

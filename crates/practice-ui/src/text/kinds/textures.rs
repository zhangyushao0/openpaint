//! Texture's words (S154j, 纹理): its name and its How cards (every shadow told; the side toward the
//! light sparse), after Drawabox's texture analysis: a texture told by the shadows its forms throw,
//! not by their outlines; fewer toward the light. In its terms: 影子, 投影, 灰面, 亮面, 留白.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 2] = [
    words("texture", TEXTURE_EXERCISE, TEXTURE),
    words("texture-sparse", TEXTURE_EXERCISE, SPARSE),
];

const TEXTURE: How = How {
    heading: TEXTURE_HEADING,
    trains: Some(TEXTURE_TRAINS),
    steps: &[
        TEXTURE_STEP_OUTLINE,
        TEXTURE_STEP_SHADOW,
        TEXTURE_STEP_EVERY,
        COMPARE_STEP,
    ],
    says: &[TEXTURE_SAYS],
};
const SPARSE: How = How {
    heading: TEXTURE_HEADING,
    trains: Some(SPARSE_TRAINS),
    steps: &[
        TEXTURE_STEP_OUTLINE,
        TEXTURE_STEP_SHADOW,
        TEXTURE_STEP_EVERY,
        TEXTURE_STEP_SPARSE,
        COMPARE_STEP,
    ],
    says: &[TEXTURE_SAYS, SPARSE_SAYS],
};

texts! { PRACTICE_TEXTURES;
    TEXTURE_EXERCISE = "Texture", "纹理";
    TEXTURE_HEADING = "Texture by its shadows", "用影子画纹理";
    TEXTURE_TRAINS = "A surface's texture told by the shadows its small forms throw, not by an \
        outline round every form.",
        "用小形体投下的影子表现表面的纹理，而不是把每个形体的轮廓都描出来。";
    SPARSE_TRAINS = "Its shadows, then fewer toward the light: the texture fades where the light \
        falls.",
        "先画影子，再在靠近光的一边减少：纹理在光照到的地方渐渐消失。";
    TEXTURE_STEP_OUTLINE = "No outline round a form", "不描形体的轮廓";
    TEXTURE_STEP_SHADOW = "With Darkest, fill the shadow it throws", "用最黑涂满它投下的影子";
    TEXTURE_STEP_EVERY = "Every shadow, nearest first", "每块影子都涂，先涂近的";
    TEXTURE_STEP_SPARSE = "Toward the light, only the biggest", "靠近光的一边只留最大的影子";
    TEXTURE_SAYS = "Look for the dark shapes the forms throw and fill each with Darkest, nearest \
        first. Leave the halftones and the lit sides paper: the texture reads from its shadows \
        alone.",
        "找出形体投下的暗形，用最黑一块块涂满，先涂近的。灰面和亮面留白：纹理只靠影子就看得出来。";
    SPARSE_SAYS = "Toward the light leave the small shadows out and keep the biggest few.",
        "靠近光的一边省掉小影子，只留最大的几块。";
}

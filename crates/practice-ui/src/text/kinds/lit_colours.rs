//! Colour under light's words (S154j, 光色, 条件色): its name, its How cards (the light and the
//! sky; the ground's bounce too), the names of the colours given beside its frame and the one
//! answer laid. In 色彩's terms: 固有色, 光源色 (阳光, 灯光), 环境色 (天光, 地面), 亮部, 暗部, 投影, 反光.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 2] = [
    words("lit-colour", LIT_EXERCISE, LIT),
    words("lit-colour-bounce", LIT_EXERCISE, BOUNCE),
];

const LIT: How = How {
    heading: LIT_HEADING,
    trains: Some(LIT_TRAINS),
    steps: &[
        LIT_STEP_GROUND,
        LIT_STEP_LIGHT,
        LIT_STEP_SHADOW,
        LIT_STEP_CAST,
        COMPARE_STEP,
    ],
    says: &[LIT_SAYS],
};
const BOUNCE: How = How {
    heading: LIT_HEADING,
    trains: Some(BOUNCE_TRAINS),
    steps: &[
        LIT_STEP_GROUND,
        LIT_STEP_LIGHT,
        LIT_STEP_SHADOW,
        LIT_STEP_CAST,
        LIT_STEP_REFLECTED,
        COMPARE_STEP,
    ],
    says: &[LIT_SAYS, BOUNCE_SAYS],
};

texts! { PRACTICE_LIT_COLOURS;
    LIT_EXERCISE = "Colour under light", "光色";
    LIT_HEADING = "A form in coloured light", "画出光下的颜色";
    LIT_TRAINS = "What a light's colour and the sky do to a local colour: the light side takes \
        the light's colour, the shadow the sky's and goes darker, and the object's own colour stays \
        in both.",
        "光的颜色和天光怎样改变固有色：亮部带上光的颜色，暗部带上天光的颜色、更暗，物体自己的颜色在亮部和暗部都还在。";
    BOUNCE_TRAINS = "The light's colour, the sky's, and the ground's: the lit ground throws its own \
        colour up into the shadow that faces it.",
        "光的颜色、天光的颜色，还有地面的颜色：受光的地面把自己的颜色反进朝向它的暗部。";
    LIT_STEP_GROUND = "Fill frame with the lit ground: the ground toward the light",
        "填满方框：受光的地面，地面的颜色往光的颜色偏";
    LIT_STEP_LIGHT = "The light side: the local colour toward the light's",
        "亮部：从固有色往光的颜色调";
    LIT_STEP_SHADOW = "The shadow side: toward the sky's colour, darker",
        "暗部：往天光的颜色调，更暗";
    LIT_STEP_CAST = "The cast shadow: the ground toward the sky's colour, darker",
        "投影：地面的颜色往天光调，更暗";
    LIT_STEP_REFLECTED = "The reflected light: the shadow toward the ground's colour",
        "反光：暗部往地面的颜色偏";
    LIT_SAYS = "Mix from the colours beside the frame: the light side from the local colour \
        toward the light, the shadow from the local colour toward the sky and darker. Keep the \
        object's own colour in both; each part's lightness matters as much as its hue.",
        "从框边的色块调起：亮部从固有色往光的颜色调，暗部从固有色往天光的颜色调，并且更暗。两边都要留住物体自己的颜色；每一块的明度和色相一样要紧。";
    BOUNCE_SAYS = "Under a coloured lamp the light side takes the lamp's colour. The part of the \
        shadow facing the lit ground leans toward the ground's colour, and stays dark.",
        "彩色灯光下，亮部带上灯的颜色。暗部朝向受光地面的那一块往地面的颜色偏，但仍然是暗的。";
    LOCAL_COLOUR_NAME = "Local colour", "固有色";
    SUNLIGHT_NAME = "Sunlight", "阳光";
    LAMPLIGHT_NAME = "Lamplight", "灯光";
    SKYLIGHT_NAME = "Skylight", "天光";
    GROUND_NAME = "Ground", "地面";
    ONE_ANSWER = "One answer", "一种答案";
}

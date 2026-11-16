//! Light on form's words (S153, 明暗): its name, the darkest pen, its How cards a level each (the
//! families alone; the halftone too; the core, the reflected light and the contact too) and its
//! reasons. In 素描's terms: 亮面, 灰面, 暗面, 明暗交界线, 反光, 投影, 三大面五大调子; the pin's
//! shadow gives where the light comes from, as a perspective drawing gives a sun.

use super::{COMPARE_STEP, How, KindWords, words};
use crate::text::{Language, Text};

pub(super) const WORDS: [KindWords; 3] = [
    words("light-families", LIGHT_EXERCISE, FAMILIES),
    words("light-halftone", LIGHT_EXERCISE, HALFTONE),
    words("light-core", LIGHT_EXERCISE, CORE),
];

const FAMILIES: How = How {
    heading: LIGHT_HEADING,
    trains: Some(FAMILIES_TRAINS),
    steps: &[
        LIGHT_STEP_FROM,
        LIGHT_STEP_FILL,
        LIGHT_STEP_DARK,
        COMPARE_STEP,
    ],
    says: &[FAMILIES_SAYS],
};
const HALFTONE: How = How {
    heading: LIGHT_HEADING,
    trains: Some(HALFTONE_TRAINS),
    steps: &[
        LIGHT_STEP_FROM,
        LIGHT_STEP_FILL,
        LIGHT_STEP_DARK,
        LIGHT_STEP_MIDDLE,
        COMPARE_STEP,
    ],
    says: &[HALFTONE_SAYS],
};
const CORE: How = How {
    heading: LIGHT_HEADING,
    trains: Some(CORE_TRAINS),
    steps: &[
        LIGHT_STEP_FROM,
        LIGHT_STEP_FILL,
        LIGHT_STEP_DARK,
        LIGHT_STEP_MIDDLE,
        LIGHT_STEP_DARKEST,
        COMPARE_STEP,
    ],
    says: &[CORE_SAYS],
};

texts! { PRACTICE_LIGHT_ON_FORM;
    LIGHT_EXERCISE = "Light on form", "明暗";
    DARKEST_PEN = "Darkest", "最黑";
    LIGHT_HEADING = "A form's light and shadow", "画出形体的明暗";
    FAMILIES_TRAINS = "From a known light, where a form's light ends and its shadow begins: the \
        faces or the side turned away go dark, and its shadow falls the way the pin's does.",
        "已知光的方向，找出形体的亮面在哪里结束、暗面从哪里开始：背着光的面或背光的一侧是暗的，投影和小杆的影子朝同一个方向。";
    HALFTONE_TRAINS = "The light and the shadow first, then the tones in the light: where the light \
        grazes a form, a halftone between the light and the terminator.",
        "先分出亮面和暗面，再分亮部的层次：光擦过形体的地方是灰面，在亮面和明暗交界线之间。";
    CORE_TRAINS = "The five tones in order: the light, the halftone, the core along the terminator, \
        the reflected light in the shadow, and the contact where the form stands.",
        "按顺序画出五大调子：亮面、灰面、明暗交界线、暗部里的反光，和形体着地处的接触阴影。";
    LIGHT_STEP_FROM = "The light: from the pin's top to its shadow's tip", "找光的方向：从杆顶到影子尖";
    LIGHT_STEP_FILL = "Fill frame with Light: the lit ground and faces", "用白填满方框：受光的地面和亮面";
    LIGHT_STEP_DARK = "With Dark, the side turned away up to the terminator, and the cast shadow",
        "用黑画背光的一侧，画到明暗交界线为止，再画投影";
    LIGHT_STEP_MIDDLE = "With Middle, the halftone where the light grazes", "用灰画光擦过的灰面";
    LIGHT_STEP_DARKEST = "With Darkest, the core along the terminator and the contact",
        "用最黑画明暗交界线和着地处的接触阴影";
    FAMILIES_SAYS = "The light comes from beyond the pin's top, the way its shadow points back. A \
        face turned from it is dark; on a round form the terminator lies where the light just \
        grazes it. The cast shadow starts where the form stands and runs the way the pin's does.",
        "光从杆顶那边照来，方向和小杆的影子相反。背着光的面是暗面；圆的形体上，光刚好擦过的地方就是明暗交界线。投影从形体着地的地方开始，和小杆的影子朝同一个方向。";
    HALFTONE_SAYS = "Lit from the side or from behind, a form keeps little light: find the \
        terminator first, then the halftone in the light beside it. A face turned further from \
        the light is never lighter than one turned to it.",
        "侧光和逆光下亮面很少：先找明暗交界线，再在它旁边的亮部里画灰面。越背着光的面，不会比向着光的面更亮。";
    CORE_SAYS = "The core along the terminator and the contact under the form are the darkest. \
        The light bounced from the lit ground stays in the shadow: leave it Dark, never as light \
        as the halftone. Only the pin is given: find the cast shadow by its light.",
        "明暗交界线和形体着地处的接触阴影最深。地面反上来的反光仍然在暗部里：留成黑，不要画得像灰面那样亮。只给了小杆：投影要按它的光自己找。";
    TERMINATOR_LIGHT_LITTLE = "The terminator lies a little toward the light.",
        "明暗交界线偏向亮面了一点。";
    TERMINATOR_LIGHT_CLEARLY = "The terminator lies clearly toward the light.",
        "明暗交界线明显偏向亮面。";
    TERMINATOR_SHADOW_LITTLE = "The terminator lies a little toward the shadow.",
        "明暗交界线偏向暗面了一点。";
    TERMINATOR_SHADOW_CLEARLY = "The terminator lies clearly toward the shadow.",
        "明暗交界线明显偏向暗面。";
    CAST_LONG_LITTLE = "The cast shadow is a little too long.", "投影长了一点。";
    CAST_LONG_CLEARLY = "The cast shadow is clearly too long.", "投影明显太长。";
    CAST_SHORT_LITTLE = "The cast shadow is a little too short.", "投影短了一点。";
    CAST_SHORT_CLEARLY = "The cast shadow is clearly too short.", "投影明显太短。";
    CAST_TURNED_LITTLE = "The cast shadow is turned a little from the pin's.",
        "投影的方向偏了一点。";
    CAST_TURNED_CLEARLY = "The cast shadow is clearly turned from the pin's.",
        "投影的方向明显偏了。";
}

/// A light-on-form reason (`light-off`: what is off and how far) as `language` reads it.
pub(crate) fn light_off(language: Language, what: &str, degree: &str) -> String {
    let clearly = degree == "clearly";
    let text: Text = match (what, clearly) {
        ("terminator-light", false) => TERMINATOR_LIGHT_LITTLE,
        ("terminator-light", true) => TERMINATOR_LIGHT_CLEARLY,
        ("terminator-shadow", false) => TERMINATOR_SHADOW_LITTLE,
        ("terminator-shadow", true) => TERMINATOR_SHADOW_CLEARLY,
        ("cast-long", false) => CAST_LONG_LITTLE,
        ("cast-long", true) => CAST_LONG_CLEARLY,
        ("cast-short", false) => CAST_SHORT_LITTLE,
        ("cast-short", true) => CAST_SHORT_CLEARLY,
        ("cast-turned", false) => CAST_TURNED_LITTLE,
        (_, _) => CAST_TURNED_CLEARLY,
    };
    text.get(language).to_owned()
}

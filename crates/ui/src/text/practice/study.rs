//! A long colour copy's words (V大's 色彩临摹): its name, its summary's line and its How card, which
//! has no demonstration and names his four brushes, the toolbox's (S104), and when to take each,
//! then his order of work with them. In his terms: 色彩临摹, 眯眼, 色调, 弱对比, 叠图对比.

use super::{How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("study", (STUDY_EXERCISE, COPIES_COUNTED), STUDY)];

/// A long colour copy's How names V大's four brushes, the toolbox's own (S104), and when to take
/// each, then his order of work with them.
const STUDY: How = How {
    heading: STUDY_EXERCISE,
    trains: None,
    steps: &[],
    says: &[STUDY_1, STUDY_BRUSHES, STUDY_2, STUDY_3, STUDY_4],
};

texts! { PRACTICE_STUDY;
    STUDY_EXERCISE = "Colour copy", "色彩临摹";
    COPIES_COUNTED = "Colour copies", "色彩临摹";
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

//! Organic arrows' words (S156l): its name and its How card, after Drawabox's arrows in our own
//! words (a flat ribbon flowing back through space: near wide, far narrow, its bends closer far
//! away, the near crossing hides the far, a half turn shows its back) and V大's 近大远小. In his
//! terms: 有机箭头, 近大远小, 丝带.

use super::{COMPARE_STEP, How, KindWords, words};

pub(super) const WORDS: [KindWords; 1] = [words("organic-arrow", ARROW_EXERCISE, ARROW)];

const ARROW: How = How {
    heading: ARROW_HEADING,
    trains: Some(ARROW_TRAINS),
    steps: &[ARROW_STEP_1, ARROW_STEP_2, ARROW_STEP_3, COMPARE_STEP],
    says: &[ARROW_SAYS],
};

texts! { PRACTICE_ARROWS;
    ARROW_EXERCISE = "Organic arrow", "有机箭头";
    ARROW_HEADING = "A ribbon flowing back", "往远处飘的丝带";
    ARROW_TRAINS = "A flat ribbon going back through space: narrower and its bends closer far \
        away, the near part hiding the far where they cross.",
        "一条扁平的丝带在空间里往远处走：越远越窄，弯得越密，交叠的地方近的挡住远的。";
    ARROW_STEP_1 = "The width marked lightly along the path with Sketch: near wide, far narrow",
        "用草稿笔沿路线轻轻标出宽度：近宽远窄";
    ARROW_STEP_2 = "Both edges with Draw in long strokes, stopping where a nearer part passes in \
        front", "用勾线笔长线画出两条边，被近处挡住的地方断开";
    ARROW_STEP_3 = "Where it turns away, the line it folds along", "转过去的地方，画出折过去的那条线";
    ARROW_SAYS = "Far away the ribbon is narrower and its bends come closer together. Where a \
        near part crosses a far one, the far edges stop at the near one's. A half turn crosses \
        the edges and shows the ribbon's back. Ghost each edge along the path, then draw it in \
        one long stroke; redraw a wrong one once with Correct.",
        "越远丝带越窄，弯也越密。近的一段从远的一段前面经过时，远处的边停在近处的边上。转半圈时两条边交叉，露出丝带的背面。每条边先沿路线在空中比划，再长长地一笔画出；画错了用修正笔重画一次。";
}

//! Objects' How card (`practice_demo.rs`, S94), after V大's method in our own words: the box
//! first, from its front corner, three edges and horizon; big to small, the biggest cut first;
//! every point found on the box's surface or inside it by a reason (diagonals, a line to the
//! vanishing point, thirds and quarters measured), never guessed; no details; beside the picture,
//! at its angle. In his terms: 方块, 点定位, 二等分, 三等分, 消失点, 切, 概括.

use super::practice_demo::{COMPARE_STEP, How};

pub(crate) const OBJECT: How = How {
    heading: super::OBJECT_IN_BOXES,
    trains: Some(OBJECT_TRAINS),
    steps: &[
        OBJECT_STEP_1,
        OBJECT_STEP_2,
        OBJECT_STEP_3,
        OBJECT_STEP_4,
        COMPARE_STEP,
    ],
    says: &[OBJECT_SAYS],
};

texts! { PRACTICE_DEMO_OBJECTS;
    OBJECT_TRAINS = "Weeks 3 and 4 · an object simplified into boxes beside its picture: the box \
        first, then big to small, every point found by a reason.",
        "第 3、4 周 · 在图旁用几何体概括单体：先找方块，再由大到小，每个点都有依据。";
    OBJECT_STEP_1 = "Complete the box with Sketch", "草稿笔补全方块";
    OBJECT_STEP_2 = "The biggest cut first: its points by halves and thirds",
        "先切最大的一块：用二等分、三等分定点";
    OBJECT_STEP_3 = "Then the smaller part; a round one's square and ellipse",
        "再找小的部分；圆的部分先找方形再画椭圆";
    OBJECT_STEP_4 = "The visible edges with Draw", "画图笔画出看得见的边";
    OBJECT_SAYS = "Find each point on the box by a reason, never in the air: diagonals, a line to \
        the vanishing point, thirds measured on the near upright. No details. Ghost each edge, \
        then draw it once. Compare: the visible edges in red; an edge of yours off its vanishing \
        point runs on dashed red past it, one off its place has its corners ringed. Construction \
        shows the box and its construction over the picture. Correct in blue and compare again.",
        "每个点都要在方块上用作图找到，不要凭空找：对角线、连向消失点的线、在近处竖边上量出的三等分。不画细节。每条边先在空中比划，再一笔画出。叠图对比：红色是看得见的边；你的边没对准消失点，用红色虚线延长越过它；位置偏了，它的两端画红圈。点“看画法”，图上会显示方块和作图。用修正笔（蓝色）改，再对比。";
}

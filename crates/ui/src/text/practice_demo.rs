//! What each kind's How card says (S92), in our own words after V大's method and in his terms
//! (草稿笔, 勾线笔, 修正笔, 叠图对比, 最鼓处, 关键点, 负形, 层级): its title, one line on what the
//! kind trains and its week in the course, its steps (one per step of its demonstration, a few
//! words each, Compare the last) and once what Compare's red, blue, ink, dashed and rings mean
//! (D108). The drawn kinds here; the colour kinds in `practice_demo_colour.rs`, perspective in
//! `practice_demo_space.rs`, stage 3 in `practice_demo_points.rs` and `practice_demo_solids.rs`.

use openpaint_practice::{Estimate, Family, Trick};

use super::{Language, Text};

/// A kind's How card's words.
#[derive(Debug, Clone, Copy)]
pub(crate) struct How {
    pub(crate) heading: Text,
    /// What it trains and its week; none for the colour copy, which has no demonstration.
    pub(crate) trains: Option<Text>,
    /// One per step of its demonstration (a value scale's own Compare only: its others are made
    /// for the row in hand, `how_steps`).
    pub(crate) steps: &'static [Text],
    /// What Compare shows and how to correct: the card's paragraphs under the steps.
    pub(crate) says: &'static [Text],
}

const LINE: How = How {
    heading: super::COPY_A_LINE,
    trains: Some(LINE_TRAINS),
    steps: &[LINE_STEP_1, LINE_STEP_2, LINE_STEP_3, COMPARE_STEP],
    says: &[LINE_SAYS],
};
const COMBINED: How = How {
    heading: super::COPY_COMBINED,
    trains: Some(GROUP_TRAINS),
    steps: &[GROUP_STEP_1, GROUP_STEP_2, GROUP_STEP_3, COMPARE_STEP],
    says: &[GROUP_SAYS],
};
const SHAPE: How = How {
    heading: super::COPY_A_SHAPE,
    trains: Some(SHAPE_TRAINS),
    steps: &[
        SHAPE_STEP_1,
        SHAPE_STEP_2,
        SHAPE_STEP_3,
        SHAPE_STEP_4,
        COMPARE_STEP,
    ],
    says: &[SHAPE_SAYS],
};

/// `family`'s How card's numbered steps as `language` reads them, one per step of its
/// demonstration: a value scale's made for the `cells` of the row in hand (S97), then the card's
/// own, Compare last.
pub(crate) fn how_steps(family: Family, cells: usize, language: Language) -> Vec<String> {
    let mut steps = if family == Family::Scale {
        super::practice_demo_colour::scale_steps(language, cells)
    } else {
        Vec::new()
    };
    let own = how(family).steps.iter();
    steps.extend(own.map(|step| step.get(language).to_owned()));
    steps
}

/// `family`'s How card.
pub(crate) const fn how(family: Family) -> How {
    use super::practice_demo_colour as c;
    use super::practice_demo_points as p;
    use super::practice_demo_space as s;
    match family {
        Family::Line => LINE,
        Family::Combined => COMBINED,
        Family::Shape => SHAPE,
        Family::Box => s::BOX,
        Family::FreeBoxes => s::FREE,
        Family::Ellipse => s::ELLIPSE,
        Family::Point => p::estimate(Estimate::Point),
        Family::Proportion => p::estimate(Estimate::Proportion),
        Family::Angle => p::estimate(Estimate::Angle),
        Family::Centre => p::trick(Trick::Centre),
        Family::Halves => p::trick(Trick::Halves),
        Family::Thirds => p::trick(Trick::Thirds),
        Family::Step => p::trick(Trick::Step),
        Family::Mirror => p::trick(Trick::Mirror),
        Family::Cylinder => super::practice_demo_solids::CYLINDER,
        Family::Cone => super::practice_demo_solids::CONE,
        Family::Sphere => super::practice_demo_solids::SPHERE,
        Family::Object => super::practice_demo_objects::OBJECT,
        Family::Division => super::practice_demo_division::DIVISION,
        Family::WholeCopy => super::practice_demo_division::WHOLE_COPY,
        Family::ValueMasses => super::practice_masses::VALUE_MASSES,
        Family::KeyShift => super::practice_key::KEY_SHIFT,
        Family::Colour => c::COLOUR,
        Family::Areas => c::AREAS,
        Family::Scale => c::SCALE,
        Family::Study => c::STUDY,
    }
}

texts! { PRACTICE_DEMO;
    AGAIN = "Again", "再看一遍";
    COMPARE_STEP = "Compare", "叠图对比";
    LINE_TRAINS = "Week 1 · a line placed by its ends and bends, then drawn in one sure stroke.",
        "第 1 周 · 先定两端和转折，再果断地一笔画出。";
    LINE_STEP_1 = "Tick its ends and where it bulges most with Sketch",
        "用草稿笔标出两端和最鼓处";
    LINE_STEP_2 = "Two passes in the air", "在空中比划两遍";
    LINE_STEP_3 = "One quick stroke with Draw", "用勾线笔一笔画出";
    LINE_SAYS = "Compare lays the original over your line in red, in place or aligned (its shape \
        alone). Don't erase: correct over it in blue and compare again, as often as you like.",
        "叠图对比把原线条用红色叠在你的线上：“原位”看位置，“对齐”只比形状。不要擦，用修正笔（蓝色）直接在上面改，再对比，改几次都行。";
    GROUP_TRAINS = "Week 1 · several lines seen as one shape: the key points first, then each \
        line.", "第 1 周 · 几根线看成一个形状：先定关键点，再画每根线。";
    GROUP_STEP_1 = "Key points with Sketch: ends, corners, meetings",
        "用草稿笔定关键点：端点、转角、交接处";
    GROUP_STEP_2 = "The longest line first, with Draw", "用勾线笔先画最长的线";
    GROUP_STEP_3 = "Each other from where it meets, at its angle", "其余的线从交接处按角度画出";
    GROUP_SAYS = "Compare, don't guess: the group lies over yours in red. In place shows where it \
        lies, Aligned its shape alone. Correct in blue and compare again.",
        "别凭感觉，要对比：整组原线条用红色叠在你的线上。“原位”看位置，“对齐”只比形状。用修正笔（蓝色）改，再对比。";
    SHAPE_TRAINS = "Weeks 1 and 2 · an object's outline as a line drawing: big shapes before \
        small turns, checked by the negative shapes.",
        "第 1、2 周 · 用线条画物体的外轮廓：先大后小，用负形检查。";
    SHAPE_STEP_1 = "Tick its top, bottom, left and right", "标出最上、最下、最左、最右的点";
    SHAPE_STEP_2 = "Block it in with long straight lines", "用长直线概括大形";
    SHAPE_STEP_3 = "Check a negative shape: the ground beside it", "检查负形：旁边的空白";
    SHAPE_STEP_4 = "The outline with Draw", "用勾线笔画出外轮廓";
    SHAPE_SAYS = "Compare lays the picture's outline over yours in red, the picture faint under \
        it: In place shows where you placed it, Aligned its shape alone. Correct in blue and \
        compare again.",
        "叠图对比把原图的外轮廓用红色叠在你的线上，下面淡淡的是原图：“原位”看你放的位置，“对齐”只比形状。用修正笔（蓝色）改，再对比。";
}

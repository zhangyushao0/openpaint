//! Week 2's perspective on a practice sheet (S84): the bar's names, the cards' words and the
//! instructions, in our own words after V大's method (near is bigger, far is smaller; edges running
//! the same way meet at one vanishing point, those parallel to the ground on the horizon; the front
//! angle wider than 90 degrees, about 120; vanishing points not too close) and Drawabox's (draw
//! through, ghost each line, an ellipse touching its frame's sides). In V大's terms: 透视盒子,
//! 视平线, 消失点, 前角, 椭圆, 补全盒子, 自由盒子, 叠图对比.

use openpaint_practice::{Reach, Roundness, Side, Tilt};

use super::{Language, Text};

texts! { PRACTICE_PERSPECTIVE;
    HORIZON = "Horizon", "视平线";
    BOX_EXERCISE = "Box", "透视盒子";
    FREE_BOXES_EXERCISE = "Free boxes", "自由盒子";
    ELLIPSE_EXERCISE = "Ellipse", "椭圆";
    COMPLETE_THE_BOX = "Complete the box", "补全盒子";
    DRAW_AN_ELLIPSE = "Draw an ellipse in its frame", "在方框里画椭圆";
    SHOWS_BOX = "In red: the exact box, its vanishing points ringed. Dashed: each of your edges \
        drawn on towards its vanishing point, blue where it heads for it, red where it misses. A \
        correction counts in place of the edge it redraws.",
        "红色是准确的盒子，消失点画了红圈。虚线是你的每条边朝它的消失点延长：指向消失点的是蓝色，偏离的是红色。修正的边代替它重画的那条边。";
    SHOWS_FREE_BOXES = "Dashed: your edges drawn on to the horizon, blue where they meet their \
        direction's point (ringed), red where one misses it. Red too: the true vertical through a \
        vertical edge that leans, and a front corner under 90°.",
        "虚线是你的边延长到视平线：交于同一方向交汇点（画圈）的是蓝色，偏离的是红色。红色还标出不竖直的竖边的真正竖直线，以及小于 90° 的前角。";
    SHOWS_ELLIPSE = "In red: the exact ellipse, touching each side at the ringed middle. Dashed \
        blue: the ellipse found in your lines.",
        "红色是准确的椭圆，与每条边相切于画圈的中点。蓝色虚线是从你的线条里找到的椭圆。";
    NO_EDGES = "No straight edge found in your lines.", "你的线条里没有找到直边。";
    NO_ELLIPSE = "No ellipse found in your lines.", "你的线条里没有找到椭圆。";
    BOXES_MEET = "Each direction meets at one point on the horizon, the verticals stay vertical \
        and every front corner is wider than 90°.",
        "每个方向的边都交于视平线上的一点，竖边竖直，前角都大于 90°。";
    ELLIPSE_FITS = "Yours fits: as round, turned the same, touching every side.",
        "你的椭圆很贴合：圆度、角度都对，四条边都相切。";
    ELLIPSE_NEXT = "Draw it again over yours in blue, then tap Compare: it reads your correction \
        alone.", "用修正笔在上面再画一次，然后点“叠图对比”：只看你修正的那一笔。";
    SIDE_TOP = "top side", "上边";
    SIDE_RIGHT = "right side", "右边";
    SIDE_BOTTOM = "bottom side", "下边";
    SIDE_LEFT = "left side", "左边";
    ROUNDER = "rounder", "更圆";
    FLATTER = "flatter", "更扁";
    TURNED_CLOCKWISE = "turned clockwise", "顺时针偏转";
    TURNED_ANTICLOCKWISE = "turned anticlockwise", "逆时针偏转";
    SHORT_OF = "falls short of the", "没碰到";
    RUNS_PAST = "runs past the", "超出了";
}

texts! { PRACTICE_PERSPECTIVE_STEPS;
    BOX_1 = "Complete the box in the empty frame: its front corner and the three edges from it are \
        given, and the horizon.",
        "在空白框里补全这个透视盒子：给出了它的前角、从前角出发的三条边，还有视平线。";
    BOX_2 = "Near is bigger, far is smaller: edges running the same way meet at one vanishing \
        point, those parallel to the ground on the horizon. Judge where the given edges would \
        meet; the points may lie off the frame.",
        "近大远小：同一方向的边交于同一个消失点，平行于地面的边交在视平线上。先判断给出的边延长后会交在哪里，消失点可能在框外。";
    BOX_3 = "Draw the other nine edges with Draw, the hidden ones too, each in one sure stroke: \
        ghost it a few times before the pen goes down. Sketch is for construction lines.",
        "用画图笔画出其余九条边，看不见的边也要画，每条边一笔画成：下笔前先空比划几次。草稿笔用来画辅助线。";
    BOX_4 = "Then tap Compare: the exact box lies over yours in red, and each of your edges is \
        drawn on towards its vanishing point. Correct in blue and compare again.",
        "然后点“叠图对比”：红色的准确盒子叠在你的盒子上，你的每条边都朝它的消失点延长。用修正笔改，再对比。";
    FREE_1 = "Draw three or four boxes freely in the empty frame, on the horizon given: each \
        turned another way, each of other proportions.",
        "在空白框里按给出的视平线自由画三四个盒子：每个转向不同，长宽高比例也不同。";
    FREE_2 = "Keep the verticals vertical. A box's edges running the same way meet at one point on \
        the horizon; a box turned another way has two points of its own.",
        "竖边保持竖直。一个盒子同一方向的边交于视平线上的同一点；转向不同的盒子有它自己的两个消失点。";
    FREE_3 = "Keep each front corner wider than a right angle, about 120°: vanishing points too \
        close distort a box. Draw the hidden edges too.",
        "每个前角都要大于直角，约 120° 为宜：消失点太近盒子会畸变。看不见的边也要画出来。";
    FREE_4 = "Then tap Compare: every straight edge is drawn on to the horizon, where each \
        direction's edges meet is ringed, and a front corner under 90° is marked.",
        "然后点“叠图对比”：每条直边都会延长到视平线，同一方向的边交汇处画圈，小于 90° 的前角会被标出。";
    ELLIPSE_1 = "Draw the ellipse that fits the frame given, touching the middle of each of its \
        sides.", "画出贴合给定方框的椭圆，与方框每条边的中点相切。";
    ELLIPSE_2 = "In perspective a side's middle lies nearer its far end: cross the diagonals for \
        the centre, and the lines through it running with the sides meet the sides at their \
        middles. Mark them lightly with Sketch.",
        "在透视中，边的中点更靠近远端：连对角线找到中心，过中心、顺着边的方向画线，与各边的交点就是中点。可用草稿笔轻轻标出。";
    ELLIPSE_3 = "Draw it with Draw from the shoulder, round two or three times in one quick, \
        smooth stroke.", "用画图笔从肩膀带动，一笔连续绕两三圈，快速流畅。";
    ELLIPSE_4 = "Then tap Compare: the exact ellipse in red, the one found in your lines dashed in \
        blue, and how they differ in words.",
        "然后点“叠图对比”：红色是准确的椭圆，蓝色虚线是从你的线条里找到的椭圆，并用文字说明差别。";
}

/// What a completed box's card says of its edges: how many were found, how many head for their
/// vanishing point (or stay vertical).
pub(crate) fn box_edges(language: Language, (found, on): (usize, usize)) -> String {
    match (language, found == on) {
        (Language::English, true) => {
            format!("Found {found} straight edges, each heading for its vanishing point.")
        }
        (Language::English, false) if found - on == 1 => format!(
            "Found {found} straight edges; {on} head for their vanishing point, 1 misses it."
        ),
        (Language::English, false) => format!(
            "Found {found} straight edges; {on} head for their vanishing point, {} miss it.",
            found - on
        ),
        (Language::Chinese, true) => format!("找到 {found} 条直边，都指向各自的消失点。"),
        (Language::Chinese, false) => format!(
            "找到 {found} 条直边：{on} 条指向它的消失点，{} 条偏离了。",
            found - on
        ),
    }
}

/// How many straight edges were found in how many free boxes.
pub(crate) fn free_found(language: Language, (edges, boxes): (usize, usize)) -> String {
    match language {
        Language::English if boxes == 1 => format!("Found {edges} straight edges in one box."),
        Language::English => format!("Found {edges} straight edges in {boxes} boxes."),
        Language::Chinese => format!("在 {boxes} 个盒子里找到 {edges} 条直边。"),
    }
}

/// Free boxes whose edges of one direction do not meet at one point.
pub(crate) fn boxes_apart(language: Language, boxes: usize) -> String {
    match language {
        Language::English if boxes == 1 => {
            "In one box the edges of a direction don't meet at one point on the horizon.".into()
        }
        Language::English => format!(
            "In {boxes} boxes the edges of a direction don't meet at one point on the horizon."
        ),
        Language::Chinese => format!("有 {boxes} 个盒子同一方向的边没有交于视平线上的一点。"),
    }
}

/// Front corners under 90 degrees.
pub(crate) fn corners_narrow(language: Language, corners: usize) -> String {
    match language {
        Language::English if corners == 1 => {
            "One front corner under 90°, marked in red: its vanishing points are too close.".into()
        }
        Language::English => format!(
            "{corners} front corners under 90°, marked in red: their vanishing points are too \
             close."
        ),
        Language::Chinese => format!("有 {corners} 个前角小于 90°（红色标出）：消失点太近了。"),
    }
}

/// Vertical edges that lean.
pub(crate) fn verticals_lean(language: Language, edges: usize) -> String {
    match language {
        Language::English if edges == 1 => "One vertical edge leans.".into(),
        Language::English => format!("{edges} vertical edges lean."),
        Language::Chinese => format!("有 {edges} 条竖边不竖直。"),
    }
}

const fn side(side: Side) -> Text {
    match side {
        Side::Top => SIDE_TOP,
        Side::Right => SIDE_RIGHT,
        Side::Bottom => SIDE_BOTTOM,
        Side::Left => SIDE_LEFT,
    }
}

/// How a drawn ellipse differs from the exact one: "Yours is flatter, turned clockwise; it falls
/// short of the left side and runs past the right side."
pub(crate) fn ellipse_differs(
    language: Language,
    (roundness, tilt): (Option<Roundness>, Option<Tilt>),
    sides: &[(Side, Reach)],
) -> String {
    let shape: Vec<&str> = [
        roundness.map(|r| match r {
            Roundness::Rounder => ROUNDER,
            Roundness::Flatter => FLATTER,
        }),
        tilt.map(|t| match t {
            Tilt::Clockwise => TURNED_CLOCKWISE,
            Tilt::Anticlockwise => TURNED_ANTICLOCKWISE,
        }),
    ]
    .into_iter()
    .flatten()
    .map(|words| words.get(language))
    .collect();
    // The sides missed each way, gathered: "falls short of the left side and the right side".
    let reached: Vec<String> = [(Reach::Short, SHORT_OF), (Reach::Past, RUNS_PAST)]
        .into_iter()
        .filter_map(|(way, how)| {
            let at: Vec<&str> = (sides.iter())
                .filter(|(_, reach)| *reach == way)
                .map(|(at, _)| side(*at).get(language))
                .collect();
            (!at.is_empty()).then(|| match language {
                Language::English => format!("{} {}", how.get(language), at.join(" and the ")),
                Language::Chinese => format!("{}{}", how.get(language), at.join("和")),
            })
        })
        .collect();
    match language {
        Language::English => {
            let mut said = String::from("Yours ");
            if !shape.is_empty() {
                said += &format!("is {}", shape.join(", "));
            }
            if let Some((last, rest)) = reached.split_last() {
                said += if shape.is_empty() { "" } else { "; it " };
                if !rest.is_empty() {
                    said += &rest.join(", ");
                    said += " and ";
                }
                said += last;
            }
            said + "."
        }
        Language::Chinese => {
            let mut parts: Vec<String> = shape.iter().map(|s| (*s).to_owned()).collect();
            parts.extend(reached);
            format!("你的椭圆{}。", parts.join("，"))
        }
    }
}

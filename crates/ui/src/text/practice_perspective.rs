//! Week 2's perspective on a practice sheet (S84): the bar's names and the cards' words (the How
//! cards are `practice_demo_space.rs`). In V大's terms: 透视盒子, 视平线, 消失点, 前角, 椭圆, 补全盒子,
//! 自由盒子.

use openpaint_practice::{Reach, Roundness, Side, Tilt};

use super::{Language, Text};

texts! { PRACTICE_PERSPECTIVE;
    HORIZON = "Horizon", "视平线";
    BOX_EXERCISE = "Box", "透视盒子";
    FREE_BOXES_EXERCISE = "Free boxes", "自由盒子";
    ELLIPSE_EXERCISE = "Ellipse", "椭圆";
    COMPLETE_THE_BOX = "Complete the box", "补全盒子";
    DRAW_AN_ELLIPSE = "Draw an ellipse in its frame", "在方框里画椭圆";
    NO_EDGES = "No straight edge found in your lines.", "你的线条里没有找到直边。";
    NO_ELLIPSE = "No ellipse found in your lines.", "你的线条里没有找到椭圆。";
    BOXES_MEET = "Every direction meets, every vertical upright, every front corner over 90°.",
        "方向都交于一点，竖边竖直，前角都大于 90°。";
    ELLIPSE_FITS = "Yours fits.", "你的椭圆很贴合。";
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

/// What a completed box's card says of its edges: how many of those found head for their
/// vanishing point (or stay vertical).
pub(crate) fn box_edges(language: Language, (found, on): (usize, usize)) -> String {
    match (language, found == on) {
        (Language::English, true) => format!("All {found} edges head for their vanishing point."),
        (Language::English, false) => {
            format!("{on} of {found} edges head for their vanishing point.")
        }
        (Language::Chinese, true) => format!("{found} 条边都指向消失点。"),
        (Language::Chinese, false) => format!("{found} 条边中 {on} 条指向消失点。"),
    }
}

/// How many free boxes were found, with how many straight edges.
pub(crate) fn free_found(language: Language, (edges, boxes): (usize, usize)) -> String {
    match language {
        Language::English if boxes == 1 => format!("1 box, {edges} edges."),
        Language::English => format!("{boxes} boxes, {edges} edges."),
        Language::Chinese => format!("{boxes} 个盒子，{edges} 条边。"),
    }
}

/// Free boxes whose edges of one direction do not meet at one point.
pub(crate) fn boxes_apart(language: Language, boxes: usize) -> String {
    match language {
        Language::English if boxes == 1 => "In 1 box a direction's edges don't meet.".into(),
        Language::English => format!("In {boxes} boxes a direction's edges don't meet."),
        Language::Chinese => format!("有 {boxes} 个盒子同一方向的边没有交于一点。"),
    }
}

/// Front corners under 90 degrees.
pub(crate) fn corners_narrow(language: Language, corners: usize) -> String {
    match language {
        Language::English if corners == 1 => "1 front corner under 90°.".into(),
        Language::English => format!("{corners} front corners under 90°."),
        Language::Chinese => format!("有 {corners} 个前角小于 90°。"),
    }
}

/// Vertical edges that lean.
pub(crate) fn verticals_lean(language: Language, edges: usize) -> String {
    match language {
        Language::English if edges == 1 => "1 vertical leans.".into(),
        Language::English => format!("{edges} verticals lean."),
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

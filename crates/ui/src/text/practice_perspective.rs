//! Week 2's perspective on a practice sheet (S84): the bar's names and the cards' words (the How
//! cards are `practice_demo_space.rs`). In V大's terms: 透视盒子, 视平线, 消失点, 前角, 椭圆, 补全盒子,
//! 自由盒子, 看画法.

use openpaint_practice::{Missed, Reach, Roundness, Side, Tilt};

use super::{Language, Text};

texts! { PRACTICE_PERSPECTIVE;
    HORIZON = "Horizon", "视平线";
    BOX_EXERCISE = "Box", "透视盒子";
    FREE_BOXES_EXERCISE = "Free boxes", "自由盒子";
    ELLIPSE_EXERCISE = "Ellipse", "椭圆";
    COMPLETE_THE_BOX = "Complete the box", "补全盒子";
    DRAW_AN_ELLIPSE = "Draw an ellipse in its frame", "在方框里画椭圆";
    CONSTRUCTION = "Construction", "看画法";
    NO_EDGES = "No straight edge found in your lines.", "你的线条里没有找到直边。";
    NO_ELLIPSE = "No ellipse found in your lines.", "你的线条里没有找到椭圆。";
    EDGES_HEAD = "Every edge heads for its vanishing point.", "每条边都对准了消失点。";
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

/// What a box's or free boxes' card says, in one short line, of what is off (S97): the edges
/// that miss their vanishing point, the verticals that lean, the front corners under 90 degrees;
/// that every edge heads for its point when none is.
pub(crate) fn off(language: Language, (missed, lean, narrow): (usize, usize, usize)) -> String {
    if missed + lean + narrow == 0 {
        return EDGES_HEAD.get(language).to_owned();
    }
    let english = |n: usize, one: &str, many: &str| match n {
        1 => format!("1 {one}"),
        n => format!("{n} {many}"),
    };
    let parts: Vec<String> = match language {
        Language::English => [
            (missed > 0).then(|| {
                english(missed, "edge misses its", "edges miss their") + " vanishing point"
            }),
            (lean > 0).then(|| english(lean, "vertical leans", "verticals lean")),
            (narrow > 0).then(|| english(narrow, "front corner", "front corners") + " under 90°"),
        ]
        .into_iter()
        .flatten()
        .collect(),
        Language::Chinese => [
            (missed > 0).then(|| format!("{missed} 条边没对准消失点")),
            (lean > 0).then(|| format!("{lean} 条竖边歪了")),
            (narrow > 0).then(|| format!("{narrow} 个前角小于 90°")),
        ]
        .into_iter()
        .flatten()
        .collect(),
    };
    match language {
        Language::English => parts.join("; ") + ".",
        Language::Chinese => parts.join("，") + "。",
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
    sides: &[Missed],
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
                .filter(|missed| missed.reach == way)
                .map(|missed| side(missed.side).get(language))
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

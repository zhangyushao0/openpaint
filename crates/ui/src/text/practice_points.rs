//! Stage 3 on a practice sheet (S90): the bar's names and the cards' words for the estimates on
//! the flat (V大's 二维抽象网格) and the points in space (点定位 and its five tricks); their How
//! cards are `practice_demo_points.rs`. In his terms: 点定位, 二等分, 三等分, 等距延伸, 对称点.

use super::{Language, Text};

texts! { PRACTICE_POINTS;
    POINTS_STAGE = "Points", "点定位";
    POINT_EXERCISE = "Point", "定点";
    PROPORTION_EXERCISE = "Proportion", "比例";
    ANGLE_EXERCISE = "Angle", "角度";
    CENTRE_EXERCISE = "Centre", "中心";
    HALVES_EXERCISE = "Halves", "二等分";
    THIRDS_EXERCISE = "Thirds", "三等分";
    STEP_EXERCISE = "Next face", "等距延伸";
    MIRROR_EXERCISE = "Mirror", "对称点";
    PLACE_A_POINT = "Place a point", "定点";
    DIVIDE_ALIKE = "Divide alike", "等比分割";
    COPY_AN_ANGLE = "Copy an angle", "复制角度";
    FIND_THE_CENTRE = "Find the face's centre", "找面的中心";
    HALVE_THE_FACE = "Halve the face", "二等分";
    THIRDS_IN_DEPTH = "Thirds in depth", "纵深三等分";
    THE_NEXT_FACE = "The next face", "等距延伸";
    MIRROR_A_POINT = "Mirror a point", "对称点";
    NO_MARK = "No mark found in your lines.", "你的线条里没有找到标记。";
    POINTS_COUNTED = "Points", "定点";
    PROPORTIONS_COUNTED = "Proportions", "比例";
    ANGLES_COUNTED = "Angles", "角度";
    CENTRES_COUNTED = "Centres", "中心";
    HALVES_COUNTED = "Halves", "二等分";
    THIRDS_COUNTED = "Thirds", "三等分";
    STEPS_COUNTED = "Next faces", "等距延伸";
    MIRRORS_COUNTED = "Mirrors", "对称点";
    OFF_ACROSS = "Off across", "横向偏差";
    OFF_DOWN = "Off down", "纵向偏差";
    OFF_ALONG = "Off along", "沿线偏差";
    TURNED_OFF = "Turned off", "角度偏差";
    OFF_ON_THE_FACE = "Off on the face", "面上偏差";
}

/// The summary's line for a family counted by its name: "Points: 2 of 2 compared".
pub(crate) fn counted(language: Language, name: Text, (checked, held): (usize, usize)) -> String {
    match language {
        Language::English => format!("{}: {checked} of {held} compared", name.get(language)),
        Language::Chinese => format!("{}：已对比 {checked}/{held}", name.get(language)),
    }
}

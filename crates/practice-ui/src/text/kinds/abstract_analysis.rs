//! Abstract analysis's words (S112): its name, its How card and its card's four steps, after V大's
//! method in our own words (W5-6): the content forgotten a while, only the shapes, the colours and
//! how they stand against each other; what draws you, by what contrast, the second and the weak,
//! how strength is spread; and the words of its choices, its views and its marking pens. In his
//! terms: 抽象分析, 明度, 虚实, 纯度, 色相, 大小, 隔离, 平均, 线性, 曲线, 带噪声, 眯眼, 强, 次, 弱.

use super::{COMPARE_STEP, How, KindWords, words};
use crate::text::{CHROMA, HUE, LIGHTNESS, Language, SIZE, Text};

pub(super) const WORDS: [KindWords; 1] = [words("abstract-analysis", ANALYSIS_EXERCISE, ANALYSIS)];

const ANALYSIS: How = How {
    heading: ANALYSE_A_PICTURE,
    trains: Some(ANALYSIS_TRAINS),
    steps: &[ANALYSIS_STEP_1, ANALYSIS_STEP_2, COMPARE_STEP],
    says: &[ANALYSIS_SAYS],
};

texts! { PRACTICE_ANALYSIS;
    ANALYSIS_EXERCISE = "Abstract analysis", "抽象分析";
    ANALYSE_A_PICTURE = "Analyse a picture", "抽象分析";
    ANALYSIS_TRAINS = "A picture seen as shapes and colours only: what draws the eye, by what \
        contrast, and how strength is spread over it.",
        "暂时忘掉画的内容，只看形状和颜色：什么吸引你，它靠什么对比，强弱怎么分布。";
    ANALYSIS_STEP_1 = "Ring what draws you most, with Strong", "用「强」圈出最吸引你的地方";
    ANALYSIS_STEP_2 = "Ring the second places with Second, hatch the weak with Weak",
        "用「次」圈次要的，用「弱」涂弱的地方";
    ANALYSIS_SAYS = "On the card, choose by what contrast the strongest place draws the eye \
        and how strength is spread, then Compare. Compare veils the picture in paper where the \
        contrast you look at is weak: what stays clear is where that contrast lives, your marks \
        over it. Squint shows the picture as a squinting eye sees it. Size, isolation and what \
        the picture shows are yours to judge.",
        "在卡片上选好最强处靠什么对比、强弱怎么分布，再叠图对比。叠图对比时，你看的那种对比弱的地方盖上一层白纱，留下来清楚的就是这种对比所在的地方，你的标记在上面。眯眼显示眯起眼睛看到的样子。大小、隔离和画的内容靠你自己判断。";
    ANALYSIS_STRONGEST = "1  What draws you most", "1  最吸引你的地方";
    ANALYSIS_STRONGEST_HOW = "Ring it with Strong", "用「强」圈出来";
    ANALYSIS_BY = "2  By what contrast", "2  它靠什么对比";
    ANALYSIS_SECOND = "3  The second and the weak", "3  次要的和弱的";
    ANALYSIS_SECOND_HOW = "Ring with Second, hatch with Weak", "用「次」圈，用「弱」涂";
    ANALYSIS_SPREAD = "4  How strength is spread", "4  强弱怎么分布";
    ANALYSIS_LOOK = "Which contrast", "看哪种对比";
    ANALYSIS_EDGES = "Edges", "虚实";
    ANALYSIS_ISOLATION = "Isolation", "隔离";
    ANALYSIS_SQUINT = "Squint", "眯眼";
    SPREAD_EVEN = "Even", "平均";
    SPREAD_LINEAR = "Linear", "线性";
    SPREAD_CURVED = "Curved", "曲线";
    SPREAD_NOISY = "Noisy", "带噪声";
    STRONG_PEN = "Strong", "强";
    SECOND_MARK_PEN = "Second", "次";
    WEAK_PEN = "Weak", "弱";
}

/// The word of a choice or a view by its key (`openpaint_practice::CONTRASTS`, `DISTRIBUTIONS`,
/// `VIEWS`); none for a key no table knows.
pub(crate) fn choice_word(key: &str) -> Option<Text> {
    Some(match key {
        "lightness" => LIGHTNESS,
        "edges" => ANALYSIS_EDGES,
        "chroma" => CHROMA,
        "hue" => HUE,
        "size" => SIZE,
        "isolation" => ANALYSIS_ISOLATION,
        "squint" => ANALYSIS_SQUINT,
        "even" => SPREAD_EVEN,
        "linear" => SPREAD_LINEAR,
        "curved" => SPREAD_CURVED,
        "noisy" => SPREAD_NOISY,
        _ => return None,
    })
}

/// What was chosen of the contrasts, in their words: "You chose: Chroma, Hue", "你选的：纯度、色相";
/// a dash for none.
pub(crate) fn analysis_chosen(language: Language, keys: &[&str]) -> String {
    let words: Vec<&str> = (keys.iter())
        .filter_map(|key| choice_word(key))
        .map(|text| text.get(language))
        .collect();
    let listed = if words.is_empty() {
        "—".to_owned()
    } else {
        match language {
            Language::English => words.join(", "),
            Language::Chinese => words.join("、"),
        }
    };
    match language {
        Language::English => format!("You chose: {listed}"),
        Language::Chinese => format!("你选的：{listed}"),
    }
}

/// The distribution chosen: "Spread: Noisy", "分布：带噪声"; a dash for none.
pub(crate) fn analysis_spread(language: Language, key: Option<&str>) -> String {
    let word = key
        .and_then(choice_word)
        .map_or("—", |text| text.get(language));
    match language {
        Language::English => format!("Spread: {word}"),
        Language::Chinese => format!("分布：{word}"),
    }
}

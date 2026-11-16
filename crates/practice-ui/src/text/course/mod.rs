//! The path (S122): the home's column, the path page, a lesson's names on the sheet and its bar,
//! an exercise's result and a lesson's end (its words made with numbers `end.rs`); the units' and
//! the skills' names and the three levels (入门, 熟练, 稳). A skill's name goes by its key
//! (`Skill::key`).

use openpaint_practice::{Needs, Offer, Outcome, Retest, Skill, SkillLevel};
use openpaint_practice_app::{LessonRow, OpenLesson};

use super::{Language, Text};

mod end;

pub(crate) use end::*;

texts! { PRACTICE_COURSE;
    PRACTICE = "Practice", "练习";
    PRACTICE_OPTIONS = "Practice options", "练习选项";
    START = "Start", "开始";
    CONTINUE = "Continue", "继续";
    REVIEW = "Review", "复习";
    ALL_UNITS = "All units", "全部单元";
    NEXT_LESSON = "Next lesson", "下一课";
    UNDER_WAY = "Lesson under way", "没上完的一课";
    ON_THE_SIDE = "Review on the side", "顺手复习";
    REDO_WAITS = "Again last time", "上次有题重来";
    UNIT_IN_THE_MAKING = "Its lessons are still being made", "这一单元的课还在准备中";
    FIRST_LESSON = "First lesson", "第一课";
    LEVEL_FIRST = "Beginning", "入门";
    LEVEL_SECOND = "Fluent", "熟练";
    LEVEL_THIRD = "Steady", "稳";
    NEEDS_PACK = "Needs a practice pack", "需要素材包";
    NEEDS_COMPOSITIONS = "Needs the composition pack", "需要构图素材包";
    NEEDS_MASSES = "Needs compositions whose values read", "需要分得出黑白灰的构图";
    NEEDS_CROPS = "Needs a pack of colour crops", "需要色彩组合素材包";
    ACCURATE = "Accurate", "准";
    ALMOST = "Almost", "差一点";
    REDO_RESULT = "Again", "重来";
    YOUR_EYE = "How close is it?", "你看这一题";
    ENOUGH = "That's all for today", "今天到这儿";
    FIRST_STEP_IN_FRONT = "Draw something in front of you in the frame: a cup, a key, your hand.",
        "在框里画一样你面前的东西：杯子、钥匙、你的手都行。";
    FIRST_STEP_COPY = "Copy the line on the left in the frame on the right.",
        "照着左边的线，在右边的框里画一条。";
    FIRST_STEP_COMPARE = "Tap Compare, see where it is off, then Next.",
        "点“叠图对比”，差在哪，看一眼，再点“下一个”。";
    RETEST = "Re-test", "复测";
    WHICH_BETTER = "Which is better", "哪张更好";
    LEFT_BETTER = "Left better", "左边好";
    ALIKE = "Same", "差不多";
    RIGHT_BETTER = "Right better", "右边好";
    EXTRA_DRILL = "Extra drill", "加练";
}

texts! { PRACTICE_UNITS;
    LINES_AND_COLOURS = "Lines and colours", "线条与颜色";
    SINGLE_SHAPES = "Single shapes", "单体图形";
    POINTS_UNIT = "Points", "点定位";
    SIMPLIFYING_UNIT = "Simplifying", "概括";
    KEY_SHIFTS_UNIT = "Key shifts", "变调";
    GAMUT_AND_LIGHT_UNIT = "Gamut and light", "色域与明暗";
    CURVES_AND_LIGHT_UNIT = "Curves and coloured light", "曲面与光色";
}

texts! { PRACTICE_SKILLS;
    LINES_SKILL = "Lines", "线条";
    COMBINED_SKILL = "Combined lines", "组合线条";
    COLOURS_SKILL = "Colours", "单色";
    SHAPES_SKILL = "Single shapes", "单体图形";
    COMBINATIONS_SKILL = "Colour combinations", "色彩组合";
    SCALES_SKILL = "Value scales", "明度推移";
    BOXES_SKILL = "Boxes", "透视盒子";
    FREE_BOXES_SKILL = "Free boxes", "自由盒子";
    ELLIPSES_SKILL = "Ellipses", "椭圆";
    ESTIMATES_SKILL = "Estimates", "平面估测";
    POINTS_SKILL = "Points in space", "空间点定位";
    SOLIDS_SKILL = "Solids", "几何体";
    OBJECTS_SKILL = "Objects", "概括单体";
    DIVISIONS_SKILL = "Divisions", "画面分割";
    MASSES_SKILL = "Value masses", "黑白灰";
    WHOLE_COPIES_SKILL = "Whole copies", "整图复制";
    KEY_SHIFTS_SKILL = "Key shifts", "变调";
    ANALYSIS_SKILL = "Abstract analysis", "抽象分析";
    ROTATION_SKILL = "Structure rotation", "结构翻转";
    GAMUT_SKILL = "Gamut masks", "色域遮罩";
    PLACEMENT_SKILL = "Placement", "摆放";
    LIGHT_SKILL = "Light on form", "明暗";
    SHADOWS_SKILL = "Cast shadows", "投影";
    CURVED_SKILL = "Curved rotation", "曲面翻转";
}

/// Each skill's name by its key (`Skill::key`).
const SKILLS: [(&str, Text); 24] = [
    ("line", LINES_SKILL),
    ("combined", COMBINED_SKILL),
    ("colour", COLOURS_SKILL),
    ("shape", SHAPES_SKILL),
    ("combination", COMBINATIONS_SKILL),
    ("scale", SCALES_SKILL),
    ("box", BOXES_SKILL),
    ("free-boxes", FREE_BOXES_SKILL),
    ("ellipse", ELLIPSES_SKILL),
    ("estimates", ESTIMATES_SKILL),
    ("points-in-space", POINTS_SKILL),
    ("solids", SOLIDS_SKILL),
    ("objects", OBJECTS_SKILL),
    ("division", DIVISIONS_SKILL),
    ("value-masses", MASSES_SKILL),
    ("whole-copy", WHOLE_COPIES_SKILL),
    ("key-shift", KEY_SHIFTS_SKILL),
    ("abstract-analysis", ANALYSIS_SKILL),
    ("structure-rotation", ROTATION_SKILL),
    ("gamut-mask", GAMUT_SKILL),
    ("placement", PLACEMENT_SKILL),
    ("light-on-form", LIGHT_SKILL),
    ("cast-shadows", SHADOWS_SKILL),
    ("curved-rotation", CURVED_SKILL),
];

/// A skill's name: "Lines", "Points in space".
pub(crate) fn skill_name(skill: Skill) -> Text {
    let key = skill.key();
    (SKILLS.iter())
        .find(|(held, _)| *held == key)
        .map_or(LINES_SKILL, |(_, name)| *name)
}

/// A level's word: 入门, 熟练, 稳.
pub(crate) const fn level_word(level: SkillLevel) -> Text {
    match level {
        SkillLevel::First => LEVEL_FIRST,
        SkillLevel::Second => LEVEL_SECOND,
        SkillLevel::Third => LEVEL_THIRD,
    }
}

/// A result's word: 准, 差一点, 重来.
pub(crate) const fn outcome(outcome: Outcome) -> Text {
    match outcome {
        Outcome::Clean => ACCURATE,
        Outcome::Near => ALMOST,
        Outcome::Redo => REDO_RESULT,
    }
}

/// What a skill greyed on the path lacks.
pub(crate) const fn needs(needs: Needs) -> Text {
    match needs {
        Needs::Compositions => NEEDS_COMPOSITIONS,
        Needs::Masses => NEEDS_MASSES,
        Needs::Crops => NEEDS_CROPS,
        Needs::Pictures | Needs::Nothing => NEEDS_PACK,
    }
}

/// A unit's name, while the app holds its kinds.
pub(crate) const fn unit_name(unit: u8) -> Option<Text> {
    match unit {
        1 => Some(LINES_AND_COLOURS),
        2 => Some(SINGLE_SHAPES),
        3 => Some(POINTS_UNIT),
        4 => Some(SIMPLIFYING_UNIT),
        5 => Some(KEY_SHIFTS_UNIT),
        6 => Some(GAMUT_AND_LIGHT_UNIT),
        7 => Some(CURVES_AND_LIGHT_UNIT),
        _ => None,
    }
}

/// A unit by its number, as its ring on the path is named: "Unit 8", "第 8 单元".
pub(crate) fn unit_number(language: Language, unit: u8) -> String {
    match language {
        Language::English => format!("Unit {unit}"),
        Language::Chinese => format!("第 {unit} 单元"),
    }
}

/// A unit's title: "Unit 1 · Lines and colours", "第 8 单元".
pub(crate) fn unit_title(language: Language, unit: u8) -> String {
    let number = unit_number(language, unit);
    match unit_name(unit) {
        Some(name) => format!("{number} · {}", name.get(language)),
        None => number,
    }
}

/// How many exercises and about how long: "5 exercises · about 4 minutes", "5 题 · 约 4 分钟".
pub(crate) fn exercises_line(language: Language, (exercises, minutes): (u8, u8)) -> String {
    match language {
        Language::English if exercises == 1 => format!("1 exercise · about {minutes} min"),
        Language::English => format!("{exercises} exercises · about {minutes} min"),
        Language::Chinese => format!("{exercises} 题 · 约 {minutes} 分钟"),
    }
}

/// An offer's line under its skill: its level, its exercises and minutes.
pub(crate) fn offer_line(language: Language, offer: &Offer) -> String {
    let counts = exercises_line(language, (offer.exercises, offer.minutes));
    format!("{} · {counts}", level_word(offer.level).get(language))
}

/// How many exercises: "5 exercises", "5 题".
pub(crate) fn exercise_count(language: Language, count: u8) -> String {
    match language {
        Language::English if count == 1 => "1 exercise".to_owned(),
        Language::English => format!("{count} exercises"),
        Language::Chinese => format!("{count} 题"),
    }
}

/// The review's row:"Lines · 2 exercises", "线条 · 2 题".
pub(crate) fn review_line(language: Language, offer: &Offer) -> String {
    let name = skill_name(offer.skill).get(language);
    format!("{name} · {}", exercise_count(language, offer.exercises))
}

/// A unit's re-test (S147): "Re-test · Unit 7", "复测 · 第 7 单元".
pub(crate) fn retest_name(language: Language, unit: u8) -> String {
    format!("{} · {}", RETEST.get(language), unit_number(language, unit))
}

/// A re-test's line: its skills, its exercises and minutes: "Boxes, Ellipses · 2 exercises · about
/// 4 min", "透视盒子、椭圆 · 2 题 · 约 4 分钟".
pub(crate) fn retest_line(language: Language, retest: &Retest) -> String {
    let names: Vec<&str> = (retest.skills.iter())
        .map(|skill| skill_name(*skill).get(language))
        .collect();
    let joined = names.join(if language == Language::Chinese {
        "、"
    } else {
        ", "
    });
    let count = u8::try_from(retest.skills.len()).unwrap_or(u8::MAX);
    format!(
        "{joined} · {}",
        exercises_line(language, (count, retest.minutes))
    )
}

/// The lesson in hand as the bar names it: "Lines 3/5", "First lesson 2/3".
pub(crate) fn in_hand(language: Language, name: Text, (place, of): (usize, usize)) -> String {
    format!("{} {place}/{of}", name.get(language))
}

/// A lesson left under way as the home names it: "Lines 3/6", "First lesson 2/3", "Re-test
/// 2/5".
pub(crate) fn open_lesson(language: Language, open: &OpenLesson) -> String {
    let name = match (open.row.first, open.row.skill) {
        (true, _) => FIRST_LESSON,
        (false, Some(skill)) => skill_name(skill),
        (false, None) => RETEST,
    };
    in_hand(language, name, open.place)
}

/// A compared exercise's heading: "Lines 3".
pub(crate) fn exercise(language: Language, name: Text, place: usize) -> String {
    format!("{} {place}", name.get(language))
}

/// A lesson done, as the bar says it: "Lines · lesson done", "线条 · 这一课画完了".
pub(crate) fn lesson_done(language: Language, name: Text) -> String {
    match language {
        Language::English => format!("{} · lesson done", name.get(language)),
        Language::Chinese => format!("{} · 这一课画完了", name.get(language)),
    }
}

/// A lesson by its number: "Lesson 9", "第 9 课".
pub(crate) fn lesson_number(language: Language, number: u32) -> String {
    match language {
        Language::English => format!("Lesson {number}"),
        Language::Chinese => format!("第 {number} 课"),
    }
}

/// A lesson's row on the sheet and its sheet in the book: "Lesson 3 · Lines", "第 1 课 · 第一课".
pub(crate) fn row_label(language: Language, row: &LessonRow) -> String {
    let number = lesson_number(language, row.number);
    match lesson_kind(language, row) {
        Some(what) => format!("{number} · {what}"),
        None => number,
    }
}

/// What a lesson practised, as its label names it after its number: "Lines", "第一课", a
/// re-test's unit; none for a lesson of nothing named.
pub(crate) fn lesson_kind(language: Language, row: &LessonRow) -> Option<String> {
    if let Some((unit, _)) = row.retest {
        return Some(retest_name(language, unit));
    }
    Some(match (row.first, row.skill) {
        (true, _) => FIRST_LESSON.get(language).to_owned(),
        (false, Some(skill)) => skill_name(skill).get(language).to_owned(),
        (false, None) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_skill_has_its_name_and_the_words_read_in_both_languages() {
        for skill in Skill::all() {
            let key = skill.key();
            assert!(SKILLS.iter().any(|(held, _)| *held == key), "{key}");
        }
        assert_eq!(Skill::all().count(), SKILLS.len());
        let line = Skill::of_key("line").expect("lines");
        let offer = Offer {
            skill: line,
            level: SkillLevel::Second,
            exercises: 5,
            minutes: 4,
            redo: false,
        };
        assert_eq!(
            offer_line(Language::Chinese, &offer),
            "熟练 · 5 题 · 约 4 分钟"
        );
        assert_eq!(unit_title(Language::Chinese, 1), "第 1 单元 · 线条与颜色");
        assert_eq!(unit_title(Language::Chinese, 6), "第 6 单元 · 色域与明暗");
        assert_eq!(
            unit_title(Language::English, 7),
            "Unit 7 · Curves and coloured light"
        );
        assert_eq!(unit_title(Language::English, 8), "Unit 8");
        let row = LessonRow {
            slot: 0,
            number: 3,
            skill: Some(line),
            first: false,
            retest: None,
        };
        assert_eq!(row_label(Language::Chinese, &row), "第 3 课 · 线条");
        let retest = LessonRow {
            retest: Some((3, true)),
            skill: None,
            ..row
        };
        assert_eq!(
            row_label(Language::Chinese, &retest),
            "第 3 课 · 复测 · 第 3 单元"
        );
        assert_eq!(
            in_hand(Language::English, FIRST_LESSON, (2, 3)),
            "First lesson 2/3"
        );
    }

    #[test]
    fn the_lesson_left_under_way_is_named_by_its_state_true_on_any_day() {
        // The home showed yesterday's unfinished lesson as today's (1.23.0, the tablet).
        let english = UNDER_WAY.get(Language::English).to_lowercase();
        assert!(!english.contains("today"), "{english}");
        assert!(!UNDER_WAY.get(Language::Chinese).contains("今天"));
    }
}

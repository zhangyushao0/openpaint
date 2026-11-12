//! The path (S122): the home's column, the path page, a lesson's names on the sheet and its bar,
//! an exercise's result and a lesson's end; the units' and the skills' names and the three levels
//! (入门, 熟练, 稳). A skill's name goes by its key (`Skill::key`).

use openpaint_practice::{Needs, Offer, Outcome, Skill, SkillLevel};
use openpaint_practice_app::{LessonRow, OpenLesson};

use super::{Language, Text};

texts! { PRACTICE_COURSE;
    PRACTICE = "Practice", "练习";
    PRACTICE_OPTIONS = "Practice options", "练习选项";
    START = "Start", "开始";
    CONTINUE = "Continue", "继续";
    REVIEW = "Review", "复习";
    ALL_UNITS = "All units", "全部单元";
    FULL_LESSON = "Full lesson", "完整一课";
    NEXT_LESSON = "Next lesson", "下一课";
    UNDER_WAY = "Today's lesson, under way", "今天这一课还没上完";
    ON_THE_SIDE = "Review on the side", "顺手复习";
    REDO_WAITS = "Again last time", "上次有题重来";
    LONGER = "To practise longer", "想坐下来多练";
    UNIT_IN_THE_MAKING = "Its lessons are still being made", "这一单元的课还在准备中";
    FULL_LINE = "A warm-up, then each skill of this unit", "热身，再把这一单元的各项练一遍";
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
    EYE_NOTE = "No score here: your eye judges. Again brings it back in a later lesson.",
        "这题没有标准分，自己判断，练的就是这双眼睛。点了“重来”，过几课还会再出现。";
    ENOUGH = "That's all for today", "今天到这儿";
    FIRST_STEP_SCRIBBLE = "Scribble in the frame: light, then hard.", "在框里随便画几笔，轻一点、重一点都试试。";
    FIRST_STEP_COPY = "Copy the line on the left in the frame on the right.",
        "照着左边的线，在右边的框里画一条。";
    FIRST_STEP_COMPARE = "Tap Compare: the original lies over your line. See where it is off, \
        then Next.", "点“叠图对比”，原图会叠在你的线上；差在哪，看一眼，再点“下一个”。";
    NOTHING_SCORED = "Nothing is scored here. A line off stays: after Compare, correct it in blue.",
        "这里不打分。画歪了不用擦，对比之后用修正笔在上面改。";
}

texts! { PRACTICE_UNITS;
    LINES_AND_COLOURS = "Lines and colours", "线条与颜色";
    SINGLE_SHAPES = "Single shapes", "单体图形";
    POINTS_UNIT = "Points", "点定位";
    SIMPLIFYING_UNIT = "Simplifying", "概括";
    KEY_SHIFTS_UNIT = "Key shifts", "变调";
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
}

/// Each skill's name by its key (`Skill::key`).
const SKILLS: [(&str, Text); 17] = [
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
        _ => None,
    }
}

/// A unit by its number, as its ring on the path is named: "Unit 6", "第 6 单元".
pub(crate) fn unit_number(language: Language, unit: u8) -> String {
    match language {
        Language::English => format!("Unit {unit}"),
        Language::Chinese => format!("第 {unit} 单元"),
    }
}

/// A unit's title: "Unit 1 · Lines and colours", "第 6 单元".
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

/// About how long a full lesson takes: "About 28 minutes", "约 28 分钟".
pub(crate) fn about_minutes(language: Language, minutes: u8) -> String {
    match language {
        Language::English => format!("About {minutes} minutes"),
        Language::Chinese => format!("约 {minutes} 分钟"),
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

/// A lesson's end, how many exercises and how long they took: "5 exercises · 3 min 12 s",
/// "5 题 · 3 分 12 秒"; under a minute only its seconds.
pub(crate) fn lesson_took(language: Language, (count, seconds): (u8, u32)) -> String {
    let (minutes, seconds) = (seconds / 60, seconds % 60);
    let took = match language {
        Language::English if minutes == 0 => format!("{seconds} s"),
        Language::English => format!("{minutes} min {seconds} s"),
        Language::Chinese if minutes == 0 => format!("{seconds} 秒"),
        Language::Chinese => format!("{minutes} 分 {seconds} 秒"),
    };
    format!("{} · {took}", exercise_count(language, count))
}

/// The review's row:"Lines · 2 exercises", "线条 · 2 题".
pub(crate) fn review_line(language: Language, offer: &Offer) -> String {
    let name = skill_name(offer.skill).get(language);
    format!("{name} · {}", exercise_count(language, offer.exercises))
}

/// What One more starts, under it: "Arcs · 5 exercises · about 4 min, on the next row of this
/// sheet".
pub(crate) fn one_more_line(language: Language, offer: &Offer) -> String {
    let name = skill_name(offer.skill).get(language);
    let counts = exercises_line(language, (offer.exercises, offer.minutes));
    match language {
        Language::English => format!("{name} · {counts}, on the next row of this sheet"),
        Language::Chinese => format!("{name} · {counts}，接着画在这张纸的下一行"),
    }
}

/// The lesson in hand as the bar names it: "Lines 3/5", "First lesson 2/3", "Warm-up 1/3".
pub(crate) fn in_hand(language: Language, name: Text, (place, of): (usize, usize)) -> String {
    format!("{} {place}/{of}", name.get(language))
}

/// Today's lesson left under way as the home names it: "Lines 3/6", "First lesson 2/3", "Full
/// lesson 5/15".
pub(crate) fn open_lesson(language: Language, open: &OpenLesson) -> String {
    let name = match (open.row.first, open.row.skill) {
        (true, _) => FIRST_LESSON,
        (false, Some(skill)) => skill_name(skill),
        (false, None) => FULL_LESSON,
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

/// A level reached at a lesson's end: "Up to “Fluent”", "到“熟练”了".
pub(crate) fn level_reached(language: Language, reached: SkillLevel) -> String {
    let word = level_word(reached).get(language);
    match language {
        Language::English => format!("Up to “{word}”"),
        Language::Chinese => format!("到“{word}”了"),
    }
}

/// A lesson by its number: "Lesson 9", "第 9 课".
pub(crate) fn lesson_number(language: Language, number: u32) -> String {
    match language {
        Language::English => format!("Lesson {number}"),
        Language::Chinese => format!("第 {number} 课"),
    }
}

/// A lesson's row on the sheet and its sheet in the book: "Lesson 3 · Lines", "第 1 课 · 第一课",
/// "Lesson 7 · Full lesson, unit 2".
pub(crate) fn row_label(language: Language, row: &LessonRow) -> String {
    let number = lesson_number(language, row.number);
    let what = match (row.first, row.skill, row.unit) {
        (true, _, _) => FIRST_LESSON.get(language).to_owned(),
        (false, Some(skill), _) => skill_name(skill).get(language).to_owned(),
        (false, None, Some(unit)) => match language {
            Language::English => format!("Full lesson, unit {unit}"),
            Language::Chinese => format!("第 {unit} 单元完整一课"),
        },
        (false, None, None) => return number,
    };
    format!("{number} · {what}")
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
        assert_eq!(
            one_more_line(Language::English, &offer),
            "Lines · 5 exercises · about 4 min, on the next row of this sheet"
        );
        assert_eq!(unit_title(Language::Chinese, 1), "第 1 单元 · 线条与颜色");
        assert_eq!(unit_title(Language::English, 7), "Unit 7");
        let row = LessonRow {
            slot: 0,
            number: 3,
            skill: Some(line),
            unit: None,
            first: false,
        };
        assert_eq!(row_label(Language::Chinese, &row), "第 3 课 · 线条");
        assert_eq!(
            in_hand(Language::English, FIRST_LESSON, (2, 3)),
            "First lesson 2/3"
        );
    }

    #[test]
    fn a_lesson_s_end_says_its_exercises_and_time_and_a_full_lesson_its_minutes() {
        assert_eq!(
            lesson_took(Language::English, (5, 192)),
            "5 exercises · 3 min 12 s"
        );
        assert_eq!(lesson_took(Language::Chinese, (3, 45)), "3 题 · 45 秒");
        assert_eq!(about_minutes(Language::English, 28), "About 28 minutes");
    }
}

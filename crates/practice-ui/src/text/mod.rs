//! Practice's words, in English and Chinese side by side (D100), as the ui's tables hold the
//! painting's (`openpaint_ui::text`, read through here too: its `Text`, `Words`, the pass's
//! language and the words both share). The tables are split by where the words show (`course.rs`
//! the path, a lesson's names, results and end; `sheet.rs`, `colour.rs`, `options.rs`,
//! `perspective.rs`, `points.rs`, `division.rs`, `book.rs`,
//! `names.rs`; each kind's name and How card in a file of its own under `kinds/`, S109b; a card's
//! `Say` keys in `says.rs`, a reason's words in `reasons.rs`), words made with numbers are
//! functions beside them. Screens write no
//! words of their own (`cargo repo-check` refuses a capitalised literal under
//! `crates/practice-ui/src` outside this module). Written Chinese-first as the ui's are: V大's
//! words (原图, 明度, 纯度, 色块, 勾线笔, 素材包), no term the ui's `RETIRED` lists.
//!
//! The path's controls (S122), their English fixed with the contract because scenarios tap them
//! by it. The home's two entrances: Practice, Studio (画室; the ui's word). The home's column and
//! the path page: Start (开始), Continue (继续, a lesson under way), Review (复习), All units
//! (全部单元), Practice book (练习簿); the home's unit tiles by their skills' names; Full lesson
//! (完整一课) only names an old record's full lesson (S124 took it off the path). On the sheet the
//! bar is the skill and its place ("skill 3/5", "First lesson 2/3"), How, Compare and Next as
//! they are; a How's replay Watch again (再看一遍). An exercise's result: Accurate (准), Almost
//! (差一点), Again (重来). A lesson's end: One more (再来一课), That's all for today (今天到这儿).
//! A skill's words go by `Skill::key`, a kind's by its `Facts::key`.
//!
//! The Pomodoro's controls (S124), their English fixed with its contract: the home's row Pomodoro
//! (番茄钟) with 15, 25 and 40 (the lengths, "min" / 分钟 after them as a label) and Start timer
//! (开始计时), Continue (继续) while a block is under way; on the bar the ring before the
//! lesson's place named by its minutes left ("18 min", "18 分"), its menu Pause (暂停) or Continue
//! and Stop timer (结束番茄钟); at the time Time (到点, no control) and Next reading End (结束); the
//! block's end Another pomodoro (再来一个番茄钟) and That's all for today; the practice options'
//! switch Break chime (休息结束提示音). The seal reads 番茄 and a short English word.
//!
//! No sentence on a screen states a rule (the author, 2026-11-13: "a screen never explains how
//! it works"). A screen holds names, states, numbers and controls: the test for any word added is
//! whether it names something or says what is, never how the app works or what will happen. So
//! S124 took off the eye-judged result's note, One more's "on the next row of this sheet", the
//! first lesson's "Nothing is scored here…", the practice options' notes on the packs, one's own
//! pictures and the colour copies, the picker's and adding's hints, what Compare does in the
//! outline lines and the notices (a test names each by its old key: `REMOVED`). S146 took off what
//! every How said Compare and Construction draw, their tolerances, who judges, and the locked
//! original's notice: a How says how to practise only, and a test refuses the words only a
//! sentence on the app's workings uses (`MECHANICS`).

pub(crate) use openpaint_ui::text::*;

mod book;
mod colour;
mod course;
mod division;
mod kinds;
mod names;
mod options;
mod perspective;
mod points;
mod pomodoro;
mod reasons;
mod says;
mod sheet;

pub(crate) use book::*;
pub(crate) use colour::*;
pub(crate) use course::*;
pub(crate) use division::*;
pub(crate) use kinds::*;
pub use names::NAMES;
pub(crate) use names::pack_title;
pub(crate) use options::*;
pub(crate) use perspective::*;
pub(crate) use points::*;
pub(crate) use pomodoro::*;
pub(crate) use reasons::reason;
pub(crate) use says::said;
pub(crate) use sheet::*;

#[cfg(test)]
mod tests {
    use openpaint_practice::{CELLS, Seal};

    use super::*;

    /// Every practice table's texts.
    fn every_text() -> Vec<Text> {
        [
            PRACTICE_TABLE,
            PRACTICE_SHEET_NOTICES,
            PRACTICE_BOOK_TABLE,
            PRACTICE_BOOK_WEEKDAYS,
            PRACTICE_COLOUR,
            PRACTICE_COURSE,
            PRACTICE_UNITS,
            PRACTICE_SKILLS,
            PRACTICE_DIVISION,
            PRACTICE_OPTIONS_TABLE,
            PRACTICE_PACKS_TABLE,
            PRACTICE_PERSPECTIVE,
            PRACTICE_POINTS,
            PRACTICE_POMODORO,
            PRACTICE_NOTICES,
            names::PRACTICE_NAMES,
        ]
        .into_iter()
        .chain(kinds::tables())
        .collect::<Vec<_>>()
        .concat()
    }

    #[test]
    fn every_text_reads_in_both_languages_and_the_chinese_in_chinese() {
        let han = |c: char| ('\u{4e00}'..='\u{9fff}').contains(&c);
        for text in every_text() {
            let (english, chinese) = (text.id(), text.get(Language::Chinese));
            assert!(!english.is_empty() && !chinese.is_empty(), "{english}");
            assert!(chinese.chars().any(han), "{english}: {chinese}");
        }
    }

    /// Every word a practice screen shows in `language`: the tables' texts and the sentences made
    /// with numbers.
    fn shown(language: Language) -> Vec<String> {
        let line = openpaint_practice::Skill::of_key("line").expect("lines");
        let offer = openpaint_practice::Offer {
            skill: line,
            level: openpaint_practice::SkillLevel::Second,
            exercises: 5,
            minutes: 4,
            redo: false,
        };
        let mut shown: Vec<String> = (every_text().into_iter())
            .map(|text| text.get(language).to_owned())
            .collect();
        for kind in kinds::every_kind() {
            shown.extend(how_steps(kind, CELLS, language));
        }
        shown.push(one_more_line(language, &offer));
        let retest = openpaint_practice::Retest {
            unit: 3,
            end: true,
            skills: vec![line],
            minutes: 6,
        };
        shown.extend([
            retest_name(language, 3),
            retest_line(language, &retest),
            drill_line(language, &offer),
            copy_date(language, 20_769),
        ]);
        shown.extend([(0, 0), (2, 1)].map(|uses| picture_goes(language, uses)));
        shown.extend((1..=8).map(|unit| unit_title(language, unit)));
        let seals = [Seal::Unit(2), Seal::TenLessons];
        shown.extend(seals.map(|seal| seal_lines(language, seal).concat()));
        shown.extend([
            cells_uneven(language, 2),
            lightness_off(language, 1, true),
            not_toward(language, 2),
            merged(language, 3, 1),
        ]);
        shown.extend(pomodoro::made(language));
        shown.extend(reasons::made(language));
        shown
    }

    #[test]
    fn the_chinese_holds_no_retired_term() {
        let zh = Language::Chinese;
        // A pen's or brush's name then 画 (draws), and 一笔画 (one stroke), are no 笔画.
        let pens = [SKETCH, DRAW, CORRECT, BIG, SMALL, SOFT].into_iter();
        let drawing: Vec<String> = (pens.chain(BUILT_IN))
            .map(|pen| format!("{}画", pen.get(zh)))
            .chain(["一笔画".to_owned()])
            .collect();
        for chinese in shown(zh) {
            let read = (drawing.iter()).fold(chinese.clone(), |read, verb| read.replace(verb, ""));
            for (term, instead) in RETIRED {
                assert!(!read.contains(term), "{chinese}: {term}, now {instead}");
            }
        }
    }

    /// The sentences S124 took off the screens for stating a rule, by the key each had, with
    /// words of each language that only they held.
    const REMOVED: [(&str, [&str; 2]); 12] = [
        ("EYE_NOTE", ["your eye judges", "练的就是这双眼睛"]),
        ("NOTHING_SCORED", ["Nothing is scored", "这里不打分"]),
        ("one_more_line", ["on the next row", "这张纸的下一行"]),
        ("PACKS_NOTE", ["use them once", "会用到它们"]),
        ("MINE_NOTE", ["use one once", "每课会用到其中一张"]),
        ("COPIES_NOTE", ["it is no lesson", "不算作一课"]),
        ("CHOOSE_HINT", ["to begin a long copy", "开始一幅色彩临摹"]),
        (
            "TIER_FIRST",
            ["Choose, then the picture", "先选难度，再选图片"],
        ),
        ("WITH_OUTLINE", ["outline: Compare lays", "叠图对比时会把"]),
        (
            "PICK_AFTER_COMPARE",
            ["can be picked after Compare", "之后才能吸原图"],
        ),
        ("PACK_IMPORTED", ["come up in", "会用到里面的图片"]),
        ("PICTURE_REFUSED", ["pictures are taken", "只支持 PNG"]),
    ];

    #[test]
    fn no_sentence_s124_removed_comes_back() {
        for language in [Language::English, Language::Chinese] {
            for text in shown(language) {
                for (key, words) in REMOVED {
                    let gone = words[usize::from(language == Language::Chinese)];
                    assert!(!text.contains(gone), "{key} is back: {text}");
                }
            }
        }
    }

    /// Words that only a sentence on the app's workings uses (D118, S146), each with what it
    /// tells: what Compare or Construction draws and what its marks mean (legend), a tolerance
    /// (threshold), a score (scoring), who judges (judging), what the app does later (later), the
    /// app itself (app). A How says how to practise; the method's own words trip none of these.
    const MECHANICS: [(&str, &str); 41] = [
        ("叠图对比：", "legend"),
        ("叠图对比把", "legend"),
        ("叠图对比时", "legend"),
        ("点“看画法”", "legend"),
        ("红圈", "legend"),
        ("红色是", "legend"),
        ("红线", "legend"),
        ("虚线", "legend"),
        ("滑块", "legend"),
        ("白纱", "legend"),
        ("Compare:", "legend"),
        ("Compare lays", "legend"),
        ("Compare veils", "legend"),
        ("Compare marks", "legend"),
        ("Construction shows", "legend"),
        ("Construction runs", "legend"),
        ("Construction ticks", "legend"),
        ("is ringed", "legend"),
        ("ringed when", "legend"),
        ("outlined", "legend"),
        ("dashed", "legend"),
        ("slider", "legend"),
        ("笔宽", "threshold"),
        ("pen's width", "threshold"),
        ("刚能看出", "threshold"),
        ("just noticeable", "threshold"),
        ("ΔE", "threshold"),
        ("打分", "scoring"),
        ("判分", "scoring"),
        ("score", "scoring"),
        ("自己判断", "judging"),
        ("靠你", "judging"),
        ("用眼睛判断", "judging"),
        ("to judge", "judging"),
        ("judge its", "judging"),
        ("judges", "judging"),
        ("再出现", "later"),
        ("come back", "later"),
        ("comes back", "later"),
        ("程序", "app"),
        ("the app", "app"),
    ];

    #[test]
    fn no_screen_explains_how_the_app_works() {
        for language in [Language::English, Language::Chinese] {
            for text in shown(language) {
                for (fragment, tells) in MECHANICS {
                    assert!(!text.contains(fragment), "{tells}: “{fragment}” in {text}");
                }
            }
        }
    }
}

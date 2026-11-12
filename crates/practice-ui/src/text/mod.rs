//! Practice's words, in English and Chinese side by side (D100), as the ui's tables hold the
//! painting's (`openpaint_ui::text`, read through here too: its `Text`, `Words`, the pass's
//! language and the words both share). The tables are split by where the words show (`course.rs`
//! the path, a lesson's names, results and end; `sheet.rs`, `colour.rs`, `options.rs`,
//! `perspective.rs`, `points.rs`, `solids.rs`, `objects.rs`, `division.rs`, `book.rs`,
//! `names.rs`; each kind's name and How card in a file of its own under `kinds/`, S109b; a card's
//! `Say` keys in `says.rs`), words made with numbers are functions beside them. Screens write no
//! words of their own (`cargo repo-check` refuses a capitalised literal under
//! `crates/practice-ui/src` outside this module). Written Chinese-first as the ui's are: V大's
//! words (原图, 明度, 纯度, 色块, 勾线笔, 素材包), no term the ui's `RETIRED` lists.
//!
//! The path's controls (S122), their English fixed with the contract because scenarios tap them
//! by it. The home's two entrances: Practice, Studio (画室; the ui's word). The home's column and
//! the path page: Start (开始), Continue (继续, a lesson under way), Review (复习), All units
//! (全部单元), Practice book (练习簿), Full lesson (完整一课); the home's unit tiles by their
//! skills' names. On the sheet the bar is the skill and its place ("skill 3/5", "First lesson
//! 2/3"), How, Compare and Next as they are; a How's replay Watch again (再看一遍). An exercise's
//! result: Accurate (准), Almost (差一点), Again (重来). A lesson's end: One more (再来一课),
//! That's all for today (今天到这儿). A skill's words go by `Skill::key`, a kind's by its
//! `Facts::key`.

pub(crate) use openpaint_ui::text::*;

mod book;
mod colour;
mod course;
mod division;
mod kinds;
mod names;
mod objects;
mod options;
mod perspective;
mod points;
mod says;
mod sheet;
mod solids;

pub(crate) use book::*;
pub(crate) use colour::*;
pub(crate) use course::*;
pub(crate) use division::*;
pub(crate) use kinds::*;
pub use names::NAMES;
pub(crate) use names::pack_title;
pub(crate) use objects::*;
pub(crate) use options::*;
pub(crate) use perspective::*;
pub(crate) use points::*;
pub(crate) use says::said;
pub(crate) use sheet::*;
pub(crate) use solids::*;

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
            PRACTICE_OBJECTS,
            PRACTICE_OPTIONS_TABLE,
            PRACTICE_PACKS_TABLE,
            PRACTICE_PERSPECTIVE,
            PRACTICE_POINTS,
            PRACTICE_SOLIDS,
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

    #[test]
    fn the_chinese_holds_no_retired_term() {
        let zh = Language::Chinese;
        // A pen's or brush's name then 画 (draws), and 一笔画 (one stroke), are no 笔画.
        let pens = [SKETCH, DRAW, CORRECT, BIG, SMALL, SOFT].into_iter();
        let drawing: Vec<String> = (pens.chain(BUILT_IN))
            .map(|pen| format!("{}画", pen.get(zh)))
            .chain(["一笔画".to_owned()])
            .collect();
        let mut made: Vec<String> = (kinds::every_kind().into_iter())
            .flat_map(|kind| how_steps(kind, CELLS, zh))
            .collect();
        made.push(picture_goes(zh, (2, 1)));
        made.extend((1..=8).map(|unit| unit_title(zh, unit)));
        made.extend([Seal::Stage(2), Seal::TenSessions].map(|seal| seal_lines(zh, seal).concat()));
        let tables = every_text().into_iter().map(|text| text.get(zh).to_owned());
        for chinese in tables.chain(made) {
            let read = (drawing.iter()).fold(chinese.clone(), |read, verb| read.replace(verb, ""));
            for (term, instead) in RETIRED {
                assert!(!read.contains(term), "{chinese}: {term}, now {instead}");
            }
        }
    }
}

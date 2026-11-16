//! A lesson's end in words made with numbers (S122, S147, S148): what it took, the level reached,
//! what One more starts (a skill's lesson or a unit's start re-test), the extra drill's row and a
//! copy's date under a blind pair. Its fixed words are the course table's (`super`).

use openpaint_practice::{Offer, SkillLevel};
use openpaint_practice_app::OneMore;

use super::{exercise_count, exercises_line, level_word, retest_name, review_line, skill_name};
use crate::text::{Language, month_name};

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

/// A level reached at a lesson's end: "Up to “Fluent”", "到“熟练”了".
pub(crate) fn level_reached(language: Language, reached: SkillLevel) -> String {
    let word = level_word(reached).get(language);
    match language {
        Language::English => format!("Up to “{word}”"),
        Language::Chinese => format!("到“{word}”了"),
    }
}

/// What One more starts, under it: "Arcs · 5 exercises · about 4 min"; a unit's start re-test
/// (S148) "Re-test · Unit 2 · 6 exercises · about 9 min".
pub(crate) fn one_more_line(language: Language, next: &OneMore) -> String {
    match next {
        OneMore::Skill(offer) => {
            let name = skill_name(offer.skill).get(language);
            let counts = exercises_line(language, (offer.exercises, offer.minutes));
            format!("{name} · {counts}")
        }
        OneMore::Retest(retest) => {
            let count = u8::try_from(retest.skills.len()).unwrap_or(u8::MAX);
            let counts = exercises_line(language, (count, retest.minutes));
            format!("{} · {counts}", retest_name(language, retest.unit))
        }
    }
}

/// An extra drill's row: "Ellipses · 5 exercises", "椭圆 · 5 题".
pub(crate) fn drill_line(language: Language, offer: &Offer) -> String {
    review_line(language, offer)
}

/// A local day as a copy's date: "12 Nov", "11月12日".
pub(crate) fn copy_date(language: Language, day: i64) -> String {
    let date = openpaint_ui::dates::Date::of(day);
    match language {
        Language::English => format!("{} {}", date.day, &month_name(date.month)[..3]),
        Language::Chinese => format!("{}月{}日", date.month, date.day),
    }
}

#[cfg(test)]
mod tests {
    use openpaint_practice::{Retest, Skill};

    use super::*;

    #[test]
    fn a_lesson_s_end_says_its_exercises_time_and_what_one_more_starts() {
        assert_eq!(
            lesson_took(Language::English, (5, 192)),
            "5 exercises · 3 min 12 s"
        );
        assert_eq!(lesson_took(Language::Chinese, (3, 45)), "3 题 · 45 秒");
        let line = Skill::of_key("line").expect("lines");
        let offer = Offer {
            skill: line,
            level: SkillLevel::Second,
            exercises: 5,
            minutes: 4,
            redo: false,
        };
        assert_eq!(
            one_more_line(Language::English, &OneMore::Skill(offer)),
            "Lines · 5 exercises · about 4 min"
        );
        let retest = Retest {
            unit: 2,
            end: false,
            skills: vec![line; 6],
            minutes: 9,
        };
        assert_eq!(
            one_more_line(Language::Chinese, &OneMore::Retest(retest)),
            "复测 · 第 2 单元 · 6 题 · 约 9 分钟"
        );
        assert_eq!(drill_line(Language::Chinese, &offer), "线条 · 5 题");
        assert_eq!(copy_date(Language::Chinese, 20_769), "11月12日");
        assert_eq!(copy_date(Language::English, 20_769), "12 Nov");
    }
}

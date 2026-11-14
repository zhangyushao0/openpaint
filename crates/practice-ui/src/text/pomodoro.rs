//! The Pomodoro (S124): the home's row, the bar's ring and its menu, the time up, the block's end
//! with its break and its seal, the options' switch. Names, states and numbers only: no sentence
//! here says how the timer works.

use openpaint_practice::{Length, Skill, SkillLevel};

use super::{Language, level_word, skill_name};

texts! { PRACTICE_POMODORO;
    POMODORO = "Pomodoro", "番茄钟";
    MINUTES = "min", "分钟";
    START_TIMER = "Start timer", "开始计时";
    PAUSE = "Pause", "暂停";
    PAUSED = "Paused", "暂停";
    STOP_TIMER = "Stop timer", "结束番茄钟";
    TIME_UP = "Time", "到点";
    END = "End", "结束";
    POMODORO_DONE = "Pomodoro done", "画满一个番茄钟";
    BREAK = "Break", "休息";
    ANOTHER_POMODORO = "Another pomodoro", "再来一个番茄钟";
    BREAK_CHIME = "Break chime", "休息结束提示音";
    TODAY = "Today", "今天";
}

/// A length as the home's row offers it, which also names its control: "25".
pub(crate) fn length_number(length: Length) -> String {
    length.minutes().to_string()
}

/// The minutes left in the bar and on a break, rounded up: "18 min", "18 分". The English names
/// the bar's ring.
pub(crate) fn minutes_left(language: Language, minutes: u32) -> String {
    match language {
        Language::English => format!("{minutes} min"),
        Language::Chinese => format!("{minutes} 分"),
    }
}

/// A block's length at its end: "25 minutes", "25 分钟".
pub(crate) fn block_length(language: Language, length: Length) -> String {
    let minutes = length.minutes();
    match language {
        Language::English => format!("{minutes} minutes"),
        Language::Chinese => format!("{minutes} 分钟"),
    }
}

/// A block's lessons and exercises: "5 lessons · 19 exercises", "5 课 · 19 题".
pub(crate) fn block_count(language: Language, (lessons, exercises): (u32, u32)) -> String {
    let plural = |n: u32, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    match language {
        Language::English => format!(
            "{} · {}",
            plural(lessons, "lesson", "lessons"),
            plural(exercises, "exercise", "exercises")
        ),
        Language::Chinese => format!("{lessons} 课 · {exercises} 题"),
    }
}

/// A skill that gained a dot in a block, with the level it reached: "Lines up to “Beginning”",
/// "线条到“入门”了".
pub(crate) fn skill_reached(language: Language, skill: Skill, level: SkillLevel) -> String {
    let (name, word) = (
        skill_name(skill).get(language),
        level_word(level).get(language),
    );
    match language {
        Language::English => format!("{name} up to “{word}”"),
        Language::Chinese => format!("{name}到“{word}”了"),
    }
}

/// What a block's seal says: 番 over 茄; in English one word.
pub(crate) fn block_seal(language: Language) -> [String; 2] {
    match language {
        Language::English => ["FOCUS".to_owned(), String::new()],
        Language::Chinese => ["番".to_owned(), "茄".to_owned()],
    }
}

/// The words of a block's end and the bar made with numbers, for the tables' tests.
#[cfg(test)]
pub(crate) fn made(language: Language) -> Vec<String> {
    let line = Skill::of_key("line").expect("lines");
    let mut made = vec![
        minutes_left(language, 18),
        block_length(language, Length::TwentyFive),
        block_count(language, (5, 19)),
        skill_reached(language, line, SkillLevel::First),
    ];
    made.extend(block_seal(language));
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pomodoro_s_numbers_read_in_both_languages() {
        assert_eq!(minutes_left(Language::English, 18), "18 min");
        assert_eq!(minutes_left(Language::Chinese, 18), "18 分");
        assert_eq!(
            block_length(Language::Chinese, Length::TwentyFive),
            "25 分钟"
        );
        assert_eq!(
            block_count(Language::English, (5, 19)),
            "5 lessons · 19 exercises"
        );
        assert_eq!(
            block_count(Language::English, (1, 1)),
            "1 lesson · 1 exercise"
        );
        assert_eq!(block_count(Language::Chinese, (5, 19)), "5 课 · 19 题");
        let line = Skill::of_key("line").expect("lines");
        assert_eq!(
            skill_reached(Language::Chinese, line, SkillLevel::First),
            "线条到“入门”了"
        );
        assert_eq!(length_number(Length::Forty), "40");
    }
}

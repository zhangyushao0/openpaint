//! The practice book (S89): its name, the calendar's months and weekdays, the days' and the
//! book's time, the lessons shown, and the seals: what each stamp says (two short lines, stacked
//! as a seal's characters are).

use openpaint_practice::Seal;
use openpaint_ui::dates::Date;

use super::{Language, Text};

texts! { PRACTICE_BOOK_TABLE;
    PRACTICE_BOOK = "Practice book", "练习簿";
    EARLIER_MONTH = "Earlier month", "上个月";
    LATER_MONTH = "Later month", "下个月";
}

texts! { PRACTICE_BOOK_WEEKDAYS;
    MONDAY = "M", "一";
    TUESDAY = "T", "二";
    WEDNESDAY = "W", "三";
    THURSDAY = "T", "四";
    FRIDAY = "F", "五";
    SATURDAY = "S", "六";
    SUNDAY = "S", "日";
}

/// The weekdays' initials, Monday first.
pub(crate) const WEEKDAYS: [Text; 7] = [
    MONDAY, TUESDAY, WEDNESDAY, THURSDAY, FRIDAY, SATURDAY, SUNDAY,
];

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const CHINESE_DAYS: [&str; 7] = ["一", "二", "三", "四", "五", "六", "日"];
const NUMERALS: [&str; 9] = ["一", "二", "三", "四", "五", "六", "七", "八", "九"];

fn month_name(month: u32) -> &'static str {
    MONTHS[(month.clamp(1, 12) - 1) as usize]
}

/// The calendar's month: "October 2026", "2026年10月".
pub(crate) fn month_title(language: Language, date: Date) -> String {
    match language {
        Language::English => format!("{} {}", month_name(date.month), date.year),
        Language::Chinese => format!("{}年{}月", date.year, date.month),
    }
}

/// A day's title: "Tuesday 28 October", "10月28日 星期二"; `weekday` 0 Monday.
pub(crate) fn day_title(language: Language, date: Date, weekday: u32) -> String {
    let index = weekday.min(6) as usize;
    match language {
        Language::English => format!("{} {} {}", DAYS[index], date.day, month_name(date.month)),
        Language::Chinese => {
            format!("{}月{}日 星期{}", date.month, date.day, CHINESE_DAYS[index])
        }
    }
}

/// A time practised, to the minute (at least one when there is any): "54 min", "1 h 12 min";
/// "54 分钟", "1 小时 12 分钟".
pub(crate) fn practice_time(language: Language, seconds: u64) -> String {
    let minutes = if seconds == 0 {
        0
    } else {
        seconds.div_ceil(60)
    };
    let (hours, minutes) = (minutes / 60, minutes % 60);
    match (language, hours) {
        (Language::English, 0) => format!("{minutes} min"),
        (Language::English, _) => format!("{hours} h {minutes} min"),
        (Language::Chinese, 0) => format!("{minutes} 分钟"),
        (Language::Chinese, _) => format!("{hours} 小时 {minutes} 分钟"),
    }
}

/// The book's totals: "9 lessons · 4 h 12 min", "9 课 · 4 小时 12 分钟".
pub(crate) fn book_totals(language: Language, lessons: u32, seconds: u64) -> String {
    let time = practice_time(language, seconds);
    match language {
        Language::English if lessons == 1 => format!("1 lesson · {time}"),
        Language::English => format!("{lessons} lessons · {time}"),
        Language::Chinese => format!("{lessons} 课 · {time}"),
    }
}

/// A day's sheet named by its lessons (S121c): "Lesson 3 · Lines, Lesson 4 · Colours",
/// "第 3 课 · 线条，第 4 课 · 单色"; a comma between them, for a lesson's name holds a dot.
pub(crate) fn lessons_joined(language: Language, names: &[String]) -> String {
    match language {
        Language::English => names.join(", "),
        Language::Chinese => names.join("，"),
    }
}

/// What a seal's stamp says: two short lines, stacked.
pub(crate) fn seal_lines(language: Language, seal: Seal) -> [String; 2] {
    let pair = |en: [&str; 2], zh: [&str; 2]| match language {
        Language::English => en.map(str::to_owned),
        Language::Chinese => zh.map(str::to_owned),
    };
    match seal {
        Seal::FirstLesson => pair(["FIRST", "STEP"], ["初", "练"]),
        Seal::TenLessons => pair(["TEN", "DONE"], ["十", "课"]),
        Seal::FiftyLessons => pair(["FIFTY", "DONE"], ["五", "十"]),
        Seal::Unit(stage) => {
            let numeral = NUMERALS[usize::from(stage.clamp(1, 9)) - 1];
            match language {
                Language::English => ["UNIT".to_owned(), stage.to_string()],
                Language::Chinese => [numeral.to_owned(), "单元".to_owned()],
            }
        }
        Seal::HundredLines => pair(["100", "LINES"], ["百", "线"]),
        Seal::HundredColours => pair(["100", "HUES"], ["百", "色"]),
        Seal::FirstCopy => pair(["FIRST", "COPY"], ["临", "摹"]),
        Seal::TenHours => pair(["TEN", "HOURS"], ["十", "时"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_and_dates_read_in_both_languages() {
        assert_eq!(practice_time(Language::English, 0), "0 min");
        assert_eq!(practice_time(Language::English, 20), "1 min");
        assert_eq!(practice_time(Language::English, 54 * 60), "54 min");
        assert_eq!(
            practice_time(Language::Chinese, 4 * 3600 + 720),
            "4 小时 12 分钟"
        );
        assert_eq!(
            book_totals(Language::Chinese, 9, 4 * 3600 + 720),
            "9 课 · 4 小时 12 分钟"
        );
        let date = Date::of(20_754);
        assert_eq!(month_title(Language::English, date), "October 2026");
        assert_eq!(month_title(Language::Chinese, date), "2026年10月");
        assert_eq!(
            day_title(Language::English, date, 2),
            "Wednesday 28 October"
        );
        assert_eq!(day_title(Language::Chinese, date, 2), "10月28日 星期三");
        assert_eq!(
            seal_lines(Language::Chinese, Seal::Unit(2)),
            ["二".to_owned(), "单元".to_owned()]
        );
    }
}

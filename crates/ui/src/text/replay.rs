//! The replay's words (S163): Actions' row, the screen's islands and bar, the export panel, the
//! notices of the video being made and the video's last line. Times on the bar read as a clock
//! ("1:24"), a video's length in seconds ("48 s"), counts grouped by thousands ("1,288"), dates
//! in the device's zone, none with spaces inside in the Chinese (2026年11月14日).

use openpaint_app::{ReplayFacts, Speed};

use super::Language;
use crate::dates::Date;

texts! { REPLAY_TABLE;
    REPLAY = "Replay…", "回放…";
    EXPORT_VIDEO = "Export video…", "导出视频…";
    EXPORT_VIDEO_HEADING = "Export video", "导出视频";
    REPLAY_PLAY = "Play", "播放";
    REPLAY_PAUSE = "Pause", "暂停";
    REPLAY_TRACK = "Replay track", "进度";
    SEGMENT_START = "Segment start", "片段起点";
    SEGMENT_END = "Segment end", "片段终点";
    LENGTH = "Length", "长度";
    HIGHLIGHT = "30 s highlight", "30 秒精华";
    FULL = "Full", "完整";
    SEGMENT = "Segment", "自选片段";
    VIDEO_SIZE = "Size", "尺寸";
    ORIGINAL = "Original", "原始";
    VIDEO_END = "End", "结尾";
    STOP_ON_FINISHED = "Stop on the finished picture", "停在完成的画上";
    STROKES_AND_TIME = "Strokes and time", "笔数与时长";
    SAVE_TO_ALBUM = "Save to album", "保存到相册";
    SHARE_VIDEO = "Share…", "分享…";
    EXPORTING_VIDEO = "Exporting video", "正在导出视频";
    VIDEO_FAILED = "Couldn't export the video", "视频导出失败";
}

/// The 1080p size's name, the same in both.
pub(crate) const HD: &str = "1080p";
/// The app's name at the right of the video's last line, the same in both.
pub(crate) const BRAND: &str = "OpenPaint";

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A speed as the bar offers it ("4×"), the same in both.
#[must_use]
pub(crate) const fn speed(speed: Speed) -> &'static str {
    match speed {
        Speed::One => "1×",
        Speed::Four => "4×",
        Speed::Sixteen => "16×",
    }
}

/// How many strokes a record holds ("1,288 strokes"): Actions' note, the video's last line.
#[must_use]
pub(crate) fn strokes(language: Language, count: usize) -> String {
    let n = grouped(count);
    match language {
        Language::English if count == 1 => format!("{n} stroke"),
        Language::English => format!("{n} strokes"),
        Language::Chinese => format!("{n} 笔"),
    }
}

/// `count` with its thousands set apart ("1,288").
fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// A time on the bar: minutes and seconds ("1:24"), the hours first from an hour on ("1:02:03").
#[must_use]
pub(crate) fn clock(ms: u64) -> String {
    let seconds = ms / 1000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// The bar's time: the playhead and the whole ("1:24 / 3:18").
#[must_use]
pub(crate) fn playhead(at_ms: u64, total_ms: u64) -> String {
    format!("{} / {}", clock(at_ms), clock(total_ms))
}

/// A segment's range ("0:52 – 1:40").
#[must_use]
pub(crate) fn range([from, to]: [u64; 2]) -> String {
    format!("{} – {}", clock(from), clock(to))
}

/// The bar's time while a segment is chosen: its range and how long it lasts
/// ("0:52 – 1:40 · 48 s").
#[must_use]
pub(crate) fn segment_time(language: Language, [from, to]: [u64; 2]) -> String {
    let seconds = (to.saturating_sub(from) + 500) / 1000;
    format!("{} · {}", range([from, to]), lasting(language, seconds))
}

/// A video's length: seconds under a minute ("48 s"), else minutes and seconds ("2 min 10 s").
#[must_use]
pub(crate) fn lasting(language: Language, seconds: u64) -> String {
    let (minutes, seconds) = (seconds / 60, seconds % 60);
    match (language, minutes, seconds) {
        (Language::English, 0, s) => format!("{s} s"),
        (Language::English, m, 0) => format!("{m} min"),
        (Language::English, m, s) => format!("{m} min {s} s"),
        (Language::Chinese, 0, s) => format!("{s} 秒"),
        (Language::Chinese, m, 0) => format!("{m} 分钟"),
        (Language::Chinese, m, s) => format!("{m} 分 {s} 秒"),
    }
}

/// The stroke at the playhead: its local date and time when it kept them, then its number
/// ("14 Nov 21:07 · stroke 412", or "Stroke 412").
#[must_use]
pub(crate) fn stroke_label(
    language: Language,
    number: usize,
    began: Option<(Date, (i64, i64))>,
) -> String {
    let n = grouped(number);
    match (language, began) {
        (Language::English, Some((date, (hour, minute)))) => {
            let month = MONTHS[(date.month.clamp(1, 12) - 1) as usize];
            format!("{} {month} {hour:02}:{minute:02} · stroke {n}", date.day)
        }
        (Language::English, None) => format!("Stroke {n}"),
        (Language::Chinese, Some((date, (hour, minute)))) => {
            let (month, day) = (date.month, date.day);
            format!("{month}月{day}日 {hour:02}:{minute:02} · 第 {n} 笔")
        }
        (Language::Chinese, None) => format!("第 {n} 笔"),
    }
}

/// The canvas's own size as the export panel offers it ("Original 2880 × 1800").
#[must_use]
pub(crate) fn original(language: Language, [width, height]: [u32; 2]) -> String {
    format!("{} {width} × {height}", ORIGINAL.get(language))
}

/// The pill's progress ("62 %").
#[must_use]
pub(crate) fn percent(percent: u8) -> String {
    format!("{percent} %")
}

/// The video saved, and how long it lasts ("Video saved to album · 48 s").
#[must_use]
pub(crate) fn video_saved(language: Language, seconds: u32) -> String {
    let lasts = lasting(language, u64::from(seconds));
    match language {
        Language::English => format!("Video saved to album · {lasts}"),
        Language::Chinese => format!("视频已存到相册 · {lasts}"),
    }
}

/// The video's last line: the record's strokes and layers, then, when a stroke kept its time,
/// the time spent and the days drawn on (`days`: the first and the last start's local dates)
/// ("1,288 strokes · 6 layers · 3 h 12 min · 14–18 Nov 2026").
#[must_use]
pub(crate) fn caption(language: Language, facts: &ReplayFacts, days: Option<[Date; 2]>) -> String {
    let mut parts = vec![
        strokes(language, facts.strokes),
        layers(language, facts.layers),
    ];
    parts.extend(facts.spent_ms.map(|ms| spent(language, ms)));
    parts.extend(days.map(|days| days_drawn(language, days)));
    parts.join(" · ")
}

fn layers(language: Language, count: usize) -> String {
    match language {
        Language::English if count == 1 => "1 layer".to_owned(),
        Language::English => format!("{count} layers"),
        Language::Chinese => format!("{count} 个图层"),
    }
}

/// The time spent, to the minute (at least one): "3 h 12 min", "3 h", "42 min".
fn spent(language: Language, ms: u64) -> String {
    let minutes = ((ms + 30_000) / 60_000).max(1);
    let (hours, minutes) = (minutes / 60, minutes % 60);
    match (language, hours, minutes) {
        (Language::English, 0, m) => format!("{m} min"),
        (Language::English, h, 0) => format!("{h} h"),
        (Language::English, h, m) => format!("{h} h {m} min"),
        (Language::Chinese, 0, m) => format!("{m} 分钟"),
        (Language::Chinese, h, 0) => format!("{h} 小时"),
        (Language::Chinese, h, m) => format!("{h} 小时 {m} 分钟"),
    }
}

/// The days drawn on, from `first` to `last`, as short as they allow: one day, days of one month,
/// of one year, or across years.
fn days_drawn(language: Language, [first, last]: [Date; 2]) -> String {
    let month = |date: Date| MONTHS[(date.month.clamp(1, 12) - 1) as usize];
    let (a, b) = (first, last);
    let same_year = a.year == b.year;
    let same_month = same_year && a.month == b.month;
    match language {
        Language::English if a == b => format!("{} {} {}", a.day, month(a), a.year),
        Language::English if same_month => format!("{}–{} {} {}", a.day, b.day, month(b), b.year),
        Language::English if same_year => {
            format!("{} {} – {} {} {}", a.day, month(a), b.day, month(b), b.year)
        }
        Language::English => format!(
            "{} {} {} – {} {} {}",
            a.day,
            month(a),
            a.year,
            b.day,
            month(b),
            b.year
        ),
        Language::Chinese if a == b => format!("{}年{}月{}日", a.year, a.month, a.day),
        Language::Chinese if same_month => {
            format!("{}年{}月{}日–{}日", a.year, a.month, a.day, b.day)
        }
        Language::Chinese if same_year => {
            format!(
                "{}年{}月{}日–{}月{}日",
                a.year, a.month, a.day, b.month, b.day
            )
        }
        Language::Chinese => format!(
            "{}年{}月{}日–{}年{}月{}日",
            a.year, a.month, a.day, b.year, b.month, b.day
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: Language = Language::English;
    const ZH: Language = Language::Chinese;

    fn day(year: i64, month: u32, day: u32) -> Date {
        Date { year, month, day }
    }

    #[test]
    fn counts_times_and_lengths_read_as_the_mock_shows_them() {
        assert_eq!(strokes(EN, 1288), "1,288 strokes");
        assert_eq!(strokes(EN, 1), "1 stroke");
        assert_eq!(strokes(ZH, 1_288_000), "1,288,000 笔");
        assert_eq!(strokes(ZH, 412), "412 笔");
        assert_eq!(playhead(84_400, 198_000), "1:24 / 3:18");
        assert_eq!(clock(3_723_000), "1:02:03");
        assert_eq!(segment_time(EN, [52_000, 100_000]), "0:52 – 1:40 · 48 s");
        assert_eq!(segment_time(ZH, [52_000, 100_000]), "0:52 – 1:40 · 48 秒");
        assert_eq!(lasting(EN, 130), "2 min 10 s");
        assert_eq!(lasting(ZH, 120), "2 分钟");
        assert_eq!(video_saved(EN, 48), "Video saved to album · 48 s");
        assert_eq!(video_saved(ZH, 48), "视频已存到相册 · 48 秒");
        assert_eq!(original(ZH, [2880, 1800]), "原始 2880 × 1800");
        assert_eq!(percent(62), "62 %");
    }

    #[test]
    fn a_stroke_shows_its_date_when_it_kept_one() {
        let began = Some((day(2026, 11, 14), (21, 7)));
        assert_eq!(stroke_label(EN, 412, began), "14 Nov 21:07 · stroke 412");
        assert_eq!(stroke_label(ZH, 412, began), "11月14日 21:07 · 第 412 笔");
        assert_eq!(stroke_label(EN, 412, None), "Stroke 412");
        assert_eq!(stroke_label(ZH, 412, None), "第 412 笔");
    }

    #[test]
    fn the_last_line_is_as_short_as_its_days_allow() {
        let facts = ReplayFacts {
            strokes: 1288,
            layers: 6,
            spent_ms: Some((3 * 60 + 12) * 60_000),
            began_ms: Some([0, 0]),
        };
        let days = Some([day(2026, 11, 14), day(2026, 11, 18)]);
        assert_eq!(
            caption(EN, &facts, days),
            "1,288 strokes · 6 layers · 3 h 12 min · 14–18 Nov 2026"
        );
        assert_eq!(
            caption(ZH, &facts, days),
            "1,288 笔 · 6 个图层 · 3 小时 12 分钟 · 2026年11月14日–18日"
        );
        let months = [day(2026, 10, 28), day(2026, 11, 2)];
        assert_eq!(days_drawn(EN, months), "28 Oct – 2 Nov 2026");
        assert_eq!(days_drawn(ZH, months), "2026年10月28日–11月2日");
        let years = [day(2025, 12, 28), day(2026, 1, 2)];
        assert_eq!(days_drawn(EN, years), "28 Dec 2025 – 2 Jan 2026");
        assert_eq!(days_drawn(ZH, years), "2025年12月28日–2026年1月2日");
        let one = [day(2026, 11, 14); 2];
        assert_eq!(days_drawn(EN, one), "14 Nov 2026");
        assert_eq!(days_drawn(ZH, one), "2026年11月14日");
        let short = ReplayFacts {
            spent_ms: Some(42 * 60_000 + 10_000),
            ..facts
        };
        assert_eq!(caption(ZH, &short, None), "1,288 笔 · 6 个图层 · 42 分钟");
        let untimed = ReplayFacts {
            strokes: 1,
            layers: 1,
            spent_ms: None,
            began_ms: None,
        };
        assert_eq!(caption(EN, &untimed, None), "1 stroke · 1 layer");
    }
}

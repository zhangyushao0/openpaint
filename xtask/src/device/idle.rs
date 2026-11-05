//! `cargo device-idle [--minutes N]`: whether the tablet is free for a device routine, read and
//! never changed (S103). It prints the screen's state (`dumpsys power`'s wakefulness), the app
//! in front (`dumpsys window`'s focused app and window) and the minutes since the last touch,
//! pen or key: the power manager's last user activity, which every input event the dispatcher
//! delivers (a pen's hover too) renews, printed "ms ago" against the uptime clock. It ends well
//! when the screen is off or nothing touched the tablet for `N` minutes (10 when not given), and
//! fails otherwise, saying why: the routine waits and asks again. The agents' own injected
//! gestures and wake-ups count as touches too.

use std::time::Duration;

use super::adb::{Adb, DeviceError, Result};

/// How long one query may take before it is stopped (as `device-info`'s).
const QUERY_LIMIT: Duration = Duration::from_secs(30);
/// Untouched this long, the tablet is free.
const MINUTES: u64 = 10;

const USAGE: &str = "cargo device-idle [--minutes N]";

/// `cargo device-idle [--minutes N]`.
pub fn idle(args: &[String]) -> Result<()> {
    let minutes = match args {
        [] => MINUTES,
        [flag, n] if flag == "--minutes" => n.parse().map_err(|_| DeviceError::Usage(USAGE))?,
        _ => return Err(DeviceError::Usage(USAGE)),
    };
    let adb = Adb::connected()?;
    let power = adb.shell_within(&["dumpsys", "-t", "10", "power"], QUERY_LIMIT)?;
    let window = adb.shell_within(&["dumpsys", "-t", "10", "window"], QUERY_LIMIT)?;
    let wakefulness = field(&power, &["mWakefulness="]).ok_or_else(|| unread("mWakefulness"))?;
    let (ago, said) = last_activity(&power).ok_or_else(|| unread("lastUserActivityTime"))?;
    let app = field(&window, &["mFocusedApp="]).map_or("none", record_name);
    let focus = field(&window, &["mCurrentFocus="]).map_or("none", record_name);
    let awake = wakefulness == "Awake";
    println!(
        "screen: {} ({wakefulness})",
        if awake { "awake" } else { "off" }
    );
    if focus == app {
        println!("in front: {app}");
    } else {
        println!("in front: {app} (focus: {focus})");
    }
    let since = ago.as_secs_f64() / 60.0;
    println!("last touch, pen or key: {since:.1} minutes ago ({said})");
    if !awake {
        println!("free: the screen is off");
        return Ok(());
    }
    if ago >= Duration::from_secs(minutes * 60) {
        println!("free: untouched for {minutes} minutes or more");
        return Ok(());
    }
    Err(DeviceError::InUse(format!(
        "the screen is awake and was touched {since:.1} minutes ago, within {minutes}; \
         wait and run cargo device-idle again"
    )))
}

/// The value after the first of `keys` that starts a line, without its line's end.
fn field<'a>(dump: &'a str, keys: &[&str]) -> Option<&'a str> {
    dump.lines()
        .find_map(|line| keys.iter().find_map(|key| line.trim().strip_prefix(key)))
        .map(str::trim)
}

/// The most recent user activity in `dumpsys power`, as time since, and the line saying it:
/// Android 12 and later print it per display group (`lastUserActivityTime=`), earlier ones once
/// (`mLastUserActivityTime=`), each as `<uptime ms> (<n> ms ago)`.
fn last_activity(power: &str) -> Option<(Duration, &str)> {
    power
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let value = line
                .strip_prefix("lastUserActivityTime=")
                .or_else(|| line.strip_prefix("mLastUserActivityTime="))?;
            Some((ms_ago(value)?, line))
        })
        .min()
}

/// `123 (456 ms ago)` as 456 ms; `(now)` or a time ahead (`(in 5 ms)`) as none ago.
fn ms_ago(value: &str) -> Option<Duration> {
    let (_, said) = value.split_once('(')?;
    let said = said.strip_suffix(')')?;
    if said == "now" || said.starts_with("in ") {
        return Some(Duration::ZERO);
    }
    let ms = said.strip_suffix(" ms ago")?.parse().ok()?;
    Some(Duration::from_millis(ms))
}

/// The component of a window or activity record (`Window{1a2b u0 dev.openpaint/.Main}`,
/// `ActivityRecord{3c4d u0 dev.openpaint/.Main t12}`): its third word; `null` as itself.
fn record_name(value: &str) -> &str {
    let inner = value
        .split_once('{')
        .map_or(value, |(_, inner)| inner.trim_end_matches('}'));
    inner.split_whitespace().nth(2).unwrap_or(inner)
}

fn unread(what: &str) -> DeviceError {
    DeviceError::Failed {
        command: "adb shell dumpsys power".into(),
        output: format!("no {what} in its answer"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POWER: &str = "Power Manager State:\n  mDirty=0x0\n  mWakefulness=Awake\n  \
        mWakefulnessChanging=false\n  mLastWakeTime=5302 (8000123 ms ago)\nDisplay Group User \
        Activity:\n  displayGroupId=0\n  userActivitySummary=0x1\n  lastUserActivityTime=7404117 \
        (601000 ms ago)\n  lastUserActivityTimeNoChangeLights=8005000 (5 ms ago)\n";

    #[test]
    fn power_and_window_answers_read() {
        assert_eq!(field(POWER, &["mWakefulness="]), Some("Awake"));
        let line = "lastUserActivityTime=7404117 (601000 ms ago)";
        assert_eq!(
            last_activity(POWER),
            Some((Duration::from_millis(601_000), line))
        );
        let older = "  mLastUserActivityTime=12 (90 ms ago)\n  mLastUserActivityTime=9 (now)";
        let now = last_activity(older).map(|(ago, _)| ago);
        assert_eq!(now, Some(Duration::ZERO));
        assert_eq!(ms_ago("7 (in 30 ms)"), Some(Duration::ZERO));
        assert_eq!(ms_ago("7"), None);
        assert_eq!(
            last_activity("lastUserActivityTimeNoChangeLights=1 (2 ms ago)"),
            None
        );
        let window = "  mCurrentFocus=Window{2f6a0b1 u0 NotificationShade}\n  \
            mFocusedApp=ActivityRecord{e5c u0 dev.openpaint/.MainActivity t71}";
        let app = field(window, &["mFocusedApp="]).map(record_name);
        assert_eq!(app, Some("dev.openpaint/.MainActivity"));
        let focus = field(window, &["mCurrentFocus="]).map(record_name);
        assert_eq!(focus, Some("NotificationShade"));
        assert_eq!(record_name("null"), "null");
    }
}

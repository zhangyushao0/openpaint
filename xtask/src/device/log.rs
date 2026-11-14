//! `cargo device-log [n]`: the app's own last `n` log lines (40 when not given), every level of
//! its tag (`openpaint`), as logcat keeps them; read-only (`logcat -d` dumps the buffer and
//! leaves it), nothing on the tablet changed. What the app said after a tablet run (S124: the
//! Pomodoro's chime, a test block's records) reaches the dev machine without hand-typed adb.

use super::adb::{Adb, DeviceError, Result};

/// The app's log tag (`apps/android/src/jni_entry.rs`).
const TAG: &str = "openpaint";
/// How many lines are shown when no number is given.
const LAST: usize = 40;

const USAGE: &str = "cargo device-log [n], n from 1";

/// `cargo device-log [n]`.
pub fn log(args: &[String]) -> Result<()> {
    let count = match args {
        [] => LAST,
        [n] => (n.parse::<usize>().ok())
            .filter(|n| *n >= 1)
            .ok_or(DeviceError::Usage(USAGE))?,
        _ => return Err(DeviceError::Usage(USAGE)),
    };
    let adb = Adb::connected()?;
    let filter = format!("{TAG}:V");
    let output = adb.text(&["logcat", "-d", "-v", "threadtime", "-s", &filter])?;
    let lines = last(&output, count);
    for line in &lines {
        println!("{line}");
    }
    println!("log: the app's last {} lines", lines.len());
    Ok(())
}

/// The last `count` lines of `output` that are log lines (logcat's "--------- beginning of"
/// banners left out).
fn last(output: &str, count: usize) -> Vec<&str> {
    let lines: Vec<&str> = (output.lines())
        .filter(|line| !line.trim().is_empty() && !line.starts_with("--------- "))
        .collect();
    lines[lines.len().saturating_sub(count)..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_lines_leave_the_banners_out() {
        let output = "--------- beginning of main\n\
                      11-13 10:00:00.000  100  100 I openpaint: one\n\
                      11-13 10:00:01.000  100  100 W openpaint: two\n\
                      --------- beginning of crash\n\
                      11-13 10:00:02.000  100  100 E openpaint: three\n";
        assert_eq!(
            last(output, 2),
            [
                "11-13 10:00:01.000  100  100 W openpaint: two",
                "11-13 10:00:02.000  100  100 E openpaint: three"
            ]
        );
        assert_eq!(last(output, 40).len(), 3);
        assert!(last("", 5).is_empty());
    }
}

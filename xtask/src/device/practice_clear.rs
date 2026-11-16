//! `cargo device-practice-clear` (S152, DECISIONS D112): practice cleared from the author's
//! library on the tablet, for a fresh start: every practice record (the sheets' records read or
//! not, their lessons' givens, the Pomodoro's blocks, the blind pairs) and every practice sheet
//! (a day's, a test's, a colour copy, a drawing from life) with its display cache. The studio's
//! artworks with their versions and display caches, the practice packs, the user's own pictures
//! and the Pomodoro's settings stay. Refused unless a backup (`device-backup`) was written today
//! (UTC), as `device-reset` is.
//!
//! The app is taken to the home (a sheet or an artwork left as its Home button leaves it, stored)
//! and stopped; the debug build's instrumentation then runs the store's own clear in a process
//! without an activity, so no runtime holds the library (`DebugControls.practiceClear`), and the
//! app is stopped again, to start afresh on the first lesson. What went is listed by count.

use std::time::SystemTime;

use super::adb::{Adb, DeviceError, PACKAGE, Result};
use super::artifacts;
use super::input::{answer, instrument};
use super::library::stop_app;
use super::practice_test::to_home;
use super::reset::{day_of, todays_backup};

pub fn practice_clear(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(DeviceError::Usage("cargo device-practice-clear"));
    }
    let backups = artifacts().join("backups");
    let today = day_of(SystemTime::now());
    let backup = todays_backup(&backups, today)?.ok_or(DeviceError::NoBackup(backups))?;
    println!("today's backup: {}", backup.display());
    let adb = Adb::connected()?;
    let running = adb.shell(&[&format!("pidof {PACKAGE}; true")])?;
    if !running.trim().is_empty() {
        to_home(&adb)?;
    }
    stop_app(&adb)?;
    let output = instrument(&adb, &[("practice-clear", "1".to_owned())])?;
    let said = answer(&output, "practice-clear").unwrap_or("no answer");
    let Some(counts) = cleared(said) else {
        return Err(DeviceError::Failed {
            command: "device-practice-clear".into(),
            output: said.to_owned(),
        });
    };
    stop_app(&adb)?;
    for count in counts {
        println!("  {count}");
    }
    println!("practice cleared: the app starts on the first lesson");
    Ok(())
}

/// The counts a clear's answer lists ("cleared", then one a field, tab-separated); none for any
/// other answer.
fn cleared(said: &str) -> Option<Vec<&str>> {
    let counts = said.strip_prefix("cleared\t")?;
    Some(counts.split('\t').collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_clear_s_answer_lists_its_counts() {
        let said = "cleared\t41 sheets' records\t2 Pomodoro blocks\tkept 7 artworks and 4 packs";
        assert_eq!(
            cleared(said),
            Some(vec![
                "41 sheets' records",
                "2 Pomodoro blocks",
                "kept 7 artworks and 4 packs"
            ])
        );
        for other in ["not cleared: the app is running", "no answer", "cleared"] {
            assert_eq!(cleared(other), None, "{other}");
        }
    }
}

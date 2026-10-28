//! `cargo device-screens [n]`: the tablet's newest `n` screenshots (5 when not given) copied to
//! `artifacts/device/screens/` and listed newest first with their times, so what the author saw on
//! the tablet reaches the dev machine (S91). Nothing on the tablet changes: a listing by `stat` and
//! `adb pull`. Android 15 keeps screenshots in `Pictures/Screenshots`; some makers' builds keep
//! them in `DCIM/Screenshots`: both are read, and a missing one lists nothing.

use std::path::Path;

use super::adb::{Adb, DeviceError, Result};
use super::{artifacts, civil};

/// Where screenshots are kept on the tablet.
const FOLDERS: [&str; 2] = ["/sdcard/Pictures/Screenshots", "/sdcard/DCIM/Screenshots"];
/// How many are copied when no number is given.
const NEWEST: usize = 5;
/// The pictures a screenshot is saved as.
const PICTURES: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

const USAGE: &str = "cargo device-screens [n], n from 1";

/// `cargo device-screens [n]`.
pub fn screens(args: &[String]) -> Result<()> {
    let count = match args {
        [] => NEWEST,
        [n] => (n.parse::<usize>().ok())
            .filter(|n| *n >= 1)
            .ok_or(DeviceError::Usage(USAGE))?,
        _ => return Err(DeviceError::Usage(USAGE)),
    };
    let adb = Adb::connected()?;
    let mut found = Vec::new();
    for folder in FOLDERS {
        // "held" when the folder is there, then seconds since 1970 and the path, one a line.
        let listing = format!(
            "if [ -d '{folder}' ]; then echo held; stat -c '%Y %n' '{folder}'/* 2>/dev/null; fi; \
             true"
        );
        let output = adb.shell(&[&listing])?;
        let pictures = listed(&output);
        match output.lines().next().map(str::trim) {
            Some("held") => println!("  {folder}: {} pictures", pictures.len()),
            _ => println!("  {folder}: not there"),
        }
        found.extend(pictures);
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found.truncate(count);
    if found.is_empty() {
        println!("screens: no screenshot in {}", FOLDERS.join(" or "));
        return Ok(());
    }
    let dir = artifacts().join("screens");
    std::fs::create_dir_all(&dir).map_err(|error| DeviceError::File {
        path: dir.clone(),
        error: error.to_string(),
    })?;
    for (time, path) in &found {
        let local = dir.join(local_name(path));
        adb.text(&["pull", path, &local.to_string_lossy()])?;
        println!("  {}  {path}  ->  {}", utc(*time), local.display());
    }
    println!(
        "screens: the newest {} copied to {}",
        found.len(),
        dir.display()
    );
    Ok(())
}

/// The pictures in `stat`'s lines: when each was last written (seconds since 1970) and its path.
fn listed(output: &str) -> Vec<(u64, String)> {
    output
        .lines()
        .filter_map(|line| {
            let (time, path) = line.trim().split_once(' ')?;
            let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
            if !PICTURES.contains(&extension.as_str()) {
                return None;
            }
            Some((time.parse().ok()?, path.to_owned()))
        })
        .collect()
}

/// A screenshot's file name as kept on the dev machine: its own, any character but a-z, A-Z,
/// 0-9, '.', '_' and '-' made '-'.
fn local_name(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// `seconds` since 1970 as `YYYY-MM-DD HH:MM:SS UTC`.
fn utc(seconds: u64) -> String {
    let (days, day) = (seconds / 86_400, seconds % 86_400);
    let (year, month, date) = civil(days);
    format!(
        "{year:04}-{month:02}-{date:02} {:02}:{:02}:{:02} UTC",
        day / 3600,
        day / 60 % 60,
        day % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_listing_keeps_pictures_newest_names_plain() {
        let output = "1793180000 /sdcard/Pictures/Screenshots/Screenshot_20261028-120000.png\n\
                      1793180100 /sdcard/Pictures/Screenshots/notes.txt\n\
                      bad /sdcard/Pictures/Screenshots/x.png\n\
                      1793170000 /sdcard/Pictures/Screenshots/Shot 2 (copy).JPG\n";
        let found = listed(output);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].0, 1_793_180_000);
        assert_eq!(found[1].1, "/sdcard/Pictures/Screenshots/Shot 2 (copy).JPG");
        assert_eq!(local_name(&found[1].1), "Shot-2--copy-.JPG");
        assert_eq!(
            local_name("/sdcard/DCIM/Screenshots/a/../b.png"),
            "b.png",
            "never a path"
        );
        assert_eq!(utc(1_791_201_600), "2026-10-05 12:00:00 UTC");
        for folder in FOLDERS {
            assert!(folder.starts_with("/sdcard/") && folder.ends_with("/Screenshots"));
        }
    }
}

//! `cargo device-check [label] [--no-install] [--no-launch]`: wakes the tablet, builds and
//! installs the debug APK (unless `--no-install`), launches the app (unless `--no-launch`, which
//! keeps a system picker over it open), waits, writes a screenshot to
//! `artifacts/device/<label>.png` and prints the app's error, panic and crash lines logged since;
//! a panic or crash fails it.
//!
//! `cargo device-input`: one gesture through the debug build's `GestureInstrumentation`, after
//! waking the screen (an asleep screen has no window to touch):
//!
//! - `pen x,y,x,y,... [pressure [tilt]] [primary|secondary|tertiary|eraser]`: one pen path, a
//!   point per 8 ms (repeat a point to hold), leaning `tilt` degrees from upright (S44), with a
//!   side button held (from the tip outwards) or the eraser end (S41, S43);
//! - `finger x,y[,x,y...] [x,y[,x,y...] [ms]]`: fingers down at the first points, moving to the
//!   second over `ms` (default 64), then lifting;
//! - `hover x,y [ms] [primary|secondary|tertiary|eraser]`: the pen held above the glass for
//!   `ms` (default 1500), as `pen` with a button or the eraser end;
//! - `tap <name>`: a finger tap on the middle of the control the app last drew under that name
//!   (D48's names; `name #n` for a shared one), looked up on the device by the debug build;
//! - `gradient on|off`: no gesture; the screen's precision check (S37) over everything, or off:
//!   a soft grey ramp from sRGB 100 to 120, its upper half at 8 bits, its lower at the window's
//!   ten.

use std::time::Duration;

use super::adb::{Adb, DeviceError, PACKAGE, Result, write_file};
use super::{artifacts, plain};
use crate::android::{ACTIVITY, APK};
use crate::process::root;

const INSTRUMENTATION: &str = "dev.openpaint/.GestureInstrumentation";
/// How long the app gets after launching before the screenshot.
const SETTLE: Duration = Duration::from_secs(3);
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

pub fn check(args: &[String]) -> Result<()> {
    let flag = |name: &str| args.iter().any(|arg| arg == name);
    // `--no-launch`: the screen as it is (a system picker over the app stays open).
    let (install, launch) = (!flag("--no-install"), !flag("--no-launch"));
    let labels: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    let label = match labels[..] {
        [] => "check",
        [label] => plain(label)?,
        _ => {
            return Err(DeviceError::Usage(
                "cargo device-check [label] [--no-install] [--no-launch]",
            ));
        }
    };
    let adb = Adb::connected()?;
    wake(&adb)?;
    if !launch {
        let since = adb.shell(&["date", "+%s"])?;
        screenshot(&adb, label)?;
        return report_log(&adb, since.trim());
    }
    if install {
        crate::android::build().map_err(|output| DeviceError::Failed {
            command: "android-build".into(),
            output,
        })?;
        let apk = root().join(APK);
        adb.text(&["install", "-r", &apk.to_string_lossy()])?;
    }
    let since = adb.shell(&["date", "+%s"])?;
    adb.shell(&["am", "start", "-W", "-n", ACTIVITY])?;
    std::thread::sleep(SETTLE);
    screenshot(&adb, label)?;
    report_log(&adb, since.trim())
}

/// The screen now as `artifacts/device/<label>.png`.
fn screenshot(adb: &Adb, label: &str) -> Result<()> {
    let path = artifacts().join(format!("{label}.png"));
    let shot = adb.bytes(&["exec-out", "screencap", "-p"])?;
    if !shot.starts_with(PNG) {
        return Err(DeviceError::Failed {
            command: "adb exec-out screencap -p".into(),
            output: "no PNG came back".into(),
        });
    }
    write_file(&path, &shot)?;
    println!("screenshot {}", path.display());
    Ok(())
}

/// The app's error lines logged since `since` (device seconds); a panic, a crash or no app
/// process fails.
fn report_log(adb: &Adb, since: &str) -> Result<()> {
    let pid = adb
        .shell(&["pidof", PACKAGE])
        .ok()
        .map(|pid| pid.trim().to_owned())
        .filter(|pid| !pid.is_empty());
    let from = format!("{since}.000");
    let log = adb.text(&["logcat", "-d", "-v", "threadtime", "-T", &from, "*:E"])?;
    let ours = |line: &&str| {
        let pid_of = line.split_whitespace().nth(2);
        pid.as_deref().is_some_and(|pid| pid_of == Some(pid)) || line.contains("openpaint")
    };
    let lines: Vec<&str> = log.lines().filter(ours).collect();
    for line in &lines {
        println!("  {line}");
    }
    let fatal: Vec<String> = lines
        .iter()
        .filter(|line| {
            line.contains("panic:") || line.contains("FATAL") || line.contains("Fatal signal")
        })
        .map(|line| (*line).to_owned())
        .collect();
    if !fatal.is_empty() {
        return Err(DeviceError::Panic(fatal));
    }
    let Some(pid) = pid else {
        return Err(DeviceError::Panic(vec!["the app is not running".into()]));
    };
    println!(
        "logcat: {} error lines from the app (pid {pid}), no panic",
        lines.len()
    );
    Ok(())
}

/// The screen on and, where no lock is set, the keyguard away: input reaches only a window.
fn wake(adb: &Adb) -> Result<()> {
    adb.shell(&["input", "keyevent", "KEYCODE_WAKEUP"])?;
    // Fails harmlessly on a secure lock, which the author unlocks.
    let _ = adb.shell(&["wm", "dismiss-keyguard"]);
    Ok(())
}

pub fn input(args: &[String]) -> Result<()> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let extras = extras(&args)?;
    let adb = Adb::connected()?;
    wake(&adb)?;
    let mut words = vec!["am", "instrument", "-w", "--no-restart"];
    for (key, value) in &extras {
        words.extend(["-e", key, value.as_str()]);
    }
    words.push(INSTRUMENTATION);
    let output = adb.shell(&words)?;
    if !output.contains("gesture=injected") {
        held_pointers(&adb);
        return Err(DeviceError::Injection(output.trim().to_owned()));
    }
    match output.lines().find_map(|line| line.split_once("tapped=")) {
        Some((_, tapped)) => println!("tapped {}", tapped.trim()),
        None => println!("injected {}", args.join(" ")),
    }
    Ok(())
}

/// After a refused gesture: the pointers the input dispatcher still holds per window (its
/// `TouchStates`), read-only. Injected events all come from device -1, and a gesture cut off
/// (an instrumentation ended by another) leaves its pointers there (S38).
fn held_pointers(adb: &Adb) {
    let Ok(dump) = adb.shell(&["dumpsys", "-t", "10", "input"]) else {
        return;
    };
    let held = dump
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("TouchStates"))
        .take_while(|line| !line.trim_start().starts_with("Display:"));
    println!("the input dispatcher holds:");
    for line in held {
        println!("  {}", line.trim());
    }
}

const INPUT_USAGE: &str = "cargo device-input \
                           pen x,y,... [pressure [tilt degrees]] [primary|secondary|tertiary|eraser] \
                           | finger x,y[,...] [x,y[,...] [ms]] \
                           | hover x,y [ms] [primary|secondary|tertiary|eraser] \
                           | tap <control name> | gradient on|off";

/// The instrumentation's `-e` arguments for a `device-input` command line.
fn extras(args: &[&str]) -> Result<Vec<(&'static str, String)>> {
    let usage = DeviceError::Usage(INPUT_USAGE);
    if let ["tap", name] = args {
        // The device's shell reads the line: the name goes in single quotes, so none inside.
        if name.is_empty() || name.chars().any(|c| c == '\'' || c.is_control()) {
            return Err(usage);
        }
        return Ok(vec![("tap", format!("'{name}'"))]);
    }
    if let ["gradient", state @ ("on" | "off")] = args {
        return Ok(vec![("gradient", (*state).to_owned())]);
    }
    // A pen or hover line may end with the pen end or side button it uses (S41).
    let (words, end) = match args {
        [
            "pen" | "hover",
            ..,
            end @ ("primary" | "secondary" | "tertiary" | "eraser"),
        ] => (&args[..args.len() - 1], Some(*end)),
        _ => (args, None),
    };
    let mut extras = gesture_extras(words)?;
    match end {
        Some("eraser") => {
            extras.retain(|(key, _)| *key != "tool");
            extras.insert(0, ("tool", "eraser".to_owned()));
        }
        Some(button) => extras.push(("button", button.to_owned())),
        None => {}
    }
    Ok(extras)
}

/// The instrumentation's `-e` arguments for a pen, finger or hover line without its end.
fn gesture_extras(args: &[&str]) -> Result<Vec<(&'static str, String)>> {
    let usage = DeviceError::Usage(INPUT_USAGE);
    // Only digits, points, commas and a sign reach the device's shell.
    if !args[1.min(args.len())..].iter().all(|arg| {
        arg.bytes()
            .all(|b| b.is_ascii_digit() || b".,-".contains(&b))
    }) {
        return Err(usage);
    }
    let extras: Vec<(&'static str, &str)> = match *args {
        ["pen", path] => vec![("tool", "stylus"), ("path", path)],
        ["pen", path, pressure] => vec![("tool", "stylus"), ("pressure", pressure), ("path", path)],
        // S44: the pen leaning that many degrees from upright.
        ["pen", path, pressure, tilt] => vec![
            ("tool", "stylus"),
            ("pressure", pressure),
            ("tilt", tilt),
            ("path", path),
        ],
        ["finger", from] => vec![("from", from)],
        ["finger", from, to] => vec![("from", from), ("to", to)],
        ["finger", from, to, ms] => vec![("from", from), ("to", to), ("ms", ms)],
        ["hover", at] => vec![("hover", at)],
        ["hover", at, ms] => vec![("hover", at), ("ms", ms)],
        _ => return Err(usage),
    };
    Ok(extras
        .into_iter()
        .map(|(key, value)| (key, value.to_owned()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_lines_become_instrumentation_arguments() {
        let pen = extras(&["pen", "100,200,300,400", "0.8"]).expect("pen");
        assert_eq!(
            pen,
            [
                ("tool", "stylus".to_owned()),
                ("pressure", "0.8".to_owned()),
                ("path", "100,200,300,400".to_owned())
            ]
        );
        let fingers = extras(&["finger", "1,2,3,4", "0,2,5,4", "200"]).expect("fingers");
        assert_eq!(fingers[2], ("ms", "200".to_owned()));
        let tap = extras(&["tap", "Move layer up"]).expect("tap");
        assert_eq!(tap, [("tap", "'Move layer up'".to_owned())]);
        assert!(extras(&["tap", "it's"]).is_err());
        let gradient = extras(&["gradient", "on"]).expect("gradient");
        assert_eq!(gradient, [("gradient", "on".to_owned())]);
        assert!(extras(&["gradient", "maybe"]).is_err());
        assert!(extras(&["tap", "a\nb"]).is_err());
        assert!(extras(&["pen", "1,2;rm -rf /"]).is_err());
        let primary = extras(&["pen", "1,2,3,4", "0.5", "primary"]).expect("button");
        assert_eq!(primary.last(), Some(&("button", "primary".to_owned())));
        assert_eq!(primary[0], ("tool", "stylus".to_owned()));
        let tilted = extras(&["pen", "1,2,3,4", "0.5", "70", "primary"]).expect("tilted");
        assert_eq!(tilted[2], ("tilt", "70".to_owned()));
        assert_eq!(tilted.len(), 5, "{tilted:?}");
        let eraser = extras(&["pen", "1,2,3,4", "eraser"]).expect("eraser");
        assert_eq!(eraser[0], ("tool", "eraser".to_owned()));
        assert_eq!(eraser.len(), 2, "{eraser:?}");
        let hover = extras(&["hover", "5,6", "800", "secondary"]).expect("hover");
        assert_eq!(hover[2], ("button", "secondary".to_owned()));
        let third = extras(&["hover", "5,6", "tertiary"]).expect("third");
        assert_eq!(
            third,
            [
                ("hover", "5,6".to_owned()),
                ("button", "tertiary".to_owned())
            ]
        );
        assert!(extras(&["finger", "1,2", "primary"]).is_err());
        assert!(extras(&["walk", "1,2"]).is_err());
        assert!(extras(&[]).is_err());
    }
}

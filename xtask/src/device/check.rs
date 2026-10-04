//! `cargo device-check [label] [--no-install]`: wakes the tablet, builds and installs the debug
//! APK (unless `--no-install`), launches the app, waits, writes a screenshot to
//! `artifacts/device/<label>.png` and prints the app's error, panic and crash lines logged since;
//! a panic or crash fails it.
//!
//! `cargo device-input`: one gesture through the debug build's `GestureInstrumentation`, after
//! waking the screen (an asleep screen has no window to touch):
//!
//! - `pen x,y,x,y,... [pressure]`: one pen path, a point per 8 ms (repeat a point to hold);
//! - `finger x,y[,x,y...] [x,y[,x,y...] [ms]]`: fingers down at the first points, moving to the
//!   second over `ms` (default 64), then lifting;
//! - `hover x,y [ms]`: the pen held above the glass for `ms` (default 1500).

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
    let install = !args.iter().any(|arg| arg == "--no-install");
    let labels: Vec<&String> = args.iter().filter(|arg| *arg != "--no-install").collect();
    let label = match labels[..] {
        [] => "check",
        [label] => plain(label)?,
        _ => {
            return Err(DeviceError::Usage(
                "cargo device-check [label] [--no-install]",
            ));
        }
    };
    let adb = Adb::connected()?;
    wake(&adb)?;
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
    report_log(&adb, since.trim())
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
        words.extend(["-e", key, value]);
    }
    words.push(INSTRUMENTATION);
    let output = adb.shell(&words)?;
    if !output.contains("gesture=injected") {
        return Err(DeviceError::Injection(output.trim().to_owned()));
    }
    println!("injected {}", args.join(" "));
    Ok(())
}

const INPUT_USAGE: &str = "cargo device-input pen x,y,... [pressure] | finger x,y[,...] \
                           [x,y[,...] [ms]] | hover x,y [ms]";

/// The instrumentation's `-e` arguments for a `device-input` command line.
fn extras<'a>(args: &[&'a str]) -> Result<Vec<(&'static str, &'a str)>> {
    let usage = DeviceError::Usage(INPUT_USAGE);
    // Only digits, points, commas and a sign reach the device's shell.
    if !args[1.min(args.len())..].iter().all(|arg| {
        arg.bytes()
            .all(|b| b.is_ascii_digit() || b".,-".contains(&b))
    }) {
        return Err(usage);
    }
    Ok(match args {
        ["pen", path] => vec![("tool", "stylus"), ("path", path)],
        ["pen", path, pressure] => vec![("tool", "stylus"), ("pressure", pressure), ("path", path)],
        ["finger", from] => vec![("from", from)],
        ["finger", from, to] => vec![("from", from), ("to", to)],
        ["finger", from, to, ms] => vec![("from", from), ("to", to), ("ms", ms)],
        ["hover", at] => vec![("hover", at)],
        ["hover", at, ms] => vec![("hover", at), ("ms", ms)],
        _ => return Err(usage),
    })
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
                ("tool", "stylus"),
                ("pressure", "0.8"),
                ("path", "100,200,300,400")
            ]
        );
        let fingers = extras(&["finger", "1,2,3,4", "0,2,5,4", "200"]).expect("fingers");
        assert_eq!(fingers[2], ("ms", "200"));
        assert!(extras(&["pen", "1,2;rm -rf /"]).is_err());
        assert!(extras(&["walk", "1,2"]).is_err());
        assert!(extras(&[]).is_err());
    }
}

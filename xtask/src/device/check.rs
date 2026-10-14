//! `cargo device-check [label] [--no-install] [--no-launch] [--fresh] [--after=<ms>]`: wakes the
//! tablet, builds and installs the debug APK (unless `--no-install`), launches the app (unless
//! `--no-launch`, which keeps a system picker over it open; with `--fresh` force-stopped first, so
//! it starts anew, S54), waits (`--after`, 3000 ms at first), writes a screenshot to
//! `artifacts/device/<label>.png` and prints the app's error, panic and crash lines logged since;
//! a panic or crash fails it. `cargo device-input` is `input.rs`.

use std::time::Duration;

use super::adb::{Adb, DeviceError, PACKAGE, Result, write_file};
use super::{artifacts, plain};
use crate::android::{ACTIVITY, APK};
use crate::process::root;

/// How long the app gets after launching before the screenshot.
const SETTLE: Duration = Duration::from_secs(3);
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

const USAGE: &str =
    "cargo device-check [label] [--no-install] [--no-launch] [--fresh] [--after=<ms>]";

pub fn check(args: &[String]) -> Result<()> {
    let flag = |name: &str| args.iter().any(|arg| arg == name);
    // `--no-launch`: the screen as it is (a system picker over the app stays open).
    let (install, launch) = (!flag("--no-install"), !flag("--no-launch"));
    let after = match args.iter().find_map(|arg| arg.strip_prefix("--after=")) {
        Some(ms) => Duration::from_millis(ms.parse().map_err(|_| DeviceError::Usage(USAGE))?),
        None => SETTLE,
    };
    let labels: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    let label = match labels[..] {
        [] => "check",
        [label] => plain(label)?,
        _ => return Err(DeviceError::Usage(USAGE)),
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
    } else if flag("--fresh") {
        super::stop_app(&adb)?;
    }
    let since = adb.shell(&["date", "+%s"])?;
    adb.shell(&["am", "start", "-W", "-n", ACTIVITY])?;
    std::thread::sleep(after);
    screenshot(&adb, label)?;
    report_log(&adb, since.trim())
}

/// The screen now as `artifacts/device/<label>.png`.
pub(super) fn screenshot(adb: &Adb, label: &str) -> Result<()> {
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
    let (pid, lines) = app_errors(adb, since)?;
    for line in &lines {
        println!("  {line}");
    }
    let fatal = fatal(&lines);
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

/// The app's pid if it runs, and its error and fatal lines logged since `since` (device
/// seconds): those of its process, or naming it.
pub(super) fn app_errors(adb: &Adb, since: &str) -> Result<(Option<String>, Vec<String>)> {
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
    let lines = log.lines().filter(ours).map(str::to_owned).collect();
    Ok((pid, lines))
}

/// The lines among `lines` that tell of a panic or a crash.
pub(super) fn fatal(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| {
            line.contains("panic:") || line.contains("FATAL") || line.contains("Fatal signal")
        })
        .cloned()
        .collect()
}

/// The screen on and, where no lock is set, the keyguard away: input reaches only a window.
pub(super) fn wake(adb: &Adb) -> Result<()> {
    adb.shell(&["input", "keyevent", "KEYCODE_WAKEUP"])?;
    // Fails harmlessly on a secure lock, which the author unlocks.
    let _ = adb.shell(&["wm", "dismiss-keyguard"]);
    Ok(())
}

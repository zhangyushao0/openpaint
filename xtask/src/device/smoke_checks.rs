//! What the device smoke checks on the device and where it draws: the build installed, the
//! system's window in front, the shared PNG, the driver's harmless lines, its pen paths.

use std::time::{Duration, Instant};

use super::adb::{Adb, DeviceError, PACKAGE};

/// Pen points along a stroke at most this far apart, in screen pixels (one every 8 ms).
pub(super) const SPACING: f32 = 24.0;
/// How long a step waits for the app or the system to get somewhere.
pub(super) const WAIT: Duration = Duration::from_secs(8);

/// "dev.openpaint <versionName> (version code <n>)" as the package manager has it installed.
pub(super) fn installed(adb: &Adb) -> Result<String, DeviceError> {
    let dump = adb.shell(&["dumpsys", "package", PACKAGE])?;
    let field = |key: &str| {
        dump.lines()
            .find_map(|line| {
                line.split_whitespace()
                    .find_map(|word| word.strip_prefix(key))
            })
            .unwrap_or("?")
            .to_owned()
    };
    let (name, code) = (field("versionName="), field("versionCode="));
    Ok(format!("{PACKAGE} {name} (version code {code})"))
}

/// A smoke step's failure, saying why.
pub(super) fn failed(why: impl Into<String>) -> DeviceError {
    DeviceError::Failed {
        command: "device-smoke".into(),
        output: why.into(),
    }
}

/// Waits until the window with the input focus is one `done` accepts, at most `WAIT`.
pub(super) fn focus_until(adb: &Adb, done: impl Fn(&str) -> bool) -> Result<(), DeviceError> {
    let started = Instant::now();
    loop {
        let dump = adb.shell_within(&["dumpsys", "window"], WAIT)?;
        let focus = dump
            .lines()
            .find_map(|line| line.trim().strip_prefix("mCurrentFocus="))
            .unwrap_or("none");
        if done(focus) {
            return Ok(());
        }
        if started.elapsed() >= WAIT {
            return Err(failed(format!("the focus stayed on {focus}")));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The app's share cache (`cache/share`, only the newest share kept) holds one PNG: the canvas
/// at `fit` at 1x, 2x or 4x.
pub(super) fn shared(adb: &Adb, fit: [f32; 4]) -> Result<(), DeviceError> {
    let listed = adb.shell(&["run-as", PACKAGE, "ls", "cache/share"])?;
    let names: Vec<&str> = listed.lines().map(str::trim).collect();
    let [name] = names[..] else {
        return Err(failed(format!("the share cache holds {names:?}")));
    };
    if !name.ends_with(".png") || name.contains('\'') {
        return Err(failed(format!("the share cache holds {name}")));
    }
    // The file's first 24 bytes as decimal numbers: text, as every other answer.
    let path = format!("'cache/share/{name}'");
    let words = [
        "run-as", PACKAGE, "od", "-A", "n", "-t", "u1", "-N", "24", &path,
    ];
    let dump = adb.shell(&words)?;
    let head: Vec<u8> = dump
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    let canvas = [fit[2], fit[3]].map(|side| side.round() as u32);
    match png_size(&head) {
        Some(size) if [1, 2, 4].iter().any(|k| size == canvas.map(|s| s * k)) => Ok(()),
        size => Err(failed(format!(
            "{name}: {size:?}, the canvas {canvas:?}; od said {}",
            dump.trim()
        ))),
    }
}

/// `at`, a fraction of the canvas's `fit` (left, top, width, height), in screen pixels.
pub(super) fn point([left, top, width, height]: [f32; 4], [fx, fy]: [f32; 2]) -> [f32; 2] {
    [left + fx * width, top + fy * height]
}

/// The pen path through `waypoints` (fractions of `fit`), points at most `SPACING` apart, as
/// `device-input pen` takes it.
pub(super) fn path(fit: [f32; 4], waypoints: &[[f32; 2]]) -> String {
    let mut points = vec![point(fit, waypoints[0])];
    for pair in waypoints.windows(2) {
        let ([ax, ay], [bx, by]) = (point(fit, pair[0]), point(fit, pair[1]));
        let steps = ((bx - ax).hypot(by - ay) / SPACING).ceil().max(1.0) as usize;
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            points.push([ax + t * (bx - ax), ay + t * (by - ay)]);
        }
    }
    let words: Vec<String> = points
        .iter()
        .map(|[x, y]| format!("{},{}", x.round(), y.round()))
        .collect();
    words.join(",")
}

/// The Adreno driver's harmless lines at each start: a 4 x 4 buffer of a format it lacks
/// (`Gralloc4`, `GraphicBufferAllocator`, `AHardwareBuffer`).
pub(super) fn gralloc(line: &str) -> bool {
    let tag = line.split_whitespace().nth(5).unwrap_or("");
    matches!(
        tag,
        "Gralloc4:" | "GraphicBufferAllocator:" | "AHardwareBuffer:"
    )
}

/// Width and height from a PNG's signature and header chunk.
pub(super) fn png_size(head: &[u8]) -> Option<[u32; 2]> {
    let signature = head.get(..8)? == b"\x89PNG\r\n\x1a\n" && head.get(12..16)? == b"IHDR";
    let side = |at: usize| Some(u32::from_be_bytes(head.get(at..at + 4)?.try_into().ok()?));
    signature.then_some([side(16)?, side(20)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_pngs_and_the_drivers_lines() {
        let fit = [0.0, 84.0, 2880.0, 1716.0];
        assert_eq!(point(fit, [0.5, 0.5]), [1440.0, 942.0]);
        assert_eq!(path(fit, &[[0.0, 0.0], [0.01, 0.0]]), "0,84,14,84,29,84");
        let numbers: Vec<f32> = path(fit, &[[0.25, 0.4], [0.33, 0.3], [0.41, 0.42]])
            .split(',')
            .map(|n| n.parse().expect("a number"))
            .collect();
        let points: Vec<&[f32]> = numbers.chunks(2).collect();
        let apart = |p: &[&[f32]]| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]);
        assert!(points.len() > 10 && points.windows(2).all(|p| apart(p) <= SPACING + 1.0));
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend(2880u32.to_be_bytes());
        png.extend(1716u32.to_be_bytes());
        assert_eq!(png_size(&png), Some([2880, 1716]));
        assert_eq!(png_size(&png[..20]), None);
        assert_eq!(png_size(b"GIF89a"), None);
        let line = "10-12 04:02:30.341  5698  5698 E Gralloc4: isSupported(1, 59, ...) failed";
        assert!(gralloc(line));
        assert!(!gralloc(
            "10-12 04:02:30.341  5698  5698 E openpaint: saving failed"
        ));
    }
}

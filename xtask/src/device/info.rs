//! `cargo device-info [label]`: what the tablet's display does with colour, read and never
//! changed, how much memory the app holds, its input devices and the dispatcher's state (S38),
//! and how often its canvas reached the glass lately.
//! Read-only queries only (`getprop`, `dumpsys`, the app's own log), each bounded in time
//! (`QUERY_LIMIT`); each answer goes whole to `artifacts/device/<label>-<query>.txt` and the
//! lines about colour are printed.

use std::time::Duration;

use super::adb::{Adb, DeviceError, PACKAGE, Result, write_file};
use super::{artifacts, plain};

/// How long one query may take before it is stopped. On 2026-10-04 (S30) `dumpsys SurfaceFlinger
/// --list` once never returned and the command waited until the caller gave up after 5 min;
/// every query is now bounded here, and `dumpsys` gives each service 10 s (`-t 10`).
const QUERY_LIMIT: Duration = Duration::from_secs(30);

/// The queries: a file name and the device command, each one read-only.
const QUERIES: [(&str, &[&str]); 9] = [
    ("props", &["getprop"]),
    ("surfaceflinger", &["dumpsys", "-t", "10", "SurfaceFlinger"]),
    ("display", &["dumpsys", "-t", "10", "display"]),
    ("color-display", &["dumpsys", "-t", "10", "color_display"]),
    // The app's own lines: the surface's format and colour space as wgpu reported them.
    ("app-log", &["logcat", "-d", "-s", "openpaint:I"]),
    // The app's memory now (S37): its graphics (GPU textures and buffers) and the rest.
    ("meminfo", &["dumpsys", "-t", "10", "meminfo", PACKAGE]),
    // The GPU memory each process holds as the driver counts it (S51): meminfo's Graphics
    // leaves out GPU memory the CPU never maps.
    ("gpumem", &["dumpsys", "-t", "10", "gpu", "--gpumem"]),
    // Input devices and the dispatcher's state (S38): why injected fingers were refused.
    ("input", &["dumpsys", "-t", "10", "input"]),
    (
        "input-log",
        &[
            "logcat",
            "-d",
            "-s",
            "InputDispatcher:*",
            "InputManager:*",
            "UiAutomation:*",
        ],
    ),
];

/// Words that mark a line about colour, matched without regard to case.
const COLOUR_WORDS: [&str; 9] = [
    "color",
    "colour",
    "dataspace",
    "gamut",
    "p3",
    "srgb",
    "wide",
    "hdr",
    "surface attached",
];

pub fn info(args: &[String]) -> Result<()> {
    let label = match args {
        [] => "info",
        [label] => plain(label)?,
        _ => return Err(DeviceError::Usage("cargo device-info [label]")),
    };
    let adb = Adb::connected()?;
    for (name, words) in QUERIES {
        let output = adb.shell_within(words, QUERY_LIMIT)?;
        let path = artifacts().join(format!("{label}-{name}.txt"));
        write_file(&path, output.as_bytes())?;
        let lines: Vec<&str> = output.lines().filter(|line| about_colour(line)).collect();
        println!(
            "{name}: {} lines, {} about colour",
            output.lines().count(),
            lines.len()
        );
        for line in lines.iter().take(LINES_SHOWN) {
            println!("  {}", line.trim());
        }
        println!("  (whole answer in {})", path.display());
    }
    presents(&adb, label)
}

/// The app's canvas layer as SurfaceFlinger names it, before its id.
const CANVAS_LAYER: &str = "SurfaceView[dev.openpaint/dev.openpaint.MainActivity](BLAST)";

/// How often the app's canvas reached the glass lately: SurfaceFlinger's present times of the
/// canvas layer's last frames (`--latency`, at most 128), as the median interval and its rate.
fn presents(adb: &Adb, label: &str) -> Result<()> {
    let layers = adb.shell_within(
        &["dumpsys", "-t", "10", "SurfaceFlinger", "--list"],
        QUERY_LIMIT,
    )?;
    write_file(
        &artifacts().join(format!("{label}-layers.txt")),
        layers.as_bytes(),
    )?;
    let Some(layer) = layers.lines().find_map(canvas_layer) else {
        println!("presents: the app's canvas layer is not on screen");
        return Ok(());
    };
    // The device's shell reads the line: the name in single quotes (it holds brackets).
    let output = adb.shell_within(
        &[
            "dumpsys",
            "-t",
            "10",
            "SurfaceFlinger",
            "--latency",
            &format!("'{layer}'"),
        ],
        QUERY_LIMIT,
    )?;
    let path = artifacts().join(format!("{label}-presents.txt"));
    write_file(&path, output.as_bytes())?;
    let mut lines = output.lines();
    let period: f64 = lines
        .next()
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or(0.0);
    let times: Vec<i64> = lines
        .filter_map(|line| line.split_whitespace().nth(1)?.parse().ok())
        .filter(|&time| time > 0 && time < i64::MAX)
        .collect();
    let mut gaps: Vec<i64> = times.windows(2).map(|pair| pair[1] - pair[0]).collect();
    gaps.sort_unstable();
    println!(
        "presents: display vsync {:.2} ms; {} frames of {layer}",
        period / 1e6,
        times.len()
    );
    if let Some(&median) = gaps.get(gaps.len() / 2) {
        let ms = median as f64 / 1e6;
        println!(
            "  median interval {ms:.2} ms ({:.1} Hz), shortest {:.2} ms, longest {:.2} ms",
            1000.0 / ms,
            gaps[0] as f64 / 1e6,
            gaps[gaps.len() - 1] as f64 / 1e6
        );
    }
    println!("  (whole answer in {})", path.display());
    Ok(())
}

/// The canvas layer's full name in a `--list` line, which reads
/// `RequestedLayerState{<name> parentId=...}` on Android 15 and `<name>` before.
fn canvas_layer(line: &str) -> Option<&str> {
    let line = line.trim();
    let line = line.strip_prefix("RequestedLayerState{").unwrap_or(line);
    let name = line.split([' ', '}']).next()?;
    (name.starts_with(CANVAS_LAYER) && !name.contains('\'')).then_some(name)
}

/// At most this many colour lines are printed per query; the file has every line.
const LINES_SHOWN: usize = 40;

fn about_colour(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    COLOUR_WORDS.iter().any(|word| line.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_reading_commands_and_colour_lines() {
        for (_, words) in QUERIES {
            assert!(matches!(words[0], "getprop" | "dumpsys" | "logcat"));
            assert!(
                !words
                    .iter()
                    .any(|word| word.contains(';') || *word == "set")
            );
        }
        assert!(about_colour(
            "  ro.surface_flinger.has_wide_color_display: [true]"
        ));
        assert!(about_colour("dataSpace=BT709 sRGB Full range"));
        assert!(!about_colour("mBrightness=0.5"));
        let name = "SurfaceView[dev.openpaint/dev.openpaint.MainActivity](BLAST)#70532";
        let listed = format!("RequestedLayerState{{{name} parentId=70531}}");
        assert_eq!(canvas_layer(&listed), Some(name));
        assert_eq!(canvas_layer(name), Some(name));
        assert_eq!(
            canvas_layer("Background for SurfaceView[dev.openpaint/x]#1"),
            None
        );
    }
}

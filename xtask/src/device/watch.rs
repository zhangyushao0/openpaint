//! `cargo device-info --watch <seconds> [label]`: the tablet's clocks, the GPU's load, the
//! battery's draw and the app's CPU time over `seconds`, sampled every `EVERY` by one read-only
//! loop in the device's shell, so it runs beside a benchmark (`device-input bench`) or while the
//! app rests. Files it cannot read are left out. The samples go whole to
//! `artifacts/device/<label>-watch.txt`; a summary is printed.

use std::collections::BTreeMap;
use std::time::Duration;

use super::adb::{Adb, DeviceError, PACKAGE, Result, write_file};
use super::{artifacts, plain};

const USAGE: &str = "cargo device-info --watch <seconds> [label]";
/// The sampled files: every CPU policy's clock, the GPU's clock and load, the battery's current
/// (µA) and voltage (µV).
const FILES: &str = "/sys/devices/system/cpu/cpufreq/policy*/scaling_cur_freq \
                     /sys/class/kgsl/kgsl-3d0/gpuclk /sys/class/kgsl/kgsl-3d0/gpu_busy_percentage \
                     /sys/class/kgsl/kgsl-3d0/devfreq/cur_freq /sys/kernel/gpu/gpu_clock \
                     /sys/kernel/gpu/gpu_busy /sys/class/devfreq/*/cur_freq \
                     /sys/class/power_supply/*/current_now /sys/class/power_supply/*/voltage_now";
/// Between samples, in the device shell's `sleep`.
const EVERY: &str = "0.2";
/// The device's clock ticks a second (`/proc/<pid>/stat`'s times).
const TICKS: f64 = 100.0;

pub fn watch(args: &[String]) -> Result<()> {
    let (seconds, label) = match args {
        [seconds] => (seconds, "watch"),
        [seconds, label] => (seconds, plain(label)?),
        _ => return Err(DeviceError::Usage(USAGE)),
    };
    let seconds: u64 = seconds.parse().map_err(|_| DeviceError::Usage(USAGE))?;
    let adb = Adb::connected()?;
    // The app's process and main thread (its tid is the pid) read before and after the loop.
    let stat = format!("p=$(pidof {PACKAGE}); cat /proc/$p/stat /proc/$p/task/$p/stat 2>/dev/null");
    // The battery's charge counter before and after (the shell reads no current).
    let battery = "dumpsys -t 10 battery | grep -E 'Charge counter|voltage|powered'";
    let script = format!(
        "{stat}; {battery}; end=$(($(date +%s) + {seconds})); while [ $(date +%s) -lt $end ]; \
         do grep -H . {FILES} 2>/dev/null; echo tick; sleep {EVERY}; done; {stat}; {battery}"
    );
    let limit = Duration::from_secs(seconds + 30);
    let output = adb.shell_within(&[&script], limit)?;
    let path = artifacts().join(format!("{label}-watch.txt"));
    write_file(&path, output.as_bytes())?;
    for line in summary(&output, seconds) {
        println!("{line}");
    }
    println!("  (every sample in {})", path.display());
    Ok(())
}

/// The summary lines of a watch over `seconds`.
fn summary(output: &str, seconds: u64) -> Vec<String> {
    let mut series: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut stats = Vec::new();
    let mut ticks = 0;
    let powered = output.contains("powered: true");
    for line in output.lines().map(str::trim) {
        if line == "tick" {
            ticks += 1;
        } else if let Some((path, value)) = line.split_once(':')
            && let Ok(value) = value.trim().parse::<i64>()
        {
            series.entry(path.to_owned()).or_default().push(value);
        } else if let Some(cpu) = cpu_ticks(line) {
            stats.push(cpu);
        }
    }
    let mut lines = vec![format!("watch: {ticks} samples over {seconds} s")];
    for (path, values) in series.iter().filter(|(path, _)| path.starts_with("/sys/")) {
        let name = path
            .trim_start_matches("/sys/")
            .replace("/scaling_cur_freq", "");
        let (unit, scale) = match path {
            p if p.ends_with("scaling_cur_freq") => ("MHz", 1e-3),
            p if p.ends_with("gpuclk") => ("MHz", 1e-6),
            p if p.ends_with("current_now") => ("mA", 1e-3),
            p if p.ends_with("voltage_now") => ("mV", 1e-3),
            _ => ("", 1.0),
        };
        lines.push(format!("  {name}: {} {unit}", spread(values, scale)));
    }
    // The charge counter (µAh) and voltage (mV) before and after: the mean draw between.
    if let (Some([before, .., after]), Some([volts, ..])) = (
        series.get("Charge counter").map(Vec::as_slice),
        series.get("voltage").map(Vec::as_slice),
    ) {
        let milliamps = (before - after) as f64 * 3600.0 / 1000.0 / seconds.max(1) as f64;
        lines.push(format!(
            "  battery: charge counter {before} -> {after} µAh, {milliamps:.0} mA at {volts} mV \
             ({:.0} mW){}",
            milliamps * *volts as f64 / 1000.0,
            if powered { ", on external power" } else { "" }
        ));
    }
    // Before and after: the process's, then the main thread's.
    if let [process, main, process_end, main_end] = stats[..] {
        let per_second = |ticks: u64| ticks as f64 / TICKS * 1e3 / seconds.max(1) as f64;
        lines.push(format!(
            "  the app's CPU {:.0} ms a second (its main thread {:.0})",
            per_second(process_end.saturating_sub(process)),
            per_second(main_end.saturating_sub(main))
        ));
    }
    lines
}

/// A `/proc/<pid>/stat` line's user and system time, in clock ticks.
fn cpu_ticks(line: &str) -> Option<u64> {
    let after = line.rsplit_once(") ")?.1;
    let fields: Vec<&str> = after.split_whitespace().collect();
    // After the name: state is field 3, utime 14 and stime 15 of the whole line.
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime + stime)
}

/// "median (min to max)" of `values` times `scale`.
fn spread(values: &[i64], scale: f64) -> String {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let at = |i: usize| sorted[i] as f64 * scale;
    match sorted.len() {
        0 => "-".to_owned(),
        n => format!(
            "median {:.0} ({:.0} to {:.0}), mean {:.0}",
            at(n / 2),
            at(0),
            at(n - 1),
            sorted.iter().sum::<i64>() as f64 * scale / n as f64
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_summarise_by_file_and_the_app_by_its_stat_lines() {
        let stat = |ticks: u64| format!("123 (dev.openpaint) S 1 2 3 4 5 6 7 8 9 10 {ticks} 0 0");
        let battery = |counter: i64| {
            format!("  USB powered: true\n  voltage: 4000\n  Charge counter: {counter}")
        };
        let output = format!(
            "{}\n{}\n{}\n/sys/devices/system/cpu/cpufreq/policy7/scaling_cur_freq:2000000\n\
             /sys/class/kgsl/kgsl-3d0/gpuclk:500000000\ntick\n{}\n{}\n{}\n",
            stat(100),
            stat(40),
            battery(5000),
            stat(300),
            stat(90),
            battery(4999)
        );
        let lines = summary(&output, 2).join("\n");
        assert!(lines.contains("1 samples over 2 s"), "{lines}");
        assert!(lines.contains("policy7: median 2000"), "{lines}");
        assert!(lines.contains("gpuclk: median 500 "), "{lines}");
        assert!(
            lines.contains("5000 -> 4999 µAh, 2 mA at 4000 mV (7 mW), on external"),
            "{lines}"
        );
        assert!(
            lines.contains("CPU 1000 ms a second (its main thread 250)"),
            "{lines}"
        );
    }
}

//! Memory headroom for the gate's test binaries (S106b): a binary starts only while `HEADROOM`
//! stays free, or alone. Free means commit on Windows (limit minus charge: what a process can
//! still allocate; GPU drivers commit far more than they touch, so physical memory stays free
//! when commit runs out), `MemAvailable` on Linux. Measured on the author's machine (2026-11-06,
//! 47.7 GB of commit, 32 to 37 GB held by other programs): the binaries take up to 8.7 GB, nearly
//! all in the first GPU binaries over their first 20 s, the same on 6 workers as on 12. The check
//! cannot hold that back; it keeps the many small binaries after them out of the last
//! `HEADROOM`, and holds the step back when something beside the gate takes the memory.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// The memory kept free: below it a binary starts only when none other runs.
pub const HEADROOM: u64 = 4 << 30;

/// How often a waiting binary looks again.
const RECHECK: Duration = Duration::from_millis(250);

/// Windows' free commit in KB, a line every half second while the process `gate` runs, for at
/// most an hour. It must watch the gate itself: writes to a closed pipe do not stop it.
fn sampler(gate: u32) -> String {
    format!(
        "$end = (Get-Date).AddHours(1); \
         while ((Get-Date) -lt $end -and \
         (Get-Process -Id {gate} -ErrorAction SilentlyContinue)) {{ \
         [Console]::Out.WriteLine((Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory); \
         [Console]::Out.Flush(); Start-Sleep -Milliseconds 500 }}"
    )
}

/// Where free memory is read from.
enum Reading {
    /// Windows' sampler, its lines noted as they come; killed when the step ends.
    Sampled(Option<Child>),
    /// /proc/meminfo, read at each look.
    Meminfo,
    /// Why there is no reading.
    Unread(String),
}

/// Bytes free at the latest reading (0 before the first), and the least read.
struct Readings {
    latest: AtomicU64,
    least: AtomicU64,
}

impl Readings {
    fn note(&self, free: u64) {
        self.latest.store(free, Ordering::Relaxed);
        self.least.fetch_min(free, Ordering::Relaxed);
    }
}

/// Free memory as the test step goes, and the starts that waited for it.
pub struct Memory {
    reading: Reading,
    readings: Arc<Readings>,
    before: u64,
    waits: AtomicUsize,
    waited_ms: AtomicU64,
}

impl Memory {
    /// Starts reading, and waits up to five seconds for the first reading.
    pub fn watch() -> Memory {
        let mut memory = Memory::new(Reading::Meminfo);
        memory.reading = match std::env::consts::OS {
            "windows" => sample_windows(Arc::clone(&memory.readings)),
            "linux" => Reading::Meminfo,
            os => Reading::Unread(format!("not read on {os}")),
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while !matches!(memory.reading, Reading::Unread(_)) && Instant::now() < deadline {
            if let Some(free) = memory.free() {
                memory.before = free;
                return memory;
            }
            thread::sleep(Duration::from_millis(50));
        }
        memory.stop();
        memory.reading = Reading::Unread("not read within 5 s".to_owned());
        memory
    }

    fn new(reading: Reading) -> Memory {
        Memory {
            reading,
            readings: Arc::new(Readings {
                latest: AtomicU64::new(0),
                least: AtomicU64::new(u64::MAX),
            }),
            before: 0,
            waits: AtomicUsize::new(0),
            waited_ms: AtomicU64::new(0),
        }
    }

    /// Bytes free now, as last read; `None` without a reading.
    fn free(&self) -> Option<u64> {
        match &self.reading {
            Reading::Sampled(_) => {}
            Reading::Meminfo => self.readings.note(meminfo_available()?),
            Reading::Unread(_) => return None,
        }
        let free = self.readings.latest.load(Ordering::Relaxed);
        (free > 0).then_some(free)
    }

    /// Waits until a binary may start: `HEADROOM` free, or none of `running` (the binaries
    /// running) left, so the step always goes on; then counts it in `running`.
    pub fn admit(&self, running: &AtomicUsize) {
        let started = Instant::now();
        let mut waited = false;
        loop {
            match self.free() {
                Some(free) if free < HEADROOM => {
                    let alone = running.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst);
                    if alone.is_ok() {
                        break;
                    }
                }
                _ => {
                    running.fetch_add(1, Ordering::SeqCst);
                    break;
                }
            }
            waited = true;
            thread::sleep(RECHECK);
        }
        if waited {
            self.waits.fetch_add(1, Ordering::Relaxed);
            let ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            self.waited_ms.fetch_add(ms, Ordering::Relaxed);
        }
    }

    /// How much was free before the binaries and at least, and the starts that waited.
    pub fn summary(&self) -> String {
        if let Reading::Unread(why) = &self.reading {
            return format!("free memory {why}, starts unchecked");
        }
        let gb = |bytes: u64| bytes as f64 / f64::from(1 << 30);
        let waits = self.waits.load(Ordering::Relaxed);
        let waited = self.waited_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        format!(
            "free memory {:.1} GB before, {:.1} GB at least; {waits} starts waited {waited:.1}s \
             for {} GB of it",
            gb(self.before),
            gb(self.readings.least.load(Ordering::Relaxed)),
            HEADROOM >> 30
        )
    }

    fn stop(&mut self) {
        if let Reading::Sampled(Some(sampler)) = &mut self.reading {
            // It may have ended by itself; either way it is gone after this.
            let _killed = sampler.kill();
            let _reaped = sampler.wait();
        }
    }
}

/// The reader stops with the step, however the step ends.
impl Drop for Memory {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Windows' reader started: each line it prints, KB free, noted in `readings` as bytes.
fn sample_windows(readings: Arc<Readings>) -> Reading {
    let spawned = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command"])
        .arg(sampler(std::process::id()))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            return Reading::Unread(format!("not read: cannot start powershell: {error}"));
        }
    };
    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Ok(kb) = line.trim().parse::<u64>() {
                    readings.note(kb.saturating_mul(1024));
                }
            }
        });
    }
    Reading::Sampled(Some(child))
}

/// `MemAvailable` from /proc/meminfo, in bytes.
fn meminfo_available() -> Option<u64> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    kb_of(&meminfo, "MemAvailable:").map(|kb| kb.saturating_mul(1024))
}

/// The kB value of the line starting with `key` in /proc/meminfo's form.
fn kb_of(meminfo: &str, key: &str) -> Option<u64> {
    let line = meminfo.lines().find_map(|line| line.strip_prefix(key))?;
    line.trim().strip_suffix("kB")?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(free: u64) -> Memory {
        let memory = Memory::new(Reading::Sampled(None));
        memory.readings.note(free);
        memory
    }

    #[test]
    fn a_binary_starts_with_headroom_free_or_alone() {
        let running = AtomicUsize::new(3);
        memory(HEADROOM).admit(&running);
        assert_eq!(
            running.load(Ordering::SeqCst),
            4,
            "headroom free: it starts"
        );
        let short = memory(HEADROOM - 1);
        let running = AtomicUsize::new(0);
        short.admit(&running);
        assert_eq!(
            running.load(Ordering::SeqCst),
            1,
            "short but alone: it starts"
        );
        let running = Arc::new(AtomicUsize::new(1));
        let ended = Arc::clone(&running);
        let ends = thread::spawn(move || {
            thread::sleep(Duration::from_millis(600));
            ended.fetch_sub(1, Ordering::SeqCst);
        });
        short.admit(&running);
        ends.join().expect("the running binary ends");
        assert_eq!(
            running.load(Ordering::SeqCst),
            1,
            "it started once the other ended"
        );
        let summary = short.summary();
        assert!(summary.contains("1 starts waited"), "{summary}");
        assert!(summary.contains("4.0 GB at least"), "{summary}");
    }

    #[test]
    fn the_windows_sampler_reads_while_the_gate_runs_and_ends_without_it() {
        if std::env::consts::OS != "windows" {
            return; // Only Windows reads through PowerShell.
        }
        let script = |gate| {
            let mut command = Command::new("powershell");
            command.args(["-NoProfile", "-NonInteractive", "-Command"]);
            command.arg(sampler(gate));
            // Long enough for PowerShell to start and read beside a whole gate.
            crate::test_run::guard::run(&mut command, Duration::from_secs(10)).expect("it starts")
        };
        let watching = script(std::process::id());
        assert!(watching.status.is_none(), "it ran until killed");
        let stdout = String::from_utf8_lossy(&watching.stdout);
        let kb: u64 = stdout
            .lines()
            .next()
            .and_then(|l| l.trim().parse().ok())
            .expect("KB");
        assert!(kb > 0, "{stdout}");
        let mut gone = Command::new(std::env::current_exe().expect("the test binary"));
        let mut gone = gone
            .arg("--list")
            .stdout(Stdio::null())
            .spawn()
            .expect("it starts");
        gone.wait().expect("it ends");
        let orphaned = script(gone.id());
        assert!(orphaned.status.is_some(), "it ended by itself");
        assert!(orphaned.stdout.is_empty());
    }

    #[test]
    fn meminfo_values_read_in_kb() {
        let meminfo = "MemTotal:       16384000 kB\nMemAvailable:    8192000 kB\n";
        assert_eq!(kb_of(meminfo, "MemAvailable:"), Some(8_192_000));
        assert_eq!(kb_of(meminfo, "SwapFree:"), None);
    }
}

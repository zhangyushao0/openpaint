//! `nav-timing`'s rows: per phase the frames, their CPU and GPU times (p50 / p95 / max) and the
//! sums of what render counted, and the heaviest frame's counts.

use std::time::Duration;

use openpaint_render::{Background, Counts};

/// One frame: the CPU's part, the GPU's (waited for after it), what render counted, and what
/// the frame before it took for background work (estimated, and measured by GPU timestamps).
pub(super) struct Frame {
    pub(super) cpu: Duration,
    pub(super) gpu: Duration,
    pub(super) counts: Counts,
    pub(super) background: Background,
}

pub(super) fn header() {
    println!(
        "{:<28} {:>6} {:>9} | {:>20} | {:>20} | {:>6} {:>10} {:>5} {:>10} {:>7} {:>6}",
        "phase",
        "frames",
        "total ms",
        "cpu p50/p95/max ms",
        "gpu p50/p95/max ms",
        "disp",
        "entries",
        "plans",
        "binned",
        "derived",
        "filled"
    );
}

/// The row of `phase` over `frames`, and the heaviest frame's counts under it.
pub(super) fn phase(phase: &str, frames: &[Frame]) {
    if frames.is_empty() {
        println!("{phase:<28} {:>6}", 0);
        return;
    }
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let total: f64 = frames.iter().map(|f| ms(f.cpu) + ms(f.gpu)).sum();
    let sum = frames
        .iter()
        .fold(Counts::default(), |a, f| added(a, f.counts));
    println!(
        "{phase:<28} {:>6} {:>9.0} | {} | {} | {:>6} {:>10} {:>5} {:>10} {:>7} {:>6}",
        frames.len(),
        total,
        spread(frames.iter().map(|f| ms(f.cpu)).collect()),
        spread(frames.iter().map(|f| ms(f.gpu)).collect()),
        sum.dispatches,
        sum.entries,
        sum.plans,
        sum.binned,
        sum.derived,
        sum.filled
    );
    let spent: f64 = frames.iter().map(|f| f.background.spent).sum();
    let measured: f64 = frames.iter().filter_map(|f| f.background.measured).sum();
    let ratio = frames.last().map_or(0.0, |f| f.background.ratio);
    println!(
        "{:<28} background: {} ms a frame by the corrected estimate (sum {:.0} ms), measured \
         sum {:.0} ms, correction now {ratio:.3}",
        "",
        spread(frames.iter().map(|f| f.background.spent / 1e6).collect()),
        spent / 1e6,
        measured / 1e6
    );
    let heaviest = frames
        .iter()
        .max_by(|a, b| (a.cpu + a.gpu).cmp(&(b.cpu + b.gpu)));
    if let Some(frame) = heaviest {
        let c = frame.counts;
        println!(
            "{:<28} heaviest frame cpu {:.1} gpu {:.1} ms: {} dispatches, {} entries, {} plans of {} \
             entries, {} derived, {} tier tiles",
            "",
            ms(frame.cpu),
            ms(frame.gpu),
            c.dispatches,
            c.entries,
            c.plans,
            c.binned,
            c.derived,
            c.filled
        );
    }
}

fn added(a: Counts, b: Counts) -> Counts {
    Counts {
        dispatches: a.dispatches + b.dispatches,
        entries: a.entries + b.entries,
        plans: a.plans + b.plans,
        binned: a.binned + b.binned,
        derived: a.derived + b.derived,
        filled: a.filled + b.filled,
    }
}

/// p50 / p95 / max (nearest rank).
fn spread(mut values: Vec<f64>) -> String {
    values.sort_by(f64::total_cmp);
    let rank = |share: f64| values[((values.len() as f64 * share).ceil() as usize).max(1) - 1];
    format!("{:>6.1} {:>6.1} {:>6.1}", rank(0.5), rank(0.95), rank(1.0))
}

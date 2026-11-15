//! The pen latency log line: a stroke's summary in milliseconds and counts, one line.

use std::fmt;

use super::{Painted, Slow, Spread, Summary};

fn ms(ns: i64) -> f64 {
    ns as f64 / 1e6
}

impl fmt::Display for Spread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "p50 {:.1} p95 {:.1} max {:.1} ms",
            ms(self.p50),
            ms(self.p95),
            ms(self.max)
        )
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pen latency, {} events in {} frames ({} skipped for the GPU, {} of them for \
             patience, at most {} in flight; the frame before still in flight at {} of {} \
             calls): event to frame start {}; event to present call {}; frame work {}; GPU waits \
             {}",
            self.events,
            self.frames,
            self.skipped,
            self.patient,
            self.in_flight,
            self.skipped + self.behind,
            self.skipped + self.frames,
            self.to_frame,
            self.to_present,
            self.frame_work,
            self.waited
        )?;
        if let Some(painted) = &self.painted {
            write!(f, "; {painted}")?;
        }
        write!(f, "; slowest")?;
        for Slow { at, ns, cost } in self.slowest {
            write!(
                f,
                " #{at} {:.1} (evaluation {:.1}, tier {:.1}, chains canvas {} tier {}, scene {:.1}, \
                 waits {:.1}",
                ms(ns),
                ms(cost.drawn_ns),
                ms(cost.tier_ns),
                cost.canvas.regions,
                cost.tier.regions,
                ms(cost.scene_ns),
                ms(cost.waited_ns)
            )?;
            if !cost.longest.is_empty() {
                write!(f, " most in {}", cost.longest)?;
            }
            write!(f, ")")?;
        }
        write!(f, " ms")
    }
}

impl fmt::Display for Painted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (evaluation, tier) = (self.evaluation, self.tier_ns);
        let ((one, ones), (more, mores)) = (self.tier_one, self.tier_more);
        write!(
            f,
            "evaluation p50 {:.1} p95 {:.1}, tier p50 {:.1} p95 {:.1} ms (one chain p50 {:.1} \
             p95 {:.1} in {ones} frames, more p50 {:.1} p95 {:.1} in {mores}); chains a frame \
             canvas p50 {} max {} (drawn in {} frames), tier p50 {} max {}; snapshots taken \
             canvas {}, tier {}; strokes laid again canvas {}, tier {}",
            ms(evaluation.p50),
            ms(evaluation.p95),
            ms(tier.p50),
            ms(tier.p95),
            ms(one.p50),
            ms(one.p95),
            ms(more.p50),
            ms(more.p95),
            self.canvas_chains.p50,
            self.canvas_chains.max,
            self.canvas_frames,
            self.tier_chains.p50,
            self.tier_chains.max,
            self.canvas.snapshots,
            self.tier.snapshots,
            self.canvas.laid_again,
            self.tier.laid_again
        )?;
        let mib = |bytes: i64| bytes as f64 / f64::from(1 << 20);
        write!(
            f,
            "; render passes a frame p50 {} max {}, their attachments loaded p50 {:.1} max {:.1} \
             MiB, stored p50 {:.1} max {:.1} MiB",
            self.passes.p50,
            self.passes.max,
            mib(self.loaded.p50),
            mib(self.loaded.max),
            mib(self.stored.p50),
            mib(self.stored.max)
        )?;
        let ([track, ops, plan, encode, writes, submit], frames) = self.one_chain;
        write!(
            f,
            "; a one-chain frame's drawing p50 track {:.2}, ops {:.2}, plan {:.2}, encode {:.2}, \
             writes {:.2}, submit {:.2} ms in {frames} frames; beside the drawing and its waits \
             p50 UI pass {:.1}, the rest {:.1} ms",
            ms(track),
            ms(ops),
            ms(plan),
            ms(encode),
            ms(writes),
            ms(submit),
            ms(self.ui.p50),
            ms(self.rest.p50)
        )
    }
}

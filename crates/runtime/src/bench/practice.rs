//! `bench practice`: a practice sheet's motions as a hand makes them, each measured as the display
//! shows it (`motion.rs`), on the practice sheet open at a line: How's card grown out of its word
//! and gone back; a line drawn with the pen, then Compare (the overlay and its card), the switch to
//! Aligned and back to In place, Compare taken away; Next three times, the view gliding to the
//! next pair; then, the session taken on to its first colour, the inline wheel's square dragged
//! round, Fill frame and Compare (the colours on the wheel). Between motions the frames still
//! wanted, then a pause as the hand moves on. The session moves on to its first colour, filled
//! and compared.

use openpaint_app::{Intent, PracticeIntent};
use openpaint_practice::Family;

use super::Watch;
use super::hand::to_screen;
use super::motion::{by_pen, drag};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;

/// The pen's line across the copy's frame, ms; the wheel dragged round, ms.
const LINE_MS: i64 = 400;
const WHEEL_MS: i64 = 1000;

impl Runtime {
    /// `practice`: see the module.
    pub(crate) fn bench_practice(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let line = (self.projection().practice()).is_some_and(|p| p.family == Some(Family::Line));
        if !line {
            return Err(RuntimeError::NotNow("a practice sheet open at a line"));
        }
        let mut watch = Watch::paced(self);
        self.bench_drawn(&mut watch)?;
        self.bench_settle()?;
        self.bench_rest()?;
        lines.push("frames drawn of the vsyncs wanting one (120 Hz); ms".to_owned());
        for when in ["opens", "closes"] {
            let line = self.bench_tap("How", ToolKind::Finger)?;
            lines.push(format!("instructions {when}: {line}"));
        }
        self.bench_copy()?;
        for (label, control) in [
            ("compare opens", "Compare"),
            ("aligned", "Aligned"),
            ("in place", "In place"),
            ("compare closes", "Compare"),
        ] {
            let line = self.bench_tap(control, ToolKind::Finger)?;
            lines.push(format!("{label}: {line}"));
        }
        for next in 1..=3 {
            let shown = |runtime: &Self| runtime.projection().practice().and_then(|p| p.current);
            let before = shown(self);
            let line = self.bench_tap("Next", ToolKind::Finger)?;
            if shown(self) == before {
                return Err(RuntimeError::NotNow("Next's tap to move on"));
            }
            lines.push(format!("next {next}: {line}"));
        }
        self.bench_colour(lines)?;
        self.bench_rest()
    }

    /// A line drawn by the pen across the shown copy's frame, its instructions closed first.
    fn bench_copy(&mut self) -> Result<(), RuntimeError> {
        let practice = self.projection().practice().cloned();
        let frame = (practice.as_ref())
            .and_then(|p| p.current.map(|index| p.sheet.pairs[index].copy))
            .ok_or(RuntimeError::NotNow("an exercise shown"))?;
        let view = self.bench_view()?;
        let at = |p: [f64; 2]| to_screen(&view, frame.to_canvas(p).map(|v| v as f32));
        let path = [at([0.3, 0.25]), at([0.45, 0.5]), at([0.7, 0.75])];
        self.bench_measured(by_pen(drag(&path, LINE_MS)))?;
        Ok(())
    }

    /// The session taken on to its first colour (its instructions closed), the inline wheel's
    /// square dragged round, Fill frame and Compare: a line each.
    fn bench_colour(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let colour = |runtime: &Self| {
            (runtime.projection().practice()).is_some_and(|p| p.family == Some(Family::Colour))
        };
        let mut watch = Watch::paced(self);
        while !colour(self) {
            let viewport = self.bench_viewport()?;
            let next = Intent::Practice(PracticeIntent::Next(viewport));
            self.bench_intents(vec![next], &mut watch)?;
        }
        let close = Intent::Practice(PracticeIntent::Instructions(false));
        self.bench_intents(vec![close], &mut watch)?;
        self.bench_settle()?;
        self.bench_rest()?;
        let ([left, top], [width, height]) = self.bench_rect("Saturation and value")?;
        let centre = [left + width / 2.0, top + height / 2.0];
        let round: Vec<[f32; 2]> = (0..=32)
            .map(|k| {
                let turn = k as f32 / 32.0 * std::f32::consts::TAU;
                let r = 0.3 * width.min(height);
                [centre[0] + r * turn.cos(), centre[1] + r * turn.sin()]
            })
            .collect();
        let wheel = self.bench_measured(drag(&round, WHEEL_MS))?;
        lines.push(format!("inline wheel dragged: {wheel}"));
        for (label, control) in [("fill frame", "Fill frame"), ("colour compared", "Compare")] {
            let line = self.bench_tap(control, ToolKind::Finger)?;
            lines.push(format!("{label}: {line}"));
        }
        Ok(())
    }

    /// The canvas area's viewport now, which Next moves the view within.
    fn bench_viewport(&self) -> Result<openpaint_app::Viewport, RuntimeError> {
        let area = (self.shell.canvas_area()).ok_or(RuntimeError::NotNow("a canvas area"))?;
        Ok(area.viewport(self.pixels_per_point))
    }
}

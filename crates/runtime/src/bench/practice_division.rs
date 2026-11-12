//! `bench practice` on stage 4's compositions (S95): in a session holding them (the composition
//! pack kept), on to its division: three main lines across the picture, a part's frame with
//! Second and two short lines with Third drawn by the pen, then Compare opened (its lines read,
//! joined and counted at once on the frame loop, the picture laid faint under them) and closed; on
//! to its whole copy: two main lines, then Draw (the division dims: its layer amended), two lines
//! drawn over it, Big (the wheel shows, the colour blocks' layer made), Fill frame and a block
//! painted (S96), Compare opened (the division hidden, the picture over the copy), the slider
//! dragged, Compare closed: a line each; in a session holding value masses instead, those
//! (`practice_masses.rs`).

use openpaint_practice_app::{Pen, PracticeIntent};

use super::Watch;
use super::hand::to_screen;
use super::motion::{by_pen, drag};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;

/// A pen's line across the frame, ms; the slider dragged, ms.
const LINE_MS: i64 = 300;
const SLIDE_MS: i64 = 400;

/// The division's lines by pen, frame units: three main lines across, a part's frame, two short.
const DIVISION: [(Pen, [[f64; 2]; 2]); 8] = [
    (Pen::Main, [[0.08, 0.35], [0.92, 0.3]]),
    (Pen::Main, [[0.08, 0.62], [0.92, 0.66]]),
    (Pen::Main, [[0.5, 0.15], [0.55, 0.85]]),
    (Pen::Second, [[0.2, 0.62], [0.2, 0.42]]),
    (Pen::Second, [[0.2, 0.42], [0.36, 0.42]]),
    (Pen::Second, [[0.36, 0.42], [0.36, 0.62]]),
    (Pen::Third, [[0.62, 0.5], [0.7, 0.45]]),
    (Pen::Third, [[0.65, 0.7], [0.75, 0.74]]),
];

impl Runtime {
    /// See the module.
    pub(super) fn bench_division(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        if !self.bench_on_to("division")? {
            lines.push("no division after this point of the lesson".to_owned());
        } else {
            for (pen, line) in DIVISION {
                self.bench_pen_line(pen, line)?;
            }
            for when in ["opens", "closes"] {
                let line = self.bench_tap("Compare", ToolKind::Finger)?;
                lines.push(format!("division compare {when}: {line}"));
            }
        }
        if !self.bench_on_to("whole-copy")? {
            // Value masses in the sessions without a whole copy (S96).
            return self.bench_masses(lines);
        }
        for (pen, line) in DIVISION.into_iter().take(2) {
            self.bench_pen_line(pen, line)?;
        }
        let line = self.bench_tap("Draw", ToolKind::Finger)?;
        lines.push(format!("whole copy's Draw (the division dims): {line}"));
        for line in [[[0.1, 0.36], [0.9, 0.31]], [[0.1, 0.63], [0.9, 0.67]]] {
            self.bench_pen_line(Pen::Draw, line)?;
        }
        // Its colour blocks (S96): Big chosen, the wheel shows; the frame filled, a block painted.
        let line = self.bench_tap("Big", ToolKind::Finger)?;
        lines.push(format!("whole copy's Big (the wheel shows): {line}"));
        let line = self.bench_tap("Fill frame", ToolKind::Finger)?;
        lines.push(format!("whole copy's Fill frame: {line}"));
        self.bench_pen_line(Pen::Big, [[0.2, 0.7], [0.8, 0.72]])?;
        let line = self.bench_tap("Compare", ToolKind::Finger)?;
        lines.push(format!("whole copy compare opens: {line}"));
        let ([left, top], [width, height]) = self.bench_rect("Original shown")?;
        let middle = top + height / 2.0;
        let slide = [[left + width / 2.0, middle], [left + width - 2.0, middle]];
        let shown = self.bench_measured(drag(&slide, SLIDE_MS))?;
        lines.push(format!("whole copy's original shown dragged: {shown}"));
        let line = self.bench_tap("Compare", ToolKind::Finger)?;
        lines.push(format!("whole copy compare closes: {line}"));
        Ok(())
    }

    /// `pen` chosen (unmeasured, as a hand's choice between lines) and a line drawn with it
    /// across the shown copy's frame from `a` to `b` (frame units).
    pub(super) fn bench_pen_line(
        &mut self,
        pen: Pen,
        [a, b]: [[f64; 2]; 2],
    ) -> Result<(), RuntimeError> {
        let chosen = self.practice().and_then(|p| p.pen.map(|(pen, _)| pen));
        if chosen != Some(pen) {
            let mut watch = Watch::paced(self);
            let choose = PracticeIntent::Pen(pen);
            self.bench_practise(vec![choose], &mut watch)?;
            self.bench_rest()?;
        }
        let practice = self.practice();
        let frame = (practice.as_ref())
            .and_then(|p| p.current.map(|index| p.sheet.pairs[index].copy))
            .ok_or(RuntimeError::NotNow("an exercise shown"))?;
        let view = self.bench_view()?;
        let at = |p: [f64; 2]| to_screen(&view, frame.to_canvas(p).map(|v| v as f32));
        self.bench_measured(by_pen(drag(&[at(a), at(b)], LINE_MS)))?;
        Ok(())
    }
}

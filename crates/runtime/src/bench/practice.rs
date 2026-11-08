//! `bench practice`: a practice sheet's motions as a hand makes them, each measured as the display
//! shows it (`motion.rs`), on the practice sheet open: How's card grown out of its word and gone
//! back; at a line, a line drawn with the pen, then Compare (the overlay and its card), the switch
//! to Aligned and back to In place, Compare taken away, and Compare again with Aligned chosen: the
//! time from the tap to the frame showing the aligned fit (S88); Next three times, the view gliding
//! to the next pair; in a session holding combined lines (stage 1), on to the first group, its
//! lines drawn along the original and compared aligned likewise; in stage 3, its first point,
//! angle and point in space marked and compared (`practice_points.rs`); in stage 4 its division
//! and whole copy drawn and compared (`practice_division.rs`, S95); then, in a session holding
//! single shapes, on to its first shape: an outline drawn, Compare, Aligned (its fit found off
//! the frame loop), In place, Compare taken away and compared aligned likewise; on to its colour
//! combination: Fill frame, a stroke, Compare (its copy read back), another area, the original
//! shown dragged, Compare taken away; on to its perspective: a box's edges or
//! free boxes' drawn, Compare (the edges found and extended, at once on the frame loop) and taken
//! away, an ellipse's loop, Compare and taken away; then, the session taken on to its next colour
//! (if one is left), the inline wheel's square dragged round, Fill frame and Compare (the colours
//! on the wheel). Between motions the frames still wanted, then a pause as the hand moves on.

use openpaint_practice::Family;
use openpaint_practice_app::PracticeIntent;

use super::Watch;
use super::hand::to_screen;
use super::motion::{by_pen, drag, tap};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;

/// The pen's line across the copy's frame, ms; a group's line drawn along its original, ms; the
/// wheel dragged round, ms.
const LINE_MS: i64 = 400;
const GROUP_LINE_MS: i64 = 800;
const WHEEL_MS: i64 = 1000;
/// The families the walk visits, by their keys (`openpaint_practice::Facts::key`): the bench names
/// kinds as a test drill does (S109), what it does at each from what its kind gives.
const LINE: &str = "line";
const GROUP: &str = "combined";
const SHAPE: &str = "shape";
const COMBINATION: &str = "combination";
const COLOUR: &str = "colour";
/// Stage 2's perspective: its key, its name in the lines, and whether it is drawn round (an
/// ellipse) or by its straight edges.
const PERSPECTIVE: [(&str, &str, bool); 3] = [
    ("box", "box", false),
    ("free-boxes", "free boxes", false),
    ("ellipse", "ellipse", true),
];

/// The family keyed `key`.
pub(super) fn of_key(key: &str) -> Result<Family, RuntimeError> {
    Family::of_key(key).ok_or(RuntimeError::NotNow("a practice kind of that key"))
}

impl Runtime {
    /// `practice`: see the module.
    pub(crate) fn bench_practice(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let Some(family) = self.practice().map(|p| p.family) else {
            return Err(RuntimeError::NotNow("a practice sheet open"));
        };
        let mut watch = Watch::paced(self);
        self.bench_drawn(&mut watch)?;
        self.bench_settle()?;
        self.bench_rest()?;
        lines.push("frames drawn of the vsyncs wanting one (120 Hz); ms".to_owned());
        for when in ["opens", "closes"] {
            let line = self.bench_tap("How", ToolKind::Finger)?;
            lines.push(format!("instructions {when}: {line}"));
        }
        if family.is_some_and(|family| family.facts().key == LINE) {
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
            self.bench_fit_shown("line", lines)?;
        } else {
            lines.push("no line shown first: its Compare left out".to_owned());
        }
        for next in 1..=3 {
            let shown = |runtime: &Self| runtime.practice().and_then(|p| p.current);
            let before = shown(self);
            let line = self.bench_tap("Next", ToolKind::Finger)?;
            if shown(self) == before {
                return Err(RuntimeError::NotNow("Next's tap to move on"));
            }
            lines.push(format!("next {next}: {line}"));
        }
        self.bench_group(lines)?;
        self.bench_points(lines)?;
        // Stage 4's division and whole copy after its objects (S95).
        self.bench_division(lines)?;
        // Stage 2's shapes come before its colour work, its perspective after it.
        self.bench_shape(lines)?;
        self.bench_combination(lines)?;
        self.bench_perspective(lines)?;
        self.bench_colour(lines)?;
        self.bench_rest()
    }

    /// In a stage-2 session: on to its perspective, a box to complete or a page of free boxes and
    /// an ellipse, each drawn (edges or a loop) and compared, the comparison then taken away: a
    /// line each.
    fn bench_perspective(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        for (key, name, round) in PERSPECTIVE {
            if !self.bench_on_to(of_key(key)?)? {
                lines.push(format!("no {name} after this point of the session"));
                continue;
            }
            if round {
                self.bench_outline()?;
            } else {
                self.bench_edges()?;
            }
            for when in ["opens", "closes"] {
                let line = self.bench_tap("Compare", ToolKind::Finger)?;
                lines.push(format!("{name} compare {when}: {line}"));
            }
        }
        Ok(())
    }

    /// Next until an exercise of `family` shows, its instructions closed and the hand at rest;
    /// false when none comes after the one shown.
    pub(super) fn bench_on_to(&mut self, family: Family) -> Result<bool, RuntimeError> {
        let shown = |runtime: &Self| runtime.practice().and_then(|p| p.family);
        let holds = self.practice().is_some_and(|p| {
            let from = p.current.unwrap_or(p.sheet.families.len());
            (p.sheet.families.iter().skip(from)).any(|held| *held == family)
        });
        if !holds {
            return Ok(false);
        }
        let mut watch = Watch::paced(self);
        while shown(self) != Some(family) {
            let viewport = self.bench_viewport()?;
            let next = PracticeIntent::Next(viewport);
            self.bench_practise(vec![next], &mut watch)?;
        }
        let close = PracticeIntent::Instructions(false);
        self.bench_practise(vec![close], &mut watch)?;
        self.bench_settle()?;
        self.bench_rest()?;
        Ok(true)
    }

    /// A box's edges drawn by the pen in the shown copy's frame: three verticals and the lines
    /// between their ends, each a stroke.
    fn bench_edges(&mut self) -> Result<(), RuntimeError> {
        let practice = self.practice();
        let frame = (practice.as_ref())
            .and_then(|p| p.current.map(|index| p.sheet.pairs[index].copy))
            .ok_or(RuntimeError::NotNow("an exercise shown"))?;
        let view = self.bench_view()?;
        let at = |p: [f64; 2]| to_screen(&view, frame.to_canvas(p).map(|v| v as f32));
        let edges = [
            [[0.3, 0.35], [0.3, 0.7]],
            [[0.5, 0.4], [0.5, 0.8]],
            [[0.7, 0.35], [0.7, 0.7]],
            [[0.3, 0.7], [0.5, 0.8]],
            [[0.5, 0.8], [0.7, 0.7]],
            [[0.3, 0.35], [0.5, 0.4]],
        ];
        for [a, b] in edges {
            self.bench_measured(by_pen(drag(&[at(a), at(b)], LINE_MS / 2)))?;
        }
        Ok(())
    }

    /// In a stage-2 session: on to its colour combination, Fill frame, a stroke of Big across the
    /// copy, Compare (the copy's frame read back, the picture over it), another area chosen, the
    /// original shown dragged to full and Compare taken away: a line each.
    fn bench_combination(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        if !self.bench_on_to(of_key(COMBINATION)?)? {
            lines.push("no colour combination after this point of the session".to_owned());
            return Ok(());
        }
        let line = self.bench_tap("Fill frame", ToolKind::Finger)?;
        lines.push(format!("combination filled: {line}"));
        self.bench_copy()?;
        for (label, control) in [("combination compared", "Compare"), ("area 2", "Area 2")] {
            let line = self.bench_tap(control, ToolKind::Finger)?;
            lines.push(format!("{label}: {line}"));
        }
        let ([left, top], [width, height]) = self.bench_rect("Original shown")?;
        let middle = top + height / 2.0;
        let slide = [[left + width / 2.0, middle], [left + width - 2.0, middle]];
        let shown = self.bench_measured(drag(&slide, LINE_MS))?;
        lines.push(format!("original shown dragged: {shown}"));
        let line = self.bench_tap("Compare", ToolKind::Finger)?;
        lines.push(format!("combination compare closes: {line}"));
        Ok(())
    }

    /// In a session holding single shapes: on to the first, an outline drawn round its copy's
    /// frame, then Compare (the picture and its outline in place), Aligned (fitted off the frame
    /// loop) and In place, Compare taken away: a line each.
    fn bench_shape(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        if !self.bench_on_to(of_key(SHAPE)?)? {
            lines.push("no single shape after this point of the session".to_owned());
            return Ok(());
        }
        self.bench_outline()?;
        for (label, control) in [
            ("shape compare opens", "Compare"),
            ("shape aligned", "Aligned"),
            ("shape in place", "In place"),
            ("shape compare closes", "Compare"),
        ] {
            let line = self.bench_tap(control, ToolKind::Finger)?;
            lines.push(format!("{label}: {line}"));
        }
        self.bench_fit_shown("shape", lines)
    }

    /// In a session holding combined lines: on to the first group, each of its lines drawn by the
    /// pen along its original a little moved and smaller (a stroke of `GROUP_LINE_MS`, the pen's
    /// samples 8 ms apart), then compared aligned (`bench_fit_shown`).
    fn bench_group(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        if !self.bench_on_to(of_key(GROUP)?)? {
            lines.push("no combined lines after this point of the session".to_owned());
            return Ok(());
        }
        let practice = self.practice();
        let shown = (practice.as_ref()).and_then(|p| {
            let index = p.current?;
            let group = p.sheet.originals[index].lines()?.to_vec();
            Some((group, p.sheet.pairs[index].copy))
        });
        let (group, frame) = shown.ok_or(RuntimeError::NotNow("combined lines shown"))?;
        let view = self.bench_view()?;
        let at = |p: &[f64; 2]| {
            let copied = [0.03 + 0.94 * p[0], 0.02 + 0.94 * p[1]];
            to_screen(&view, frame.to_canvas(copied).map(|v| v as f32))
        };
        for line in &group {
            let path: Vec<[f32; 2]> = (line.iter().step_by(16).chain(line.last()))
                .map(at)
                .collect();
            self.bench_measured(by_pen(drag(&path, GROUP_LINE_MS)))?;
        }
        self.bench_fit_shown("group", lines)
    }

    /// Compare tapped with Aligned chosen, as a hand comparing aligned keeps it (the comparison
    /// taken away first): the motion, and when the frame showing the aligned overlay came after
    /// the tap's lift; then In place chosen again and Compare taken away, unmeasured.
    fn bench_fit_shown(&mut self, name: &str, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let mut watch = Watch::paced(self);
        let aligned = PracticeIntent::Aligned(true);
        self.bench_practise(vec![aligned], &mut watch)?;
        self.bench_rest()?;
        let at = self.bench_control("Compare")?;
        let fitted = |runtime: &Self| {
            runtime
                .practice()
                .is_some_and(|p| p.comparing && !p.fitting)
        };
        let motion = self.bench_motion_until(tap(at, ToolKind::Finger), fitted)?;
        self.bench_rest()?;
        let shown =
            (motion.shown).map_or_else(|| "NOT shown".to_owned(), |ms| format!("{ms:.0} ms"));
        let work =
            (self.compare.fitted_ms()).map_or_else(|| "-".to_owned(), |ms| format!("{ms:.1}"));
        lines.push(format!(
            "{name} compared aligned, the fit shown after the tap's lift: {shown} (the fit {work} \
             ms on its worker); {}",
            motion.line()
        ));
        let in_place = PracticeIntent::Aligned(false);
        self.bench_practise(vec![in_place], &mut watch)?;
        self.bench_tap("Compare", ToolKind::Finger)?;
        Ok(())
    }

    /// A loop drawn by the pen round the middle of the shown copy's frame, with a line inside.
    fn bench_outline(&mut self) -> Result<(), RuntimeError> {
        let practice = self.practice();
        let frame = (practice.as_ref())
            .and_then(|p| p.current.map(|index| p.sheet.pairs[index].copy))
            .ok_or(RuntimeError::NotNow("an exercise shown"))?;
        let view = self.bench_view()?;
        let at = |p: [f64; 2]| to_screen(&view, frame.to_canvas(p).map(|v| v as f32));
        let round: Vec<[f32; 2]> = (0..=24)
            .map(|k| {
                let turn = f64::from(k) / 24.0 * std::f64::consts::TAU;
                at([0.5 + 0.3 * turn.cos(), 0.5 + 0.35 * turn.sin()])
            })
            .collect();
        self.bench_measured(by_pen(drag(&round, LINE_MS * 2)))?;
        let fold = [at([0.35, 0.5]), at([0.65, 0.52])];
        self.bench_measured(by_pen(drag(&fold, LINE_MS)))?;
        Ok(())
    }

    /// A line drawn by the pen across the shown copy's frame, its instructions closed first.
    fn bench_copy(&mut self) -> Result<(), RuntimeError> {
        let practice = self.practice();
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
        if !self.bench_on_to(of_key(COLOUR)?)? {
            lines.push("no colour after this point of the session".to_owned());
            return Ok(());
        }
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

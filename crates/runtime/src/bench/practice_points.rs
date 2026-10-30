//! `bench practice` on stage 3 (S90, S93): in a session holding them, on to its first point and
//! its first angle estimated on the flat, its first point in space and its first solid, a mark,
//! lines or the solid drawn by the pen (a cross on the point, a line along the angle, the answer
//! of the point in space a little off, each of the solid's ellipses round once and its sides),
//! then Compare opened (the copy read and the answer drawn, at once on the frame loop) and closed:
//! a line each.

use openpaint_practice::{Family, FlatAnswer, Original, SpaceAnswer};

use super::hand::to_screen;
use super::motion::{by_pen, drag};
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;

/// A stroke of the pen, and round an ellipse, ms.
const STROKE_MS: i64 = 300;
const ROUND_MS: i64 = 900;
/// How far off the answer the bench draws, frame units; an ellipse drawn in pieces.
const OFF: f64 = 0.02;
const PIECES: usize = 48;

impl Runtime {
    /// See the module.
    pub(super) fn bench_points(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let first = |space: bool| {
            (self.projection().practice()).and_then(|p| {
                let from = p.current.unwrap_or(p.sheet.originals.len());
                (p.sheet.originals.iter().skip(from)).find_map(|original| match original {
                    Original::Space(task) if space => Some(task.trick.family()),
                    Original::Solid(task) if !space => Some(task.solid.family()),
                    _ => None,
                })
            })
        };
        let (space, solid) = (first(true), first(false));
        let families = [(Family::Point, "point"), (Family::Angle, "angle")];
        let space = space.map(|family| (family, "point in space"));
        let solid = solid.map(|family| (family, "solid"));
        for (family, name) in families.into_iter().chain(space).chain(solid) {
            if !self.bench_on_to(family)? {
                lines.push(format!("no {name} after this point of the session"));
                continue;
            }
            self.bench_answer()?;
            for when in ["opens", "closes"] {
                let line = self.bench_tap("Compare", ToolKind::Finger)?;
                lines.push(format!("{name} compare {when}: {line}"));
            }
        }
        Ok(())
    }

    /// The shown estimate's, point in space's or solid's answer drawn by the pen a little off: a
    /// cross for a point, a stroke along each line, once round each ellipse.
    fn bench_answer(&mut self) -> Result<(), RuntimeError> {
        let shown = (self.projection().practice()).and_then(|p| {
            let index = p.current?;
            let strokes: Vec<Vec<[f64; 2]>> = match &p.sheet.originals[index] {
                Original::Flat(flat) => match flat.answer() {
                    FlatAnswer::Point(at) => crossed(at),
                    FlatAnswer::Line(line) => vec![line.to_vec()],
                },
                Original::Space(task) => match task.answer() {
                    SpaceAnswer::Point(at) => crossed(at),
                    SpaceAnswer::Lines(lines) => lines.iter().map(|l| l.to_vec()).collect(),
                },
                Original::Solid(task) => (task.answer.ellipses.iter())
                    .map(|e| e.outline(PIECES))
                    .chain(task.answer.sides.iter().map(|side| side.to_vec()))
                    .collect(),
                _ => return None,
            };
            Some((strokes, p.sheet.pairs[index].copy))
        });
        let (strokes, frame) = shown.ok_or(RuntimeError::NotNow("a point shown"))?;
        let view = self.bench_view()?;
        let at =
            |p: [f64; 2]| to_screen(&view, frame.to_canvas([p[0] + OFF, p[1]]).map(|v| v as f32));
        for stroke in strokes {
            let ms = if stroke.len() > 2 {
                ROUND_MS
            } else {
                STROKE_MS
            };
            let path: Vec<[f32; 2]> = stroke.into_iter().map(at).collect();
            self.bench_measured(by_pen(drag(&path, ms)))?;
        }
        Ok(())
    }
}

/// A small cross at `at`, its two strokes.
fn crossed(at: [f64; 2]) -> Vec<Vec<[f64; 2]>> {
    let arm = 0.02;
    vec![
        vec![[at[0] - arm, at[1]], [at[0] + arm, at[1]]],
        vec![[at[0], at[1] - arm], [at[0], at[1] + arm]],
    ]
}

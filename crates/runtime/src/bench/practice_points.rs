//! `bench practice` on stages 3 and 4 (S90, S93, S94): in a session holding them, on to its first
//! point and its first angle estimated on the flat, its first point in space, its first object and
//! its first solid, a mark, lines, the object's visible edges or the solid drawn by the pen (a cross
//! on the point, a line along the angle, the answer of the point in space a little off, each edge
//! and seen curve of the object, each of the solid's ellipses round once and its sides), then
//! Compare opened (the copy read and the answer drawn, at once on the frame loop) and closed: a
//! line each.

use openpaint_practice::Family;

use super::hand::to_screen;
use super::motion::{by_pen, drag};
use super::practice::of_key;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::ToolKind;

/// A stroke of the pen, and round an ellipse, ms.
const STROKE_MS: i64 = 300;
const ROUND_MS: i64 = 900;
/// How far off the answer the bench draws, frame units.
const OFF: f64 = 0.02;
/// The families visited, by their keys: an estimate's two; V大's five tricks, of which the first
/// held is visited; the objects; the three solids, likewise.
const ESTIMATES: [(&str, &str); 2] = [("point", "point"), ("angle", "angle")];
const TRICKS: [&str; 5] = ["centre", "halves", "thirds", "next-face", "mirror"];
const OBJECT: &str = "objects";
const SOLIDS: [&str; 3] = ["cylinder", "cone", "sphere"];

impl Runtime {
    /// See the module.
    pub(super) fn bench_points(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let first = |keys: &[&str]| {
            self.practice().and_then(|p| {
                let from = p.current.unwrap_or(p.sheet.families.len());
                (p.sheet.families.iter().skip(from))
                    .find(|family| keys.contains(&family.facts().key))
                    .copied()
            })
        };
        let (space, solid) = (first(&TRICKS), first(&SOLIDS));
        let mut families: Vec<(Family, &str)> = Vec::new();
        for (key, name) in ESTIMATES {
            families.push((of_key(key)?, name));
        }
        let space = space.map(|family| (family, "point in space"));
        let solid = solid.map(|family| (family, "solid"));
        // Stage 4's objects come before its solid.
        let object = [(of_key(OBJECT)?, "object")];
        let all = families.into_iter().chain(space).chain(object).chain(solid);
        for (family, name) in all {
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

    /// The shown exercise's answer as its kind gives it (`Family::answer`) drawn by the pen a little
    /// off: a cross for a point, a stroke along each line, once round each ellipse.
    fn bench_answer(&mut self) -> Result<(), RuntimeError> {
        let shown = self.practice().and_then(|p| {
            let index = p.current?;
            let strokes = p.sheet.families[index].answer(&p.sheet.originals[index]);
            (!strokes.is_empty()).then(|| (strokes, p.sheet.pairs[index].copy))
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

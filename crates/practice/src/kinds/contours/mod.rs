//! Contours on organic forms (`Kind::Contour`, S156k, Drawabox's organic forms with contour lines,
//! 截面线): how a bending tube turns in space, told by its cross-sections, each an ellipse whose
//! minor axis lies along the spine and whose degree opens where the tube turns towards the eye. A
//! sausage (`sausage.rs`) is shown lit on paper (`picture.rs`); beside it its outline (`outline.rs`)
//! with a tick where each contour goes; the answer is each section's exact ellipse
//! (`sections.rs`), laid over the copy at Compare for the learner's eye to judge (D123). Nothing is
//! rendered ahead and nothing is stored but kind and seed (`task.rs`).
//!
//! A lesson holds one sausage (about 4 minutes); the levels: a gentle bend going back, its spine
//! printed and the near end's contour given; a strong bend turning towards the eye; an S-bend
//! passing in front of itself, no spine, only the half of each section that faces the eye drawn,
//! and one round each end.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod outline;
pub(crate) mod picture;
pub(crate) mod sausage;
pub(crate) mod sections;
pub(crate) mod seen;
pub(crate) mod task;
pub(crate) mod walk;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::course::picker::laid;
use crate::demos::demo::Demo;
use crate::kinds::objects::OBJECT_SIDE;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare, Painted};
use crate::marks::compared::Comparison;
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, arc};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;
use marks::ContourCompared;
use task::{Bend, ContourTask};

pub(crate) struct Contours;

const FACTS: [Facts; 1] = [Facts::of("contours", ToolSet::DRAWING, 240)];
/// An ellipse as a hand draws it once round, in pieces.
const ROUND: usize = 48;

impl Described for Contours {
    fn code(&self) -> u8 {
        34
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Contour(bend) => Some(place(&Bend::ALL, &bend)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        (Bend::ALL.get(usize::from(variant))).map(|bend| Kind::Contour(*bend))
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Contour(bend) = exercise.kind else {
            return held(Blank);
        };
        held(ContourTask::generate(bend, exercise.seed))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        demo::contours()
    }

    /// One sausage, the level's bend.
    fn lesson(&self, level: SkillLevel, _turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let bend = match level {
            SkillLevel::First => Bend::Gentle,
            SkillLevel::Second => Bend::Strong,
            SkillLevel::Third => Bend::Crossing,
        };
        laid([Kind::Contour(bend)])
    }

    fn window(&self) -> usize {
        3
    }

    /// A tube bending up and away, three contour ellipses across it opening as it turns.
    fn glyph(&self) -> Vec<TileMark> {
        let line = |points: &[[f32; 2]]| TileMark::Line(points.to_vec());
        vec![
            line(&[
                [6.0, 24.0],
                [18.0, 24.0],
                [27.0, 20.0],
                [34.0, 13.0],
                [42.0, 6.0],
            ]),
            line(&[
                [6.0, 32.0],
                [19.0, 32.0],
                [30.0, 27.0],
                [38.0, 19.0],
                [47.0, 12.0],
            ]),
            arc([6.0, 28.0], [2.0, 4.0], (90.0, 270.0)),
            arc([44.5, 9.0], [3.9, 3.9], (230.0, 410.0)),
            ring([12.0, 28.0], [4.0, 1.4], 90.0),
            ring([28.0, 23.5], [4.0, 2.0], 60.0),
            ring([39.0, 13.0], [3.8, 2.8], 55.0),
        ]
    }
}

/// The ellipse about `centre` with semi-axes `axes` (major first), its major axis at `angle`
/// degrees (y down), in a tile's units.
fn ring(centre: [f32; 2], [a, b]: [f32; 2], angle: f32) -> TileMark {
    let (sin, cos) = angle.to_radians().sin_cos();
    TileMark::Line(
        (0..=24)
            .map(|k| {
                let t = k as f32 / 24.0 * std::f32::consts::TAU;
                let (x, y) = (a * t.cos(), b * t.sin());
                [centre[0] + x * cos - y * sin, centre[1] + x * sin + y * cos]
            })
            .collect(),
    )
}

/// A sausage to draw the contours of.
impl Original for ContourTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    /// Once anything lies in the copy's frame, the exact sections laid over it for the eye, a
    /// construction to lay with them; the copy's strokes are never read.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        let compared = || ContourCompared { task: self.clone() };
        (asked.held()).then(|| Compare::Checked(Comparison::of(compared())))
    }

    /// The lit sausage, `OBJECT_SIDE` square.
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        Some(([OBJECT_SIDE; 2], self.picture(OBJECT_SIDE)))
    }

    /// Once round each asked ellipse; at 稳 the stretches that face the eye.
    fn answer(&self) -> Vec<Vec<Point>> {
        (self.contours.iter())
            .flat_map(|contour| {
                if self.bend.through() {
                    vec![contour.whole(ROUND)]
                } else {
                    (contour.stretches.iter())
                        .filter(|stretch| !stretch.hidden)
                        .map(|stretch| stretch.points.clone())
                        .collect()
                }
            })
            .collect()
    }
}

//! Objects simplified into boxes (`Kind::Object`, S94): an object's design, body, exact answer,
//! construction, rules and picture, and its How. An object is V大's box simplified, so it builds
//! on the solids (`solids/`): its answer is a solid's kind (`SolidAnswer`) and its check theirs
//! (`SolidCheck::of_object`).
//!
//! A lesson holds one object (about 4.5 minutes): the first level each carve in turn,
//! the second a cut or a top drawn in, slanted planes, the third a round part or a
//! top drawn in, an ellipse or a slant on top of the box. Whether an object has one change or two
//! is its seed's.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod object;
pub(crate) mod object_answer;
pub(crate) mod object_body;
pub(crate) mod object_changes;
pub(crate) mod object_construction;
pub(crate) mod object_design;
pub(crate) mod object_hull;
pub(crate) mod object_picture;
pub(crate) mod object_rays;
pub(crate) mod object_rules;

use crate::course::picker::{laid, turned};
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Described, Facts, Kind, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::Painted;
use crate::marks::compared::{Compared, DrawnCopy};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, arc, boxed};
use crate::maths::path::Point;
use crate::packs::pack::ItemRef;
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;
use object::ObjectTask;
use object_design::Carve;
use std::sync::Arc;

pub(crate) struct Objects;

const FACTS: [Facts; 1] = [Facts::of("objects", ToolSet::DRAWING, 270)];
/// The carves the first level turns through, three a lesson's place (stage 4's drills), of which
/// it lays the first; the carves the second and the third take in turn.
const TURNED: usize = 3;
const SECOND: [Carve; 2] = [Carve::Cut, Carve::Taper];
const THIRD: [Carve; 2] = [Carve::Round, Carve::Taper];
/// The side its picture is drawn at, in pixels: one to a frame's canvas pixel.
pub const OBJECT_SIDE: u32 = 400;

impl Described for Objects {
    fn code(&self) -> u8 {
        14
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Object(carve) => Some(place(&Carve::ALL, &carve)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Carve::ALL
            .get(usize::from(variant))
            .map(|c| Kind::Object(*c))
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Object(carve) = exercise.kind else {
            return held(Blank);
        };
        held(ObjectTask::generate(carve, exercise.seed))
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        demo::object()
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let carves = match level {
            SkillLevel::First => {
                let mut objects = turned((&Carve::ALL, TURNED), turn, Kind::Object);
                objects.truncate(1);
                return objects;
            }
            SkillLevel::Second => SECOND,
            SkillLevel::Third => THIRD,
        };
        laid([Kind::Object(carves[turn.turn % carves.len()])])
    }

    fn window(&self) -> usize {
        3
    }

    /// A low block seen from above, a post standing on its top.
    fn glyph(&self) -> Vec<TileMark> {
        let mut marks = boxed([0.0, 4.0], 0.85);
        let (centre, axes) = ([27.4, 16.6], [5.0, 2.0]);
        let top = [centre[0], 7.0];
        marks.extend([
            arc(centre, axes, (0.0, 180.0)),
            arc(top, axes, (0.0, 360.0)),
            TileMark::Line(vec![
                [centre[0] - axes[0], top[1]],
                [centre[0] - axes[0], centre[1]],
            ]),
            TileMark::Line(vec![
                [centre[0] + axes[0], top[1]],
                [centre[0] + axes[0], centre[1]],
            ]),
        ]);
        marks
    }
}

/// An object to simplify into its box.
impl Original for ObjectTask {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self)
    }

    fn check(&self, copy: &DrawnCopy) -> Option<Arc<dyn Compared>> {
        Some(marks::ObjectCompared::of(self, copy))
    }

    /// Its picture, `OBJECT_SIDE` square (S94).
    fn painted(&self, _picture: Option<&SheetPicture>) -> Option<Painted> {
        Some(([OBJECT_SIDE; 2], ObjectTask::picture(self, OBJECT_SIDE)))
    }

    /// A stroke along each visible edge and seen curve.
    fn answer(&self) -> Vec<Vec<Point>> {
        (self.answer.sides.iter().map(|side| side.to_vec()))
            .chain(
                (self.answer.outlines.iter().flatten())
                    .filter(|stretch| !stretch.hidden)
                    .map(|stretch| stretch.points.clone()),
            )
            .collect()
    }
}

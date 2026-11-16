//! A structure rotation's task (S113, V大's weeks 5 and 6 结构翻转): one of unit 4's carved objects
//! (`objects/`) looked at twice. The picture's view (A) is an object's as unit 4 shows it, keeping
//! every rule, every change showing in it (the learner cannot draw what A hides). The asked view
//! (B) is the same design turned about its upright by quarter turns (`Design::turned`: another
//! corner at the front, never mirrored) and seen by a look of its own; it keeps its box's rules and
//! its edges' (`Rules::Drawn`) and shows at least one change, else whatever it shows. Its box's
//! front corner and edges are printed in the copy's frame; its answer and construction are an
//! object's, exact to the last bits.
//!
//! The levels turn it more (`Turning`): at 入门 a quarter, the eye's height and distance kept, one
//! change, a block or a cut; at 熟练 from another height (from above where A is across or from below,
//! or the reverse; a post, always seen from above, from another band of heights), a quarter or a
//! half, a cut or a top drawn in, one or two changes; at 稳 a round part standing on its base, from
//! another height and turned, only the new box's near upright given and its angle by a small box
//! beside it (`angle.rs`). Two changes pass unit 4's rules about once in a thousand objects laid
//! and an asked view then seldom: 熟练 tries them as its seed asks, 稳 lays the round part.

use crate::geometry::perspective::{PerspectiveBox, Place};
use crate::kinds::objects::object::{Look, ObjectTask, Rules, plain_design};
use crate::kinds::objects::object_design::{Carve, Design, SECOND, designed};
use crate::kinds::rotations::angle::angle_box;
use crate::maths::random::Random;

/// Objects laid before the generator keeps a plain one; asked views tried for each picture's (an
/// asked box out of the frame or its horizon off it is refused before any ray is cast).
const TRIES: usize = 1200;
const ASKED_TRIES: usize = 32;
/// A post's height above the eye's level (over half the box), as unit 4 lays it, and how far apart
/// the two views' are: another band of heights.
const POST_HEIGHTS: [f64; 2] = [0.6, 1.3];
const POST_APART: f64 = 0.3;
/// How often a view of a box below the eye turns to one across it rather than above it.
const TO_ACROSS: f64 = 0.6;

/// How the asked view is turned from the picture's: a level's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Turning {
    /// A quarter about the upright, the eye's height and distance kept (入门).
    Quarter,
    /// From another height, a quarter or a half (熟练).
    Height,
    /// From another height and turned, only the new box's near upright and a small angle box
    /// given (稳).
    Bare,
}

/// A structure rotation's variant: its object's first carve and how it is turned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rotation {
    pub carve: Carve,
    pub turning: Turning,
}

impl Rotation {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 5] = [
        Self::of(Carve::Block, Turning::Quarter),
        Self::of(Carve::Cut, Turning::Quarter),
        Self::of(Carve::Cut, Turning::Height),
        Self::of(Carve::Taper, Turning::Height),
        Self::of(Carve::Round, Turning::Bare),
    ];

    const fn of(carve: Carve, turning: Turning) -> Self {
        Self { carve, turning }
    }
}

/// A structure rotation, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct RotationTask {
    pub rotation: Rotation,
    /// The object as its picture shows it (view A).
    pub shown: ObjectTask,
    /// The object turned, as it is asked drawn (view B): its box given, its exact answer.
    pub asked: ObjectTask,
    /// Quarter turns about the upright from A to B (1 to 3).
    pub quarters: u8,
    /// The small box at B's angle where only B's near upright is given (稳); none otherwise.
    pub angle: Option<PerspectiveBox>,
}

impl RotationTask {
    /// The rotation `rotation` and `seed` make.
    #[must_use]
    pub fn generate(rotation: Rotation, seed: u64) -> Self {
        let mut random = Random::new(seed);
        (0..TRIES)
            .find_map(|_| laid(rotation, &mut random))
            .unwrap_or_else(|| plain(rotation))
    }
}

/// One rotation laid from `random`: a design and its picture's view, then asked views until one
/// keeps its rules; none when none does.
fn laid(rotation: Rotation, random: &mut Random) -> Option<RotationTask> {
    let two = rotation.turning == Turning::Height && random.unit() < SECOND;
    let design = designed(rotation.carve, random, two)?;
    let look = Look::laid(random, design.posted());
    let carve = rotation.carve;
    let (shown, kept) = ObjectTask::seen_by(carve, &design, look, random, Rules::Pictured)?;
    (0..ASKED_TRIES).find_map(|_| {
        let (turned, other, quarters) = turn(rotation.turning, (&design, look, kept), random);
        let (asked, _) = ObjectTask::seen_by(carve, &turned, other, random, Rules::Drawn)?;
        let angle = match rotation.turning {
            Turning::Bare => Some(angle_box(&asked.seen)?),
            Turning::Quarter | Turning::Height => None,
        };
        Some(RotationTask {
            rotation,
            shown: shown.clone(),
            asked,
            quarters,
            angle,
        })
    })
}

/// `design` turned as `turning` asks from the picture's `look` (its middle's height and distance
/// `kept`), laid from `random`: the design turned, the asked view's look and its quarter turns.
fn turn(
    turning: Turning,
    (design, look, kept): (&Design, Look, [f64; 2]),
    random: &mut Random,
) -> (Design, Look, u8) {
    let quarters = match turning {
        Turning::Quarter if random.unit() < 0.5 => 1,
        Turning::Quarter => 3,
        Turning::Height | Turning::Bare => 1 + (random.unit() * 3.0) as u8 % 3,
    };
    let turned = design.turned(quarters);
    let yaw = random.range(25.0, 65.0).to_radians();
    let (place, kept) = match turning {
        Turning::Quarter => (look.place, Some(kept)),
        Turning::Height | Turning::Bare => other_height(design, (look.place, kept), random),
    };
    (turned, Look { yaw, place, kept }, quarters)
}

/// Where the asked view's box lies against the eye's level when the picture's lies at `place` with
/// its middle's height and distance `kept`: below the eye where it is across or above it, across
/// or above where it is below; a post below it still, in the other band of heights at the same
/// distance.
fn other_height(
    design: &Design,
    (place, [height, depth]): (Place, [f64; 2]),
    random: &mut Random,
) -> (Place, Option<[f64; 2]>) {
    if design.posted() {
        let half = design.size[1] / 2.0;
        let [low, high] = POST_HEIGHTS;
        let over = -height - half;
        let other = if over < (low + high) / 2.0 {
            random.range(over + POST_APART, high)
        } else {
            random.range(low, over - POST_APART)
        };
        return (Place::Below, Some([-(half + other), depth]));
    }
    let other = match place {
        Place::Across | Place::Above => Place::Below,
        Place::Below if random.unit() < TO_ACROSS => Place::Across,
        Place::Below => Place::Above,
    };
    (other, None)
}

/// A plain rotation, the generator's last resort (never met by a seed tested): the plain design
/// from a fixed sequence, turned a quarter.
fn plain(rotation: Rotation) -> RotationTask {
    let mut random = Random::new(0x0001_13EC);
    let design = plain_design();
    let carve = rotation.carve;
    let look = Look::laid(&mut random, false);
    let (shown, kept) = ObjectTask::plainly(carve, &design, look, &mut random);
    let (turned, other, quarters) = turn(Turning::Quarter, (&design, look, kept), &mut random);
    let (asked, _) = ObjectTask::plainly(carve, &turned, other, &mut random);
    let angle = (rotation.turning == Turning::Bare)
        .then(|| angle_box(&asked.seen))
        .flatten();
    RotationTask {
        rotation,
        shown,
        asked,
        quarters,
        angle,
    }
}

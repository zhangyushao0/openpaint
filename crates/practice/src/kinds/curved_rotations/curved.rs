//! A curved structure rotation's task (S154f, V大's weeks 7 and 8 曲面翻转): one curved object
//! (`design.rs`) looked at twice. The picture's view (A) lays it one way in its box and keeps
//! every rule (each segment, each joint and what stands on the side seen); the asked view (B) lays
//! it another way (standing where A lies, lying where A stands, or lying along the box's other
//! side) and keeps the rules its answer needs (`view.rs`). B's box is given in the copy's frame as
//! the level says; its answer and construction are exact.
//!
//! The levels are V大's three stages, from precise to estimated (`Handed`): at 入门 two or three
//! segments, standing at B where A lies, B's box, its axis and each joint ticked on it given; at
//! 熟练 three segments and a spout or a handle, lying or standing, the box and the axis given, the
//! joints estimated; at 稳 three or four and a spout or a handle, only the axis, the box's near
//! upright and a small box at B's angle beside it (`rotations/angle.rs`).

use crate::geometry::perspective::PerspectiveBox;
use crate::kinds::curved_rotations::body::Lying;
use crate::kinds::curved_rotations::design::{
    Attached, Attachment, Design, Segment, SegmentKind, designed,
};
use crate::kinds::curved_rotations::view::{CurvedView, Rules};
use crate::kinds::rotations::angle::angle_box;
use crate::maths::random::Random;

/// Designs laid before the generator keeps a plain one; each view's eyes tried for a design.
const TRIES: usize = 600;
const VIEW_TRIES: usize = 24;
/// How often the asked view is seen from above from 熟练 on.
const FROM_ABOVE: f64 = 0.7;

/// What the asked view gives in the copy's frame: a level's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Handed {
    /// The box, the axis and each joint ticked on it (入门).
    Ticked,
    /// The box and the axis (熟练).
    Axis,
    /// The axis, the box's near upright and a small box at its angle (稳).
    Bare,
}

/// A curved structure rotation's variant: how many segments at least and at most, what stands on
/// the side and what the asked view gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CurvedRotation {
    pub segments: [u8; 2],
    pub attached: Attached,
    pub handed: Handed,
}

impl CurvedRotation {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 6] = [
        Self::of([2, 2], Attached::Nothing, Handed::Ticked),
        Self::of([3, 3], Attached::Nothing, Handed::Ticked),
        Self::of([3, 3], Attached::Spout, Handed::Axis),
        Self::of([3, 3], Attached::Handle, Handed::Axis),
        Self::of([3, 4], Attached::Spout, Handed::Bare),
        Self::of([3, 4], Attached::Handle, Handed::Bare),
    ];

    const fn of(segments: [u8; 2], attached: Attached, handed: Handed) -> Self {
        Self {
            segments,
            attached,
            handed,
        }
    }
}

/// A curved structure rotation, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvedTask {
    pub curved: CurvedRotation,
    pub design: Design,
    /// The object as its picture shows it (view A).
    pub shown: CurvedView,
    /// The object laid another way, as it is asked drawn (view B).
    pub asked: CurvedView,
    /// The small box at B's angle where only its near upright is given (稳); none otherwise.
    pub angle: Option<PerspectiveBox>,
}

impl CurvedTask {
    /// The task `curved` and `seed` make.
    #[must_use]
    pub fn generate(curved: CurvedRotation, seed: u64) -> Self {
        let mut random = Random::new(seed);
        (0..TRIES)
            .find_map(|_| laid(curved, &mut random))
            .unwrap_or_else(|| plain(curved))
    }
}

/// One task laid from `random`: a design and its picture's view, then asked views until one keeps
/// its rules; none when none does.
fn laid(curved: CurvedRotation, random: &mut Random) -> Option<CurvedTask> {
    let design = designed(curved.segments, curved.attached, random)?;
    let [a, b] = lyings(curved.handed, random);
    let picture = |random: &mut Random| CurvedView::seen_by(&design, a, random, Rules::Pictured);
    let shown = (0..VIEW_TRIES).find_map(|_| picture(random))?;
    (0..VIEW_TRIES).find_map(|_| {
        let asked = CurvedView::seen_by(&design, b, random, Rules::Drawn)?;
        let angle = match curved.handed {
            Handed::Bare => Some(angle_box(&asked.seen)?),
            Handed::Ticked | Handed::Axis => None,
        };
        Some(CurvedTask {
            curved,
            design: design.clone(),
            shown: shown.clone(),
            asked,
            angle,
        })
    })
}

/// How the two views lay the body, each seen from above or not: at 入门 lying in the picture and
/// standing asked; else one standing and the other lying, either way, or at 稳 lying along the
/// box's one side and the other. The picture is seen from above (its undersides are dark), and so
/// is the asked view at 入门; from 熟练 on it is seen from below now and then.
fn lyings(handed: Handed, random: &mut Random) -> [(Lying, bool); 2] {
    let roll = random.unit();
    let lying = lying(random);
    let standing = upright(random);
    let above = handed == Handed::Ticked || random.unit() < FROM_ABOVE;
    let [a, b] = match handed {
        Handed::Ticked => [lying, standing],
        _ if roll < 0.4 => [lying, standing],
        Handed::Bare if roll > 0.75 => {
            let other = Lying {
                axis: 2 - lying.axis,
                side: if lying.side == 1 { 1 } else { lying.axis },
                ..lying
            };
            [lying, other]
        }
        _ => [standing, lying],
    };
    [(a, true), (b, above)]
}

/// The body lying along the box's width or depth, either way, its side up or along the other.
fn lying(random: &mut Random) -> Lying {
    let axis = if random.unit() < 0.5 { 0 } else { 2 };
    let forward = random.unit() < 0.5;
    let side = if random.unit() < 0.5 { 1 } else { 2 - axis };
    Lying {
        axis,
        forward,
        side,
        out: random.unit() < 0.5,
    }
}

/// The body standing, its side along the box's width or depth, either way.
fn upright(random: &mut Random) -> Lying {
    Lying {
        axis: 1,
        forward: true,
        side: if random.unit() < 0.5 { 0 } else { 2 },
        out: random.unit() < 0.5,
    }
}

/// A plain task, the generator's last resort (never met by a seed tested): a cylinder under a
/// frustum seen from a fixed sequence, whatever rule it breaks.
fn plain(curved: CurvedRotation) -> CurvedTask {
    let mut random = Random::new(0x0001_54F0);
    let segment = |shape, along, wide| Segment { shape, along, wide };
    let design = Design {
        segments: vec![
            segment(SegmentKind::Cylinder, [0.0, 0.5], [1.0, 1.0]),
            segment(SegmentKind::Frustum, [0.5, 1.0], [1.0, 0.5]),
        ],
        attachment: None::<Attachment>,
        proportion: 1.5,
    };
    let [a, b] = lyings(Handed::Ticked, &mut random);
    let shown = CurvedView::built(&design, a, &mut random);
    let asked = CurvedView::built(&design, b, &mut random);
    let angle = (curved.handed == Handed::Bare)
        .then(|| angle_box(&asked.seen))
        .flatten();
    CurvedTask {
        curved,
        design,
        shown,
        asked,
        angle,
    }
}

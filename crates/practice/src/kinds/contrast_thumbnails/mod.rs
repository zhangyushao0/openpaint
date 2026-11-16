//! Contrast thumbnails (`Kind::Thumbnail`, S154, V大's W7-8 抽象对比用于创作: abstract analysis
//! run backwards for one's own picture, in Proko's form of a value thumbnail of three greys): a
//! composition's big shapes printed faint in one frame of its shape, its values withheld, and a
//! ring where the focus is to be (`rings.rs`). The learner decides the contrast that makes it,
//! light against dark at the ring, and a strong, middle and weak rhythm round it in the value
//! masses' three greys (`ToolSet::GREYS`). Compare reads the thumbnail's own pixels and veils it
//! where it is weaker than its strongest, the rules as marks (`marks.rs`); the eye judges.
//!
//! A lesson holds one thumbnail (about 5 minutes): a simple composition with the ring at its own
//! focus, then a medium one with the ring moved off it, then a medium one with a second ring.

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod rings;
pub(crate) mod steps;

use std::sync::Arc;

use crate::course::picker::Picker;
use crate::demos::demo::Demo;
use crate::kinds::{
    Blank, Described, Facts, Kind, Needs, Original, SkillLevel, ToolSet, Turn, held, place,
};
use crate::marks::asked::{Asked, Compare, Reading};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, curve, edge};
use crate::packs::pack::{ItemRef, Tier};
use crate::sheets::exercise::Exercise;
use crate::sheets::sheet_picture::SheetPicture;
use marks::ThumbnailReading;
use rings::{Emphasis, Ring, rings};

pub(crate) struct ContrastThumbnails;

const FACTS: [Facts; 1] = [Facts::of("contrast-thumbnail", ToolSet::GREYS, 300)
    .constructed()
    .needs(Needs::Masses)];

impl Described for ContrastThumbnails {
    fn code(&self) -> u8 {
        31
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Thumbnail(emphasis) => Some(place(&Emphasis::ALL, &emphasis)),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Emphasis::ALL
            .get(usize::from(variant))
            .copied()
            .map(Kind::Thumbnail)
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::Thumbnail(emphasis) = exercise.kind else {
            return held(Blank);
        };
        held(Thumbnailed {
            picture: exercise.item(),
            emphasis,
            seed: exercise.seed,
        })
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::thumbnail())
    }

    /// One composition whose masses read: simple with the ring at its focus, medium with the
    /// ring moved, medium with two rings.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let (tier, kind): (Tier, fn(Tier) -> Kind) = match level {
            SkillLevel::First => (Tier::Simple, |_| Kind::Thumbnail(Emphasis::Own)),
            SkillLevel::Second => (Tier::Medium, |_| Kind::Thumbnail(Emphasis::Moved)),
            SkillLevel::Third => (Tier::Medium, |_| Kind::Thumbnail(Emphasis::Two)),
        };
        let picked = Picker::new(turn, Tier::Medium).composition(tier, kind);
        picked.into_iter().collect()
    }

    fn window(&self) -> usize {
        3
    }

    /// A picture's edge, a hill's line, a dark tree against it in a ring.
    fn glyph(&self) -> Vec<TileMark> {
        let ring = curve(|t| {
            let turn = std::f32::consts::TAU * t;
            [19.0 + 8.0 * turn.cos(), 19.0 + 8.0 * turn.sin()]
        });
        let tree: Vec<[f32; 2]> = (0..8)
            .map(|k| {
                let turn = std::f32::consts::TAU * k as f32 / 8.0;
                [19.0 + 3.5 * turn.cos(), 18.0 + 4.5 * turn.sin()]
            })
            .collect();
        vec![
            edge(),
            TileMark::Line(vec![[9.0, 24.0], [24.0, 21.0], [45.0, 25.0]]),
            TileMark::Tint(tree, 0.8),
            TileMark::Line(ring),
        ]
    }
}

/// A contrast thumbnail's original: the composition whose shapes it prints, how its emphasis is
/// laid and its seed (where its rings go).
#[derive(Debug)]
pub(crate) struct Thumbnailed {
    picture: ItemRef,
    emphasis: Emphasis,
    seed: u64,
}

impl Thumbnailed {
    /// Its rings on `picture`'s masses; none without them.
    fn rings(&self, picture: &SheetPicture) -> Option<Vec<Ring>> {
        let masses = picture.masses.as_ref()?;
        Some(rings(
            &masses.answer,
            picture.fit.size,
            self.emphasis,
            self.seed,
        ))
    }
}

impl Original for Thumbnailed {
    /// The composition's mass edges and the rings, from its picture's masses.
    fn printed(&self, picture: Option<&SheetPicture>) -> Vec<SheetMark> {
        let Some(picture) = picture else {
            return Vec::new();
        };
        match (&picture.masses, self.rings(picture)) {
            (Some(masses), Some(rings)) => marks::printed(masses, &rings),
            _ => Vec::new(),
        }
    }

    /// Once anything lies in its frame: read from its own pixels against its rings.
    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        if !asked.held() {
            return None;
        }
        let picture = asked.picture?;
        let rings = self.rings(picture)?;
        Some(Compare::Read(Reading::of(ThumbnailReading {
            rings,
            extent: picture.fit.size,
        })))
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.picture)
    }
}

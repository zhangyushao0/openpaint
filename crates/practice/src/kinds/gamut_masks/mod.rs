//! Gamut masks (`Kind::GamutMask`, S153, unit 6's first skill): Gurney's gamut mask as an exercise
//! of its own, building on key shifts as combined lines build on lines (D111): a crop painted
//! again with only the colours inside a mask laid on the hue and chroma wheel, its lightness kept
//! (`mask.rs`); the check, a direct measure the app judges (`check.rs`, D123); how a copy is read
//! (`reading.rs`, the key shift's reading as it stands); the marks, the How, the tile and the
//! lessons.
//!
//! A lesson holds one crop (about 6 minutes), none shown before while any is left. The levels
//! differ in the mask and the crop: the first Gurney's triangle taking in the greys, its apex a
//! pure hue `AWAY` from the crop's main hue, on a crop of three or four areas; the second his long
//! diamond across the wheel, two opposite hues and the greys between, narrow, its ends `AWAY`, on
//! a crop of five; the third a small triangle off the middle (no grey inside) toward a hue
//! `FAR_AWAY`, on a crop of six or seven (the nearest tier the packs hold where they hold none).

pub(crate) mod check;
pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod mask;
pub(crate) mod reading;

use std::sync::Arc;

use crate::course::picker::Picker;
use crate::demos::demo::Demo;
use crate::kinds::key_shifts::key_shift::{AWAY, FAR_AWAY};
use crate::kinds::{
    Blank, Described, Facts, Kind, Needs, Original, Picking, SkillLevel, ToolSet, Turn, held,
};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed};
use crate::maths::random::{Random, derived};
use crate::packs::pack::{ItemRef, Tier};
use crate::sheets::exercise::Exercise;
use mask::{Mask, MaskShape};

pub(crate) struct GamutMasks;

const FACTS: [Facts; 1] = [Facts::of("gamut-mask", ToolSet::PAINTING, 360)
    .picks(Picking::LOCKED)
    .needs(Needs::Crops)];

impl Described for GamutMasks {
    fn code(&self) -> u8 {
        24
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::GamutMask(mask) => Some(mask.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Mask::of_code(variant).map(Kind::GamutMask)
    }

    fn generate(&self, exercise: &Exercise) -> Arc<dyn Original> {
        let Kind::GamutMask(mask) = exercise.kind else {
            return held(Blank);
        };
        held(Masked {
            picture: exercise.item(),
            mask,
        })
    }

    fn how(&self, _kind: Kind, _cells: usize) -> Option<Demo> {
        Some(demo::gamut_mask())
    }

    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let (shape, tier, least) = match level {
            SkillLevel::First => (MaskShape::Triangle, Tier::Simple, AWAY),
            SkillLevel::Second => (MaskShape::Diamond, Tier::Medium, AWAY),
            SkillLevel::Third => (MaskShape::Small, Tier::Hard, FAR_AWAY),
        };
        masked((shape, least), tier, turn).into_iter().collect()
    }

    fn window(&self) -> usize {
        3
    }

    /// A disc and Gurney's triangle on it, the disc beyond it tinted; three swatches beside.
    fn glyph(&self) -> Vec<TileMark> {
        let disc: Vec<[f32; 2]> = (0..24)
            .map(|k| {
                let turn = std::f32::consts::TAU * k as f32 / 24.0;
                [21.0 + 14.0 * turn.cos(), 20.0 + 14.0 * turn.sin()]
            })
            .collect();
        let triangle = [[29.0, 8.0], [14.0, 23.0], [22.0, 28.0]];
        let mut marks = vec![
            TileMark::Tint(disc.clone(), 0.18),
            TileMark::Paper(triangle.to_vec()),
            TileMark::Line(closed(&disc)),
            TileMark::Line(closed(&triangle)),
        ];
        for top in [9.0, 18.0, 27.0] {
            let swatch = [
                [42.0, top],
                [47.0, top],
                [47.0, top + 5.0],
                [42.0, top + 5.0],
            ];
            marks.push(TileMark::Tint(swatch.to_vec(), 0.45));
            marks.push(TileMark::Line(closed(&swatch)));
        }
        marks
    }
}

/// A gamut mask's original: the crop it repaints and the mask it is painted inside.
#[derive(Debug)]
pub(crate) struct Masked {
    picture: ItemRef,
    mask: Mask,
}

impl Original for Masked {
    fn givens(&self) -> Vec<SheetMark> {
        marks::givens(self.mask)
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        reading::compare(asked)
    }

    fn picture(&self) -> Option<&ItemRef> {
        Some(&self.picture)
    }
}

/// A gamut mask of `shape` on a crop of `tier` (or the nearest tier held) `turn`'s picker picks,
/// the hue it points to picked by the seed's own sequence among those keeping at least `least`
/// degrees from the crop's main hue; none without crops.
fn masked(
    (shape, least): (MaskShape, f64),
    tier: Tier,
    turn: &Turn<'_>,
) -> Option<(Kind, Option<ItemRef>)> {
    let mut random = Random::new(derived(turn.seed, u64::MAX - 5));
    let (_, item) = Picker::new(turn, Tier::Simple).picked_crop(tier)?;
    let main = turn.pictures.crop_hue(&item);
    let mask = Mask::chosen(shape, main, least, &mut random);
    Some((Kind::GamutMask(mask), Some(item)))
}

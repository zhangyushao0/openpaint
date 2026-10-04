//! Adjustments (S28, Procreate's Adjustments on a layer): hue, saturation and brightness, and
//! tone curves. Both are point operations, so an applied adjustment is an operation record
//! like a stroke or a fill: its values, the selection it was made inside and its place among
//! the layer's strokes (`AdjustEdit`). The evaluator applies it to the layer's content so far
//! at every evaluation, so incremental equals cold and reopening shows what was applied
//! (DECISIONS D54). On a mask it acts on the grey the mask shows.

use crate::artwork::Artwork;
use crate::command::{Applied, Change};
use crate::error::CoreError;
use crate::parts::MaskParts;
use crate::region::{RegionEdit, RegionMask, check_mask, push_edit};
use crate::tone::ToneCurve;
use crate::value::{CanvasSize, invalid};

/// Which adjustment, as the Adjustments menu lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentKind {
    HueSaturationBrightness,
    Curves,
}

/// An adjustment and its values.
#[derive(Debug, Clone, PartialEq)]
pub enum Adjustment {
    Hsb(Hsb),
    Curves(Curves),
}

impl Adjustment {
    /// The adjustment of `kind` that changes nothing, as it starts.
    #[must_use]
    pub fn none(kind: AdjustmentKind) -> Self {
        match kind {
            AdjustmentKind::HueSaturationBrightness => Self::Hsb(Hsb::NONE),
            AdjustmentKind::Curves => Self::Curves(Curves::default()),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> AdjustmentKind {
        match self {
            Self::Hsb(_) => AdjustmentKind::HueSaturationBrightness,
            Self::Curves(_) => AdjustmentKind::Curves,
        }
    }

    /// True when it changes no pixel.
    #[must_use]
    pub fn is_none(&self) -> bool {
        match self {
            Self::Hsb(hsb) => *hsb == Hsb::NONE,
            Self::Curves(curves) => curves.is_identity(),
        }
    }
}

/// Hue, saturation and brightness as Procreate's sliders show them: each in `0..=1`, 0.5
/// changing nothing. On straight sRGB-encoded colour, in this order: hue turns the colour round
/// the HSV hue circle by `hue - 0.5` of a turn; saturation scales the distance from the
/// colour's grey (Rec. 601 luma) by `2 saturation`, clamped; brightness `b` below 0.5 scales
/// towards black (`c 2b`), above it moves towards white (`c + (1 - c)(2b - 1)`). Alpha stays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsb {
    pub hue: f32,
    pub saturation: f32,
    pub brightness: f32,
}

impl Hsb {
    pub const NONE: Self = Self {
        hue: 0.5,
        saturation: 0.5,
        brightness: 0.5,
    };

    /// Each value clamped to `0..=1`; one that is not a number changes nothing.
    #[must_use]
    pub fn new(hue: f32, saturation: f32, brightness: f32) -> Self {
        let unit = |value: f32| {
            if value.is_nan() {
                0.5
            } else {
                value.clamp(0.0, 1.0)
            }
        };
        Self {
            hue: unit(hue),
            saturation: unit(saturation),
            brightness: unit(brightness),
        }
    }
}

/// One of the curves of `Curves`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneChannel {
    /// Every channel at once (Procreate's "Gamma").
    Composite,
    Red,
    Green,
    Blue,
}

/// A curve for each colour channel and one over all three, applied after them, on straight
/// 8-bit colour through tables (`luts`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Curves {
    pub composite: ToneCurve,
    pub red: ToneCurve,
    pub green: ToneCurve,
    pub blue: ToneCurve,
}

impl Curves {
    #[must_use]
    pub const fn curve(&self, channel: ToneChannel) -> &ToneCurve {
        match channel {
            ToneChannel::Composite => &self.composite,
            ToneChannel::Red => &self.red,
            ToneChannel::Green => &self.green,
            ToneChannel::Blue => &self.blue,
        }
    }

    /// These curves with `channel`'s replaced by `curve`.
    #[must_use]
    pub fn with(&self, channel: ToneChannel, curve: ToneCurve) -> Self {
        let mut next = self.clone();
        match channel {
            ToneChannel::Composite => next.composite = curve,
            ToneChannel::Red => next.red = curve,
            ToneChannel::Green => next.green = curve,
            ToneChannel::Blue => next.blue = curve,
        }
        next
    }

    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.all().iter().all(|curve| curve.is_identity())
    }

    /// Per colour channel the 8-bit table of its curve, then the composite one.
    #[must_use]
    pub fn luts(&self) -> [[u8; 256]; 3] {
        let composite = self.composite.lut();
        [&self.red, &self.green, &self.blue]
            .map(|curve| curve.lut().map(|value| composite[usize::from(value)]))
    }

    /// Composite, red, green, blue.
    fn all(&self) -> [&ToneCurve; 4] {
        [&self.composite, &self.red, &self.green, &self.blue]
    }
}

/// An applied adjustment on a layer or its mask, after its first `after` strokes, changing
/// only the pixels of `clip` when it was made inside a selection. `inverted` when it was made
/// on an inverted mask: it then acts on the grey the mask showed (`1 - stored`).
#[derive(Debug, Clone, PartialEq)]
pub struct AdjustEdit {
    pub(crate) after: usize,
    pub(crate) clip: Option<RegionMask>,
    pub(crate) adjustment: Adjustment,
    pub(crate) inverted: bool,
}

impl AdjustEdit {
    #[must_use]
    pub const fn after(&self) -> usize {
        self.after
    }

    #[must_use]
    pub const fn clip(&self) -> Option<&RegionMask> {
        self.clip.as_ref()
    }

    #[must_use]
    pub const fn adjustment(&self) -> &Adjustment {
        &self.adjustment
    }

    #[must_use]
    pub const fn inverted(&self) -> bool {
        self.inverted
    }

    /// The same edit seen from two versions: its clip shared, its values equal.
    pub(crate) fn same(&self, other: &Self) -> bool {
        let clip = match (&self.clip, &other.clip) {
            (Some(a), Some(b)) => a.shares(b),
            (None, None) => true,
            _ => false,
        };
        clip && self.after == other.after
            && self.inverted == other.inverted
            && self.adjustment == other.adjustment
    }

    /// The edit as plain data.
    #[must_use]
    pub fn to_parts(&self) -> AdjustParts {
        let adjustment = match &self.adjustment {
            Adjustment::Hsb(hsb) => AdjustmentParts::Hsb([hsb.hue, hsb.saturation, hsb.brightness]),
            Adjustment::Curves(curves) => {
                AdjustmentParts::Curves(curves.all().map(|curve| curve.points().to_vec()))
            }
        };
        AdjustParts {
            after: self.after as u64,
            clip: self.clip.as_ref().map(RegionMask::to_parts),
            inverted: self.inverted,
            adjustment,
        }
    }

    pub(crate) fn from_parts(parts: &AdjustParts, canvas: CanvasSize) -> Result<Self, CoreError> {
        let clip = parts
            .clip
            .as_ref()
            .map(|clip| RegionMask::from_runs(canvas, clip.origin, clip.size, &clip.runs))
            .transpose()?;
        let adjustment = match &parts.adjustment {
            AdjustmentParts::Hsb([hue, saturation, brightness]) => {
                let unit = |v: &f32| (0.0..=1.0).contains(v);
                if ![hue, saturation, brightness].into_iter().all(unit) {
                    return Err(invalid("adjustment"));
                }
                Adjustment::Hsb(Hsb::new(*hue, *saturation, *brightness))
            }
            AdjustmentParts::Curves(points) => {
                let curve =
                    |points: &Vec<[f32; 2]>| ToneCurve::new(points).ok_or(invalid("tone curve"));
                Adjustment::Curves(Curves {
                    composite: curve(&points[0])?,
                    red: curve(&points[1])?,
                    green: curve(&points[2])?,
                    blue: curve(&points[3])?,
                })
            }
        };
        Ok(Self {
            after: usize::try_from(parts.after).map_err(|_| invalid("adjustment place"))?,
            clip,
            adjustment,
            inverted: parts.inverted,
        })
    }
}

/// An adjustment as plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct AdjustParts {
    pub after: u64,
    pub clip: Option<MaskParts>,
    pub inverted: bool,
    pub adjustment: AdjustmentParts,
}

/// An adjustment's values as plain data.
#[derive(Debug, Clone, PartialEq)]
pub enum AdjustmentParts {
    /// Hue, saturation, brightness.
    Hsb([f32; 3]),
    /// The points of the composite, red, green and blue curves.
    Curves([Vec<[f32; 2]>; 4]),
}

impl Artwork {
    /// Adjusts the active layer above its strokes so far, or its mask when that is the target
    /// (acting on the grey it shows), inside `clip` when given.
    ///
    /// # Errors
    /// `LayerLocked` when the active layer is locked; `MaskOff` when its mask is the target and
    /// disabled; `InvalidValue` when the clip leaves the canvas.
    pub fn adjust_region(
        &self,
        clip: Option<RegionMask>,
        adjustment: Adjustment,
    ) -> Result<Applied, CoreError> {
        let (index, target) = self.target()?;
        if let Some(clip) = &clip {
            check_mask(self.canvas(), clip)?;
        }
        let (after, inverted) = match target {
            Some(mask) => (mask.strokes.len(), mask.inverted),
            None => (self.state().layers[index].strokes.len(), false),
        };
        let edit = AdjustEdit {
            after,
            clip,
            adjustment,
            inverted,
        };
        Ok(Applied {
            artwork: push_edit(self, index, RegionEdit::Adjust(edit)),
            change: Change::Content,
        })
    }
}

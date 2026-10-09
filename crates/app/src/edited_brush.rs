//! What the brush editor shows of the current tool's brush (S46): its name, whether Reset and
//! Delete apply, every setting's value and the range its control gives, which choices are
//! offered and the dynamics curves with how high their points may go.

use openpaint_core::{RecipeParts, Rotation, Stamp, Tip};

use crate::brush_edit::curve_of_ref;
use crate::brush_fit;
use crate::brush_knobs::{Axis, Dynamic, Knob, KnobRange, curve_top};
use crate::brushes::{BrushKey, SizeRange};

#[derive(Debug, Clone, PartialEq)]
pub struct EditedBrush {
    pub key: BrushKey,
    pub name: String,
    /// A built-in (Reset), not a copy (Delete).
    pub built_in: bool,
    /// Reset would change something: a built-in that differs from its file, or the tool's
    /// StreamLine from the file's.
    pub resettable: bool,
    pub recipe: RecipeParts,
    pub size: SizeRange,
    /// The recipe of the built-in the brush descends from, as its file has it.
    pub(crate) origin: RecipeParts,
}

impl EditedBrush {
    /// `knob`'s value now, in its control's units.
    #[must_use]
    pub fn value(&self, knob: Knob) -> f32 {
        brush_fit::value(&self.recipe, self.size, knob)
    }

    /// The values `knob`'s control gives now; `None` when it does nothing for this brush (grey).
    #[must_use]
    pub fn range(&self, knob: Knob) -> Option<KnobRange> {
        brush_fit::range(&self.recipe, self.size, knob)
    }

    /// The stamps' shape; `None` for a continuous stroke.
    #[must_use]
    pub fn stamp(&self) -> Option<Stamp> {
        self.stamped().then_some(self.recipe.shape.stamp)
    }

    #[must_use]
    pub fn rotation(&self) -> Option<Rotation> {
        self.stamped().then_some(self.recipe.shape.rotation)
    }

    fn stamped(&self) -> bool {
        self.recipe.shape.spacing > 0.0
    }

    /// The tip while the brush builds up; `None` while it covers.
    #[must_use]
    pub fn tip(&self) -> Option<Tip> {
        self.recipe.deposit.as_ref().map(|deposit| deposit.tip)
    }

    /// Lays its density through the sketch paper.
    #[must_use]
    pub fn paper(&self) -> bool {
        self.recipe
            .deposit
            .as_ref()
            .is_some_and(|deposit| deposit.paper.is_some())
    }

    /// Whether a stamp's shape may be chosen: not while the paper keeps the stroke continuous.
    #[must_use]
    pub fn offers_stamps(&self) -> bool {
        !self.paper()
    }

    /// Whether the paper may be laid: under a flat tip, on a continuous stroke.
    #[must_use]
    pub fn offers_paper(&self) -> bool {
        self.tip() == Some(Tip::Flat) && !self.stamped()
    }

    /// Whether the bell tip may be chosen: not on paper.
    #[must_use]
    pub fn offers_bell(&self) -> bool {
        !self.paper()
    }

    /// The curve of `dynamic` over `axis`, `[x, y]` in the axis's units.
    #[must_use]
    pub fn curve(&self, dynamic: Dynamic, axis: Axis) -> &[[f32; 2]] {
        curve_of_ref(&self.recipe, dynamic, axis)
    }

    /// The highest factor its points may take now: the curve's top (`curve_top`), or less for a
    /// size curve whose stamps would no longer fit.
    #[must_use]
    pub fn curve_limit(&self, dynamic: Dynamic, axis: Axis) -> f32 {
        let top = curve_top(dynamic, axis);
        match dynamic {
            Dynamic::Size => brush_fit::curve_limit(&self.recipe, self.curve(dynamic, axis), top),
            Dynamic::Opacity => top,
        }
    }

    /// Whether the curve differs from the built-in's file (its Reset gives that back).
    #[must_use]
    pub fn curve_changed(&self, dynamic: Dynamic, axis: Axis) -> bool {
        self.curve(dynamic, axis) != curve_of_ref(&self.origin, dynamic, axis)
    }
}

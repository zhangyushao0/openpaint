//! What an exercise's own tools are and what the eyedropper reads over it, as data (S76, S95,
//! S96, S100, S104, S107, S123): the pens a kind offers, whether it paints, fills and what a fill
//! fills, the wheel, Undo and Redo, the eyedropper, a whole copy's layers; and what can be picked.
//! A kind's facts hold one of each (`Facts::tools`, `Facts::picks`), most of them a preset here;
//! the flow lets the pens listed be chosen and fills where its tools fill, the screen lays them out
//! from these fields, and neither asks which kind it is.

use crate::compositions::levels::Level;
use crate::compositions::value_masses::Value;

/// The pens an exercise's own tools offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pen {
    /// A light pencil: marks and construction lines, left out of the aligned fit.
    Sketch,
    /// The ink pen the copy is drawn with.
    Draw,
    /// The blue pen that corrects a compared copy.
    Correct,
    /// Painting in the colour picked: a big brush for the masses, a small one for the edges and
    /// the hierarchy, a soft one for soft edges.
    Big,
    Small,
    Soft,
    /// A division's levels in V大's colours (S95): its main lines red, the second green, the
    /// third blue.
    Main,
    Second,
    Third,
    /// Value masses' three greys (S96): the dark, the middle and the light.
    Dark,
    Middle,
    Light,
}

impl Pen {
    /// The pens a line or a shape is drawn with, and those a colour combination or a value scale
    /// is painted with.
    pub const DRAWING: [Self; 3] = [Self::Sketch, Self::Draw, Self::Correct];
    pub const PAINTING: [Self; 3] = [Self::Big, Self::Small, Self::Soft];
    /// A division's pens, and a whole copy's: its division's, then Draw for its line drawing (and
    /// the painting pens for its colour blocks, S96).
    pub const DIVIDING: [Self; 3] = [Self::Main, Self::Second, Self::Third];
    pub const COPYING: [Self; 4] = [Self::Main, Self::Second, Self::Third, Self::Draw];
    /// Value masses' greys (S96).
    pub const VALUES: [Self; 3] = [Self::Dark, Self::Middle, Self::Light];

    /// Whether it paints in the colour picked.
    #[must_use]
    pub fn paints(self) -> bool {
        Self::PAINTING.contains(&self)
    }

    /// The value it paints, if it is one of value masses' greys.
    #[must_use]
    pub const fn value(self) -> Option<Value> {
        match self {
            Self::Dark => Some(Value::Dark),
            Self::Middle => Some(Value::Middle),
            Self::Light => Some(Value::Light),
            _ => None,
        }
    }

    /// The division's level it draws, if it is a division's pen.
    #[must_use]
    pub const fn level(self) -> Option<Level> {
        match self {
            Self::Main => Some(Level::Main),
            Self::Second => Some(Level::Second),
            Self::Third => Some(Level::Third),
            _ => None,
        }
    }
}

/// What a fill of the exercise fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    /// Its empty frame (Fill frame).
    Frame,
    /// The cell in hand, which the pen touching a cell takes (Fill cell, a value scale's).
    Cell,
}

/// The exercise's own tools beside its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolSet {
    /// None of its own: the full interface only, painted with the toolbox's brushes, no All
    /// tools (a long colour copy, S104).
    pub full: bool,
    /// Its pens in their row, chosen among (Correct only once compared); the painting pens are
    /// `paints`.
    pub pens: &'static [Pen],
    /// The painting pens Big, Small and Soft in the colour picked, beside the eyedropper: in a
    /// row of their own under its pens, or after Fill when it has none.
    pub paints: bool,
    /// The colour wheel.
    pub wheel: bool,
    /// Fill, and what it fills; with greys among its pens, in the grey chosen.
    pub fill: Option<Fill>,
    /// Undo and Redo.
    pub history: bool,
    /// The eyedropper: beside the painting pens, else beside Fill.
    pub eyedropper: bool,
    /// A whole copy's (S95, S96): its pens draw on layers of their own (`layers.rs` in the app),
    /// the wheel and Fill only while a painting pen paints its colour blocks.
    pub layered: bool,
}

impl ToolSet {
    const NONE: Self = Self {
        full: false,
        pens: &[],
        paints: false,
        wheel: false,
        fill: None,
        history: false,
        eyedropper: false,
        layered: false,
    };
    /// Sketch, Draw and, once compared, Correct; Undo and Redo (lines, shapes, geometry).
    pub(crate) const DRAWING: Self = Self {
        pens: &Pen::DRAWING,
        history: true,
        ..Self::NONE
    };
    /// A division's levels' pens, Main, Second and Third; Undo and Redo (S95).
    pub(crate) const DIVIDING: Self = Self {
        pens: &Pen::DIVIDING,
        history: true,
        ..Self::NONE
    };
    /// Its division's pens and Draw, the painting pens for its colour blocks with the eyedropper,
    /// Fill frame while one paints, Undo and Redo (a whole copy, S95, S96).
    pub(crate) const COPYING: Self = Self {
        pens: &Pen::COPYING,
        paints: true,
        wheel: true,
        fill: Some(Fill::Frame),
        history: true,
        eyedropper: true,
        layered: true,
        ..Self::NONE
    };
    /// The three greys and Fill frame in the grey chosen; Undo and Redo (value masses, S96).
    pub(crate) const GREYS: Self = Self {
        pens: &Pen::VALUES,
        fill: Some(Fill::Frame),
        history: true,
        ..Self::NONE
    };
    /// The wheel and Fill frame: one flat colour, no pens (a colour).
    pub(crate) const COLOUR: Self = Self {
        wheel: true,
        fill: Some(Fill::Frame),
        ..Self::NONE
    };
    /// The wheel, Fill cell, Undo, Redo and the eyedropper; no pens (a value scale, S100).
    pub(crate) const CELLS: Self = Self {
        wheel: true,
        fill: Some(Fill::Cell),
        history: true,
        eyedropper: true,
        ..Self::NONE
    };
    /// The wheel, Fill frame, Undo, Redo, the painting pens in the colour picked and the
    /// eyedropper (a colour combination, a key shift).
    pub(crate) const PAINTING: Self = Self {
        paints: true,
        wheel: true,
        fill: Some(Fill::Frame),
        history: true,
        eyedropper: true,
        ..Self::NONE
    };
    /// None of its own (a long colour copy, S104).
    pub(crate) const FULL: Self = Self {
        full: true,
        ..Self::NONE
    };

    /// Whether `pen` may be chosen: one of its pens (Correct once `compared`), or a painting pen
    /// where it paints.
    #[must_use]
    pub fn offers(&self, pen: Pen, compared: bool) -> bool {
        let listed = self.pens.contains(&pen) && (pen != Pen::Correct || compared);
        listed || (self.paints && pen.paints())
    }

    /// Whether its pens are value masses' greys: its fill takes the grey chosen.
    #[must_use]
    pub fn greys(&self) -> bool {
        self.pens.iter().any(|pen| pen.value().is_some())
    }

    /// Whether its pens draw a division's levels.
    #[must_use]
    pub fn levels(&self) -> bool {
        self.pens.iter().any(|pen| pen.level().is_some())
    }
}

/// What the eyedropper reads over an exercise's frames (D105, S96, S107): by default the canvas,
/// as anywhere (its original, once compared, too).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picking {
    /// Whether its original may be picked at all: not a key shift's, whose colours are judged by
    /// eye (S107).
    pub original: bool,
    /// Its original is a swatch, picked whole once compared, never a point compared; its copy as
    /// the canvas (a colour). Its wheel stays while compared.
    pub swatch: bool,
    /// Its given ends' colours in its one frame (a value scale).
    pub ends: bool,
    /// Its original's colour at the point; while compared, that point compared with the copy's
    /// (a colour combination, a colour copy, a whole copy).
    pub probed: bool,
    /// A point of the copy left bare compares nothing: its colour blocks are compared where
    /// painted (a whole copy, S96).
    pub bare: bool,
}

impl Default for Picking {
    fn default() -> Self {
        Self::CANVAS
    }
}

impl Picking {
    pub(crate) const CANVAS: Self = Self {
        original: true,
        swatch: false,
        ends: false,
        probed: false,
        bare: false,
    };
    pub(crate) const SWATCH: Self = Self {
        swatch: true,
        ..Self::CANVAS
    };
    pub(crate) const ENDS: Self = Self {
        ends: true,
        ..Self::CANVAS
    };
    pub(crate) const PROBED: Self = Self {
        probed: true,
        ..Self::CANVAS
    };
    pub(crate) const BLOCKS: Self = Self {
        probed: true,
        bare: true,
        ..Self::CANVAS
    };
    pub(crate) const LOCKED: Self = Self {
        original: false,
        ..Self::CANVAS
    };
}

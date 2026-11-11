//! What the app and the UI read of a family (S109, S109b, D111): its key, how its copy is made and
//! compared, its frames, the tools beside its frame, what the eyedropper reads over it, what a
//! drill of it needs kept, what the practice book keeps of it and the summary's rows. Each kind's
//! description gives its families' (`Described::facts`); the flow and the screens ask these,
//! never which family it is.

use crate::records::metric::Metric;
use crate::sessions::family::Family;

/// How a family's copy is made and compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// Figure with the pens as lines and laid over the original, in place or aligned (lines,
    /// combined lines, single shapes).
    Lines,
    /// Figure as lines and checked as geometry against an exact answer or rule, never aligned
    /// (boxes, ellipses, estimates, points in space, solids, objects).
    Geometry,
    /// Figure with a division's pens in V大's three colours: no answer, the picture laid under the
    /// lines (a division, a whole copy's division, S95).
    Divided,
    /// A single colour matched by picking.
    Colour,
    /// Painted with brushes and read from the copy's pixels (colour combinations, value scales,
    /// colour copies, value masses, key shifts).
    Painted,
}

/// The exercise's own tools beside its frame (S76, S95, S96, S100, S104).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tools {
    /// Sketch, Draw and, once compared, Correct; Undo and Redo (lines, shapes, geometry).
    Drawing,
    /// A division's levels' pens, Main, Second and Third; Undo and Redo (S95).
    Dividing,
    /// Its division's pens and Draw, the painting pens for its colour blocks with the eyedropper,
    /// Fill frame while one paints, Undo and Redo (a whole copy, S95, S96).
    Copying,
    /// The three greys and Fill frame in the grey chosen; Undo and Redo (value masses, S96).
    Greys,
    /// The wheel and Fill frame: one flat colour, no pens (a colour).
    Colour,
    /// The wheel, Fill cell (the cell in hand, which the pen touching a cell takes), Undo, Redo
    /// and the eyedropper; no pens (a value scale, S100).
    Cells,
    /// The wheel, Fill frame, Undo, Redo, the painting pens Big, Small and Soft in the colour
    /// picked and the eyedropper (a colour combination, a key shift).
    Painting,
    /// None of its own: the full interface and the toolbox's brushes (a long colour copy, S104).
    Full,
}

/// What the eyedropper reads over an exercise's frames (D105, S96, S107).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picks {
    /// The canvas, as anywhere (its original, once compared, too).
    Canvas,
    /// Its original's swatch once compared; its copy as the canvas (a colour).
    Swatch,
    /// Its given ends' colours in its one frame (a value scale).
    Ends,
    /// Its original's colour at the point; while compared, that point compared with the copy's (a
    /// colour combination, a colour copy).
    Probed,
    /// As `Probed`, but a point of the copy left bare compares nothing: its colour blocks are
    /// compared where painted (a whole copy, S96).
    Blocks,
    /// Its original never: its colours are judged by eye (a key shift, S107).
    Locked,
}

/// What a drill of a family needs kept before it starts (S91, S95, S96, S101, S107).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Needs {
    Nothing,
    /// Any pack's picture (a single shape).
    Pictures,
    /// A composition (a division, a whole copy).
    Compositions,
    /// A composition whose value masses read.
    Masses,
    /// A crop (a colour combination, a key shift).
    Crops,
}

/// What the practice book keeps of a compared exercise (S89, D109).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    Nothing,
    /// Its original and its first copy (a single line).
    Line,
    /// Its original and the first colour laid (a colour).
    Pick,
}

/// What the app and the UI read of a family: the key its words go by in the UI's tables
/// (`ui/src/text/practice/`; its tile, its test drill's name too), how its copy is made and
/// compared, whether its answer is built from given parts in the one frame drawn in (S97, S98:
/// the original's frame beside it left empty, room for what Compare draws beyond the frame),
/// whether it copies a composition's picture (S95, S96), its tools, what the eyedropper reads,
/// what a drill needs, what the book keeps, whether its picture's ground shows faded in its
/// original's frame (a single shape's, S99) and the metrics the summary sets side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    pub family: Family,
    pub key: &'static str,
    pub work: Work,
    /// Whether its copy is laid over its original as lines, so Aligned may be offered (lines,
    /// combined lines, single shapes): what the flow reads in place of `work` (S122b), which
    /// goes (S122a).
    pub aligned: bool,
    /// Whether its copy is compared at all: Compare is offered and it ends with a result. Not a
    /// free scribble's (S122): no Compare on its bar, Next alone (S122b, S122c).
    pub compared: bool,
    pub constructed: bool,
    pub composed: bool,
    pub tools: Tools,
    pub picks: Picks,
    pub needs: Needs,
    pub kept: Kept,
    pub grounded: bool,
    pub rows: &'static [Metric],
}

impl Facts {
    /// A family of `work` keyed `key`, neither constructed nor composed, with the tools its work
    /// draws or paints with, the eyedropper reading the canvas, nothing needed or kept.
    pub(crate) const fn of(family: Family, key: &'static str, work: Work) -> Self {
        let tools = match work {
            Work::Lines | Work::Geometry => Tools::Drawing,
            Work::Divided => Tools::Dividing,
            Work::Colour => Tools::Colour,
            Work::Painted => Tools::Painting,
        };
        Self {
            family,
            key,
            work,
            aligned: matches!(work, Work::Lines),
            compared: true,
            constructed: false,
            composed: false,
            tools,
            picks: Picks::Canvas,
            needs: Needs::Nothing,
            kept: Kept::Nothing,
            grounded: false,
            rows: &[],
        }
    }

    pub(crate) const fn constructed(self) -> Self {
        Self {
            constructed: true,
            ..self
        }
    }

    pub(crate) const fn composed(self) -> Self {
        Self {
            composed: true,
            ..self
        }
    }

    pub(crate) const fn uncompared(self) -> Self {
        Self {
            compared: false,
            ..self
        }
    }

    pub(crate) const fn tools(self, tools: Tools) -> Self {
        Self { tools, ..self }
    }

    pub(crate) const fn picks(self, picks: Picks) -> Self {
        Self { picks, ..self }
    }

    pub(crate) const fn needs(self, needs: Needs) -> Self {
        Self { needs, ..self }
    }

    pub(crate) const fn kept(self, kept: Kept) -> Self {
        Self { kept, ..self }
    }

    pub(crate) const fn grounded(self) -> Self {
        Self {
            grounded: true,
            ..self
        }
    }

    pub(crate) const fn rows(self, rows: &'static [Metric]) -> Self {
        Self { rows, ..self }
    }
}

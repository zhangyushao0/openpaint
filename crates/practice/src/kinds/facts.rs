//! What the app and the UI read of a kind (S109, S109b, D111, S122, S123): its topic's key,
//! whether its copy is laid over its original and whether it is compared at all, its frames, its
//! own tools and what the eyedropper reads over it (`tools.rs`), what a lesson of it needs kept,
//! what the practice book keeps and the seals count of it and about how long one of its exercises
//! takes. Each kind's description gives its topics' (`Described::facts`, `Described::topic`); the
//! flow and the screens read these fields, never which kind it is.

use super::tools::{Picking, ToolSet};

/// What a lesson of a kind needs kept before it starts (S91, S95, S96, S101, S107).
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
    /// The drawing as first drawn, before its first Compare (S151): the first lesson's drawing of
    /// what lay in front of the learner and a drawing from life, its "before" and its "after".
    Drawing,
}

/// What a compared exercise counts toward among the seals (`records/seals.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counted {
    Nothing,
    /// A hundred lines (a single line, combined lines).
    Line,
    /// A hundred colours (a colour).
    Colour,
}

/// What the app and the UI read of a kind's topic: the key its words go by in the UI's tables
/// (`ui/src/text/practice/`; its How, and the How told once, too), whether its copy is laid over
/// its original as lines (so Aligned may be offered) and whether it is compared at all, whether
/// its answer is built from given parts in the one frame drawn in (S97, S98: the original's frame
/// beside it left empty, room for what Compare draws beyond the frame), its tools, what the
/// eyedropper reads, what a lesson needs, what the book keeps and the seals count, whether its
/// picture's ground shows faded in its original's frame (a single shape's, S99) and about how many
/// seconds one exercise of it takes, looked at, drawn or painted, compared and corrected once (the
/// estimates of S75 to S107): what a lesson's minutes are counted from (`schedule::offer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    pub key: &'static str,
    /// Whether its copy is laid over its original as lines, so Aligned may be offered (lines,
    /// combined lines, single shapes).
    pub aligned: bool,
    /// Whether its copy is compared at all: Compare is offered and it ends with a result. Not a
    /// free scribble's (S122): no Compare on its bar, Next alone.
    pub compared: bool,
    pub constructed: bool,
    pub tools: ToolSet,
    pub picks: Picking,
    pub needs: Needs,
    pub kept: Kept,
    pub counted: Counted,
    pub grounded: bool,
    /// Whether its pack picture lies under the marks in its one frame, on a locked layer of the
    /// sheet ("Picture", never an undo step, never picked; abstract analysis, S112).
    pub under: bool,
    /// Whether its original's frame is covered until its first Compare: drawn with the picture out
    /// of sight (a drawing from life, its photo, S151).
    pub hidden: bool,
    /// The shape of the picture its generator draws, which its frames take (a texture's strip, 2
    /// to 1, S154); none: square, or its pack picture's (S99).
    pub shape: Option<[u16; 2]>,
    pub seconds: u16,
}

impl Facts {
    /// A topic keyed `key` drawn or painted with `tools`, one exercise of which takes about
    /// `seconds`: compared, neither aligned nor constructed, the eyedropper reading the canvas,
    /// nothing needed, kept or counted.
    pub(crate) const fn of(key: &'static str, tools: ToolSet, seconds: u16) -> Self {
        Self {
            key,
            aligned: false,
            compared: true,
            constructed: false,
            tools,
            picks: Picking::CANVAS,
            needs: Needs::Nothing,
            kept: Kept::Nothing,
            counted: Counted::Nothing,
            grounded: false,
            under: false,
            hidden: false,
            shape: None,
            seconds,
        }
    }

    pub(crate) const fn shaped(self, shape: [u16; 2]) -> Self {
        Self {
            shape: Some(shape),
            ..self
        }
    }

    pub(crate) const fn under(self) -> Self {
        Self {
            under: true,
            ..self
        }
    }

    pub(crate) const fn hidden(self) -> Self {
        Self {
            hidden: true,
            ..self
        }
    }

    pub(crate) const fn aligned(self) -> Self {
        Self {
            aligned: true,
            ..self
        }
    }

    pub(crate) const fn constructed(self) -> Self {
        Self {
            constructed: true,
            ..self
        }
    }

    pub(crate) const fn uncompared(self) -> Self {
        Self {
            compared: false,
            ..self
        }
    }

    pub(crate) const fn picks(self, picks: Picking) -> Self {
        Self { picks, ..self }
    }

    pub(crate) const fn needs(self, needs: Needs) -> Self {
        Self { needs, ..self }
    }

    pub(crate) const fn kept(self, kept: Kept) -> Self {
        Self { kept, ..self }
    }

    pub(crate) const fn counted(self, counted: Counted) -> Self {
        Self { counted, ..self }
    }

    pub(crate) const fn grounded(self) -> Self {
        Self {
            grounded: true,
            ..self
        }
    }
}

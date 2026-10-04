//! `BlendMode`: how a layer's pixels combine with what lies under it (Procreate's four first
//! modes). The formulas are the renderer's; core only names the choice.

/// How a layer (or a clipping chain, through its base) composites onto what lies under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum BlendMode {
    /// Source over: the layer covers what is under it by its alpha.
    #[default]
    Normal,
    /// Darkens: the colours multiply where both have paint.
    Multiply,
    /// Lightens: the inverted colours multiply.
    Screen,
    /// Adds contrast: Multiply under mid grey, Screen above it, decided by what is under.
    Overlay,
}

impl BlendMode {
    /// Every mode, in the order the layer panel offers them.
    pub const ALL: [Self; 4] = [Self::Normal, Self::Multiply, Self::Screen, Self::Overlay];
}

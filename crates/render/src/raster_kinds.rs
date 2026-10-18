//! What a raster plan is made of (S59, `raster_plan.rs`): instances, the draws and passes over
//! the atlas and the region's float targets, and its counts with what they cost the tablet's GPU.

use std::ops::Range;

use crate::protocol::Blend;

/// One instance as the raster core's vertex stages read it: whole numbers, then signed ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Instance {
    pub(crate) a: [u32; 4],
    pub(crate) b: [i32; 4],
}

/// How a stroke's instances lay into its region of the atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Coverage {
    /// Pieces of a covering stroke: the largest coverage.
    Cover,
    /// Pieces of an accumulating one: their densities summed; and its paper contact, the largest.
    Density,
    Contact,
    /// Stamps, kept the largest or summed.
    StampCover,
    StampDensity,
}

/// One draw into the atlas: the instances of one stroke's region, cut to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoverageDraw {
    pub(crate) coverage: Coverage,
    /// The region: x, y, width, height in atlas pixels.
    pub(crate) scissor: [u32; 4],
    pub(crate) instances: Range<u32>,
}

/// How a layer's (or a chain's, or a merged layer's) end lays it onto what lies under it
/// (`raster_states.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndBlend {
    /// Onto the opaque canvas: Normal (over), Multiply, Screen; over is Normal onto a level too.
    Over,
    Multiply,
    ScreenCanvas,
    /// Into a clipping chain, its alpha kept: Normal (atop), Screen (Multiply as onto the canvas).
    Atop,
    ScreenChain,
    /// A chain's base: the chain becomes the layer as its mask shows it.
    Base,
    /// By its mode onto a copy of what lies under it (written whole, unblended): Overlay, and
    /// (S65) Multiply and Screen onto a level, which is not opaque.
    Backdrop,
    /// A group's end (S65): the canvas faded by its opacity over the copy of what lay under the
    /// group, from a copy of the canvas (written whole).
    Group,
}

/// What a draw onto a scratch target does (its pipeline).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawKind {
    /// A stroke's tiles from the atlas, by its paint op's blend.
    Stroke(Blend),
    Fill(Blend),
    Patch,
    /// A smudge from the copy of what it applies to (S61).
    Smudge,
    /// S65: a point adjustment and a filter from the copy of what they apply to; a moved
    /// content cut to its selection laid over the level under it; a whole layer's moved content
    /// cut to the canvas, in place.
    Adjust,
    Filter,
    Moved,
    Cut,
    End(EndBlend),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Draw {
    pub(crate) kind: DrawKind,
    pub(crate) instances: Range<u32>,
}

/// The raster core's scratch targets (`raster.rs`), float pixels of the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Scratch {
    Canvas,
    Chain,
    Layer,
    Mask,
    /// What an end written whole, a smudge, an adjustment or a filter reads, copied.
    Backdrop,
    /// What lay under the group being composited (S65), copied at its first layer's end.
    Group,
    /// A moved selection's or a merged layer's content `n` levels above the layer's paint (S65),
    /// from 1 to `MOVE_DEPTH`.
    Level(u8),
}

/// Scratch targets a region may use: the six, then a level per move the evaluator stacks.
pub(crate) const SCRATCH: usize = 6 + crate::protocol::MOVE_DEPTH as usize;

impl Scratch {
    /// Where it lies among `SCRATCH`.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Canvas => 0,
            Self::Chain => 1,
            Self::Layer => 2,
            Self::Mask => 3,
            Self::Backdrop => 4,
            Self::Group => 5,
            Self::Level(n) => 5 + n as usize,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Strokes' coverage into the atlas, cleared first.
    Coverage(Vec<CoverageDraw>),
    /// Draws onto the layer or its mask, cleared first when `clear`.
    Draw {
        target: Scratch,
        clear: bool,
        draws: Vec<Draw>,
    },
    /// The canvas cleared to the paper, or a target to nothing.
    Clear(Scratch),
    /// A target copied whole into another (the backdrop, or S65 the group's).
    Copy {
        from: Scratch,
        to: Scratch,
    },
    /// A smudge (S61), an adjustment or a filter (S65) on the layer, a level or the mask
    /// (`target`): the pixels `rect` (x, y, width, height) copied into the backdrop, then its draw
    /// over them from that copy.
    Rewrite {
        target: Scratch,
        rect: [u32; 4],
        draw: Draw,
    },
    /// A layer's end onto `target`, reading `reads` (the layer's paint, the chain, a level, the
    /// group's backdrop), the mask and the backdrop.
    End {
        target: Scratch,
        reads: Scratch,
        draw: Draw,
    },
    /// S63, the stroke being drawn (`drawing.rs`): targets set from snapshots' slots, as the
    /// units before the stroke left them; targets kept into slots.
    Restore(Vec<SlotCopy>),
    Keep(Vec<SlotCopy>),
    /// S64, the stroke being drawn in one pass of the atlas, cleared first: its kept coverage
    /// read from each page's slots into its regions (`held`: per page, the instances), then
    /// `draws` laying the spans settled since and its tail over it; then `keep`, the regions
    /// holding the kept coverage and the settled spans (not the tail), copied into the slots.
    Kept {
        held: Vec<(usize, Range<u32>)>,
        draws: Vec<CoverageDraw>,
        keep: Vec<AtlasCopy>,
    },
    /// S64: kept coverage slots taken anew set to nothing (each a copy of a tile of nothing), so
    /// what a region copies into a part of one leaves nothing in the rest.
    Zero(Vec<SlotAt>),
}

/// A slot of a stroke's drawing (S63): its page and the page pixel at its top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotAt {
    pub(crate) page: usize,
    pub(crate) at: [u32; 2],
}

/// A copy of `size` pixels between a scratch target's region pixels from `at` and a snapshot's
/// slot from `offset` in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotCopy {
    pub(crate) target: Scratch,
    pub(crate) slot: SlotAt,
    pub(crate) offset: [u32; 2],
    pub(crate) at: [u32; 2],
    pub(crate) size: [u32; 2],
}

/// A copy of `size` pixels of the atlas from `at` into a kept coverage's slot from `offset` in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AtlasCopy {
    pub(crate) at: [u32; 2],
    pub(crate) slot: SlotAt,
    pub(crate) offset: [u32; 2],
    pub(crate) size: [u32; 2],
}

/// What a plan holds, counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RasterCounts {
    /// Strokes reaching the region, of the scene's.
    pub strokes: usize,
    pub scene_strokes: usize,
    /// Coverage passes, and render passes in all (the finish one more).
    pub batches: usize,
    pub passes: usize,
    /// Piece instances (a paper stroke's twice: density and contact) and stamp instances.
    pub pieces: usize,
    pub stamps: usize,
    /// Composite tile instances.
    pub tiles: usize,
    /// Atlas pixels the regions take, summed over the batches.
    pub region_pixels: u64,
    /// Fills and patches, and the pixels their quads take; layer and chain ends.
    pub edits: usize,
    pub edit_pixels: u64,
    pub ends: usize,
    /// Smudges, and the pixels each copies and draws, summed (S61).
    pub smudges: usize,
    pub smudge_pixels: u64,
    /// Adjustments, filters, moves' and merged layers' ends and groups' ends (S65), and the
    /// pixels they copy and draw, summed.
    pub records: usize,
    pub record_pixels: u64,
}

impl RasterCounts {
    /// What drawing a plan of these counts over a region of `size` pixels costs the tablet's GPU,
    /// estimated in ns (S57's measurements on the Adreno 735: about 40 µs a pass, the pile's
    /// composites about 1 ns an output pixel; a smudge's copy and residual read as the
    /// evaluator's smudge op costs, D82's table); the frame budget corrects it by the raster
    /// core's measured times (`pace.rs`).
    pub(crate) fn estimate(&self, [width, height]: [u32; 2]) -> f64 {
        const PASS: f64 = 40_000.0;
        const COMPOSITE: f64 = 1.0;
        const COVERAGE: f64 = 0.5;
        const INSTANCE: f64 = 20.0;
        const EDIT: f64 = 6.0;
        const END: f64 = 1.5;
        const SMUDGE: f64 = 4.0;
        // A copy and a pass over each pixel, a point operation or a bilinear read (S65's guess,
        // corrected by the readings like the rest).
        const RECORD: f64 = 2.0;
        let region = f64::from(width) * f64::from(height);
        let tile = 32.0 * 32.0;
        self.passes as f64 * PASS
            + self.tiles as f64 * tile * COMPOSITE
            + self.region_pixels as f64 * COVERAGE
            + (self.pieces + self.stamps) as f64 * INSTANCE
            + self.edit_pixels as f64 * EDIT
            + (self.ends + 1) as f64 * region * END
            + self.smudge_pixels as f64 * SMUDGE
            + self.record_pixels as f64 * RECORD
    }
}

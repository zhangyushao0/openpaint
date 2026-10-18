//! Drawing on the raster core (S63, DECISIONS D90): what one output (the 1x canvas, or a view
//! tier) keeps for the strokes drawn on it. Per 256-px output tile, in slots of pages of its own:
//! a snapshot of the region's float targets (the canvas, the layer, its mask or clipping chain as
//! the place needs) as the units before the next stroke's place leave them, which outlives the
//! stroke: a stroke committed there since is laid onto it when it is next used (`at` names the unit
//! it stands before), a change elsewhere in the artwork leaves it, one reaching it lets it go
//! (`drawing_follow.rs`); and, while a stroke is drawn, its coverage so far (`kept`; its density
//! and paper contact for a pencil), to which every frame adds the spans that settled since, in
//! order, as the blend unit would lay them in one go. A frame then draws only where the stroke
//! changed: the targets restored from the snapshots, the stroke laid from its kept coverage and its
//! tail, the units after it (`drawing_frame.rs`); the raster worker takes snapshots ahead while
//! nothing touches the screen (`drawing_ahead.rs`).

use std::collections::BTreeMap;

use openpaint_core::{Artwork, LayerId};

use crate::active::ActiveTrack;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::raster::{Pages, Target};
use crate::raster_plan::{Restored, Scratch, SlotAt, SlotCopy};
use crate::readback::target_texture;
use crate::view_raster_jobs::RasterDone;
use crate::view_tiles::TIER_TILE;

/// Slots a snapshot page holds across and down: 16 slots of 256 x 256 float pixels.
const SNAPSHOT_PAGE: u32 = 4;

/// One output's drawing.
#[derive(Default)]
pub(crate) struct Drawing {
    /// Which stroke its kept coverage holds, and how far.
    pub(crate) track: ActiveTrack,
    /// Per output tile (column, row) the stroke being drawn reached: its kept coverage.
    pub(crate) kept: BTreeMap<[u32; 2], Kept>,
    /// Per output tile: its snapshot.
    pub(crate) under: BTreeMap<[u32; 2], Under>,
    /// What the snapshots stand for.
    pub(crate) base: Option<Base>,
    /// Snapshots the raster worker takes now.
    pub(crate) taking: Vec<Taking>,
    /// Counts snapshots made, so a later one is told from an earlier one of the same tile.
    made: u64,
    snapshots: Pool,
    coverage: Pool,
    /// A bind group reading each coverage page (S64: the kept coverage is read, not copied), a
    /// tile of nothing in their format, and the slots taken this frame, to set to it.
    reads: Vec<wgpu::BindGroup>,
    nothing: Option<Target>,
    taken: Vec<SlotAt>,
}

/// A stroke's kept coverage (or density) and contact in one tile: which of its 32-px cells (8 a
/// row) the slots hold kept spans in, and which the spans settled this frame reach (the frame
/// copies them into the slots: `drawing_kept.rs`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Kept {
    pub(crate) slots: [SlotAt; 2],
    pub(crate) held: u64,
    pub(crate) laid: u64,
}

/// A tile's snapshot: its targets each in a slot, the unit it stands before, and which it is.
#[derive(Debug, Clone)]
pub(crate) struct Under {
    pub(crate) slots: Vec<(Scratch, SlotAt)>,
    pub(crate) at: usize,
    pub(crate) made: u64,
}

/// What the snapshots stand for: a version, the place a stroke lays (its layer, on the mask), the
/// unit it lays at there and what its snapshots hold; `epoch` counts the changes that let the
/// snapshots being taken go.
#[derive(Debug, Clone)]
pub(crate) struct Base {
    pub(crate) artwork: Artwork,
    pub(crate) place: (LayerId, bool),
    pub(crate) at: usize,
    pub(crate) restored: Restored,
    pub(crate) epoch: u64,
}

/// A snapshot the raster worker takes: its job, the tile, the snapshot it starts from (`None`:
/// from the paper) and the epoch then, the slots it keeps into, the unit it stands before.
#[derive(Debug, Clone)]
pub(crate) struct Taking {
    pub(crate) id: u64,
    pub(crate) tile: [u32; 2],
    pub(crate) from: Option<u64>,
    pub(crate) epoch: u64,
    pub(crate) slots: Vec<(Scratch, SlotAt)>,
    pub(crate) at: usize,
}

/// Pages of slots, a texture each, and which slots are free.
#[derive(Default)]
struct Pool {
    pages: Vec<Target>,
    /// Per page, per slot (row-major): taken.
    taken: Vec<Vec<bool>>,
}

impl Drawing {
    /// Whether a stroke is tracked: one is drawn, or was until the last frame (its lift or its
    /// drop is still to draw).
    pub(crate) fn tracking(&self) -> bool {
        self.track.tracking()
    }

    /// Bytes of its pages.
    pub(crate) fn bytes(&self) -> u64 {
        self.snapshots.bytes() + self.coverage.bytes()
    }

    /// The pages its slots lie in, for the plans' slot passes.
    pub(crate) fn pages(&self) -> Pages<'_> {
        Pages {
            snapshots: &self.snapshots.pages,
            coverage: &self.coverage.pages,
            reads: &self.reads,
            nothing: self.nothing.as_ref(),
        }
    }

    /// The bytes of the snapshot pages `slots` more snapshot slots would make (the float
    /// targets' texels: 16 bytes).
    pub(crate) fn bytes_for(&self, slots: usize) -> u64 {
        let free: usize = (self.snapshots.taken.iter())
            .map(|taken| taken.iter().filter(|taken| !**taken).count())
            .sum();
        let per_page = (SNAPSHOT_PAGE * SNAPSHOT_PAGE) as usize;
        let pages = slots.saturating_sub(free).div_ceil(per_page) as u64;
        let side = u64::from(SNAPSHOT_PAGE * TIER_TILE);
        pages * side * side * 16
    }

    /// The snapshot pages, for a job of the raster worker's.
    pub(crate) fn snapshot_pages(&self) -> Vec<Target> {
        self.snapshots.pages.clone()
    }

    /// The stroke's kept coverage goes (its lift, its drop, a new stroke); the pages stay.
    pub(crate) fn end_stroke(&mut self) {
        self.kept.clear();
        self.coverage.free_all();
        self.taken.clear();
    }

    /// Every snapshot goes (a new place); those being taken are let go once answered.
    pub(crate) fn forget_all(&mut self) {
        let tiles: Vec<[u32; 2]> = self.under.keys().copied().collect();
        for tile in tiles {
            self.forget(tile);
        }
        if let Some(base) = &mut self.base {
            base.epoch += 1;
        }
    }

    /// Lets the pages go too (the GPU memory budget's step, a trim), unless a stroke is tracked
    /// or the worker takes snapshots into them; returns their bytes.
    pub(crate) fn let_go(&mut self) -> u64 {
        if self.tracking() || !self.taking.is_empty() {
            return 0;
        }
        let bytes = self.bytes();
        self.end_stroke();
        self.forget_all();
        self.base = None;
        self.snapshots = Pool::default();
        self.coverage = Pool::default();
        self.reads.clear();
        self.nothing = None;
        self.taken.clear();
        bytes
    }

    /// Whether tile `tile`'s snapshot stands before the unit the place's stroke lays at.
    pub(crate) fn up_to_date(&self, tile: [u32; 2]) -> bool {
        match (&self.base, self.under.get(&tile)) {
            (Some(base), Some(under)) => under.at == base.at,
            _ => false,
        }
    }

    /// Of `tiles`, up to `most` whose snapshot is missing or lacks strokes and is not being taken.
    pub(crate) fn wanted(
        &self,
        tiles: impl IntoIterator<Item = [u32; 2]>,
        most: usize,
    ) -> Vec<[u32; 2]> {
        (tiles.into_iter())
            .filter(|tile| !self.up_to_date(*tile))
            .filter(|tile| self.taking.iter().all(|taking| taking.tile != *tile))
            .take(most)
            .collect()
    }

    /// Tile `tile`'s snapshot goes.
    pub(crate) fn forget(&mut self, tile: [u32; 2]) {
        if let Some(under) = self.under.remove(&tile) {
            for (_, slot) in under.slots {
                self.snapshots.free(slot);
            }
        }
    }

    /// Slots for a snapshot of `targets`, in pages of `format`.
    pub(crate) fn snapshot_slots(
        &mut self,
        core: &Core,
        (targets, format): (&[Scratch], wgpu::TextureFormat),
    ) -> Result<Vec<(Scratch, SlotAt)>, RenderError> {
        let side = SNAPSHOT_PAGE * TIER_TILE;
        let size = ([side; 2], format, wgpu::TextureUsages::COPY_DST);
        let mut slots = Vec::with_capacity(targets.len());
        for target in targets {
            slots.push((*target, self.snapshots.take(core, size)?));
        }
        Ok(slots)
    }

    /// Tile `tile`'s snapshot is now `slots`, standing before unit `at` (the slots it had freed
    /// unless they are the same).
    pub(crate) fn set_under(
        &mut self,
        tile: [u32; 2],
        (slots, at): (Vec<(Scratch, SlotAt)>, usize),
    ) {
        if let Some(old) = self.under.get(&tile)
            && old.slots != slots
        {
            self.forget(tile);
        }
        self.made += 1;
        let made = self.made;
        self.under.insert(tile, Under { slots, at, made });
    }

    /// Frees snapshot slots `slots` (a snapshot taken for nothing).
    pub(crate) fn free_slots(&mut self, slots: &[(Scratch, SlotAt)]) {
        for (_, slot) in slots {
            self.snapshots.free(*slot);
        }
    }
}

impl Drawing {
    /// The raster worker's answer `done`, when it is to a snapshot taken for this drawing: kept
    /// as the tile's snapshot unless a change reached the tile since or another snapshot of it
    /// came first (its slots freed then).
    pub(crate) fn answer(&mut self, done: &RasterDone) -> bool {
        let Some(at) = self.taking.iter().position(|taking| taking.id == done.id) else {
            return false;
        };
        let taking = self.taking.remove(at);
        let made = self.under.get(&taking.tile).map(|under| under.made);
        let epoch = self.base.as_ref().map(|base| base.epoch);
        if done.drawn && epoch == Some(taking.epoch) && made == taking.from {
            self.set_under(taking.tile, (taking.slots, taking.at));
        } else {
            self.free_slots(&taking.slots);
        }
        true
    }
}

/// A snapshot's targets in `slots` as copies of `size` pixels from `offset` in each slot and `at`
/// in a region.
pub(crate) fn slot_copies(
    slots: &[(Scratch, SlotAt)],
    (offset, at, size): ([u32; 2], [u32; 2], [u32; 2]),
) -> Vec<SlotCopy> {
    (slots.iter())
        .map(|(target, slot)| SlotCopy {
            target: *target,
            slot: *slot,
            offset,
            at,
            size,
        })
        .collect()
}

/// Output tile `tile`'s pixels inside an output of `size` pixels.
pub(crate) fn tile_rect(tile: [u32; 2], size: [u32; 2]) -> PixelRect {
    let min = tile.map(|at| at * TIER_TILE);
    PixelRect {
        min: min.map(|at| at as i32),
        max: [0, 1].map(|axis| (min[axis] + TIER_TILE).min(size[axis]) as i32),
    }
}

impl Pool {
    fn bytes(&self) -> u64 {
        (self.pages.iter())
            .map(|page| {
                let texture = &page.texture;
                let texel = texture.format().block_copy_size(None).unwrap_or(16);
                u64::from(texture.width()) * u64::from(texture.height()) * u64::from(texel)
            })
            .sum()
    }

    fn free_all(&mut self) {
        for taken in &mut self.taken {
            taken.fill(false);
        }
    }

    fn free(&mut self, slot: SlotAt) {
        let Some(page) = self.pages.get(slot.page) else {
            return;
        };
        let across = page.texture.width() / TIER_TILE;
        let at = (slot.at[1] / TIER_TILE * across + slot.at[0] / TIER_TILE) as usize;
        if let Some(taken) = self.taken[slot.page].get_mut(at) {
            *taken = false;
        }
    }

    /// A free slot, a page of `size` pixels, `format` and `usage` made when none is left.
    fn take(
        &mut self,
        core: &Core,
        (size, format, usage): ([u32; 2], wgpu::TextureFormat, wgpu::TextureUsages),
    ) -> Result<SlotAt, RenderError> {
        let free = (self.taken.iter()).enumerate().find_map(|(page, taken)| {
            let at = taken.iter().position(|taken| !taken)?;
            Some((page, at))
        });
        let (page, at) = match free {
            Some(free) => free,
            None => {
                let texture = target_texture(&core.gpu, (size, format), usage)?;
                self.pages.push(Target::of(texture));
                let slots = (size[0] / TIER_TILE) * (size[1] / TIER_TILE);
                self.taken.push(vec![false; slots as usize]);
                (self.pages.len() - 1, 0)
            }
        };
        self.taken[page][at] = true;
        let across = self.pages[page].texture.width() / TIER_TILE;
        let at = at as u32;
        Ok(SlotAt {
            page,
            at: [at % across * TIER_TILE, at / across * TIER_TILE],
        })
    }
}

#[path = "drawing_follow.rs"]
mod follow;

#[path = "drawing_coverage.rs"]
mod coverage;

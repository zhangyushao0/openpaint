//! `Arena`: a target's derived raster boundaries in one word buffer, so the evaluator binds them
//! all at once and each op names its block: finished smudge residuals (a residual's header, its
//! table cut to the tiles it holds and their nodes, `Working::block`) and filter results (S33,
//! its rectangle and its pixels, `filter.rs`). Blocks are appended; when the buffer is full, the
//! blocks the scene still shows move into a new one and the others go.
//!
//! S51: the arena keeps within a room (the GPU memory budget's share, else the device's binding
//! limit). A residual is derived from its stroke (D46), so it is a cache: when the blocks shown
//! pass the room, the least recently needed residuals go, never one the evaluation at hand needs,
//! and a smudge's residual may hold only the tiles near what is evaluated (`TileSet`); either is
//! evaluated again from the artwork when a later evaluation needs it. It never exceeds the
//! device's storage binding limit.

use std::collections::{BTreeSet, HashMap, HashSet};

use openpaint_core::{LayerId, Stroke};

use super::RESIDUAL_TILE;
use super::residual::{Working, words_buffer};
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::gpu::Gpu;
use crate::protocol::encode;

/// A block's owner. A finished smudge: its layer and its samples' address; while its block
/// holds the stroke, the address cannot be reused, and a stroke's layer content before it never
/// changes once it is committed (strokes and edits are only ever appended), so the key names
/// one residual. A filter result: the number `Filters` gave what it depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    Smudge(LayerId, usize),
    Filter(u64),
}

/// Residual tiles (`RESIDUAL_TILE` canvas pixels a side) by number, row major over the canvas
/// (as a residual's table numbers them): the part of a residual a block holds, or an evaluation
/// needs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TileSet(pub(crate) BTreeSet<u32>);

impl TileSet {
    /// The residual tiles meeting the canvas pixels `rect` on a canvas of `size` pixels.
    pub(crate) fn of(rect: &PixelRect, size: [u32; 2]) -> Self {
        let tiles = size.map(|side| side.div_ceil(RESIDUAL_TILE));
        let side = i64::from(RESIDUAL_TILE);
        let at = |value: i32, most: u32, up: i64| {
            let tile = (i64::from(value) + up).div_euclid(side);
            u32::try_from(tile.clamp(0, i64::from(most))).unwrap_or(most)
        };
        let [x0, y0] = [0, 1].map(|axis| at(rect.min[axis], tiles[axis], 0));
        let [x1, y1] = [0, 1].map(|axis| at(rect.max[axis], tiles[axis], side - 1));
        Self(
            (y0..y1)
                .flat_map(|row| (x0..x1).map(move |column| row * tiles[0] + column))
                .collect(),
        )
    }

    /// The tiles in either.
    pub(crate) fn join(&mut self, other: &Self) {
        self.0.extend(other.0.iter().copied());
    }

    fn holds(&self, other: &Self) -> bool {
        other.0.is_subset(&self.0)
    }

    /// Whether residual tile `tile` lies in it.
    pub(crate) fn has(&self, tile: u32) -> bool {
        self.0.contains(&tile)
    }
}

/// What a reservation keeps: the blocks the scene shows (others go first), those the evaluation
/// at hand needs (never dropped), the room in bytes the arena keeps within when it can, and the
/// device's binding limit, which it never passes.
pub(crate) struct Keep<'a> {
    pub(crate) shown: &'a HashSet<Key>,
    pub(crate) needed: &'a HashSet<Key>,
    pub(crate) room: u64,
    pub(crate) limit: u64,
}

/// Words the first arena holds.
const FIRST_WORDS: u64 = 1 << 18;

struct Block {
    base: u64,
    words: u64,
    /// The residual tiles it holds; `None`: all of them (and every filter result).
    part: Option<TileSet>,
    /// The evaluation that last needed it (`Arena::touch`): the least recently needed go first.
    used: u64,
    /// Keeps a smudge's samples, and with them its key, alive.
    _stroke: Option<Stroke>,
}

pub(crate) struct Arena {
    buffer: wgpu::Buffer,
    /// Words the buffer holds, and those taken.
    capacity: u64,
    used: u64,
    blocks: HashMap<Key, Block>,
    /// The evaluation being prepared, counted.
    now: u64,
}

/// The key of `stroke` on `layer`.
pub(crate) fn key(layer: LayerId, stroke: &Stroke) -> Key {
    Key::Smudge(layer, stroke.samples().as_ptr() as usize)
}

impl Arena {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        Self {
            buffer: words_buffer(gpu, 4),
            capacity: 4,
            used: 0,
            blocks: HashMap::new(),
            now: 0,
        }
    }

    pub(crate) const fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Bytes its buffer holds.
    pub(crate) fn bytes(&self) -> u64 {
        self.buffer.size()
    }

    /// Where the block of `key` starts, in words.
    pub(crate) fn base(&self, key: &Key) -> Option<u64> {
        self.blocks.get(key).map(|block| block.base)
    }

    /// Whether the block of `key` holds the residual tiles `part` (`None`: all of them).
    pub(crate) fn holds(&self, key: &Key, part: Option<&TileSet>) -> bool {
        self.blocks
            .get(key)
            .is_some_and(|block| match (&block.part, part) {
                (None, _) => true,
                (Some(held), Some(wanted)) => held.holds(wanted),
                (Some(_), None) => false,
            })
    }

    /// The tiles the block of `key` holds, when it holds only some.
    pub(crate) fn part(&self, key: &Key) -> Option<&TileSet> {
        self.blocks.get(key).and_then(|block| block.part.as_ref())
    }

    /// Whether the block of `key` holds a whole residual.
    pub(crate) fn whole(&self, key: &Key) -> bool {
        self.blocks
            .get(key)
            .is_some_and(|block| block.part.is_none())
    }

    /// Lets the block of `key` go (its room is given back when the buffer is compacted).
    pub(crate) fn forget(&mut self, key: &Key) {
        self.blocks.remove(key);
    }

    /// Words the block of `key` holds (0 without one).
    pub(crate) fn words(&self, key: &Key) -> u64 {
        self.blocks.get(key).map_or(0, |block| block.words)
    }

    /// A new frame begins: blocks its evaluations touch count as needed most recently.
    pub(crate) fn next(&mut self) {
        self.now += 1;
    }

    /// The block of `key` is needed by an evaluation of this frame.
    pub(crate) fn touch(&mut self, key: &Key) {
        if let Some(block) = self.blocks.get_mut(key) {
            block.used = self.now;
        }
    }

    /// The keys this frame's evaluations needed so far.
    pub(crate) fn last_needed(&self) -> HashSet<Key> {
        let now = self.now;
        (self.blocks.iter())
            .filter(|(_, block)| block.used == now)
            .map(|(key, _)| *key)
            .collect()
    }

    /// Keeps only blocks `keep` shows, within its room (the least recently needed go first,
    /// never a needed one), in a buffer just large enough (S49's third step, S51's room).
    ///
    /// # Errors
    /// As `reserve`.
    pub(crate) fn shrink(&mut self, gpu: &Gpu, keep: &Keep<'_>) -> Result<(), RenderError> {
        self.compact(gpu, keep, 0)
    }

    /// Copies the tiles `part` of `working` (`None`: all) in as the block of `key`, keeping
    /// `stroke` alive with it. The bases of other blocks may change (`reserve`), so they are
    /// read again after this.
    ///
    /// # Errors
    /// As `reserve`.
    pub(crate) fn place(
        &mut self,
        gpu: &Gpu,
        (key, stroke): (Key, &Stroke),
        (working, part): (&Working, Option<TileSet>),
        keep: &Keep<'_>,
    ) -> Result<(), RenderError> {
        let block = working.block(part.as_ref());
        let words = block.head.len() as u64 + block.node_words;
        let base = self.reserve(gpu, (key, Some(stroke.clone())), words, keep)?;
        if let Some(entry) = self.blocks.get_mut(&key) {
            entry.part = part;
        }
        gpu.queue
            .write_buffer(&self.buffer, base * 4, &encode(&block.head));
        if block.node_words > 0 {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            let nodes = base + block.head.len() as u64;
            for (from, to, words) in block.runs {
                let source = (block.nodes_at + from) * 4;
                encoder.copy_buffer_to_buffer(
                    working.buffer(),
                    source,
                    &self.buffer,
                    (nodes + to) * 4,
                    words * 4,
                );
            }
            gpu.queue.submit([encoder.finish()]);
        }
        Ok(())
    }

    /// Room for a block of `words` words owned by `key` (keeping `stroke` alive with it; one
    /// it held before is let go), its base returned; the caller fills it. When the buffer is
    /// full, the blocks `keep` keeps move into a new one, so the bases of the others change.
    ///
    /// # Errors
    /// `Admission` when the needed blocks and this one exceed the binding limit.
    pub(crate) fn reserve(
        &mut self,
        gpu: &Gpu,
        (key, stroke): (Key, Option<Stroke>),
        words: u64,
        keep: &Keep<'_>,
    ) -> Result<u64, RenderError> {
        self.blocks.remove(&key);
        // The last blocks no scene shows give their room back: a preview's earlier value (S35).
        while let Some((key, base)) = self
            .blocks
            .iter()
            .find(|(key, block)| block.base + block.words == self.used && !keep.shown.contains(key))
            .map(|(key, block)| (*key, block.base))
        {
            self.blocks.remove(&key);
            self.used = base;
        }
        if self.used + words > self.capacity {
            self.compact(gpu, keep, words)?;
        }
        let base = self.used;
        self.used += words;
        let block = Block {
            base,
            words,
            part: None,
            used: self.now,
            _stroke: stroke,
        };
        self.blocks.insert(key, block);
        Ok(base)
    }

    /// Words the blocks hold that `needed` does not name: what a cut to the room could give back.
    pub(crate) fn spare_words(&self, needed: &HashSet<Key>) -> u64 {
        (self.blocks.iter())
            .filter(|(key, _)| !needed.contains(key))
            .map(|(_, block)| block.words)
            .sum()
    }

    /// Drops the least recently needed blocks `needed` does not name (with `frame`, this frame's
    /// too) while the words `held` pass `most`; returns the words held then.
    fn evict(&mut self, needed: &HashSet<Key>, (mut held, most): (u64, u64), frame: bool) -> u64 {
        if held <= most {
            return held;
        }
        let now = self.now;
        let mut spare: Vec<(u64, Key, u64)> = (self.blocks.iter())
            .filter(|(key, block)| !needed.contains(key) && (frame || block.used != now))
            .map(|(key, block)| (block.used, *key, block.words))
            .collect();
        spare.sort_unstable_by_key(|(used, _, words)| (*used, u64::MAX - words));
        for (_, key, words) in spare {
            if held <= most {
                break;
            }
            self.blocks.remove(&key);
            held -= words;
        }
        held
    }

    /// Moves the blocks `keep` keeps into a buffer with room for `more` words beyond them: every
    /// block not shown goes, and while they pass the room, the least recently needed of those
    /// not needed now, down to three quarters of it, so that the next blocks find room without
    /// another move; this frame's other evaluations' blocks go last, and only to stay within
    /// what the device binds. Past the room the buffer keeps half as much again spare.
    fn compact(&mut self, gpu: &Gpu, keep: &Keep<'_>, more: u64) -> Result<(), RenderError> {
        self.blocks.retain(|key, _| keep.shown.contains(key));
        let (room, limit) = (keep.room.min(keep.limit) / 4, keep.limit / 4);
        let mut kept: u64 = self.blocks.values().map(|block| block.words).sum();
        for (most, frame) in [(room / 4 * 3, false), (limit / 4 * 3, true)] {
            kept = self.evict(keep.needed, (kept + more, most), frame) - more;
        }
        let needed = kept + more;
        let words = if needed <= room {
            needed.next_power_of_two().max(FIRST_WORDS).min(room)
        } else {
            needed + needed / 2
        }
        .min(limit);
        if needed > words {
            log::warn!(
                "the arena cannot bind what is needed: {} blocks, {} needed now, {:.1} MB",
                self.blocks.len(),
                keep.needed.len(),
                (needed * 4) as f64 / (1024.0 * 1024.0)
            );
            return Err(RenderError::Admission {
                what: "smudge and filter arena",
                limit: keep.limit,
            });
        }
        let buffer = words_buffer(gpu, words);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let mut used = 0;
        let mut blocks: Vec<&mut Block> = self.blocks.values_mut().collect();
        blocks.sort_by_key(|block| block.base);
        for block in blocks {
            encoder.copy_buffer_to_buffer(
                &self.buffer,
                block.base * 4,
                &buffer,
                used * 4,
                block.words * 4,
            );
            block.base = used;
            used += block.words;
        }
        gpu.queue.submit([encoder.finish()]);
        self.buffer = buffer;
        self.capacity = words;
        self.used = used;
        Ok(())
    }
}

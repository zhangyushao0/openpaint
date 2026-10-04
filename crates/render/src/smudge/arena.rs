//! `Arena`: a target's finished smudge residuals in one word buffer, so the evaluator binds them
//! all at once and each smudge op names its block. A block is a residual's header, its table cut
//! to its tiles and its nodes (`Working::block`). Blocks are appended; when the buffer is full,
//! the blocks the current scene still shows move into a larger one and the others go (they are
//! rebuilt from the artwork if a later version shows them again).

use std::collections::{HashMap, HashSet};

use openpaint_core::{LayerId, Stroke};

use super::residual::{Working, words_buffer};
use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::protocol::encode;

/// A finished smudge: its layer and its samples' address. While its block holds the stroke,
/// the address cannot be reused, and a stroke's layer content before it never changes once it
/// is committed (strokes and edits are only ever appended), so the key names one residual.
pub(crate) type Key = (LayerId, usize);

/// Words the first arena holds.
const FIRST_WORDS: u64 = 1 << 18;

struct Block {
    base: u64,
    words: u64,
    /// Keeps the samples, and with them the key, alive.
    _stroke: Stroke,
}

pub(crate) struct Arena {
    buffer: wgpu::Buffer,
    /// Words the buffer holds, and those taken.
    capacity: u64,
    used: u64,
    blocks: HashMap<Key, Block>,
}

/// The key of `stroke` on `layer`.
pub(crate) fn key(layer: LayerId, stroke: &Stroke) -> Key {
    (layer, stroke.samples().as_ptr() as usize)
}

impl Arena {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        Self {
            buffer: words_buffer(gpu, 4),
            capacity: 4,
            used: 0,
            blocks: HashMap::new(),
        }
    }

    pub(crate) const fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Where the block of `key` starts, in words.
    pub(crate) fn base(&self, key: &Key) -> Option<u64> {
        self.blocks.get(key).map(|block| block.base)
    }

    /// Copies `working` in as the block of `key`, keeping `stroke` alive with it. When the
    /// buffer is full, only the blocks of `live` move into the new one; the bases of the others
    /// change, so they are read again after this.
    ///
    /// # Errors
    /// `Admission` when the live blocks and this one exceed `limit` bytes.
    pub(crate) fn place(
        &mut self,
        gpu: &Gpu,
        (key, stroke): (Key, &Stroke),
        working: &Working,
        (live, limit): (&HashSet<Key>, u64),
    ) -> Result<(), RenderError> {
        let block = working.block();
        let words = block.head.len() as u64 + block.node_words;
        if self.used + words > self.capacity {
            self.compact(gpu, live, words, limit)?;
        }
        let base = self.used;
        gpu.queue
            .write_buffer(&self.buffer, base * 4, &encode(&block.head));
        if block.node_words > 0 {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            let at = (base + block.head.len() as u64) * 4;
            let bytes = block.node_words * 4;
            encoder.copy_buffer_to_buffer(
                working.buffer(),
                block.nodes_at * 4,
                &self.buffer,
                at,
                bytes,
            );
            gpu.queue.submit([encoder.finish()]);
        }
        self.used += words;
        let stroke = stroke.clone();
        let block = Block {
            base,
            words,
            _stroke: stroke,
        };
        self.blocks.insert(key, block);
        Ok(())
    }

    /// Moves the blocks of `live` into a buffer with room for `more` words beyond them; drops
    /// every other block.
    fn compact(
        &mut self,
        gpu: &Gpu,
        live: &HashSet<Key>,
        more: u64,
        limit: u64,
    ) -> Result<(), RenderError> {
        self.blocks.retain(|key, _| live.contains(key));
        let kept: u64 = self.blocks.values().map(|block| block.words).sum();
        let needed = kept + more;
        let words = needed.next_power_of_two().max(FIRST_WORDS).min(limit / 4);
        if needed > words {
            return Err(RenderError::Admission {
                what: "smudge arena",
                limit,
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

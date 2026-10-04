//! A residual being evaluated (`Working`): one word buffer holding its header, a tile table over
//! the whole canvas and the nodes of the tiles its contacts reached so far, in the order they
//! were reached. A node is `A` then `R` (`NODE_WORDS` floats as bits); a tile not in the table
//! is the identity (`A` 1, `R` 0). Growing keeps every node where it is, so a residual grown a
//! few contacts at a time holds the same values as one grown at once.

use std::collections::BTreeSet;

use super::{NODE_WORDS, RESIDUAL_HEADER, RESIDUAL_TILE, TILE_NODES};
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::gpu::Gpu;

pub(crate) struct Working {
    size: [u32; 2],
    columns: u32,
    /// Per tile, row major: its slot, one more than its place among the allocated tiles; 0 for
    /// none.
    table: Vec<u32>,
    /// The allocated tiles in slot order.
    tiles: Vec<u32>,
    /// Tiles the node words have room for.
    capacity: u32,
    buffer: wgpu::Buffer,
}

/// A residual as an arena block: its header and table cut to the tiles it holds, and where its
/// nodes lie in the working buffer.
pub(crate) struct Block {
    pub(crate) head: Vec<u32>,
    pub(crate) nodes_at: u64,
    pub(crate) node_words: u64,
}

impl Working {
    /// An empty residual for a canvas of `size` pixels.
    pub(crate) fn new(gpu: &Gpu, size: [u32; 2]) -> Self {
        let columns = size[0].div_ceil(RESIDUAL_TILE);
        let rows = size[1].div_ceil(RESIDUAL_TILE);
        let table = vec![0; (columns * rows) as usize];
        let buffer = words_buffer(gpu, u64::from(RESIDUAL_HEADER) + table.len() as u64);
        let working = Self {
            size,
            columns,
            table,
            tiles: Vec::new(),
            capacity: 0,
            buffer,
        };
        working.write_head(gpu);
        working
    }

    pub(crate) const fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// The canvas's size in pixels (one node each).
    pub(crate) const fn size(&self) -> [u32; 2] {
        self.size
    }

    pub(crate) fn tiles(&self) -> &[u32] {
        &self.tiles
    }

    /// The canvas pixels of residual tile `tile`.
    pub(crate) fn tile_rect(&self, tile: u32) -> PixelRect {
        let (column, row) = (tile % self.columns, tile / self.columns);
        let side = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
        let origin = [column * RESIDUAL_TILE, row * RESIDUAL_TILE];
        PixelRect {
            min: origin.map(side),
            max: [
                side((origin[0] + RESIDUAL_TILE).min(self.size[0])),
                side((origin[1] + RESIDUAL_TILE).min(self.size[1])),
            ],
        }
    }

    /// Word where the nodes start.
    fn nodes_at(&self) -> u64 {
        u64::from(RESIDUAL_HEADER) + self.table.len() as u64
    }

    /// Allocates the tiles of `wanted` it does not hold yet, keeping every node where it is;
    /// returns them in slot order. The buffer never exceeds `limit` bytes.
    ///
    /// # Errors
    /// `Admission` when the residual would not fit `limit`.
    pub(crate) fn allocate(
        &mut self,
        gpu: &Gpu,
        wanted: &BTreeSet<u32>,
        limit: u64,
    ) -> Result<Vec<u32>, RenderError> {
        let new: Vec<u32> = wanted
            .iter()
            .copied()
            .filter(|tile| self.table[*tile as usize] == 0)
            .collect();
        if new.is_empty() {
            return Ok(new);
        }
        let needed = self.tiles.len() as u32 + new.len() as u32;
        if needed > self.capacity {
            let capacity = needed.max(self.capacity * 2).max(4);
            let bytes = |tiles: u32| 4 * (self.nodes_at() + node_words(tiles));
            let capacity = if bytes(capacity) > limit {
                needed
            } else {
                capacity
            };
            if bytes(capacity) > limit {
                return Err(RenderError::Admission {
                    what: "smudge residual",
                    limit,
                });
            }
            self.grow(gpu, capacity, bytes(capacity));
        }
        for tile in &new {
            self.tiles.push(*tile);
            self.table[*tile as usize] = self.tiles.len() as u32;
        }
        self.write_head(gpu);
        Ok(new)
    }

    /// A buffer for `capacity` tiles holding the nodes so far; the head is written after.
    fn grow(&mut self, gpu: &Gpu, capacity: u32, bytes: u64) {
        let grown = words_buffer(gpu, bytes / 4);
        let used = 4 * node_words(self.tiles.len() as u32);
        if used > 0 {
            let at = 4 * self.nodes_at();
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(&self.buffer, at, &grown, at, used);
            gpu.queue.submit([encoder.finish()]);
        }
        self.buffer = grown;
        self.capacity = capacity;
    }

    /// The header and the table; written apart from the nodes, so a copy never meets it.
    fn write_head(&self, gpu: &Gpu) {
        let rows = self.table.len() as u32 / self.columns.max(1);
        let mut words = vec![0, 0, self.columns, rows];
        words.extend_from_slice(&self.table);
        gpu.queue
            .write_buffer(&self.buffer, 0, &crate::protocol::encode(&words));
    }

    /// The residual as an arena block: the table cut to the box of its tiles.
    pub(crate) fn block(&self) -> Block {
        let (mut min, mut max) = ([u32::MAX; 2], [0; 2]);
        for tile in &self.tiles {
            let at = [tile % self.columns, tile / self.columns];
            for axis in 0..2 {
                min[axis] = min[axis].min(at[axis]);
                max[axis] = max[axis].max(at[axis]);
            }
        }
        let mut head = vec![0; RESIDUAL_HEADER as usize];
        if !self.tiles.is_empty() {
            let size = [max[0] - min[0] + 1, max[1] - min[1] + 1];
            head = vec![min[0], min[1], size[0], size[1]];
            for row in min[1]..=max[1] {
                let start = (row * self.columns + min[0]) as usize;
                head.extend_from_slice(&self.table[start..start + size[0] as usize]);
            }
        }
        Block {
            head,
            nodes_at: self.nodes_at(),
            node_words: node_words(self.tiles.len() as u32),
        }
    }
}

/// Node words of `tiles` tiles.
pub(crate) fn node_words(tiles: u32) -> u64 {
    u64::from(tiles) * u64::from(TILE_NODES) * u64::from(NODE_WORDS)
}

/// A storage buffer of `words` words that copies may read and write.
pub(crate) fn words_buffer(gpu: &Gpu, words: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("smudge residual"),
        size: (words * 4).max(16),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

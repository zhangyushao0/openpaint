//! `Snapshot`: a layer's pixels at float precision, kept per 16-pixel tile of the canvas (S35's
//! adjustment previews, S48's in-progress stroke). One storage buffer holds a table with an
//! element per canvas tile (the slot holding it in its first float: exact below 2^24), then the
//! slots,
//! one tile's 256 premultiplied, unrounded colours each, in the order the tiles were taken. An
//! evaluation writes a tile's pixels there instead of the target (`params.snapshot.w`), and
//! `OP_RESUME` reads them back exactly, so resuming a layer from it equals evaluating its ops.
//!
//! The buffer grows by doubling (its slots copied over) up to the device's storage binding and
//! buffer limits and the room the GPU memory budget leaves it (S49); a tile beyond them is
//! refused and its evaluations take every op again.

use crate::core::Core;
use crate::program::SnapshotUse;
use crate::protocol::TILE;

/// Floats per pixel, and the bytes of one tile's slot.
const PIXEL_BYTES: u64 = 16;
const SLOT_BYTES: u64 = (TILE * TILE) as u64 * PIXEL_BYTES;
/// Slots a new snapshot has room for: 1 MB.
const FIRST_SLOTS: u32 = 256;

pub(crate) struct Snapshot {
    buffer: wgpu::Buffer,
    /// The canvas's tiles per row.
    columns: u32,
    /// Per canvas tile, row major: its slot + 1, or 0 while not held.
    slots: Vec<u32>,
    used: u32,
    capacity: u32,
    /// Four-float elements the table takes before the first slot: one per canvas tile.
    head: u32,
}

impl Snapshot {
    /// An empty snapshot of a canvas of `size` pixels within `room` bytes; `None` when the room
    /// cannot hold its table and one tile (S49).
    pub(crate) fn new(core: &Core, size: [u32; 2], room: u64) -> Option<Self> {
        let [columns, rows] = size.map(|side| side.div_ceil(TILE));
        let tiles = columns as usize * rows as usize;
        let head = u32::try_from(tiles).unwrap_or(u32::MAX);
        let most = Self::most(core, head, room);
        if most == 0 {
            return None;
        }
        let capacity = FIRST_SLOTS.min(most);
        Some(Self {
            buffer: buffer(core, head, capacity),
            columns,
            slots: vec![0; tiles],
            used: 0,
            capacity,
            head,
        })
    }

    /// The most slots a buffer after a table of `head` units can hold on this device within
    /// `room` bytes.
    fn most(core: &Core, head: u32, room: u64) -> u32 {
        let limits = core.gpu.device.limits();
        let bytes = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size)
            .min(room);
        let room = bytes.saturating_sub(u64::from(head) * PIXEL_BYTES) / SLOT_BYTES;
        u32::try_from(room).unwrap_or(u32::MAX)
    }

    /// Bytes its buffer holds.
    pub(crate) fn bytes(&self) -> u64 {
        self.buffer.size()
    }

    /// The canvas's tiles.
    pub(crate) fn count(&self) -> usize {
        self.slots.len()
    }

    /// The canvas tile holding `pixel` (row major).
    pub(crate) fn tile_of(&self, [x, y]: [u32; 2]) -> usize {
        (y / TILE) as usize * self.columns as usize + (x / TILE) as usize
    }

    /// Whether canvas tile `tile` (row major) is held.
    pub(crate) fn holds(&self, tile: usize) -> bool {
        self.slots.get(tile).is_some_and(|slot| *slot > 0)
    }

    /// Slots for the canvas tiles `tiles` (row major, none held yet), the buffer grown as needed
    /// within `room` bytes; their pixels are undefined until an evaluation writes them. False,
    /// taking none, when they would pass the device's limits or the room.
    pub(crate) fn take(&mut self, core: &Core, tiles: &[usize], room: u64) -> bool {
        let Some(needed) = u32::try_from(tiles.len())
            .ok()
            .and_then(|count| self.used.checked_add(count))
        else {
            return false;
        };
        if needed > self.capacity {
            let most = Self::most(core, self.head, room);
            if needed > most {
                log::info!(
                    "snapshot full: {needed} tiles exceed the {most} the device and budget hold"
                );
                return false;
            }
            self.grow(core, needed.max(self.capacity.saturating_mul(2)).min(most));
        }
        let mut table = Vec::with_capacity(tiles.len());
        for &tile in tiles {
            self.slots[tile] = self.used + 1;
            table.push((tile, self.used as f32));
            self.used += 1;
        }
        // Each entry where the table holds it; runs of neighbours go as one write.
        table.sort_unstable_by_key(|(tile, _)| *tile);
        for run in table.chunk_by(|a, b| b.0 == a.0 + 1) {
            let values: Vec<u8> = run
                .iter()
                .flat_map(|(_, slot)| [*slot, 0.0, 0.0, 0.0])
                .flat_map(f32::to_le_bytes)
                .collect();
            let at = run[0].0 as u64 * PIXEL_BYTES;
            core.gpu.queue.write_buffer(&self.buffer, at, &values);
        }
        true
    }

    /// Holds no tile any more; the buffer stays for the tiles taken next.
    pub(crate) fn clear(&mut self) {
        self.slots.fill(0);
        self.used = 0;
    }

    /// A buffer of `capacity` slots holding what this one held.
    fn grow(&mut self, core: &Core, capacity: u32) {
        let grown = buffer(core, self.head, capacity);
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.buffer, 0, &grown, 0, self.buffer.size());
        core.gpu.queue.submit([encoder.finish()]);
        self.buffer = grown;
        self.capacity = capacity;
    }

    /// The snapshot as a dispatch binds it, written (`writes`) or read.
    pub(crate) fn reading(&self, writes: bool) -> SnapshotUse<'_> {
        SnapshotUse {
            buffer: &self.buffer,
            layout: [self.columns, self.head],
            writes,
        }
    }
}

/// A buffer for a table of `head` four-float units and `capacity` slots.
fn buffer(core: &Core, head: u32, capacity: u32) -> wgpu::Buffer {
    let usage =
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC;
    core.gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("snapshot"),
        size: u64::from(head) * PIXEL_BYTES + u64::from(capacity.max(1)) * SLOT_BYTES,
        usage,
        mapped_at_creation: false,
    })
}

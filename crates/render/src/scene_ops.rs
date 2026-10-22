//! One scene's ops, fill masks and patches as the GPU buffers the raster core binds.

use std::ops::Range;
use std::sync::{Mutex, PoisonError};

use wgpu::util::DeviceExt;

use crate::edits::patch_bytes;
use crate::gpu::Gpu;
use crate::protocol::{Op, Record, encode};
use crate::scene::{Item, Scene};

/// Room for fresh ops behind a new scene's: the in-progress stroke's new spans and paint op
/// fit in it; a stroke's frame laying a long stroke whole makes the buffer larger once.
const FRESH_ROOM: usize = 256;

/// One scene on the GPU: its ops (item index = op index) followed by the stroke being drawn's
/// fresh ops, its fill masks and its transforms' patches.
pub(crate) struct SceneOps {
    /// The scene's ops, then room for the stroke being drawn's (`Fresh`): the raster core reads
    /// every op from this one buffer, an operand at a time (`word` in raster_paint.wgsl).
    ops: Mutex<wgpu::Buffer>,
    /// The scene's ops: where the fresh ones start.
    count: u32,
    pub(crate) masks: wgpu::Buffer,
    pub(crate) patches: wgpu::Buffer,
}

impl SceneOps {
    /// The buffer holding the scene's ops now.
    pub(crate) fn ops(&self) -> wgpu::Buffer {
        self.ops
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The buffer holding the scene's ops with `fresh` written behind them, made larger first
    /// when they do not fit (the scene's ops copied over on the GPU). The write lands before
    /// the next submission, after every one before it (`wgpu::Queue::write_buffer`), so frames
    /// of one scene with different fresh ops, each submitted after its own write, each read
    /// their own.
    pub(crate) fn with_fresh(&self, gpu: &Gpu, fresh: &[Op]) -> wgpu::Buffer {
        let mut held = self.ops.lock().unwrap_or_else(PoisonError::into_inner);
        let at = u64::from(self.count) * OP_BYTES;
        let needed = at + fresh.len() as u64 * OP_BYTES;
        if needed > held.size() {
            let larger = empty(gpu, needed.next_power_of_two());
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(&held, 0, &larger, 0, at);
            gpu.submit([encoder.finish()]);
            *held = larger;
        }
        if !fresh.is_empty() {
            gpu.queue.write_buffer(&held, at, &encode(fresh));
        }
        held.clone()
    }

    /// The scene's op count: where the fresh ops start.
    pub(crate) const fn count(&self) -> u32 {
        self.count
    }

    /// The ops of `scene`, whose items `inserted` were inserted into the scene these ops hold
    /// (every later item moved by their count), in a buffer of their own: the ops before and
    /// after them copied over on the GPU, theirs written. A raster worker's job may still read
    /// this buffer (it drops the scene before it submits), so it is never changed under one.
    pub(crate) fn inserted(&self, gpu: &Gpu, scene: &Scene, inserted: Range<usize>) -> Self {
        let held = self.ops();
        let count = scene.items.len();
        let ops = empty(gpu, ((count + FRESH_ROOM) as u64) * OP_BYTES);
        let at = inserted.start as u64 * OP_BYTES;
        let after = u64::from(self.count) * OP_BYTES - at;
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        if at > 0 {
            encoder.copy_buffer_to_buffer(&held, 0, &ops, 0, at);
        }
        if after > 0 {
            let to = inserted.end as u64 * OP_BYTES;
            encoder.copy_buffer_to_buffer(&held, at, &ops, to, after);
        }
        let written: Vec<Op> = scene.items[inserted].iter().map(Item::op).collect();
        if !written.is_empty() {
            gpu.queue.write_buffer(&ops, at, &encode(&written));
        }
        gpu.submit([encoder.finish()]);
        Self {
            ops: Mutex::new(ops),
            count: u32::try_from(count).unwrap_or(u32::MAX),
            masks: self.masks.clone(),
            patches: self.patches.clone(),
        }
    }
}

const OP_BYTES: u64 = size_of::<Op>() as u64;

/// The scene's ops (item index = op index), fill masks and patches as GPU buffers; `patches`
/// is a buffer that already holds this scene's patch words (the scene before had the same
/// patches), so a transform being dragged uploads its pixels once.
pub(crate) fn scene_ops(gpu: &Gpu, scene: &Scene, patches: Option<wgpu::Buffer>) -> SceneOps {
    let ops: Vec<Op> = scene.items.iter().map(|item| item.op()).collect();
    let patches = patches.unwrap_or_else(|| {
        let mut bytes = patch_bytes(scene);
        if bytes.is_empty() {
            bytes.resize(4, 0);
        }
        gpu.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("patches"),
                contents: &bytes,
                usage: wgpu::BufferUsages::STORAGE,
            })
    });
    // The smudges' base words are written after their residuals are placed.
    let masks = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    // Rewritten in place when a preview's values change, fresh ops written behind.
    let held = empty(gpu, ((ops.len() + FRESH_ROOM) as u64) * OP_BYTES);
    if !ops.is_empty() {
        gpu.queue.write_buffer(&held, 0, &encode(&ops));
    }
    SceneOps {
        ops: Mutex::new(held),
        count: u32::try_from(ops.len()).unwrap_or(u32::MAX),
        masks: buffer(gpu, masks, &scene.masks),
        patches,
    }
}

/// An op buffer of `bytes`, written by the queue and copied from when it grows.
fn empty(gpu: &Gpu, bytes: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene ops"),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

/// A buffer holding `records`. An empty list still gets one zeroed record, because a binding
/// must hold at least one element; no tile refers to it.
fn buffer<R: Record>(gpu: &Gpu, usage: wgpu::BufferUsages, records: &[R]) -> wgpu::Buffer {
    let mut contents = encode(records);
    if contents.is_empty() {
        contents.resize(size_of::<R>(), 0);
    }
    gpu.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene ops"),
            contents: &contents,
            usage,
        })
}

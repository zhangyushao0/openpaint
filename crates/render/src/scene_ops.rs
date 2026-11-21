//! One scene's ops, fill masks and patches as the GPU buffers the raster core binds.

use std::ops::Range;
use std::sync::{Mutex, PoisonError};

use wgpu::util::DeviceExt;

use crate::edits::patch_bytes;
use crate::gpu::Gpu;
use crate::protocol::{Op, encode};
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
    /// The ops buffer of the scene before, which the next append writes into instead of a new one
    /// (`inserted`, where nothing but the target's own submissions reads it).
    spare: Option<wgpu::Buffer>,
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
    /// (every later item moved by their count), in another buffer: the ops before and after them
    /// copied over on the GPU, theirs written. A raster worker's job may still read this buffer
    /// (it drops the scene before it submits), so it is never changed under one. With `reuse`
    /// (a target no worker draws for, whose submissions read the buffers in order) the other
    /// buffer is the spare, the ops buffer of the scene before this one, and this one's is the
    /// next one's: two buffers taking turns instead of one made each append.
    pub(crate) fn inserted(
        &mut self,
        gpu: &Gpu,
        scene: &Scene,
        (inserted, reuse): (Range<usize>, bool),
    ) -> Self {
        let held = self.ops();
        let count = scene.items.len();
        let needed = ((count + FRESH_ROOM) as u64) * OP_BYTES;
        let spare = (self.spare.take()).filter(|spare| reuse && spare.size() >= needed);
        let ops = spare.unwrap_or_else(|| {
            let bytes = if reuse {
                needed.next_power_of_two()
            } else {
                needed
            };
            empty(gpu, bytes)
        });
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
            spare: reuse.then_some(held),
        }
    }
}

const OP_BYTES: u64 = size_of::<Op>() as u64;

/// The scene's ops (item index = op index), fill masks and patches as GPU buffers; `patches`
/// is a buffer that already holds this scene's patch words (the scene before had the same
/// patches), so a transform being dragged uploads its pixels once. With `before`, the scene
/// before's on a target no worker draws for (its submissions read the buffers in order), its
/// buffers are written over where they hold the new contents (its ops buffer or its spare, the
/// other the spare) instead of new ones made.
pub(crate) fn scene_ops(
    gpu: &Gpu,
    scene: &Scene,
    (patches, before): (Option<wgpu::Buffer>, Option<&SceneOps>),
) -> SceneOps {
    let ops: Vec<Op> = scene.items.iter().map(|item| item.op()).collect();
    let patches = patches.unwrap_or_else(|| {
        let bytes = patch_bytes(scene);
        written(
            gpu,
            ("patches", bytes),
            before.map(|before| &before.patches),
        )
    });
    // The smudges' base words are written after their residuals are placed.
    let masks = written(
        gpu,
        ("scene ops", encode(&scene.masks)),
        before.map(|before| &before.masks),
    );
    // Rewritten in place when a preview's values change, fresh ops written behind.
    let needed = ((ops.len() + FRESH_ROOM) as u64) * OP_BYTES;
    let (held, spare) = match before {
        Some(before) => {
            let (current, spare) = (before.ops(), before.spare.clone());
            match spare {
                Some(spare) if spare.size() >= needed => (spare, Some(current)),
                _ if current.size() >= needed => (current, None),
                _ => (empty(gpu, needed.next_power_of_two()), Some(current)),
            }
        }
        None => (empty(gpu, needed), None),
    };
    if !ops.is_empty() {
        gpu.queue.write_buffer(&held, 0, &encode(&ops));
    }
    SceneOps {
        ops: Mutex::new(held),
        count: u32::try_from(ops.len()).unwrap_or(u32::MAX),
        masks,
        spare,
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

/// A storage buffer named `label` holding `contents` (at least one zeroed record, because a
/// binding must hold at least one element; no tile refers to it): `held` written over when it is
/// large enough, else a new one.
fn written(
    gpu: &Gpu,
    (label, mut contents): (&str, Vec<u8>),
    held: Option<&wgpu::Buffer>,
) -> wgpu::Buffer {
    if contents.len() < RECORD_BYTES {
        contents.resize(RECORD_BYTES, 0);
    }
    if let Some(held) = held.filter(|held| held.size() >= contents.len() as u64) {
        gpu.queue.write_buffer(held, 0, &contents);
        return held.clone();
    }
    gpu.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &contents,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
}

/// An empty list's one zeroed word (a mask word, a patch word).
const RECORD_BYTES: usize = 4;

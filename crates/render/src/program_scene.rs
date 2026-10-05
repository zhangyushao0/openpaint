//! One scene's ops, fill masks and patches as the evaluator's GPU buffers.

use wgpu::util::DeviceExt;

use crate::edits::patch_bytes;
use crate::gpu::Gpu;
use crate::protocol::{Op, Record, encode};
use crate::scene::Scene;

/// One scene on the GPU: its ops (item index = op index), its fill masks and its transforms'
/// patches.
pub(crate) struct SceneOps {
    pub(crate) ops: wgpu::Buffer,
    pub(crate) masks: wgpu::Buffer,
    pub(crate) patches: wgpu::Buffer,
}

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
    SceneOps {
        // Rewritten in place when a preview's values change (S35).
        ops: buffer(gpu, masks, &ops),
        masks: buffer(gpu, masks, &scene.masks),
        patches,
    }
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
            label: Some("evaluate"),
            contents: &contents,
            usage,
        })
}

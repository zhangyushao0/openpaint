//! One view tier (S32): its target, an atlas of `TIER_TILE`-pixel slots holding view tiles of
//! the canvas's output at the tier's scale, which tiles hold the shown version (ready), and the
//! index the blit finds them by. Evaluated through the canvas target's scene: one scene and
//! one set of smudge residuals serve the canvas and its tier.

use openpaint_core::Artwork;

use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;
use crate::output::Output;
use crate::plan::{Layout, Plan};
use crate::program::{Bindings, Dispatch};
use crate::readback::{read_rect, target_texture};
use crate::scene::Fresh;
use crate::view_tiles::{TIER_TILE, TileMap};

/// Slots in an atlas row at most: 4096 pixels.
const ACROSS: u32 = 16;

pub(crate) struct Tier {
    pub(crate) map: TileMap,
    /// Per view tile, row-major: its slot holds the shown version's pixels.
    ready: Vec<bool>,
    atlas: wgpu::Texture,
    bindings: Bindings,
    /// Per view tile, `R32Uint`: its slot plus one while ready, else 0 (`blit.wgsl`).
    index: wgpu::Texture,
    /// The index differs from `ready` and the slots.
    stale: bool,
}

impl Tier {
    /// A tier at `scale` over `artwork`'s canvas with room for `slots` tiles.
    pub(crate) fn new(
        core: &Core,
        artwork: &Artwork,
        scale: u32,
        slots: u32,
    ) -> Result<Self, RenderError> {
        let canvas = artwork.canvas();
        let canvas = [canvas.width(), canvas.height()];
        let output = Output::at(scale, [0, 0])?;
        let across = slots.clamp(1, ACROSS);
        let rows = slots.div_ceil(across).max(1);
        let map = TileMap::new(canvas, output, (across, rows));
        let [columns, tiles_down] = map.tiles();
        let atlas = atlas_texture(core, (across, rows))?;
        let index = core.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("view tier index"),
            size: wgpu::Extent3d {
                width: columns,
                height: tiles_down,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Ok(Self {
            ready: vec![false; (columns * tiles_down) as usize],
            bindings: Bindings::without_kept(&core.gpu, &atlas),
            map,
            atlas,
            index,
            stale: true,
        })
    }

    pub(crate) fn scale(&self) -> u32 {
        self.map.scale()
    }

    pub(crate) fn atlas(&self) -> &wgpu::Texture {
        &self.atlas
    }

    pub(crate) fn index(&self) -> &wgpu::Texture {
        &self.index
    }

    /// Bytes of its textures.
    pub(crate) fn bytes(&self) -> u64 {
        let [columns, rows] = self.map.tiles();
        let atlas = u64::from(self.atlas.width()) * u64::from(self.atlas.height()) * 4;
        atlas + u64::from(columns) * u64::from(rows) * 4
    }

    fn at(&self, [column, row]: [u32; 2]) -> usize {
        (row * self.map.tiles()[0] + column) as usize
    }

    pub(crate) fn is_ready(&self, tile: [u32; 2]) -> bool {
        self.ready[self.at(tile)]
    }

    pub(crate) fn set_ready(&mut self, tile: [u32; 2], ready: bool) {
        let at = self.at(tile);
        self.stale |= self.ready[at] != ready;
        self.ready[at] = ready;
    }

    /// Frees `tile`'s slot.
    pub(crate) fn release(&mut self, tile: [u32; 2]) {
        self.set_ready(tile, false);
        self.map.release(tile);
    }

    /// A free slot for `tile`, the atlas grown by rows if none is left (up to the device's
    /// texture limit); false when there is no room.
    pub(crate) fn hold(
        &mut self,
        core: &Core,
        tile: [u32; 2],
        missing: u32,
    ) -> Result<bool, RenderError> {
        if self.map.free().is_none() {
            let (across, rows) = self.map.slots();
            let limit = core.gpu.device.limits().max_texture_dimension_2d / TIER_TILE;
            let more = missing
                .div_ceil(across)
                .max(1)
                .min(limit.saturating_sub(rows));
            if more == 0 {
                return Ok(false);
            }
            self.grow(core, more)?;
        }
        let Some(slot) = self.map.free() else {
            return Ok(false);
        };
        self.map.hold(tile, slot);
        self.stale = true;
        Ok(true)
    }

    /// `rows` more rows of slots: a taller atlas, the held slots copied into it.
    fn grow(&mut self, core: &Core, rows: u32) -> Result<(), RenderError> {
        let (across, old) = self.map.slots();
        let atlas = atlas_texture(core, (across, old + rows))?;
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(
            self.atlas.as_image_copy(),
            atlas.as_image_copy(),
            self.atlas.size(),
        );
        core.gpu.queue.submit([encoder.finish()]);
        self.bindings = Bindings::without_kept(&core.gpu, &atlas);
        self.atlas = atlas;
        self.map.grow(rows);
        Ok(())
    }

    /// The tier with only its ready tiles `keep` takes, in an atlas just large enough (each
    /// copied on the GPU); `None` when it keeps none.
    pub(crate) fn kept(
        &self,
        core: &Core,
        artwork: &Artwork,
        keep: impl Fn([u32; 2]) -> bool,
    ) -> Result<Option<Self>, RenderError> {
        let tiles: Vec<([u32; 2], u32)> = self
            .map
            .held()
            .filter(|(tile, _)| self.is_ready(*tile) && keep(*tile))
            .collect();
        if tiles.is_empty() {
            return Ok(None);
        }
        let mut kept = Self::new(core, artwork, self.scale(), tiles.len() as u32)?;
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        let side = wgpu::Extent3d {
            width: TIER_TILE,
            height: TIER_TILE,
            depth_or_array_layers: 1,
        };
        for (tile, from) in tiles {
            let Some(to) = kept.map.free() else {
                break;
            };
            kept.map.hold(tile, to);
            kept.set_ready(tile, true);
            let at = |texture, slot| {
                let [x, y] = self.map.slot_origin(slot);
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                }
            };
            let target = kept.map.slot_origin(to);
            encoder.copy_texture_to_texture(
                at(&self.atlas, from),
                wgpu::TexelCopyTextureInfo {
                    texture: &kept.atlas,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: target[0],
                        y: target[1],
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                side,
            );
        }
        core.gpu.queue.submit([encoder.finish()]);
        Ok(Some(kept))
    }

    /// Evaluates the target pixels `regions` (inside held slots) from `canvas`'s scene of
    /// `request`, the in-progress stroke from its first sample on: every pixel cold.
    pub(crate) fn evaluate(
        &mut self,
        core: &Core,
        canvas: &CanvasTarget,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
    ) -> Result<(), RenderError> {
        if regions.is_empty() {
            return Ok(());
        }
        let (scene, ops) =
            canvas
                .scene_of(request.artwork, request.active)
                .ok_or(RenderError::Invalid(
                    "the canvas was not evaluated for this frame",
                ))?;
        let fresh = match request.active.filter(|_| scene.has_active()) {
            Some(active) => Fresh::new(active, 0, scene.active_clip),
            None => Fresh::default(),
        };
        let size = [self.atlas.width(), self.atlas.height()];
        let plan = Plan::laid_out(scene, &fresh, (size, Layout::Tiles(&self.map)), regions);
        if plan.tiles.is_empty() {
            return Ok(());
        }
        let paper = request.background;
        let output = Output::at(self.scale(), [0, 0])?;
        core.evaluate.dispatch(
            &core.gpu,
            &mut self.bindings,
            &Dispatch {
                ops,
                fresh: &fresh.ops,
                plan: &plan,
                only: None,
                output,
                background: [paper.r(), paper.g(), paper.b(), 1.0],
                reset: false,
                residuals: canvas.residuals(),
                base: Affine::IDENTITY,
                snapshot: None,
            },
        )
    }

    /// Writes the index when it changed.
    pub(crate) fn upload_index(&mut self, core: &Core) {
        if !self.stale {
            return;
        }
        let [columns, rows] = self.map.tiles();
        let mut words = vec![0u32; (columns * rows) as usize];
        for (tile, slot) in self.map.held() {
            if self.is_ready(tile) {
                words[self.at(tile)] = slot + 1;
            }
        }
        let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        core.gpu.queue.write_texture(
            self.index.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(columns * 4),
                rows_per_image: None,
            },
            self.index.size(),
        );
        self.stale = false;
    }

    /// A ready tile's pixels, read back (waits for the GPU).
    pub(crate) fn read(
        &self,
        core: &Core,
        tile: [u32; 2],
    ) -> Result<Option<RgbaImage>, RenderError> {
        let Some(slot) = self.map.slot(tile).filter(|_| self.is_ready(tile)) else {
            return Ok(None);
        };
        let rect = self.map.tile_rect(tile);
        let size = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]) as u32);
        read_rect(&core.gpu, &self.atlas, self.map.slot_origin(slot), size).map(Some)
    }
}

/// An atlas of `across` by `rows` slots.
fn atlas_texture(core: &Core, (across, rows): (u32, u32)) -> Result<wgpu::Texture, RenderError> {
    let usage = wgpu::TextureUsages::STORAGE_BINDING
        | wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST;
    let size = [across * TIER_TILE, rows * TIER_TILE];
    target_texture(&core.gpu, (size, core.gpu.display), usage)
}

//! The screen canvas on the raster core (S62, DECISIONS D89): what of it waits to be drawn, the
//! frame loop's raster targets (shared with the view tiers' changes drawn in place), and the tiles
//! handed to the raster worker (`canvas_raster_fill.rs` draws). A change (an undo, a layer edit, a
//! fill, an opening) is drawn at once on the frame loop when it is small and fits the frame's
//! budget, else the canvas's 256-px tiles it reaches wait and the worker draws them, the screen's
//! middle first, at the GPU's pace (D87), each into a slot of a staging texture the frame loop
//! then copies into the canvas, unless a change reached the tile meanwhile: so only the frame loop
//! writes the canvas, and a tile drawn for an older version never covers a newer one (a stroke
//! being drawn, a later change). Once nothing waits, the worker takes the snapshots under the
//! next stroke ahead (S63, `drawing_ahead.rs`).

use openpaint_core::Artwork;

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::raster::RasterTargets;
use crate::raster_route::Routed;
use crate::readback::target_texture;
use crate::view_raster_jobs::{RasterDone, RasterOrder};
use crate::view_tiles::TIER_TILE;
use crate::waiting::Waiting;

/// Canvas tiles with the raster worker at most: as many as a view tier hands over (S60).
pub(crate) const AHEAD: usize = crate::view_raster_fill::AHEAD;

/// What a tile is expected to cost the GPU before any was drawn, ns: a dense one on the tablet.
const FIRST: f64 = 6_000_000.0;

/// A canvas tile handed to the raster worker: its job, its pixels, the staging slot it is drawn
/// into, and whether a change reached it since.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Handed {
    pub(crate) id: u64,
    pub(crate) rect: PixelRect,
    pub(crate) slot: u32,
    pub(crate) stale: bool,
}

/// The raster core's side of a canvas target.
pub(crate) struct CanvasRaster {
    /// The canvas's 256-px tiles that wait to be drawn.
    pub(crate) waiting: Waiting,
    /// Changes not drawn yet nor waiting: canvas pixels.
    pub(crate) fresh: Vec<PixelRect>,
    pub(crate) handed: Vec<Handed>,
    /// Answers to tiles handed over, not applied yet, and whether each was drawn.
    pub(crate) done: Vec<(Handed, bool)>,
    /// The texture the worker draws handed tiles into, a slot each, and its view.
    staging: Option<(wgpu::Texture, wgpu::TextureView)>,
    /// The canvas texture as the raster core draws into it.
    pub(crate) view: wgpu::TextureView,
    /// The frame loop's targets, for the canvas and its view tiers (S59's, moved here).
    pub(crate) targets: Option<RasterTargets>,
    /// When the targets last drew (the frame clock's seconds), and whether they drew since the
    /// view tiers last looked (`ViewCache::update` stamps it).
    pub(crate) used_at: f64,
    pub(crate) drew: bool,
    /// Orders for the worker not taken yet (the canvas's and its tiers'), the tiers' answers not
    /// taken yet, the next job's id.
    pub(crate) orders: Vec<RasterOrder>,
    pub(crate) answers: Vec<RasterDone>,
    pub(crate) next: u64,
    /// The worker's targets' bytes, and the last tile's GPU time (ns), as it said last.
    pub(crate) drawer: u64,
    pub(crate) last: f64,
}

impl CanvasRaster {
    /// Nothing waits on a canvas of `texture`.
    pub(crate) fn new(texture: &wgpu::Texture) -> Self {
        Self {
            waiting: Waiting::of([texture.width(), texture.height()], TIER_TILE),
            fresh: Vec::new(),
            handed: Vec::new(),
            done: Vec::new(),
            staging: None,
            view: texture.create_view(&Default::default()),
            targets: None,
            used_at: 0.0,
            drew: false,
            orders: Vec::new(),
            answers: Vec::new(),
            next: 0,
            drawer: 0,
            last: 0.0,
        }
    }

    /// Something waits to be drawn (or applied).
    pub(crate) fn pending(&self) -> bool {
        !(self.waiting.is_empty()
            && self.fresh.is_empty()
            && self.handed.is_empty()
            && self.done.is_empty())
    }

    /// The canvas pixels `rects` changed: drawn or made to wait by the next frame, the handed
    /// tiles they reach drawn again.
    pub(crate) fn change(&mut self, rects: &[PixelRect]) {
        self.stale(rects);
        self.fresh.extend_from_slice(rects);
    }

    /// Handed tiles `rects` reach are drawn again once answered (S63: the stroke being drawn drew
    /// there since they were handed over).
    pub(crate) fn stale(&mut self, rects: &[PixelRect]) {
        let answered = self.done.iter_mut().map(|(handed, _)| handed);
        for handed in self.handed.iter_mut().chain(answered) {
            if rects
                .iter()
                .any(|rect| !rect.intersect(&handed.rect).is_empty())
            {
                handed.stale = true;
            }
        }
    }

    /// Everything waiting goes (the evaluator draws the canvas now, S62's routing): the tiles
    /// handed over are not applied when answered, those answered not at all.
    pub(crate) fn forget(&mut self) {
        self.waiting.take_all();
        self.fresh.clear();
        self.done.clear();
        for handed in &mut self.handed {
            handed.stale = true;
        }
    }

    /// The worker's answer to a job: the canvas's applied by its next frame (one a change
    /// reached since, dropped: its tile waits still), the tiers' kept for them.
    pub(crate) fn answer(&mut self, done: RasterDone) {
        self.drawer = done.bytes;
        if done.drawn {
            self.last = done.time;
        }
        match self.handed.iter().position(|handed| handed.id == done.id) {
            Some(at) => {
                let handed = self.handed.remove(at);
                if !handed.stale {
                    self.done.push((handed, done.drawn));
                }
            }
            None => self.answers.push(done),
        }
    }

    /// The next job's id.
    pub(crate) fn next_id(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// What the next tile is expected to cost the GPU, ns: what the last drawn did.
    pub(crate) fn expected(&self) -> f64 {
        if self.last > 0.0 { self.last } else { FIRST }
    }

    /// A staging slot no handed tile holds, and the staging texture's view for a canvas in
    /// `format`, made on first use.
    pub(crate) fn slot(
        &mut self,
        core: &Core,
        format: wgpu::TextureFormat,
    ) -> Result<Option<(u32, wgpu::TextureView)>, RenderError> {
        if self.staging.is_none() {
            let size = [AHEAD as u32 * TIER_TILE, TIER_TILE];
            let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
            let texture = target_texture(&core.gpu, (size, format), usage)?;
            let view = texture.create_view(&Default::default());
            self.staging = Some((texture, view));
        }
        let held = |slot: u32| {
            (self.handed.iter()).any(|handed| handed.slot == slot)
                || (self.done.iter()).any(|(handed, _)| handed.slot == slot)
        };
        let free = (0..AHEAD as u32).find(|slot| !held(*slot));
        Ok(free.zip(self.staging.as_ref().map(|(_, view)| view.clone())))
    }

    /// The staging texture, while one is made.
    pub(crate) fn staging(&self) -> Option<&wgpu::Texture> {
        self.staging.as_ref().map(|(texture, _)| texture)
    }

    /// Bytes of the targets, the staging texture and the worker's targets.
    pub(crate) fn bytes(&self) -> u64 {
        let staging = self.staging().map_or(0, |texture| {
            u64::from(texture.width()) * u64::from(texture.height()) * 4
        });
        self.targets.as_ref().map_or(0, RasterTargets::bytes) + staging + self.drawer
    }

    /// Lets the targets go, the staging texture once nothing is drawn into it, and the worker's
    /// (an order); returns their bytes.
    pub(crate) fn let_go(&mut self) -> u64 {
        let bytes = self.bytes();
        self.targets = None;
        if self.handed.is_empty() && self.done.is_empty() {
            self.staging = None;
        }
        if self.drawer > 0 {
            self.orders.push(RasterOrder::LetGo);
            self.drawer = 0;
        }
        bytes
    }
}

impl CanvasTarget {
    /// The scene the last evaluation drew for `request` goes to the raster core (S62).
    pub(crate) fn rastered(&self, request: &FrameRequest<'_>) -> bool {
        (self.scene_serial(request.artwork, request.active))
            .is_some_and(|(scene, _, _)| scene.rastered().is_some())
    }

    /// Which core draws the scene the last evaluation drew for `artwork` (`Ok`: the raster
    /// core; else why the evaluator), for the log; `None` before one drew it.
    #[must_use]
    pub fn routing(&self, artwork: &Artwork) -> Option<Result<(), Routed>> {
        (self.scene_serial(artwork, None)).map(|(scene, _, _)| scene.routing.kind())
    }

    /// What the canvas and its view tiers ask of the raster worker since the last call, in
    /// order (S60): the runtime hands it over.
    pub fn take_orders(&mut self) -> Vec<RasterOrder> {
        std::mem::take(&mut self.raster.orders)
    }

    /// The raster worker's answer to a tile: the canvas's applied by its next frame, a view
    /// tier's by the view's next update, a snapshot under the next stroke kept (S63).
    pub fn answer(&mut self, done: RasterDone) {
        if self.drawing.answer(&done) {
            self.raster.drawer = done.bytes;
            return;
        }
        self.raster.answer(done);
    }

    /// The raster core's targets for the canvas and its tiers (its atlas and float targets, the
    /// staging texture and the worker's), their bytes and when they last drew (the frame clock's
    /// seconds); `None` while none is held.
    #[must_use]
    pub fn raster_held(&self) -> Option<(u64, f64)> {
        let bytes = self.raster.bytes();
        (bytes > 0).then_some((bytes, self.raster.used_at))
    }

    /// Lets the raster core's targets go (the budget's step); the next region drawn makes them
    /// again. Returns their bytes.
    pub fn let_go_raster(&mut self) -> u64 {
        self.raster.let_go()
    }
}

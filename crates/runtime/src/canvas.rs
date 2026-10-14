//! The open canvas: the app session, the target its artwork is evaluated into, where that
//! target shows on screen, and whether the user is leaving it for the gallery.

use std::time::Instant;

use openpaint_app::{
    Clipboard, Contact, ContactPhase, FillFailed, Intent, LiftFailed, SelectionIntent, Session,
    StrokeIntent, Title, ToolIntent, Toolbox, TransformJob, ViewMapping,
};
use openpaint_core::{Artwork, CanvasSize, Color, Patch, RegionMask, StrokeTool};
use openpaint_render::{
    BrushCursor, CanvasPlacement, CanvasTarget, Core, ViewCache, ViewFrame, tip_radius,
};
use openpaint_ui::{CanvasArea, ScreenArea};

use crate::error::RuntimeError;
use crate::pictures::Drawn;
use crate::seeds::Seeds;

/// The paper under the layers until artworks carry their own background; exports show it too.
pub(crate) const PAPER: Color = Color::WHITE;
/// Largest gallery thumbnail in pixels: a gallery card's picture at density 1.75.
pub(crate) const THUMBNAIL_MAX: [u32; 2] = [448, 336];

/// What a canvas opens with.
#[derive(Debug)]
pub(crate) enum Content {
    /// An artwork as the library holds it, its title, and what shows it before it is evaluated
    /// (its cached canvas or gallery picture, S54).
    Stored(Artwork, Title, Box<Held>),
    /// A blank artwork the size of the canvas area, not in the library yet.
    Blank,
    /// As `Blank`, after the last artwork could not be opened.
    BlankAfterFailedOpen,
}

/// Leaving the canvas for the gallery: the artwork is stored first, and the canvas stays until
/// that succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leaving {
    Staying,
    /// Asked to leave; waits for a save still running.
    Waiting,
    /// The last version and the thumbnail are on their way to the store.
    Storing,
}

pub(crate) struct Canvas {
    session: Session,
    /// The gallery's name for the artwork, which exported files carry.
    title: Title,
    target: CanvasTarget,
    /// The view tiers shown instead of the target while the view magnifies it (S32).
    view: ViewCache,
    /// The last view update left tiles to fill or a tier to start.
    view_pending: bool,
    /// False until the whole canvas has been evaluated once.
    evaluated: bool,
    /// The version shown before this frame's first change other than a stroke (undo, redo, a
    /// layer edit), until it is evaluated.
    changed_from: Option<Artwork>,
    pub(crate) leaving: Leaving,
    /// What the layer panel's pictures were made of (S38).
    pub(crate) pictures: Drawn,
    /// When it opened (or its device was made again), until the whole canvas was evaluated:
    /// the time to the full canvas is logged then (S49).
    opened: Option<Opening>,
    /// What the display cache holds for it and must match (S54).
    cache: CacheState,
}

impl Canvas {
    /// `content` shown at fit; a blank artwork is the size of the area it fits in; strokes
    /// start with the toolbox's brush and StreamLine. `None` while the area is empty. Later area
    /// changes (rotation) refit the view, which is relative to fit, and never resize the canvas.
    pub(crate) fn open(
        core: &Core,
        area: &CanvasArea,
        content: &Content,
        toolbox: &Toolbox,
    ) -> Result<Option<Self>, RuntimeError> {
        let area = &area.fit;
        if area.size.contains(&0) {
            return Ok(None);
        }
        let brush = toolbox.style()?;
        let (mut session, title) = match content {
            Content::Stored(artwork, title, _) => {
                (Session::open(artwork.clone(), brush), title.clone())
            }
            Content::Blank => (Session::new(blank_size(area)?, brush), Title::untitled()),
            Content::BlankAfterFailedOpen => (
                Session::after_failed_open(blank_size(area)?, brush),
                Title::untitled(),
            ),
        };
        session.set_streamline(toolbox.streamline());
        let canvas = session.artwork().canvas();
        let size = [canvas.width(), canvas.height()];
        log::info!("canvas opened at {} x {}", size[0], size[1]);
        let mut canvas = Self {
            session,
            title,
            target: CanvasTarget::new(core, size)?,
            view: ViewCache::new(core),
            view_pending: false,
            evaluated: false,
            changed_from: None,
            leaving: Leaving::Staying,
            pictures: Drawn::default(),
            opened: Some(Opening::start(core)),
            cache: CacheState::default(),
        };
        if let Content::Stored(_, _, held) = content {
            canvas.show_opened(core, held);
        }
        Ok(Some(canvas))
    }

    /// Applies the frame's intents in order. A refused intent is logged and skipped: one bad
    /// sample must not stop the frame. A gallery intent the session allows starts leaving;
    /// while leaving, nothing else is applied. Brush and tool intents go to `toolbox`, and the
    /// session draws its next stroke with the style that results. While the eyedropper is on,
    /// canvas contacts pick colours instead of drawing (`pick`). The copy menu's commands copy
    /// into and paste from `clipboard`, the app's. Every contact touching down takes the next of
    /// `seeds` for the stroke it may begin (S45).
    pub(crate) fn dispatch(
        &mut self,
        core: &Core,
        intents: Vec<Intent>,
        (toolbox, clipboard, seeds): (&mut Toolbox, &mut Clipboard, &mut Seeds),
    ) {
        for intent in intents {
            if self.leaving != Leaving::Staying {
                return;
            }
            if self.toolbox_intent(core, &intent, toolbox) {
                match toolbox.style() {
                    Ok(style) => self.session.set_brush(style),
                    Err(error) => log::error!("no brush style: {error}"),
                }
                self.session.set_streamline(toolbox.streamline());
                continue;
            }
            if let Intent::Stroke(StrokeIntent::Contact(contact)) = &intent
                && contact.phase == ContactPhase::Down
            {
                self.session.set_seed(seeds.next());
            }
            // Committing a stroke is followed by the evaluator itself (D82), but not its mirrored
            // copies or a snapped shape edited (S42).
            let stroke = matches!(intent, Intent::Stroke(_));
            let gallery = matches!(intent, Intent::Gallery(_));
            let cancel = matches!(&intent, Intent::Stroke(StrokeIntent::Contact(contact))
                if contact.phase == ContactPhase::Cancel && self.session.active().is_some());
            let before = self.session.shown().clone();
            let result = match intent {
                Intent::Clipboard(action) => self.session.clipboard(action, clipboard),
                intent => self.session.dispatch(intent),
            };
            if cancel && self.session.active().is_none() {
                log::info!("the platform cancelled the stroke being drawn; it is dropped");
            }
            match result {
                Ok(()) if gallery => self.leaving = Leaving::Waiting,
                Ok(()) if !stroke || self.session.stroke_reshown() => {
                    self.changed_from.get_or_insert(before);
                }
                Ok(()) => {}
                Err(error) => log::warn!("intent refused: {error}"),
            }
            // A committed stroke's brush is Recent's newest (S47).
            if let Some(drawn) = self.session.stroke_committed().filter(|_| stroke) {
                toolbox.drew(drawn);
            }
            // A transform just begun lifts its pixels now, so the frame shows it at once.
            if let Some(job) = self.session.transform_request() {
                self.session.transform_lifted(lift(core, &job));
            }
        }
    }

    /// Commits the stroke being drawn as its lift would, when the pen can no longer lift it: the
    /// app is left or its window goes mid-stroke (S50). Its brush becomes Recent's newest; the
    /// next evaluation shows it committed (its end tapered) and so does the gallery picture.
    pub(crate) fn end_stroke(&mut self, toolbox: &mut Toolbox) {
        let before = self.session.shown().clone();
        match self.session.end_stroke() {
            Ok(false) => return,
            Ok(true) => log::info!("the stroke being drawn was committed as drawn so far"),
            Err(error) => log::warn!("the stroke being drawn was committed with: {error}"),
        }
        if let Some(drawn) = self.session.stroke_committed() {
            toolbox.drew(drawn);
        }
        self.changed_from.get_or_insert(before);
    }

    /// Hands a brush or tool intent to `toolbox`, and a canvas contact to the eyedropper while it
    /// is on. Returns whether `intent` was the toolbox's.
    fn toolbox_intent(&mut self, core: &Core, intent: &Intent, toolbox: &mut Toolbox) -> bool {
        match intent {
            Intent::Brush(brush) => toolbox.dispatch(brush),
            Intent::Tool(tool) => {
                toolbox.dispatch_tool(tool);
                // Choosing a tool or the eyedropper leaves the Selection tool and commits a
                // transform (Procreate).
                let chosen = matches!(
                    tool,
                    ToolIntent::Use(_) | ToolIntent::Switch | ToolIntent::Eyedropper(true)
                );
                if chosen {
                    self.leave_selection();
                }
            }
            Intent::Stroke(StrokeIntent::Contact(contact)) if toolbox.picking() => {
                self.pick(core, contact, toolbox);
            }
            _ => return false,
        }
        true
    }

    /// Leaves the Selection tool (a transform under way is committed), noting what changed.
    fn leave_selection(&mut self) {
        let before = self.session.shown().clone();
        let leave = Intent::Selection(SelectionIntent::Tool(false));
        if let Err(error) = self.session.dispatch(leave) {
            log::warn!("intent refused: {error}");
        }
        if !before.same_version(self.session.shown()) {
            self.changed_from.get_or_insert(before);
        }
    }

    /// The eyedropper: a contact on the canvas (pen or finger) previews the colour the canvas
    /// shows under it, read back from the target (render picking), as it touches down and
    /// moves; the loupe shows it. Lifting picks the last colour previewed, never one at the
    /// release point: it joins the recent colours and the tool used before comes back. A
    /// cancelled contact ends the eyedropper and keeps the colour.
    fn pick(&mut self, core: &Core, contact: &Contact, toolbox: &mut Toolbox) {
        match contact.phase {
            ContactPhase::Down | ContactPhase::Move => {}
            ContactPhase::Up => return toolbox.dispatch_tool(&ToolIntent::Pick),
            ContactPhase::Cancel => return toolbox.dispatch_tool(&ToolIntent::Eyedropper(false)),
            ContactPhase::Hover | ContactPhase::Leave => return,
        }
        let Some(sample) = contact.samples.last() else {
            return;
        };
        // Tiles still waiting would read as they were (S49).
        if let Err(error) = self.settle(core) {
            log::error!("evaluating the canvas before picking failed: {error}");
        }
        let [x, y] = sample.position.map(f32::floor);
        let picked = if x >= 0.0 && y >= 0.0 {
            self.target.pick(core, [x as u32, y as u32])
        } else {
            Ok(None)
        };
        match picked {
            Ok(Some([r, g, b, _])) => {
                let channel = |value: u8| f32::from(value) / 255.0;
                match Color::rgb(channel(r), channel(g), channel(b)) {
                    Ok(color) => toolbox.dispatch_tool(&ToolIntent::Preview(color)),
                    Err(error) => log::error!("picked no colour: {error}"),
                }
            }
            // Off the canvas: nothing to preview there; the last preview stays.
            Ok(None) => {}
            Err(error) => log::error!("picking failed: {error}"),
        }
    }

    /// The fill worker computed request `sequence`; a new preview (or the committed fill) is
    /// evaluated by the pixels it changes.
    pub(crate) fn fill_finished(&mut self, sequence: u64, result: Result<RegionMask, FillFailed>) {
        let before = self.session.shown().clone();
        self.session.fill_finished(sequence, result);
        if !before.same_version(self.session.shown()) {
            self.changed_from.get_or_insert(before);
        }
    }

    /// The target shown in the canvas area through the session's view as `shown` shows it (the
    /// same, or on its way there while the view settles), on a screen of `pixels_per_point`,
    /// with the pen's hover outline when it shows.
    pub(crate) fn placement(
        &self,
        (area, pixels_per_point): (&CanvasArea, f32),
        cursor: Option<BrushCursor>,
        shown: impl FnOnce(ViewMapping) -> ViewMapping,
    ) -> CanvasPlacement<'_> {
        let frame = self.view_frame((area, pixels_per_point), shown);
        CanvasPlacement {
            target: &self.target,
            origin: frame.origin,
            size: frame.size,
            to_canvas: frame.to_canvas,
            cursor,
            pixels_per_point,
            view: Some(&self.view),
        }
    }

    /// Where the canvas shows in the canvas area through the session's view as `shown` shows
    /// it, on a screen of `pixels_per_point`.
    pub(crate) fn view_frame(
        &self,
        (area, pixels_per_point): (&CanvasArea, f32),
        shown: impl FnOnce(ViewMapping) -> ViewMapping,
    ) -> ViewFrame {
        let canvas = self.session.artwork().canvas();
        let viewport = area.viewport(pixels_per_point);
        let mapping = shown(self.session.view().mapping(canvas, &viewport));
        ViewFrame {
            to_canvas: mapping.to_canvas_rows(),
            origin: area.screen.origin,
            size: area.screen.size,
        }
    }

    /// While the pen erases or smudges: the tip in canvas pixels, where the stroke is now and
    /// its radius at the pen's pressure there, so the hand sees what it takes away or drags
    /// (neither lays colour of its own to show where it is).
    pub(crate) fn erasing_tip(&self) -> Option<([f32; 2], f32)> {
        let active = self.session.active()?;
        let samples = active.samples();
        let position = samples.last()?.position();
        let radius = tip_radius(active.style(), samples)?;
        matches!(
            active.style().tool(),
            StrokeTool::Erase | StrokeTool::Smudge
        )
        .then_some(([position.x(), position.y()], radius))
    }

    pub(crate) fn session(&self) -> &Session {
        &self.session
    }

    pub(crate) fn title(&self) -> &Title {
        &self.title
    }

    pub(crate) fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    pub(crate) fn target(&self) -> &CanvasTarget {
        &self.target
    }
}

#[path = "canvas_gpu.rs"]
mod gpu;
use gpu::Opening;
pub(crate) use gpu::{Freed, Rooms};

#[path = "canvas_cache.rs"]
mod cache;
use cache::CacheState;
pub(crate) use cache::Held;

/// A transform's pixels: the job's layer sampled through the renderer (the single evaluator, a
/// target of its own), then its painted pixels inside the job's selection, if any. Waits for
/// the GPU, once per transform.
fn lift(core: &Core, job: &TransformJob) -> Result<Option<Patch>, LiftFailed> {
    let started = Instant::now();
    let pixels = core
        .layer_pixels(&job.artwork, job.layer)
        .inspect_err(|error| log::error!("sampling the layer for a transform failed: {error}"))
        .map_err(|_| LiftFailed)?;
    let patch = openpaint_raster::lift(job.artwork.canvas(), pixels.pixels(), job.clip.as_ref());
    log::info!(
        "transform lifted {:?} px in {} ms",
        patch.as_ref().map(Patch::size),
        started.elapsed().as_millis()
    );
    Ok(patch)
}

/// The size of a blank artwork that fills `area`.
pub(crate) fn blank_size(area: &ScreenArea) -> Result<CanvasSize, RuntimeError> {
    Ok(CanvasSize::new(area.size[0], area.size[1])?)
}

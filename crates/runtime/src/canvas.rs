//! The open canvas: the app session, the target its artwork is evaluated into, where that
//! target shows on screen, and whether the user is leaving it for the gallery.

use std::time::Instant;

use openpaint_app::{
    Contact, ContactPhase, FillFailed, Intent, LiftFailed, SelectionIntent, Session, StrokeIntent,
    Thumbnail, Title, ToolIntent, Toolbox, TransformJob, ViewMapping,
};
use openpaint_core::{Artwork, CanvasSize, Color, Patch, RegionMask, StrokeTool};
use openpaint_render::{
    BrushCursor, CanvasPlacement, CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator,
    ViewCache, ViewFrame, changed_regions, tip_radius,
};
use openpaint_ui::{CanvasArea, ScreenArea};

use crate::error::RuntimeError;
use crate::pictures::Drawn;

/// The paper under the layers until artworks carry their own background; exports show it too.
pub(crate) const PAPER: Color = Color::WHITE;
/// Largest gallery thumbnail in pixels: a gallery card's picture at density 1.75.
pub(crate) const THUMBNAIL_MAX: [u32; 2] = [448, 336];

/// What a canvas opens with.
#[derive(Debug, Clone)]
pub(crate) enum Content {
    /// An artwork as the library holds it, and its title.
    Stored(Artwork, Title),
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
            Content::Stored(artwork, title) => {
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
        Ok(Some(Self {
            session,
            title,
            target: CanvasTarget::new(core, size)?,
            view: ViewCache::new(core),
            view_pending: false,
            evaluated: false,
            changed_from: None,
            leaving: Leaving::Staying,
            pictures: Drawn::default(),
        }))
    }

    /// Applies the frame's intents in order. A refused intent is logged and skipped: one bad
    /// sample must not stop the frame. A gallery intent the session allows starts leaving;
    /// while leaving, nothing else is applied. Brush and tool intents go to `toolbox`, and the
    /// session draws its next stroke with the style that results. While the eyedropper is on,
    /// canvas contacts pick colours instead of drawing (`pick`).
    pub(crate) fn dispatch(&mut self, core: &Core, intents: Vec<Intent>, toolbox: &mut Toolbox) {
        for intent in intents {
            if self.leaving != Leaving::Staying {
                return;
            }
            let tool = match &intent {
                Intent::Brush(brush) => {
                    toolbox.dispatch(brush);
                    true
                }
                Intent::Tool(tool) => {
                    toolbox.dispatch_tool(tool);
                    // Choosing a tool or the eyedropper leaves the Selection tool and commits a
                    // transform (Procreate).
                    if matches!(tool, ToolIntent::Use(_) | ToolIntent::Eyedropper(true)) {
                        let before = self.session.shown().clone();
                        let leave = Intent::Selection(SelectionIntent::Tool(false));
                        if let Err(error) = self.session.dispatch(leave) {
                            log::warn!("intent refused: {error}");
                        }
                        if !before.same_version(self.session.shown()) {
                            self.changed_from.get_or_insert(before);
                        }
                    }
                    true
                }
                Intent::Stroke(StrokeIntent::Contact(contact)) if toolbox.picking() => {
                    self.pick(core, contact, toolbox);
                    true
                }
                _ => false,
            };
            if tool {
                match toolbox.style() {
                    Ok(style) => self.session.set_brush(style),
                    Err(error) => log::error!("no brush style: {error}"),
                }
                self.session.set_streamline(toolbox.streamline());
                continue;
            }
            // Committing a stroke is followed by the evaluator itself (D17).
            let stroke = matches!(intent, Intent::Stroke(_));
            let gallery = matches!(intent, Intent::Gallery(_));
            let before = self.session.shown().clone();
            match self.session.dispatch(intent) {
                Ok(()) if gallery => self.leaving = Leaving::Waiting,
                Ok(()) if !stroke => {
                    self.changed_from.get_or_insert(before);
                }
                Ok(()) => {}
                Err(error) => log::warn!("intent refused: {error}"),
            }
            // A transform just begun lifts its pixels now, so the frame shows it at once.
            if let Some(job) = self.session.transform_request() {
                self.session.transform_lifted(lift(core, &job));
            }
        }
    }

    /// The eyedropper: a contact on the canvas (pen or finger) previews the colour the canvas
    /// shows under it, read back from the target (render picking), as it touches down and
    /// moves; the loupe shows it. Lifting picks the last colour previewed, never one at the
    /// release point: it joins the recent colours and the tool used before comes back. A
    /// cancelled contact ends the eyedropper and keeps the colour.
    fn pick(&self, core: &Core, contact: &Contact, toolbox: &mut Toolbox) {
        match contact.phase {
            ContactPhase::Down | ContactPhase::Move => {}
            ContactPhase::Up => return toolbox.dispatch_tool(&ToolIntent::Pick),
            ContactPhase::Cancel => return toolbox.dispatch_tool(&ToolIntent::Eyedropper(false)),
            ContactPhase::Hover | ContactPhase::Leave => return,
        }
        let Some(sample) = contact.samples.last() else {
            return;
        };
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

    /// Brings the target up to date with what the session shows (the current version, or a
    /// fill's preview over it). Committing the stroke being drawn is followed by the evaluator
    /// itself (D17); any other change passes the pixels that differ between the version shown
    /// before it and the one shown now; the first evaluation covers the whole canvas.
    pub(crate) fn evaluate(&mut self, core: &Core) -> Result<(), RuntimeError> {
        let mut regions = if self.evaluated {
            Vec::new()
        } else {
            vec![PixelRect::full(self.target.size())]
        };
        if let Some(before) = self.changed_from.take() {
            regions.extend(changed_regions(&before, self.session.shown()));
        }
        let request = FrameRequest {
            artwork: self.session.shown(),
            active: self.session.active(),
            background: PAPER,
        };
        RegionEvaluator::new(core, &mut self.target).evaluate(&request, &regions)?;
        self.target.show_selection(core, self.session.selection());
        self.evaluated = true;
        Ok(())
    }

    /// The gallery picture of what the canvas shows now; call after `evaluate`.
    pub(crate) fn thumbnail(&self, core: &Core) -> Result<Thumbnail, RuntimeError> {
        let image = core.thumbnail(&self.target, THUMBNAIL_MAX)?;
        let size = image.size();
        Thumbnail::new(size, image.into_pixels()).ok_or(RuntimeError::Render(
            openpaint_render::RenderError::Invalid("a thumbnail of the wrong length"),
        ))
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

    /// Brings the view tiers up to date with the target, just evaluated, for showing it
    /// through `frame` at `now` (seconds on the frame clock, S32); call after `evaluate`.
    pub(crate) fn update_view(
        &mut self,
        core: &Core,
        (frame, now): (&ViewFrame, f64),
    ) -> Result<(), RuntimeError> {
        let request = FrameRequest {
            artwork: self.session.shown(),
            active: self.session.active(),
            background: PAPER,
        };
        self.view_pending = self
            .view
            .update(core, &mut self.target, &request, (frame, now))?
            .pending();
        Ok(())
    }

    /// The view tiers still change what shows: tiles missing, or a tier about to start.
    pub(crate) fn view_pending(&self) -> bool {
        self.view_pending
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

//! The open canvas: the app session, the target its artwork is evaluated into, where that
//! target shows on screen, and whether the user is leaving it for the gallery.

use openpaint_app::{
    Contact, ContactPhase, FillFailed, Intent, Session, StrokeIntent, Thumbnail, Title, ToolIntent,
    Toolbox,
};
use openpaint_core::{Artwork, CanvasSize, Color, RegionMask};
use openpaint_render::{
    CanvasPlacement, CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, changed_regions,
};
use openpaint_ui::ScreenArea;

use crate::error::RuntimeError;

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
    /// False until the whole canvas has been evaluated once.
    evaluated: bool,
    /// The version shown before this frame's first change other than a stroke (undo, redo, a
    /// layer edit), until it is evaluated.
    changed_from: Option<Artwork>,
    pub(crate) leaving: Leaving,
}

impl Canvas {
    /// `content` shown at fit; a blank artwork is the size of `area`; strokes start with the
    /// toolbox's brush and StreamLine. `None` while the area is empty. Later area changes
    /// (rotation) refit the view, which is relative to fit, and never resize the canvas.
    pub(crate) fn open(
        core: &Core,
        area: &ScreenArea,
        content: &Content,
        toolbox: &Toolbox,
    ) -> Result<Option<Self>, RuntimeError> {
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
            evaluated: false,
            changed_from: None,
            leaving: Leaving::Staying,
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
        }
    }

    /// The eyedropper: a contact on the canvas (pen or finger) picks the colour the canvas shows
    /// under it, read back from the target (render picking), as it touches down and moves; the
    /// colour shows at once. Lifting settles it: it joins the recent colours and the tool used
    /// before comes back. A cancelled contact ends the eyedropper with the colour it showed.
    fn pick(&self, core: &Core, contact: &Contact, toolbox: &mut Toolbox) {
        let settled = match contact.phase {
            ContactPhase::Down | ContactPhase::Move => false,
            ContactPhase::Up => true,
            ContactPhase::Cancel => {
                toolbox.dispatch_tool(&ToolIntent::Eyedropper(false));
                return;
            }
            ContactPhase::Hover | ContactPhase::Leave => return,
        };
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
                    Ok(color) => toolbox.dispatch_tool(&ToolIntent::Color { color, settled }),
                    Err(error) => log::error!("picked no colour: {error}"),
                }
            }
            // Off the canvas: nothing to pick there; lifting there keeps the eyedropper on.
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

    /// The target shown in the canvas area through the session's view, on a screen of
    /// `pixels_per_point`.
    pub(crate) fn placement(
        &self,
        area: &ScreenArea,
        pixels_per_point: f32,
    ) -> CanvasPlacement<'_> {
        let canvas = self.session.artwork().canvas();
        let viewport = area.viewport(pixels_per_point);
        let mapping = self.session.view().mapping(canvas, &viewport);
        CanvasPlacement {
            target: &self.target,
            origin: area.origin,
            size: area.size,
            to_canvas: mapping.to_canvas_rows(),
        }
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

/// The size of a blank artwork that fills `area`.
pub(crate) fn blank_size(area: &ScreenArea) -> Result<CanvasSize, RuntimeError> {
    Ok(CanvasSize::new(area.size[0], area.size[1])?)
}

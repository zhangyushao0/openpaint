//! The open canvas: what is open on it (`work.rs`: an artwork's session or a practice sheet's),
//! the target its artwork is evaluated into, where that target shows on screen, and whether the
//! user is leaving it for the gallery. The frame's intents are applied in `dispatch.rs`, the
//! eyedropper in `pick.rs`, the target, its view tiers and the GPU's work in `gpu.rs`, the display
//! cache it showed in `cache.rs`.

use openpaint_app::{FillFailed, Session, Title, Toolbox, ViewMapping, Viewport};
use openpaint_core::{Artwork, CanvasSize, Color, RegionMask, StrokeStyle, StrokeTool};
use openpaint_practice_app::{PracticePens, SheetSession, copy_brush};

use openpaint_render::{
    BrushCursor, CanvasPlacement, CanvasTarget, Core, ViewCache, ViewFrame, tip_radius,
};
use openpaint_ui::{CanvasArea, ScreenArea};

use crate::error::RuntimeError;
use crate::pictures::Drawn;

mod cache;
mod dispatch;
mod gpu;
mod pick;
mod work;

use cache::CacheState;
pub(crate) use cache::Held;
use gpu::Opening;
pub(crate) use gpu::{Freed, Rooms};
use work::Work;

/// The paper under the layers until artworks carry their own background; exports show it too.
pub(crate) const PAPER: Color = Color::WHITE;
/// Largest gallery thumbnail in pixels: a gallery card's picture at density 1.75.
pub(crate) const THUMBNAIL_MAX: [u32; 2] = [448, 336];

/// What a canvas opens with.
#[derive(Debug)]
pub(crate) enum Content {
    /// An artwork as the library holds it, its title, and what shows it before it is evaluated
    /// (its cached canvas or gallery picture).
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

/// Where the canvas goes once it is left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Then {
    Gallery,
    /// The next lesson's sheet: One more's (or the chain's), the lesson the end named, of the
    /// purpose of the sheet left (a test's stays a test's).
    Practice(openpaint_practice_app::Start, openpaint_practice::Purpose),
}

/// A practice sheet: its session record, the finished sessions of its stage before it and the
/// sheet as drawn.
pub(crate) type SheetOf = Option<openpaint_practice_app::Practising>;

pub(crate) struct Canvas {
    /// An artwork's session, or a practice sheet's.
    work: Work,
    /// The gallery's name for the artwork, which exported files carry.
    title: Title,
    target: CanvasTarget,
    /// The view tiers shown instead of the target while the view magnifies it.
    view: ViewCache,
    /// The last view update left tiles to fill or a tier to start; the view it had.
    view_pending: bool,
    shown: Option<ViewFrame>,
    /// False until the whole canvas has been evaluated once.
    evaluated: bool,
    /// The version shown before this frame's first change other than a stroke (undo, redo, a
    /// layer edit), until it is evaluated.
    changed_from: Option<Artwork>,
    pub(crate) leaving: Leaving,
    pub(crate) then: Then,
    /// What the layer panel's pictures were made of.
    pub(crate) pictures: Drawn,
    /// When it opened (or its device was made again), until the whole canvas was evaluated:
    /// the time to the full canvas is logged then.
    opened: Option<Opening>,
    /// When the snapshots under the next stroke were wanted, until they are taken: the time it
    /// took is logged then.
    ahead: Option<Opening>,
    /// What the display cache holds for it and must match.
    cache: CacheState,
}

impl Canvas {
    /// `content` shown at fit (a practice sheet, given its session in `sheet`, on its shown
    /// pair in `viewport`); a blank artwork is the size of the area it fits in; strokes start
    /// with the toolbox's brush and StreamLine, a long colour copy's with Hard Paint at a size for
    /// its frame (S104). `None` while the area is empty. Later area changes (rotation) refit the
    /// view, which is relative to fit, and never resize the canvas.
    pub(crate) fn open(
        core: &Core,
        (area, viewport): (&CanvasArea, &Viewport),
        content: &Content,
        (toolbox, sheet): (&mut Toolbox, SheetOf),
    ) -> Result<Option<Self>, RuntimeError> {
        let area = &area.fit;
        if area.size.contains(&0) {
            return Ok(None);
        }
        if let Some((key, size)) = sheet.as_ref().and_then(copy_brush) {
            toolbox.paint_with(&key, size);
        }
        let brush = toolbox.style()?;
        let artwork = |session| Work::Artwork(Box::new(session));
        let (mut work, title) = match (content, sheet) {
            (Content::Stored(artwork, title, _), Some(sheet)) => {
                let pens = PracticePens::built_in()?;
                let open = SheetSession::open(artwork.clone(), (brush, pens), sheet, viewport);
                (Work::Sheet(Box::new(open)), title.clone())
            }
            (Content::Stored(stored, title, _), None) => {
                (artwork(Session::open(stored.clone(), brush)), title.clone())
            }
            (Content::Blank, _) => (
                artwork(Session::new(blank_size(area)?, brush)),
                Title::untitled(),
            ),
            (Content::BlankAfterFailedOpen, _) => (
                artwork(Session::after_failed_open(blank_size(area)?, brush)),
                Title::untitled(),
            ),
        };
        work.session_mut().set_streamline(toolbox.streamline());
        let canvas = work.session().artwork().canvas();
        let size = [canvas.width(), canvas.height()];
        log::info!("canvas opened at {} x {}", size[0], size[1]);
        let mut canvas = Self {
            work,
            title,
            target: CanvasTarget::new(core, size)?,
            view: ViewCache::new(core),
            view_pending: false,
            shown: None,
            evaluated: false,
            changed_from: None,
            leaving: Leaving::Staying,
            then: Then::Gallery,
            pictures: Drawn::default(),
            opened: Some(Opening::start(core)),
            ahead: None,
            cache: CacheState::default(),
        };
        if let Content::Stored(_, _, held) = content {
            canvas.show_opened(core, held);
        }
        Ok(Some(canvas))
    }

    /// Commits the stroke being drawn as its lift would, when the pen can no longer lift it: the
    /// app is left or its window goes mid-stroke. Its brush becomes Recent's newest; the
    /// next evaluation shows it committed (its end tapered) and so does the gallery picture.
    pub(crate) fn end_stroke(&mut self, toolbox: &mut Toolbox) {
        let before = self.work.session().shown().clone();
        match self.work.session_mut().end_stroke() {
            Ok(false) => return,
            Ok(true) => log::info!("the stroke being drawn was committed as drawn so far"),
            Err(error) => log::warn!("the stroke being drawn was committed with: {error}"),
        }
        if let Some(drawn) = self.work.session().stroke_committed() {
            toolbox.drew(drawn);
        }
        self.changed_from.get_or_insert(before);
    }

    /// The fill worker computed request `sequence`; a new preview (or the committed fill) is
    /// evaluated by the pixels it changes.
    pub(crate) fn fill_finished(&mut self, sequence: u64, result: Result<RegionMask, FillFailed>) {
        let before = self.work.session().shown().clone();
        self.work.session_mut().fill_finished(sequence, result);
        if !before.same_version(self.work.session().shown()) {
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
        let canvas = self.work.session().artwork().canvas();
        let viewport = area.viewport(pixels_per_point);
        let mapping = shown(self.work.session().view().mapping(canvas, &viewport));
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
        let active = self.work.session().active()?;
        let samples = active.samples();
        let position = samples.last()?.position();
        let radius = tip_radius(active.style(), samples)?;
        matches!(
            active.style().tool(),
            StrokeTool::Erase | StrokeTool::Smudge
        )
        .then_some(([position.x(), position.y()], radius))
    }

    /// The session the open artwork or sheet is drawn in.
    pub(crate) fn session(&self) -> &Session {
        self.work.session()
    }

    pub(crate) fn title(&self) -> &Title {
        &self.title
    }

    /// The session, for what the workers answer and the store asks; the frame's intents go
    /// through `dispatch`.
    pub(crate) fn session_mut(&mut self) -> &mut Session {
        self.work.session_mut()
    }

    /// The practice sheet open on the canvas, if one is.
    pub(crate) fn sheet(&self) -> Option<&SheetSession> {
        self.work.sheet()
    }

    /// The practice sheet open on the canvas, if one is.
    pub(crate) fn sheet_mut(&mut self) -> Option<&mut SheetSession> {
        self.work.sheet_mut()
    }

    /// The toolbox's style for the strokes to come (a sheet's pen draws instead while it does).
    pub(crate) fn set_brush(&mut self, brush: StrokeStyle) {
        self.work.set_brush(brush);
    }

    /// The diameter of the pen a sheet draws with instead of the brush: the hover outline's.
    pub(crate) fn pen(&self) -> Option<f32> {
        self.work.pen()
    }

    pub(crate) fn target(&self) -> &CanvasTarget {
        &self.target
    }
}

/// The size of a blank artwork that fills `area`.
pub(crate) fn blank_size(area: &ScreenArea) -> Result<CanvasSize, RuntimeError> {
    Ok(CanvasSize::new(area.size[0], area.size[1])?)
}

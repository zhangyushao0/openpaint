//! The stroke intent: the pen draws on the active layer. A pen contact the canvas owns begins a
//! stroke when it touches down, extends it while it moves and commits it as one new artwork
//! version when it lifts; a cancelled contact leaves the artwork as it was.
//!
//! Only the pen draws, with the style the toolbox gives (painting, smudging or erasing); its
//! eraser end draws too, with the eraser the toolbox holds for it (S41). Fingers do nothing
//! (D18).
//!
//! The stroke follows the pen through its StreamLine (`streamline.rs`), frozen when the stroke
//! begins: the samples stored are the smoothed line, and the lift moves the line's end onto the
//! pen before the stroke is committed (not a smudge's: its line follows strictly, D46). Held
//! still at its end, it snaps to the shape it resembles (QuickShape, `stroke_shape.rs`); with
//! the Drawing Guide on, it is drawn again mirrored, each copy a stroke of its own committed
//! with it as one step (`stroke_copies.rs`).

use openpaint_core::{
    ActiveStroke, Artwork, CanvasPoint, RegionMask, StrokeSample, StrokeSession, StrokeStyle,
    StrokeTool,
};

use crate::error::AppError;
use crate::history::History;
use crate::input::{Contact, ContactPhase, Device, InputSample};
use crate::streamline::{Follower, StreamLine, catch_up};
use crate::symmetry::{Guide, GuideIntent, Mirror};

#[path = "stroke_copies.rs"]
mod copies;
#[path = "stroke_shape.rs"]
mod shape;

pub use shape::ShapeProjection;

#[derive(Debug, Clone, PartialEq)]
pub enum StrokeIntent {
    /// A contact the canvas owns; positions in canvas pixels.
    Contact(Contact),
    /// The pen is still drawing at `now_ms` (the input clock), sent every frame while it is on
    /// the glass; it holds still while it stays within `still` canvas pixels (QuickShape).
    Hold { now_ms: i64, still: f32 },
    /// Shows the handles of the shape just snapped (true), or is done with it (false).
    EditShape(bool),
    /// Drags handle `handle` of the shape just snapped to `to` (canvas pixels).
    MoveHandle { handle: usize, to: [f32; 2] },
    /// The Drawing Guide's controls.
    Guide(GuideIntent),
}

/// The stroke being drawn and the contact drawing it.
#[derive(Debug)]
struct Drawing {
    contact: i32,
    /// Platform time of the first sample; stroke sample times count from it.
    start_ms: i64,
    session: StrokeSession,
    /// The line's end on its leash, and the pen's last position on the glass.
    follower: Follower,
    pen: [f32; 2],
    /// The style and selection it began with, for its mirrored copies (and a snapped shape
    /// edited later).
    style: StrokeStyle,
    clip: Option<RegionMask>,
    mirrors: Vec<Mirror>,
    hold: shape::Hold,
}

/// The stroke intent's state: the style and StreamLine the next stroke starts with, the
/// Drawing Guide, the stroke in progress and the shape snapped last.
#[derive(Debug)]
pub(crate) struct Strokes {
    style: StrokeStyle,
    /// The seed the next stroke's stamps jitter by (S45).
    seed: u32,
    streamline: StreamLine,
    guide: Guide,
    drawing: Option<Drawing>,
    /// The artwork with the mirrored copies of the stroke being drawn, which the canvas shows
    /// under it.
    preview: Option<Artwork>,
    shaped: Option<shape::Shaped>,
    /// The last intent changed what the canvas shows other than by drawing or committing the
    /// stroke the renderer follows.
    reshown: bool,
}

impl Strokes {
    pub(crate) fn new(style: StrokeStyle, guide: Guide) -> Self {
        Self {
            style,
            seed: 0,
            streamline: StreamLine::OFF,
            guide,
            drawing: None,
            preview: None,
            shaped: None,
            reshown: false,
        }
    }

    /// The next stroke starts with `style`; a stroke in progress keeps its own.
    pub(crate) fn set_style(&mut self, style: StrokeStyle) {
        self.style = style;
    }

    /// The next stroke follows the pen through `streamline`; a stroke in progress keeps its own.
    pub(crate) const fn set_streamline(&mut self, streamline: StreamLine) {
        self.streamline = streamline;
    }

    /// The next stroke's stamps jitter by `seed`; a stroke in progress keeps its own.
    pub(crate) const fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
    }

    pub(crate) fn active(&self) -> Option<&ActiveStroke> {
        self.drawing
            .as_ref()
            .map(|drawing| drawing.session.active())
    }

    /// The artwork with the stroke's mirrored copies, while one with copies is drawn.
    pub(crate) const fn shown(&self) -> Option<&Artwork> {
        self.preview.as_ref()
    }

    pub(crate) const fn guide(&self) -> Guide {
        self.guide
    }

    /// Whether the last intent changed what shows other than by the stroke the renderer follows
    /// (copies, a shape edited).
    pub(crate) const fn reshown(&self) -> bool {
        self.reshown
    }

    /// Applies `intent` to `history`, which changes only when a stroke is committed or a shape
    /// just snapped is edited. A stroke begun while `clip` is selected stays inside it.
    pub(crate) fn handle(
        &mut self,
        history: &mut History,
        intent: StrokeIntent,
        clip: Option<&RegionMask>,
    ) -> Result<(), AppError> {
        self.reshown = false;
        match intent {
            StrokeIntent::Contact(contact) => {
                let mut artwork = history.current().clone();
                let result = self.contact(&mut artwork, contact, clip);
                history.record(artwork);
                result
            }
            StrokeIntent::Hold { now_ms, still } => self.hold(now_ms, still),
            StrokeIntent::EditShape(on) => {
                self.edit_shape(on);
                Ok(())
            }
            StrokeIntent::MoveHandle { handle, to } => self.move_handle(history, handle, to),
            StrokeIntent::Guide(guide) => {
                self.guide.handle(guide, history.current().canvas());
                Ok(())
            }
        }
    }

    fn contact(
        &mut self,
        artwork: &mut Artwork,
        contact: Contact,
        clip: Option<&RegionMask>,
    ) -> Result<(), AppError> {
        if !matches!(contact.device, Device::Pen | Device::Eraser) {
            return Ok(());
        }
        match contact.phase {
            ContactPhase::Down => self.begin(artwork, &contact, clip),
            ContactPhase::Move => self.extend(&contact),
            ContactPhase::Up => {
                let id = contact.id;
                let extended = self.extend(&without_lift(contact));
                let committed = self.finish(artwork, id);
                extended.and(committed)
            }
            ContactPhase::Cancel => {
                if self.drawing_with(contact.id) {
                    self.drawing = None;
                    self.reshown |= self.preview.take().is_some();
                }
                Ok(())
            }
            ContactPhase::Hover | ContactPhase::Leave => Ok(()),
        }
    }

    fn drawing_with(&self, contact: i32) -> bool {
        self.drawing
            .as_ref()
            .is_some_and(|drawing| drawing.contact == contact)
    }

    /// A pen that touches down while another stroke is open keeps that stroke as drawn: its
    /// lift was lost.
    fn begin(
        &mut self,
        artwork: &mut Artwork,
        contact: &Contact,
        clip: Option<&RegionMask>,
    ) -> Result<(), AppError> {
        if let Some(open) = self.drawing.take() {
            self.commit(open, artwork)?;
        }
        let Some(first) = contact.samples.first() else {
            return Ok(());
        };
        let start_ms = first.time_ms;
        let pen = first.position;
        let first = stroke_sample(first, pen, start_ms)?;
        let leash = self.streamline.leash(self.style.size());
        // Its stamps' seed, which its mirrored copies and a shape it snaps to keep.
        let style = self.style.clone().with_seed(self.seed);
        let mut session = artwork.begin_stroke(style.clone(), first)?;
        if let Some(clip) = clip {
            session = session.clipped(clip.clone())?;
        }
        self.drawing = Some(Drawing {
            contact: contact.id,
            start_ms,
            session,
            follower: Follower::new(leash, pen),
            pen,
            style,
            clip: clip.cloned(),
            mirrors: self.guide.mirrors(),
            hold: shape::Hold::default(),
        });
        let rest = Contact {
            samples: contact.samples[1..].to_vec(),
            ..contact.clone()
        };
        self.extend(&rest)
    }

    /// All samples or none.
    fn extend(&mut self, contact: &Contact) -> Result<(), AppError> {
        let Some(drawing) = self.drawing.as_mut() else {
            return Ok(());
        };
        if drawing.contact != contact.id {
            return Ok(());
        }
        if let Some(last) = contact.samples.last()
            && drawing.hold.snapped()
        {
            drawing.pen = last.position;
            return self.follow_snapped();
        }
        let mut follower = drawing.follower;
        let samples = contact
            .samples
            .iter()
            .map(|sample| {
                let at = follower.follow(sample.position);
                stroke_sample(sample, at, drawing.start_ms)
            })
            .collect::<Result<Vec<_>, _>>()?;
        drawing.session.extend(&samples)?;
        drawing.follower = follower;
        if let Some(last) = contact.samples.last() {
            drawing.pen = last.position;
        }
        if !samples.is_empty() {
            self.show_copies()?;
        }
        Ok(())
    }

    fn finish(&mut self, artwork: &mut Artwork, contact: i32) -> Result<(), AppError> {
        if self.drawing_with(contact)
            && let Some(drawing) = self.drawing.take()
        {
            return self.commit(drawing, artwork);
        }
        Ok(())
    }

    /// Commits `drawing` (with its copies) into `artwork`. Its line's end moves onto the pen's
    /// last position on the glass first, unless it snapped to a shape or smudges: a smudge keeps
    /// its end where the line got to, as draweave's: the colour it already carried cannot be
    /// carried again along a moved end, and it commits exactly what was shown. If the moved end
    /// is refused (it cannot be: it lies between the line and the pen), the stroke is committed
    /// as drawn and the refusal returned.
    fn commit(&mut self, mut drawing: Drawing, artwork: &mut Artwork) -> Result<(), AppError> {
        let smudge = drawing.session.active().style().tool() == StrokeTool::Smudge;
        let moved = if smudge || drawing.hold.snapped() {
            Ok(())
        } else {
            catch_up_with_pen(&mut drawing)
        };
        self.reshown |= self.preview.take().is_some();
        *artwork = drawing.copies().committed(&drawing.session)?;
        self.shaped = shape::Shaped::of(drawing);
        moved
    }
}

fn catch_up_with_pen(drawing: &mut Drawing) -> Result<(), AppError> {
    let samples = drawing.session.active().samples();
    let points: Vec<[f32; 2]> = samples
        .iter()
        .map(|sample| [sample.position().x(), sample.position().y()])
        .collect();
    let length = drawing.follower.catch_up_length();
    let Some((first, moved)) = catch_up(&points, drawing.pen, length) else {
        return Ok(());
    };
    let revised = samples[first..]
        .iter()
        .zip(moved)
        .map(|(sample, [x, y])| {
            let (pressure, tilt, azimuth) = (sample.pressure(), sample.tilt(), sample.azimuth());
            let position = CanvasPoint::new(x, y)?;
            StrokeSample::new(position, pressure, tilt, azimuth, sample.time_ms())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(drawing.session.revise(first, &revised)?)
}

/// The lifting contact without the samples of the pen leaving the glass. Android reports the
/// lift with pressure 0, at a point the tip may already have drifted to; drawn, it would end
/// the stroke in a tail tapering to nothing, bent towards that point, instead of the brush's
/// round tip where the ink stopped.
fn without_lift(mut contact: Contact) -> Contact {
    contact.samples.retain(|sample| sample.pressure > 0.0);
    contact
}

/// `sample` drawn at `[x, y]` (where the line follows it), timed from `start_ms`.
fn stroke_sample(
    sample: &InputSample,
    [x, y]: [f32; 2],
    start_ms: i64,
) -> Result<StrokeSample, AppError> {
    let elapsed = u32::try_from((sample.time_ms - start_ms).max(0)).unwrap_or(u32::MAX);
    Ok(StrokeSample::new(
        CanvasPoint::new(x, y)?,
        sample.pressure,
        sample.tilt,
        sample.azimuth,
        elapsed,
    )?)
}

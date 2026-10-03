//! The stroke intent: the pen draws on the active layer. A pen contact the canvas owns begins a
//! stroke when it touches down, extends it while it moves and commits it as one new artwork
//! version when it lifts; a cancelled contact leaves the artwork as it was.
//!
//! Only the pen draws, with the style the toolbox gives (painting or erasing). Fingers and the
//! pen's eraser end do nothing: what they do on the canvas is the author's decision (gesture
//! and pen-button mapping, ARCHITECTURE section 10).
//!
//! The stroke follows the pen through its StreamLine (`streamline.rs`), frozen when the stroke
//! begins: the samples stored are the smoothed line, and the lift moves the line's end onto the
//! pen before the stroke is committed.

use openpaint_core::{
    ActiveStroke, Artwork, CanvasPoint, StrokeSample, StrokeSession, StrokeStyle,
};

use crate::error::AppError;
use crate::input::{Contact, ContactPhase, Device, InputSample};
use crate::streamline::{Follower, StreamLine, catch_up};

#[derive(Debug, Clone, PartialEq)]
pub enum StrokeIntent {
    /// A contact the canvas owns; positions in canvas pixels.
    Contact(Contact),
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
}

/// The stroke intent's state: the style and StreamLine the next stroke starts with and the
/// stroke in progress.
#[derive(Debug)]
pub(crate) struct Strokes {
    style: StrokeStyle,
    streamline: StreamLine,
    drawing: Option<Drawing>,
}

impl Strokes {
    pub(crate) const fn new(style: StrokeStyle) -> Self {
        Self {
            style,
            streamline: StreamLine::OFF,
            drawing: None,
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

    pub(crate) fn active(&self) -> Option<&ActiveStroke> {
        self.drawing
            .as_ref()
            .map(|drawing| drawing.session.active())
    }

    /// Applies `intent` to `artwork`, which changes only when a stroke is committed.
    pub(crate) fn handle(
        &mut self,
        artwork: &mut Artwork,
        intent: StrokeIntent,
    ) -> Result<(), AppError> {
        let StrokeIntent::Contact(contact) = intent;
        if contact.device != Device::Pen {
            return Ok(());
        }
        match contact.phase {
            ContactPhase::Down => self.begin(artwork, &contact),
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
    fn begin(&mut self, artwork: &mut Artwork, contact: &Contact) -> Result<(), AppError> {
        if let Some(open) = self.drawing.take() {
            let (committed, result) = commit(open);
            *artwork = committed;
            result?;
        }
        let Some(first) = contact.samples.first() else {
            return Ok(());
        };
        let start_ms = first.time_ms;
        let pen = first.position;
        let first = stroke_sample(first, pen, start_ms)?;
        let leash = self.streamline.leash(self.style.size());
        let session = artwork.begin_stroke(self.style.clone(), first)?;
        self.drawing = Some(Drawing {
            contact: contact.id,
            start_ms,
            session,
            follower: Follower::new(leash, pen),
            pen,
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
        Ok(())
    }

    fn finish(&mut self, artwork: &mut Artwork, contact: i32) -> Result<(), AppError> {
        if self.drawing_with(contact)
            && let Some(drawing) = self.drawing.take()
        {
            let (committed, result) = commit(drawing);
            *artwork = committed;
            return result;
        }
        Ok(())
    }
}

/// Commits `drawing` with its line's end moved onto the pen's last position on the glass. If
/// the moved end is refused (it cannot be: it lies between the line and the pen), the stroke is
/// committed as drawn and the refusal returned.
fn commit(mut drawing: Drawing) -> (Artwork, Result<(), AppError>) {
    let result = catch_up_with_pen(&mut drawing);
    (drawing.session.finish().artwork, result)
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

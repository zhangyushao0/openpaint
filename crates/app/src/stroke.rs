//! The stroke intent: the pen draws on the active layer. A pen contact the canvas owns begins a
//! stroke when it touches down, extends it while it moves and commits it as one new artwork
//! version when it lifts; a cancelled contact leaves the artwork as it was.
//!
//! Only the pen draws. Fingers and the eraser end do nothing yet: what a finger does on the
//! canvas is the author's decision (ARCHITECTURE section 10), the eraser arrives in S12.

use openpaint_core::{
    ActiveStroke, Artwork, CanvasPoint, Color, StrokeSample, StrokeSession, StrokeStyle,
};

use crate::error::AppError;
use crate::input::{Contact, ContactPhase, Device, InputSample};

/// The brush until brush settings exist (S11): a 12-pixel round black pen at full opacity.
const BRUSH_SIZE: f32 = 12.0;

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
}

/// The stroke intent's state: the brush and the stroke in progress.
#[derive(Debug)]
pub(crate) struct Strokes {
    style: StrokeStyle,
    drawing: Option<Drawing>,
}

impl Strokes {
    pub(crate) fn new() -> Result<Self, AppError> {
        Ok(Self {
            style: StrokeStyle::new(BRUSH_SIZE, 1.0, Color::BLACK)?,
            drawing: None,
        })
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
                let extended = self.extend(&contact);
                self.finish(artwork, contact.id);
                extended
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
            *artwork = open.session.finish().artwork;
        }
        let Some(first) = contact.samples.first() else {
            return Ok(());
        };
        let start_ms = first.time_ms;
        let session = artwork.begin_stroke(self.style, stroke_sample(first, start_ms)?)?;
        self.drawing = Some(Drawing {
            contact: contact.id,
            start_ms,
            session,
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
        let samples = contact
            .samples
            .iter()
            .map(|sample| stroke_sample(sample, drawing.start_ms))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(drawing.session.extend(&samples)?)
    }

    fn finish(&mut self, artwork: &mut Artwork, contact: i32) {
        if self.drawing_with(contact)
            && let Some(drawing) = self.drawing.take()
        {
            *artwork = drawing.session.finish().artwork;
        }
    }
}

/// `sample` as a stroke sample timed from `start_ms`.
fn stroke_sample(sample: &InputSample, start_ms: i64) -> Result<StrokeSample, AppError> {
    let [x, y] = sample.position;
    let elapsed = u32::try_from((sample.time_ms - start_ms).max(0)).unwrap_or(u32::MAX);
    Ok(StrokeSample::new(
        CanvasPoint::new(x, y)?,
        sample.pressure,
        sample.tilt,
        sample.azimuth,
        elapsed,
    )?)
}

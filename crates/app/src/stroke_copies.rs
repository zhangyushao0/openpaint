//! The stroke's mirrored copies while the Drawing Guide is on (S42): each copy is a stroke of
//! its own with the stroke's style and selection, its samples mirrored (`symmetry.rs`); pressure,
//! tilt, azimuth and time stay the drawn ones. While the stroke is drawn the canvas shows the
//! artwork with the copies so far (`Strokes::shown`) under the stroke itself; at the lift the
//! copies and then the stroke are committed as one version, so one undo takes them all, and
//! the stroke lies last, where the renderer followed it.

use openpaint_core::{Artwork, CanvasPoint, RegionMask, StrokeSample, StrokeSession, StrokeStyle};

use super::{Drawing, Strokes};
use crate::error::AppError;
use crate::symmetry::Mirror;

/// What a stroke's copies are drawn with.
#[derive(Debug, Clone, Copy)]
pub(super) struct Copies<'a> {
    pub(super) style: &'a StrokeStyle,
    pub(super) clip: Option<&'a RegionMask>,
    pub(super) mirrors: &'a [Mirror],
}

impl Drawing {
    pub(super) fn copies(&self) -> Copies<'_> {
        Copies {
            style: &self.style,
            clip: self.clip.as_ref(),
            mirrors: &self.mirrors,
        }
    }
}

impl Copies<'_> {
    /// The version `session`'s stroke commits: the stroke alone as drawn, or its copies, then
    /// the stroke, as one version of the artwork it began on.
    pub(super) fn committed(&self, session: &StrokeSession) -> Result<Artwork, AppError> {
        if self.mirrors.is_empty() {
            return Ok(session.clone().finish().artwork);
        }
        let samples = session.active().samples();
        let copied = self.drawn(session.base(), samples)?;
        draw(&copied, *self, samples)
    }

    /// `base` with the copies of a stroke along `samples`, in the guide's order.
    pub(super) fn drawn(
        &self,
        base: &Artwork,
        samples: &[StrokeSample],
    ) -> Result<Artwork, AppError> {
        self.mirrors
            .iter()
            .try_fold(base.clone(), |artwork, mirror| {
                let mirrored = samples
                    .iter()
                    .map(|sample| mirrored(sample, mirror))
                    .collect::<Result<Vec<_>, _>>()?;
                draw(&artwork, *self, &mirrored)
            })
    }
}

impl Strokes {
    /// Shows the copies of the stroke drawn so far, while it has any.
    pub(super) fn show_copies(&mut self) -> Result<(), AppError> {
        let Some(drawing) = &self.drawing else {
            return Ok(());
        };
        if drawing.mirrors.is_empty() {
            return Ok(());
        }
        let session = &drawing.session;
        let preview = drawing
            .copies()
            .drawn(session.base(), session.active().samples())?;
        self.preview = Some(preview);
        self.reshown = true;
        Ok(())
    }
}

/// `artwork` with one more stroke along `samples` (never empty), drawn as `copies` says.
fn draw(
    artwork: &Artwork,
    copies: Copies<'_>,
    samples: &[StrokeSample],
) -> Result<Artwork, AppError> {
    let mut session = artwork.begin_stroke(copies.style.clone(), samples[0])?;
    if let Some(clip) = copies.clip {
        session = session.clipped(clip.clone())?;
    }
    session.extend(&samples[1..])?;
    Ok(session.finish().artwork)
}

fn mirrored(sample: &StrokeSample, mirror: &Mirror) -> Result<StrokeSample, AppError> {
    let position = sample.position();
    let [x, y] = mirror.apply([position.x(), position.y()]);
    Ok(StrokeSample::new(
        CanvasPoint::new(x, y)?,
        sample.pressure(),
        sample.tilt(),
        sample.azimuth(),
        sample.time_ms(),
    )?)
}

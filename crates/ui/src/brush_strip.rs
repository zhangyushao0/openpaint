//! The brush editor's preview strip as the panel keeps it (S46): the strokes the pen drew on it,
//! in strip pixels, the stroke it draws now, where the strip lay in the last pass and the
//! picture the renderer made of it. The pen's contacts that touch down on the strip (outside its
//! Clear) are routed from the raw input before the canvas's, so they carry pressure and tilt;
//! fingers draw nothing. Its strokes stay until Clear or the editor closes, and each is drawn
//! with the brush as set now, so a change shows on all of them.

use std::sync::Arc;

use openpaint_app::{Contact, ContactPhase, Device, InputBatch, InputSample};
use openpaint_core::{CanvasPoint, StrokeSample};
use openpaint_render::RgbaImage;

use crate::components::{ScreenArea, preview_strip};

/// The strip's height, in points.
pub(crate) const STRIP_HEIGHT: f32 = 160.0;

/// What the renderer draws the strip of: its size in pixels and the pen's strokes on it.
#[derive(Debug, Clone, PartialEq)]
pub struct StripStrokes {
    pub size: [u32; 2],
    pub strokes: Vec<Arc<[StrokeSample]>>,
}

/// The stroke the pen draws on the strip now.
#[derive(Debug)]
struct Drawing {
    contact: i32,
    /// The time its first sample was taken, by the input clock.
    start_ms: i64,
    samples: Vec<StrokeSample>,
}

#[derive(Default)]
pub(crate) struct Strip {
    strokes: Vec<Arc<[StrokeSample]>>,
    drawing: Option<Drawing>,
    /// Where the strip and its Clear lay in the last pass, in pixels.
    area: Option<ScreenArea>,
    clear: Option<ScreenArea>,
    /// The renderer's latest picture, waiting to be loaded, and the one loaded.
    arrived: Option<RgbaImage>,
    texture: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for Strip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Strip")
            .field("strokes", &self.strokes.len())
            .field("drawing", &self.drawing)
            .field("area", &self.area)
            .finish_non_exhaustive()
    }
}

impl Strip {
    /// The strip under its heading; Clear takes its strokes away.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(image) = self.arrived.take() {
            let [width, height] = image.size().map(|side| side as usize);
            let picture = egui::ColorImage::from_rgba_unmultiplied([width, height], image.pixels());
            let options = egui::TextureOptions::LINEAR;
            match &mut self.texture {
                Some(texture) => texture.set(picture, options),
                None => self.texture = Some(ui.ctx().load_texture("brush strip", picture, options)),
            }
        }
        let picture = self.texture.as_ref().map(egui::TextureHandle::id);
        let drawn = !self.strokes.is_empty() || self.drawing.is_some();
        let shown = preview_strip(ui, (picture, drawn), STRIP_HEIGHT);
        if shown.cleared {
            self.strokes.clear();
            self.drawing = None;
        }
        let ppp = ui.ctx().pixels_per_point();
        self.area = Some(ScreenArea::from_points(shown.rect, ppp));
        self.clear = Some(ScreenArea::from_points(shown.clear_rect, ppp));
    }

    /// Not shown in this pass (yet): the pen draws nothing there.
    pub(crate) fn hide(&mut self) {
        self.area = None;
        self.clear = None;
    }

    /// What the renderer draws now, once the strip has been laid out.
    pub(crate) fn strokes(&self) -> Option<StripStrokes> {
        let area = self.area?;
        let mut strokes = self.strokes.clone();
        if let Some(drawing) = self.drawing.as_ref().filter(|d| !d.samples.is_empty()) {
            strokes.push(drawing.samples.as_slice().into());
        }
        Some(StripStrokes {
            size: area.size,
            strokes,
        })
    }

    pub(crate) fn set_picture(&mut self, image: RgbaImage) {
        self.arrived = Some(image);
    }

    /// Takes the pen's contacts that touch down on the strip, until they lift.
    pub(crate) fn route(&mut self, batches: &[InputBatch]) {
        let Some(area) = self.area else {
            return;
        };
        for contact in batches.iter().flat_map(|batch| &batch.contacts) {
            if !matches!(contact.device, Device::Pen | Device::Eraser) {
                continue;
            }
            self.contact(contact, area);
        }
    }

    fn contact(&mut self, contact: &Contact, area: ScreenArea) {
        let first = contact.samples.first().map(|sample| sample.position);
        let on_strip = first.is_some_and(|at| {
            area.contains(at) && !self.clear.is_some_and(|clear| clear.contains(at))
        });
        if contact.phase == ContactPhase::Down && on_strip {
            self.drawing = Some(Drawing {
                contact: contact.id,
                start_ms: contact.samples.first().map_or(0, |s| s.time_ms),
                samples: Vec::new(),
            });
        }
        let Some(drawing) = self.drawing.as_mut().filter(|d| d.contact == contact.id) else {
            return;
        };
        match contact.phase {
            ContactPhase::Down | ContactPhase::Move | ContactPhase::Up => {
                let origin = area.origin.map(|side| side as f32);
                let start = drawing.start_ms;
                let samples = contact
                    .samples
                    .iter()
                    .filter_map(|s| sample(s, origin, start));
                drawing.samples.extend(samples);
            }
            ContactPhase::Cancel => self.drawing = None,
            ContactPhase::Hover | ContactPhase::Leave => {}
        }
        if contact.phase == ContactPhase::Up
            && let Some(drawing) = self.drawing.take()
            && !drawing.samples.is_empty()
        {
            self.strokes.push(drawing.samples.into());
        }
    }
}

/// A pen sample in strip pixels from the strip's corner `origin`, its time from the stroke's
/// start; none for the pressure-less sample Android reports as the pen leaves the glass.
fn sample(input: &InputSample, origin: [f32; 2], start_ms: i64) -> Option<StrokeSample> {
    if input.pressure <= 0.0 {
        return None;
    }
    let [x, y] = input.position;
    let at = CanvasPoint::new(x - origin[0], y - origin[1]).ok()?;
    let time = u32::try_from(input.time_ms.saturating_sub(start_ms)).unwrap_or(u32::MAX);
    StrokeSample::new(at, input.pressure, input.tilt, input.azimuth, time).ok()
}

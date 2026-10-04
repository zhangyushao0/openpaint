//! What a scenario run collects: canvas checkpoints, screenshots, view checks and exported
//! files, in step order, and how each record is taken from the runtime.

use std::sync::PoisonError;
use std::time::Duration;

use openpaint_render::RgbaImage;
use openpaint_runtime::Runtime;

use crate::error::ScenarioError;
use crate::scenario::Step;
use crate::system::{Expected, Export, Written};

/// The canvas at a checkpoint and the hash it should have.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub name: String,
    pub expected: u64,
    pub canvas: RgbaImage,
}

impl Checkpoint {
    #[must_use]
    pub fn actual(&self) -> u64 {
        self.canvas.content_hash()
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self.actual() == self.expected
    }
}

/// The screen of the last frame.
#[derive(Debug, Clone)]
pub struct Screenshot {
    pub name: String,
    pub screen: RgbaImage,
}

/// The view at a view checkpoint and what it should be: zoom relative to fit, rotation in
/// degrees, and pan in points when the step gives one.
#[derive(Debug, Clone)]
pub struct ViewCheck {
    pub name: String,
    pub expected: (f32, f32, Option<[f32; 2]>),
    pub actual: (f32, f32, [f32; 2]),
}

impl ViewCheck {
    /// Zoom within 1 %, rotation within half a degree, pan within a point.
    #[must_use]
    pub fn passed(&self) -> bool {
        let (zoom, rotation, pan) = self.expected;
        let (actual_zoom, actual_rotation, actual_pan) = self.actual;
        let turn = (actual_rotation - rotation + 180.0).rem_euclid(360.0) - 180.0;
        (actual_zoom / zoom - 1.0).abs() <= 0.01
            && turn.abs() <= 0.5
            && pan.is_none_or(|[x, y]| {
                (actual_pan[0] - x).abs() <= 1.0 && (actual_pan[1] - y).abs() <= 1.0
            })
    }
}

#[derive(Debug, Clone)]
pub enum Record {
    Checkpoint(Checkpoint),
    Screenshot(Screenshot),
    Export(Export),
    View(ViewCheck),
}

#[derive(Debug, Clone)]
pub struct Report {
    /// In step order.
    pub records: Vec<Record>,
    /// Steps whose contact landed on a control given as window pixels, each with the name to
    /// use instead: a scenario names every control it touches.
    pub unnamed: Vec<String>,
    /// From starting the runtime (GPU device included) to the last step.
    pub elapsed: Duration,
    pub frames: u64,
    pub adapter: String,
}

impl Report {
    pub fn checkpoints(&self) -> impl Iterator<Item = &Checkpoint> {
        self.records.iter().filter_map(|record| match record {
            Record::Checkpoint(checkpoint) => Some(checkpoint),
            _ => None,
        })
    }

    pub fn exports(&self) -> impl Iterator<Item = &Export> {
        self.records.iter().filter_map(|record| match record {
            Record::Export(export) => Some(export),
            _ => None,
        })
    }

    pub fn views(&self) -> impl Iterator<Item = &ViewCheck> {
        self.records.iter().filter_map(|record| match record {
            Record::View(view) => Some(view),
            _ => None,
        })
    }

    /// True when every checkpoint, export and view checkpoint passed and every control touched
    /// was named.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.unnamed.is_empty()
            && self.checkpoints().all(Checkpoint::passed)
            && self.exports().all(Export::passed)
            && self.views().all(ViewCheck::passed)
    }
}

/// The view a view step looks at.
pub(crate) fn view_check(runtime: &Runtime, step: &Step) -> Option<Record> {
    let Step::View {
        name,
        zoom,
        rotation,
        pan,
    } = step
    else {
        return None;
    };
    let view = runtime.projection().view();
    Some(Record::View(ViewCheck {
        name: name.clone(),
        expected: (*zoom, *rotation, *pan),
        actual: (view.zoom(), view.rotation().to_degrees(), view.pan()),
    }))
}

/// What a checkpoint, screenshot or export step looks at.
pub(crate) fn record(
    runtime: &Runtime,
    step: &Step,
    written: &Written,
) -> Result<Record, ScenarioError> {
    let canvas = || {
        runtime
            .read_canvas()?
            .ok_or(ScenarioError::Missing("no canvas is open"))
    };
    Ok(match step {
        Step::Checkpoint { name, canvas: hash } => Record::Checkpoint(Checkpoint {
            name: name.clone(),
            expected: *hash,
            canvas: canvas()?,
        }),
        Step::Exported {
            name,
            scale,
            pixels,
        } => {
            let bytes = written
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take();
            let bytes = bytes.ok_or(ScenarioError::Missing("no export was written"))?;
            let expected = Expected {
                scale: *scale,
                pixels: *pixels,
            };
            Record::Export(Export::of(name, bytes, &canvas()?, expected)?)
        }
        Step::Screenshot { name, .. } => {
            let image = runtime.read_offscreen()?;
            let screen = image.ok_or(ScenarioError::Missing("no frame was drawn"))?;
            Record::Screenshot(Screenshot {
                name: name.clone(),
                screen,
            })
        }
        _ => return Err(ScenarioError::Missing("not a step that records")),
    })
}

//! The brush panel's pictures on a worker of their own (S46): the editor's preview strip
//! (`strip.rs`) whenever what it shows changed, and each row's picture of its brush once the
//! brush is new or changed, while the list shows (the built-ins' are made at start). One at a
//! time, the strip first, so the frame never waits; derived, never stored.

use std::collections::HashMap;
use std::sync::Arc;

use openpaint_app::{Brush, BrushKey, BrushLibrary, Toolbox};
use openpaint_core::StrokeTool;
use openpaint_render::{Core, RgbaImage};
use openpaint_ui::Shell;

use crate::error::RuntimeError;
use crate::preview::{brush_picture, brush_previews, preview_size};
use crate::strip::{StripJob, evaluate_strip};
use crate::worker::Queue;

enum Job {
    Row(Brush, [u32; 2]),
    Strip(StripJob),
}

enum Done {
    Row(Brush, Option<RgbaImage>),
    Strip(Option<RgbaImage>),
}

pub(crate) struct BrushPictures {
    worker: Queue<Job, Done>,
    /// The brush each row's picture was made of.
    rows: HashMap<BrushKey, Brush>,
    /// The strip asked for last, and the picture the worker made of it last.
    strip: Option<StripJob>,
    shown: Option<RgbaImage>,
    /// Pixels per point the row pictures are made at.
    pixels_per_point: f32,
}

impl BrushPictures {
    /// Makes the pictures of `library`'s brushes at once, at `pixels_per_point`, for `shell`.
    pub(crate) fn start(
        core: Arc<Core>,
        library: &BrushLibrary,
        (shell, pixels_per_point): (&mut Shell, f32),
    ) -> Result<Self, RuntimeError> {
        shell.set_brush_previews(brush_previews(&core, library, pixels_per_point)?);
        let rows = library
            .brushes()
            .iter()
            .map(|brush| (brush.key().clone(), brush.clone()))
            .collect();
        let worker = Queue::spawn("openpaint-brushes", move |job| make(&core, job))?;
        Ok(Self {
            worker,
            rows,
            strip: None,
            shown: None,
            pixels_per_point,
        })
    }

    /// True while a picture is being made or waits to be taken.
    pub(crate) fn busy(&self) -> bool {
        self.worker.busy()
    }

    /// The strip's last picture, while the editor shows.
    pub(crate) fn strip(&self) -> Option<&RgbaImage> {
        self.shown.as_ref()
    }

    /// One frame: what the worker made goes to the panel; then, if it is idle, the strip as the
    /// editor shows it now when that changed, else the first row picture out of date while the
    /// list shows. Returns whether a picture arrived, which the next frame shows.
    pub(crate) fn step(&mut self, toolbox: &Toolbox, shell: &mut Shell) -> bool {
        let mut arrived = false;
        while let Some(done) = self.worker.take() {
            arrived = true;
            match done {
                Done::Row(brush, picture) => {
                    let key = brush.key().clone();
                    if let Some(picture) = picture {
                        shell.set_brush_picture(key.clone(), picture);
                    }
                    self.rows.insert(key, brush);
                }
                Done::Strip(Some(picture)) => {
                    shell.set_strip_picture(picture.clone());
                    self.shown = Some(picture);
                }
                Done::Strip(None) => {}
            }
        }
        if self.worker.busy() {
            return arrived;
        }
        let Some(asked) = shell.strip() else {
            self.strip = None;
            self.shown = None;
            self.rows(toolbox.library(), shell);
            return arrived;
        };
        if let Ok(style) = toolbox.style() {
            let job = StripJob {
                size: asked.size,
                style: style.with_tool(StrokeTool::Paint),
                strokes: asked.strokes,
            };
            if self.strip.as_ref() != Some(&job) {
                self.strip = Some(job.clone());
                self.worker.submit(Job::Strip(job));
            }
        }
        arrived
    }

    /// The first brush of `library` whose row picture was made of another brush, or none.
    fn rows(&mut self, library: &BrushLibrary, shell: &Shell) {
        if !shell.brushes_listed() {
            return;
        }
        let stale = library
            .brushes()
            .iter()
            .find(|brush| self.rows.get(brush.key()) != Some(brush));
        if let Some(brush) = stale {
            let size = preview_size(self.pixels_per_point);
            self.worker.submit(Job::Row(brush.clone(), size));
        }
    }
}

fn make(core: &Core, job: Job) -> Done {
    match job {
        Job::Row(brush, size) => {
            let picture = brush_picture(core, &brush, size)
                .inspect_err(|error| log::error!("no picture of {}: {error}", brush.name()))
                .ok();
            Done::Row(brush, picture)
        }
        Job::Strip(job) => Done::Strip(
            evaluate_strip(core, &job)
                .inspect_err(|error| log::error!("no preview strip: {error}"))
                .ok(),
        ),
    }
}

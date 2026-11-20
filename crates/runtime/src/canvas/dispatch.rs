//! The frame's intents on the open canvas: the replay's to it (`replay.rs`; while it is open
//! every other intent is dropped), brush and tool intents to the toolbox (choosing a tool
//! leaves the Selection tool), canvas contacts to the eyedropper while it is on (`pick.rs`), the
//! copy menu to the session with the app's clipboard, the rest through what is open (`Work`); a
//! practice sheet's own intents after them (`practise`); a transform's pixels lifted at once.

use std::time::Instant;

use openpaint_app::{
    Availability, Clipboard, ContactPhase, Intent, LiftFailed, SelectionIntent, StrokeIntent,
    ToolIntent, Toolbox, TransformJob,
};
use openpaint_core::Patch;
use openpaint_practice_app::{Refused, SheetSession};
use openpaint_render::Core;

use super::{Canvas, Leaving};
use crate::seeds::Seeds;

impl Canvas {
    /// Applies the frame's intents in order. A refused intent is logged and skipped: one bad
    /// sample must not stop the frame. A gallery intent the session allows starts leaving;
    /// while leaving, nothing else is applied. Brush and tool intents go to `toolbox`, and the
    /// canvas draws its next stroke with the style that results. While the eyedropper is on,
    /// canvas contacts pick colours instead of drawing (`pick`). The copy menu's commands copy
    /// into and paste from `clipboard`, the app's. Every contact touching down takes the next of
    /// `seeds` for the stroke it may begin. A sheet's own intents follow (`practise`). The
    /// replay's intents go to it; while it is open nothing else applies (pen and fingers on the
    /// canvas do nothing, S163).
    pub(crate) fn dispatch(
        &mut self,
        core: &Core,
        intents: Vec<Intent>,
        (toolbox, clipboard, seeds): (&mut Toolbox, &mut Clipboard, &mut Seeds),
    ) {
        for intent in intents {
            if self.leaving != Leaving::Staying {
                return;
            }
            if let Intent::Replay(replay) = intent {
                self.replay_intent(replay);
                continue;
            }
            if self.replay.is_some() {
                continue;
            }
            if self.toolbox_intent(core, &intent, toolbox) {
                match toolbox.style() {
                    Ok(style) => self.work.set_brush(style),
                    Err(error) => log::error!("no brush style: {error}"),
                }
                self.work.session_mut().set_streamline(toolbox.streamline());
                continue;
            }
            if let Intent::Stroke(StrokeIntent::Contact(contact)) = &intent
                && contact.phase == ContactPhase::Down
            {
                self.work.session_mut().set_seed(seeds.next());
            }
            // Committing a stroke is followed by the canvas target itself, but not its mirrored
            // copies or a snapped shape edited.
            let stroke = matches!(intent, Intent::Stroke(_));
            let gallery = matches!(intent, Intent::Gallery(_));
            let cancel = matches!(&intent, Intent::Stroke(StrokeIntent::Contact(contact))
                if contact.phase == ContactPhase::Cancel && self.work.session().active().is_some());
            let before = self.work.session().shown().clone();
            let result = match intent {
                Intent::Clipboard(action) => self.work.session_mut().clipboard(action, clipboard),
                intent => self.work.dispatch(intent),
            };
            let session = self.work.session_mut();
            if cancel && session.active().is_none() {
                log::info!("the platform cancelled the stroke being drawn; it is dropped");
            }
            match result {
                Ok(()) if gallery => self.leaving = Leaving::Waiting,
                Ok(()) if !stroke || session.stroke_reshown() => {
                    self.changed_from.get_or_insert(before);
                }
                Ok(()) => {}
                Err(error) => log::warn!("intent refused: {error}"),
            }
            // A committed stroke's brush is Recent's newest.
            if let Some(drawn) = session.stroke_committed().filter(|_| stroke) {
                toolbox.drew(drawn);
            }
            // A transform just begun lifts its pixels now, so the frame shows it at once.
            if let Some(job) = session.transform_request() {
                session.transform_lifted(lift(core, &job));
            }
        }
    }

    /// A practice sheet's own intent `apply` as the frame's intents are: none while leaving, a
    /// refusal logged (off a sheet, as nothing left), a success evaluated against what showed
    /// before. Whether it applied.
    pub(crate) fn practise<E: std::fmt::Display>(
        &mut self,
        apply: impl FnOnce(&mut SheetSession) -> Result<(), E>,
    ) -> bool {
        if self.leaving != Leaving::Staying || self.replay.is_some() {
            return false;
        }
        let Some(sheet) = self.work.sheet_mut() else {
            let refused = Refused::Unavailable(Availability::NothingLeft);
            log::warn!("intent refused: {refused}");
            return false;
        };
        let before = sheet.session().shown().clone();
        match apply(sheet) {
            Ok(()) => {
                self.changed_from.get_or_insert(before);
                true
            }
            Err(error) => {
                log::warn!("intent refused: {error}");
                false
            }
        }
    }

    /// Hands a brush or tool intent to `toolbox`, and a canvas contact to the eyedropper while it
    /// is on. Returns whether `intent` was the toolbox's.
    fn toolbox_intent(&mut self, core: &Core, intent: &Intent, toolbox: &mut Toolbox) -> bool {
        match intent {
            Intent::Brush(brush) => toolbox.dispatch(brush),
            Intent::Tool(tool) => {
                toolbox.dispatch_tool(tool);
                // Choosing a tool or the eyedropper leaves the Selection tool and commits a
                // transform (Procreate).
                let chosen = matches!(
                    tool,
                    ToolIntent::Use(_) | ToolIntent::Switch | ToolIntent::Eyedropper(true)
                );
                if chosen {
                    self.leave_selection();
                }
            }
            Intent::Stroke(StrokeIntent::Contact(contact)) if toolbox.picking() => {
                self.pick(core, contact, toolbox);
            }
            _ => return false,
        }
        true
    }

    /// Leaves the Selection tool (a transform under way is committed), noting what changed.
    fn leave_selection(&mut self) {
        let before = self.work.session().shown().clone();
        let leave = Intent::Selection(SelectionIntent::Tool(false));
        if let Err(error) = self.work.dispatch(leave) {
            log::warn!("intent refused: {error}");
        }
        if !before.same_version(self.work.session().shown()) {
            self.changed_from.get_or_insert(before);
        }
    }
}

/// A transform's pixels: the job's layer sampled through the renderer (a target of its own), then
/// its painted pixels inside the job's selection, if any. Waits for the GPU, once per transform.
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

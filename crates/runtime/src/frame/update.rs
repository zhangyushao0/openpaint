//! A frame's work before drawing: the workers' answers, the input, the replay's playhead, the UI
//! pass, the intents routed and applied, the budget's step, the canvas evaluated and its view
//! tiers.

use openpaint_app::{Intent, TransformIntent};
use openpaint_render::UiFrame;
use openpaint_ui::CanvasArea;

use super::Runtime;
use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::pointer::pointer_events;
use crate::practice;
use crate::storage::Job;
use crate::transfer::route;
use crate::ui_pass::{UiPass, run_ui};

impl Runtime {
    /// Everything before drawing: storage results, input, UI, intents, evaluation, storage
    /// calls. Asks for another frame when the screen changed what it offers, so it never
    /// shows a stale state while idle, while waiting for the storage worker and while
    /// something moves (D44), the replay playing or its video being made among it.
    pub(crate) fn update(&mut self, size: [u32; 2]) -> Result<(UiFrame, bool), RuntimeError> {
        // This frame's budget for background work, corrected by the GPU's measured times;
        // while nothing touches the screen, what the GPU's queue has room for.
        let idle = self.begin_background();
        // A benchmark times the frame's GPU work from its first submission on.
        self.core.frame_begun();
        self.receive()?;
        let batches = std::mem::take(&mut self.pending);
        let now = self.frame_clock.wall_seconds();
        self.clocked(now);
        self.replay_clocks();
        // A use of the pen (touching or hovering), a finger or a control on a practice sheet.
        if let (false, Some(canvas)) = (batches.is_empty(), self.screen.canvas_mut())
            && let Some(sheet) = practice::sheet(canvas)
        {
            sheet.practised(now);
        }
        let mut events = pointer_events(&batches, self.pixels_per_point);
        events.append(&mut self.typed);
        self.shell.set_now(self.clock.now_ms());
        self.shell.set_contacts(&batches);
        let language = self.language();
        self.shell.set_language(language);
        let shown = self.screen();
        let practised = practice::Shown::of(&self.screen, &self.practice);
        let mut hooked = practised.hooked(&mut self.practice_screens);
        let laid = std::time::Instant::now();
        let pass = run_ui(
            &self.ui,
            UiPass {
                shell: &mut self.shell,
                projection: (&shown, hooked.hooks()),
                size,
                pixels_per_point: self.pixels_per_point,
                events,
                time: self.frame_clock.seconds(),
            },
        );
        self.ui_ns = u64::try_from(laid.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.keyboard = pass.keyboard;
        let intents = self.shell.route(&batches, hooked.hooks());
        let practising = self.practice_screens.take();
        let mut intents = route(&mut self.transfers, self.screen.canvas(), intents);
        // A picture read for inserting goes to the canvas open now, before this frame's taps.
        if let Some(picture) = self.transfers.take_picture() {
            intents.insert(0, Intent::Transform(TransformIntent::Insert(picture)));
        }
        let area = self.shell.canvas_area();
        self.open_canvas()?;
        let arrived = self.apply(intents, practising);
        if let Some(state) = self.toolbox.save_request() {
            self.storage.submit(Job::SaveEditor(Box::new(state)));
        }
        self.keep_within_budget()?;
        self.answer_raster();
        self.evaluate_screen(area)?;
        self.core.end_background();
        self.send_raster(idle);
        let changed = self.screen() != shown
            || practice::Shown::of(&self.screen, &self.practice) != practised
            || self.language() != language;
        // Also until the next tier's atlas is asked for, made ahead once nothing touches.
        let viewing = (self.screen.canvas())
            .is_some_and(|canvas| canvas.waiting() || canvas.view().wants_spare());
        let shows = changed || arrived || viewing || self.replay_moving();
        let again = pass.repaint || shows || self.storing() || self.shell.animating();
        Ok((pass.frame, again))
    }

    /// The screen's evaluations this frame: where it looks on the canvas (its waiting tiles there
    /// go first), what changed and storage's steps, then the view tiers through the view.
    fn evaluate_screen(&mut self, area: Option<CanvasArea>) -> Result<(), RuntimeError> {
        let now = self.frame_clock.wall_seconds();
        let (shell, pen) = (&self.shell, self.hovering());
        let frame = match (self.screen.canvas_mut(), area) {
            (Some(canvas), Some(area)) => {
                let frame = canvas.view_frame((&area, self.pixels_per_point), |mapping| {
                    shell.displayed(mapping)
                });
                canvas.look_at(&frame, pen);
                Some(frame)
            }
            _ => None,
        };
        let workers = (&mut self.storage, &mut self.cache);
        let practice = (&mut self.practice, &mut self.seeds);
        let at = (area, now, self.shell.shown_part());
        if let Some(picture) = self.screen.progress(&self.core, workers, at, practice)? {
            self.shell.carry(picture);
        }
        if let (Some(canvas), Some(frame)) = (self.screen.canvas_mut(), frame) {
            canvas.update_view(&self.core, (&frame, self.frame_clock.seconds()))?;
        }
        Ok(())
    }

    /// The open canvas is replayed and its replay plays or makes its video: frames are wanted
    /// every vsync, as for motion.
    pub(crate) fn replay_moving(&self) -> bool {
        self.screen.canvas().is_some_and(Canvas::replay_moving)
    }
}

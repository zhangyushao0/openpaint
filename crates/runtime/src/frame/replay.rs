//! The replay's part of the frame loop (S163): its playhead on the frame clock, what the video
//! worker made, the replay's requests of the host and the encoder it answers with, handed to the
//! video worker with the end card's line laid in the language shown; the speed chosen last and
//! the host's text faces kept across artworks (`Replays`).

use openpaint_app::{HostAnswer, HostRequest, Intent, ReplayIntent, Speed};
use openpaint_ui::design::TextFonts;
use openpaint_ui::{caption_band, video_caption};

use super::Runtime;
use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::shared::SharedCore;
use crate::video::VideoWorker;

/// What the frame loop keeps for the replay across artworks.
pub(crate) struct Replays {
    /// The speed chosen last in the app's run, which the next replay opens at.
    speed: Speed,
    /// The host's text faces, which the video's last line is laid with as the screen's text.
    pub(crate) fonts: TextFonts,
    video: VideoWorker,
}

impl Replays {
    pub(crate) fn start(core: SharedCore) -> Result<Self, RuntimeError> {
        Ok(Self {
            speed: Speed::default(),
            fonts: TextFonts::default(),
            video: VideoWorker::start(core)?,
        })
    }

    /// `dispatch` on `canvas`, its replay opening at the speed chosen last: the speed it chose
    /// kept, its Cancel told to the video worker.
    pub(crate) fn around(&mut self, canvas: &mut Canvas, dispatch: impl FnOnce(&mut Canvas)) {
        canvas.set_replay_speed(self.speed);
        dispatch(canvas);
        self.speed = canvas.replay_speed();
        if canvas.take_video_cancel() {
            self.video.cancel();
        }
    }
}

impl Runtime {
    /// Before the frame's UI pass: the replay moved on by the frame clock.
    pub(crate) fn replay_clocks(&mut self) {
        let now = self.frame_clock.millis();
        if let Some(canvas) = self.screen.canvas_mut() {
            canvas.tick_replay(now);
        }
    }

    /// The video worker's progress and its job's end go to the open canvas's replay.
    pub(crate) fn receive_video(&mut self) {
        let made = self.replays.video.made();
        let ended = self.replays.video.take();
        match self.screen.canvas_mut() {
            Some(canvas) => canvas.video_progress(made, ended),
            None if ended.is_some() => log::warn!("a video ended with no canvas open"),
            None => {}
        }
    }

    /// The replay's next request of the host, once.
    pub(crate) fn replay_request(&mut self) -> Option<HostRequest> {
        self.screen.canvas_mut()?.replay_mut()?.request()
    }

    /// The replay waits for the host's encoder.
    pub(crate) fn replay_waiting(&self) -> bool {
        (self.screen.canvas()).is_some_and(Canvas::replay_waiting)
    }

    /// The host's answer while the replay waits: an encoder makes the video job, which goes to
    /// the video worker with the end card's line laid, in the language shown.
    pub(crate) fn answer_replay(&mut self, answer: HostAnswer) {
        let language = self.language();
        let Some(replay) = self.screen.canvas_mut().and_then(Canvas::replay_mut) else {
            log::warn!("an encoder came for no replay");
            return;
        };
        let Some(job) = replay.answer(answer) else {
            return;
        };
        let caption = job.caption.map(|facts| {
            let band = caption_band(job.order.size);
            let fonts = &self.replays.fonts;
            video_caption(fonts, &facts, (language, self.utc_offset), band)
        });
        self.replays.video.submit(job, caption);
    }

    /// A video is being made, or its end waits for the next frame (the harness waits for it).
    #[must_use]
    pub fn making_video(&self) -> bool {
        self.replays.video.busy()
    }

    /// The replay's `intent` applied to the open canvas at once, as its control's tap applies it
    /// in a frame (`Open` as Actions' row): for tests, which drive the replay without the places
    /// of its controls. The next frame shows it.
    pub fn replay_intent(&mut self, intent: ReplayIntent) {
        let Some(canvas) = self.screen.canvas_mut() else {
            log::warn!("a replay intent with no canvas open");
            return;
        };
        let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
        let intents = vec![Intent::Replay(intent)];
        let core = &self.core;
        (self.replays).around(canvas, |canvas| canvas.dispatch(core, intents, app));
    }
}

//! The replay on the open canvas (S163). While it is open the canvas shows the replay's frame
//! instead of the session's version, drawn by the target's own path (consecutive moments share
//! their records: moving on is the scene's append, the stroke being drawn the live path) and the
//! view tiers; the canvas lies at fit in the area the shell gives it (between the top band and
//! the bar), with a thin ring at the tip of the stroke being drawn; every intent but the replay's
//! is dropped (`dispatch.rs`, `transfer::route`); the selection does not show. The session, its
//! history and the view stay as they were under it. Its gallery picture and display cache stay
//! the artwork's: the picture is drawn from the session's version off the target while the replay
//! shows, and no cache is written.

use openpaint_app::{
    AppError, Availability, EditorAction, Intent, Replay, ReplayIntent, ReplayShown, Session,
    Speed, Thumbnail, VideoError, ViewMapping,
};
use openpaint_core::Artwork;
use openpaint_render::{
    BrushCursor, Core, FrameRequest, RenderError, thumbnail_reference, thumbnail_size,
};

use super::work::Work;
use super::{Canvas, PAPER, THUMBNAIL_MAX};
use crate::error::RuntimeError;

/// The ring at the tip of the stroke being drawn: its radius and its line, in points.
const RING_RADIUS: f32 = 6.0;
const RING_WIDTH: f32 = 1.0;

impl Canvas {
    /// One replay intent: `Open` through the session (what it ends ends first, whether it is
    /// offered; never on a practice sheet), then the replay of its current version opens at the
    /// speed last chosen; the rest are the replay's own (`Close` drops it). A refusal is logged.
    pub(super) fn replay_intent(&mut self, intent: ReplayIntent) {
        let before = self.shown_artwork().clone();
        let result = match intent {
            ReplayIntent::Open => self.open_replay(),
            intent => self.replay_dispatch(intent),
        };
        if let Err(error) = result {
            log::warn!("intent refused: {error}");
        }
        if !before.same_version(self.shown_artwork()) {
            self.changed_from.get_or_insert(before);
        }
    }

    fn open_replay(&mut self) -> Result<(), AppError> {
        if self.replay.is_some() {
            return Ok(());
        }
        if self.work.sheet().is_some() {
            return Err(AppError::Unavailable {
                action: EditorAction::Replay,
                availability: Availability::NothingLeft,
            });
        }
        self.work.dispatch(Intent::Replay(ReplayIntent::Open))?;
        let artwork = self.work.session().artwork();
        let replay = Replay::open(artwork, &self.title, self.replay_speed);
        log::info!("replay opened at {:?}", self.replay_speed);
        self.replay = Some(Box::new(replay));
        Ok(())
    }

    fn replay_dispatch(&mut self, intent: ReplayIntent) -> Result<(), AppError> {
        let replay = self.replay.as_mut().ok_or(AppError::Unavailable {
            action: EditorAction::Replay,
            availability: Availability::NothingLeft,
        })?;
        replay.dispatch(&intent)?;
        match intent {
            ReplayIntent::Close => {
                self.replay = None;
                log::info!("replay closed");
            }
            ReplayIntent::Speed(speed) => self.replay_speed = speed,
            ReplayIntent::Cancel => self.video_cancelled = true,
            _ => {}
        }
        Ok(())
    }

    /// The frame clock at `now_ms`: the replay moves on, what it shows evaluated by what changed.
    pub(crate) fn tick_replay(&mut self, now_ms: u64) {
        let Some(replay) = self.replay.as_mut() else {
            return;
        };
        let before = replay.frame().artwork.clone();
        replay.tick(now_ms);
        if !before.same_version(&replay.frame().artwork) {
            self.changed_from.get_or_insert(before);
        }
    }

    /// The replay is open.
    pub(crate) fn replaying(&self) -> bool {
        self.replay.is_some()
    }

    /// The replay, while it is open: its requests of the host, its answers.
    pub(crate) fn replay_mut(&mut self) -> Option<&mut Replay> {
        self.replay.as_deref_mut()
    }

    /// The replay waits for the host's encoder.
    pub(crate) fn replay_waiting(&self) -> bool {
        self.replay.as_ref().is_some_and(|replay| replay.waiting())
    }

    /// The replay plays or makes its video: frames are wanted every vsync.
    pub(crate) fn replay_moving(&self) -> bool {
        self.replay
            .as_ref()
            .is_some_and(|replay| replay.playing() || replay.making())
    }

    /// What the projection carries of the replay: none on a practice sheet, its screen while it
    /// is open, else the session's own offer (`None`).
    pub(crate) fn replay_shown(&self) -> Option<ReplayShown> {
        if self.work.sheet().is_some() {
            return Some(ReplayShown::None);
        }
        let replay = self.replay.as_ref()?;
        Some(ReplayShown::Open(Box::new(replay.projection())))
    }

    /// Leaving the app pauses it.
    pub(crate) fn pause_replay(&mut self) {
        if let Some(replay) = self.replay.as_mut().filter(|replay| replay.playing())
            && let Err(error) = replay.dispatch(&ReplayIntent::Play(false))
        {
            log::warn!("the replay did not pause: {error}");
        }
    }

    /// The speed the next replay opens at, the app's.
    pub(crate) fn set_replay_speed(&mut self, speed: Speed) {
        self.replay_speed = speed;
    }

    /// The speed last chosen.
    pub(crate) const fn replay_speed(&self) -> Speed {
        self.replay_speed
    }

    /// Whether the user cancelled the video since the last call: the worker stops.
    pub(crate) fn take_video_cancel(&mut self) -> bool {
        std::mem::take(&mut self.video_cancelled)
    }

    /// The video worker made `made` frames so far, and its job ended with `ended` when given.
    pub(crate) fn video_progress(&mut self, made: u32, ended: Option<Result<(), VideoError>>) {
        let Some(replay) = self.replay.as_mut() else {
            if ended.is_some() {
                log::warn!("a video ended after its replay closed");
            }
            return;
        };
        if replay.making() {
            replay.progress(made);
        }
        if let Some(ended) = ended {
            replay.finished(ended);
        }
    }

    /// The thin ring at the tip of the stroke the replay draws, through `mapping` on a screen of
    /// `pixels_per_point`; none between strokes.
    pub(super) fn replay_ring(
        &self,
        mapping: &ViewMapping,
        pixels_per_point: f32,
    ) -> Option<BrushCursor> {
        let active = self.replay.as_ref()?.frame().active.as_ref()?;
        let tip = active.samples().last()?.position();
        Some(BrushCursor {
            centre: mapping.to_screen([tip.x(), tip.y()]),
            radius: RING_RADIUS * pixels_per_point,
            width: RING_WIDTH * pixels_per_point,
        })
    }

    /// What the canvas shows: the replay's frame while it is open, else the session's version.
    pub(super) fn shown_artwork(&self) -> &Artwork {
        match &self.replay {
            Some(replay) => &replay.frame().artwork,
            None => self.work.session().shown(),
        }
    }

    /// The gallery picture of the session's version while the replay shows on the target: drawn
    /// cold off the frame loop's target and reduced as the GPU's thumbnail reduces it.
    pub(super) fn session_picture(&self, core: &Core) -> Result<Thumbnail, RuntimeError> {
        let artwork = self.work.session().shown();
        let canvas = artwork.canvas();
        let size = [canvas.width(), canvas.height()];
        let image = core.export_region(artwork, PAPER, ([0, 0], size))?;
        let small = thumbnail_size(size, THUMBNAIL_MAX);
        let pixels = thumbnail_reference(&image, small);
        let wrong = RenderError::Invalid("a thumbnail of the wrong length");
        Thumbnail::new(small, pixels).ok_or(RuntimeError::Render(wrong))
    }
}

/// What the canvas evaluates: the replay's frame while it is open (the moment's artwork and the
/// stroke being drawn), else what `work`'s session shows and the stroke being drawn; on the paper.
pub(super) fn shown_request<'a>(work: &'a Work, replay: Option<&'a Replay>) -> FrameRequest<'a> {
    match replay {
        Some(replay) => FrameRequest {
            artwork: &replay.frame().artwork,
            active: replay.frame().active.as_ref(),
            background: PAPER,
        },
        None => session_request(work.session()),
    }
}

fn session_request(session: &Session) -> FrameRequest<'_> {
    FrameRequest {
        artwork: session.shown(),
        active: session.active(),
        background: PAPER,
    }
}

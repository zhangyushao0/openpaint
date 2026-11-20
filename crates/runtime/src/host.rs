//! What the host tells the runtime outside input and frames: leaving the app (the replay
//! pauses), its answers to the system dialogs the app asked for and the replay's encoder, the
//! device's time zone, and in debug builds the screen's precision check.

use std::time::Duration;

use openpaint_app::{BytesSource, FileJob, GalleryIntent, HostAnswer, HostRequest};
use openpaint_practice_app::OptionsIntent;
use openpaint_ui::CanvasArea;

use crate::canvas::Leaving;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::practice;
use crate::screen::leave_job;
use crate::storage::Job;

/// Longest `leave` waits for the store; a longer save goes on in the background.
const LEAVE_WAIT: Duration = Duration::from_secs(2);

impl Runtime {
    /// The user is leaving the app (Android `onPause`): the Pomodoro's timer holds, then a stroke
    /// still being drawn is committed as drawn so far, then the open artwork's current version (a failed one included) and
    /// its thumbnail are handed to the store, and this waits until they are written, at most
    /// `LEAVE_WAIT`; a longer save goes on in the background (it is one batch: it lands whole or
    /// not at all, also if the process is killed meanwhile). In the gallery, the calls handed over
    /// are waited for.
    pub fn leave(&mut self) {
        // The Pomodoro's timer holds while the app is hidden (S124), stored as it stands; a
        // block's end stays for the sheet's return.
        self.practice.hidden(self.frame_clock.wall_seconds());
        practice::timer_saved(&mut self.practice, &mut self.storage);
        if let Some(canvas) = self.screen.canvas_mut() {
            canvas.end_stroke(&mut self.toolbox);
            canvas.pause_replay();
            // Time hidden is no practice: a sheet's time so far goes to the store.
            if let Some(sheet) = practice::sheet(canvas) {
                sheet.hidden();
            }
            practice::save_record(canvas, (&mut self.practice, &mut self.storage));
        }
        if let Some(state) = self.toolbox.save_request() {
            self.storage.submit(Job::SaveEditor(Box::new(state)));
        }
        // A save running when the user left may hold an older version: hand over the newest
        // once it is done.
        let mut handed = false;
        let now = self.frame_clock.wall_seconds();
        for _ in 0..2 {
            if let Some(canvas) = self.screen.canvas_mut()
                && canvas.leaving == Leaving::Staying
                && !canvas.session().saving()
            {
                let version = canvas.session_mut().leave_request();
                if version.is_some() || !handed {
                    self.storage
                        .submit(leave_job(canvas, &self.core, version, (now, false)));
                    handed = true;
                }
            }
            if !self.storage.drain(LEAVE_WAIT) {
                log::warn!("leaving while the store is still writing");
            }
            if let Err(error) = self.receive() {
                log::error!("after leaving: {error}");
            }
        }
    }

    /// The next thing the app asks the host to show or make (a system picker, a file to share,
    /// the share sheet, the replay's video encoder), once; call after each frame until `None`.
    pub fn host_request(&mut self) -> Option<HostRequest> {
        self.transfers
            .request()
            .or_else(|| self.replay_request())
            .or_else(|| self.screen.gallery_mut()?.host_request())
    }

    /// The host's answer to the last request that asks for one. The work it starts runs on the
    /// transfer worker (the video worker for the replay's encoder) at once, also while the app
    /// has no window; the host should ask for a frame.
    pub fn answer(&mut self, answer: HostAnswer) {
        if self.transfers.waiting() {
            log::info!("the host answered: {answer:?}");
            if let Some(job) = self.transfers.answer(answer) {
                self.transfer.submit(job);
            }
        } else if self.replay_waiting() {
            log::info!("the host answered the replay: {answer:?}");
            self.answer_replay(answer);
        } else if let Some((gallery, options)) =
            self.screen.options_mut().filter(|(g, _)| g.waiting())
        {
            log::info!("the host answered the gallery: {answer:?}");
            match gallery.host_answer(answer) {
                Some(FileJob::Mode { tag, source }) => {
                    if let Some(job) = practice::chosen(options, (tag, source)) {
                        self.storage.submit(job);
                    }
                }
                Some(job) => self.storage.submit(Job::File(job)),
                None => {}
            }
        } else {
            log::warn!("an answer from the host nobody waits for: {answer:?}");
        }
    }

    /// The device's offset from UTC in minutes now (the host's time zone), for the dates of kept
    /// versions and the practice book's days; 0 until the host says.
    pub fn set_utc_offset(&mut self, minutes: i32) {
        self.utc_offset = minutes;
        self.shell.set_utc_offset(minutes);
    }

    /// Where the canvas shows and fits on the screen, in pixels, once a frame was laid out (the
    /// device smoke places its strokes in the fit).
    #[must_use]
    pub fn canvas_area(&self) -> Option<CanvasArea> {
        self.shell.canvas_area()
    }

    /// For debug builds only (`cargo device-input open-file`): the gallery's Open OpenPaint
    /// file… with the file `bytes` as the picker's answer at once, no picker shown: the file goes
    /// the import's way and its new card comes first; a practice pack (its magic) goes Import
    /// practice pack…'s way instead.
    ///
    /// # Errors
    /// `NotNow` when the gallery does not show; `App` while it opens an artwork or a file is
    /// being read or written.
    pub fn open_file_now(&mut self, bytes: Vec<u8>) -> Result<(), RuntimeError> {
        let (gallery, options) =
            (self.screen.options_mut()).ok_or(RuntimeError::NotNow("the gallery"))?;
        if bytes.starts_with(b"OpenPaintPack\0") {
            options.dispatch(OptionsIntent::ImportPack, gallery)?;
        } else {
            gallery.dispatch(GalleryIntent::OpenFile)?;
        }
        while let Some(request) = gallery.host_request() {
            log::info!("not shown, the file given at once: {request:?}");
        }
        let job = match gallery.host_answer(HostAnswer::Image(Box::new(BytesSource(bytes)))) {
            Some(FileJob::Mode { tag, source }) => practice::chosen(options, (tag, source)),
            other => other.map(Job::File),
        };
        let job = job.ok_or(RuntimeError::NotNow("a gallery taking the file"))?;
        self.storage.submit(job);
        Ok(())
    }

    /// For debug builds only (`cargo device-input gradient on|off`): the screen's precision
    /// check over everything from the next frame on (a soft grey ramp, its upper half at 8 bits,
    /// its lower half at the window's), or off again.
    pub fn show_gradient(&mut self, on: bool) {
        if let Some(painter) = &mut self.painter {
            painter.show_gradient(&self.core, on);
        }
    }
}

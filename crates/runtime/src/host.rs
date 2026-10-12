//! What the host tells the runtime outside input and frames: leaving the app, its answers to
//! the system dialogs the app asked for, the device's time zone, and in debug builds the
//! screen's precision check.

use std::time::Duration;

use openpaint_app::{HostAnswer, HostRequest};

use crate::canvas::Leaving;
use crate::frame::Runtime;
use crate::screen::leave_job;
use crate::storage::Job;

/// Longest `leave` waits for the store; a longer save goes on in the background.
const LEAVE_WAIT: Duration = Duration::from_secs(2);

impl Runtime {
    /// The user is leaving the app (Android `onPause`): a stroke still being drawn is committed
    /// as drawn so far (S50), then the open artwork's current version (a failed one included) and
    /// its thumbnail are handed to the store, and this waits until they are written, at most
    /// `LEAVE_WAIT`; a longer save goes on in the background (it is one batch: it lands whole or
    /// not at all, also if the process is killed meanwhile). In the gallery, the calls handed over
    /// are waited for.
    pub fn leave(&mut self) {
        if let Some(canvas) = self.screen.canvas_mut() {
            canvas.end_stroke(&mut self.toolbox);
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
    /// the share sheet), once; call after each frame until `None`.
    pub fn host_request(&mut self) -> Option<HostRequest> {
        self.transfers
            .request()
            .or_else(|| self.screen.gallery_mut()?.host_request())
    }

    /// The host's answer to the last request that asks for one. The work it starts runs on the
    /// transfer worker at once, also while the app has no window; the host should ask for a
    /// frame.
    pub fn answer(&mut self, answer: HostAnswer) {
        if self.transfers.waiting() {
            log::info!("the host answered: {answer:?}");
            if let Some(job) = self.transfers.answer(answer) {
                self.transfer.submit(job);
            }
        } else if let Some(gallery) = self.screen.gallery_mut().filter(|g| g.waiting()) {
            log::info!("the host answered the gallery: {answer:?}");
            if let Some(job) = gallery.host_answer(answer) {
                self.storage.submit(Job::File(job));
            }
        } else {
            log::warn!("an answer from the host nobody waits for: {answer:?}");
        }
    }

    /// The device's offset from UTC in minutes now (the host's time zone), for the dates of kept
    /// versions; 0 until the host says.
    pub fn set_utc_offset(&mut self, minutes: i32) {
        self.shell.set_utc_offset(minutes);
    }

    /// For debug builds only (`cargo device-input gradient on|off`, S37): the screen's precision
    /// check over everything from the next frame on (a soft grey ramp, its upper half at 8 bits,
    /// its lower half at the window's), or off again.
    pub fn show_gradient(&mut self, on: bool) {
        if let Some(painter) = &mut self.painter {
            painter.show_gradient(&self.core, on);
        }
    }
}

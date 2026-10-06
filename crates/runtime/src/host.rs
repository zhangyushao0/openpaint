//! What the host tells the runtime outside input and frames: the device's time zone, and in
//! debug builds the screen's precision check.

use crate::frame::Runtime;

impl Runtime {
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

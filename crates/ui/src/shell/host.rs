//! What the host sets on the shell between passes: the input clock and the contacts of the next
//! frame, the device's zone, the app's version, the language and a mode's names.

use openpaint_app::Language;

use super::Shell;
use crate::text::Names;

impl Shell {
    /// The input clock now (the time input samples carry) for the next frame; call before `show`.
    pub fn set_now(&mut self, now_ms: i64) {
        self.now_ms = now_ms;
    }

    /// The contacts the next frame routes; call before `show`, so what it lays over the canvas
    /// lies where they move it (`foreseen`).
    pub fn set_contacts(&mut self, batches: &[openpaint_app::InputBatch]) {
        self.contacts = batches.to_vec();
    }

    /// The device's offset from UTC in minutes, for the dates the home and the replay show.
    pub fn set_utc_offset(&mut self, minutes: i32) {
        self.home.utc_offset = minutes;
    }

    /// The app's version and storage format, which Actions' last line shows.
    pub fn set_version(&mut self, version: &str, format: u32) {
        self.panels.actions.version = Some((version.to_owned(), format));
    }

    /// The language the interface shows from the next pass on (D100).
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// The names a mode gives, which show in the language from the next pass on (D112).
    pub fn set_names(&mut self, names: Names) {
        self.names = names;
    }
}

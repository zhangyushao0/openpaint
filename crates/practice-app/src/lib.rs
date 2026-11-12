//! Practice's flow (G9, D112, S122): the open practice sheet as the session's mode, its pens,
//! Compare, a painted copy's reading, the eyedropper and the views it glides to, a lesson's
//! results and its end; the log of every practice sheet, the learner and the path it tells of,
//! the practice book; the practice options on the home. A practice sheet is stored like an
//! artwork (its strokes and fills through the same history, autosave and library), but the
//! gallery never lists it: it belongs to its record, which `openpaint-practice-store` keeps in
//! the library's record space.
//!
//! Depends on core, practice and app. No GPU, no UI, no file system (`cargo repo-check`).
#![forbid(unsafe_code)]

mod book;
mod copies;
mod copy;
mod layers;
mod lesson;
mod log;
mod mode;
mod options;
mod path;
mod pens;
mod pictures;
mod projection;
mod refused;
mod sheet;
mod shown;

pub use crate::book::{BookProjection, BookRecord, LineRead, Mark, SheetRead};
pub use crate::copies::{CopyShown, PictureShown};
pub use crate::layers::{BLOCKS_LAYER, DIVISION_DIMMED, DIVISION_LAYER, LINES_LAYER};
pub use crate::lesson::{End, Judged, LessonRow, LessonShown};
pub use crate::log::PracticeLog;
pub use crate::mode::{append_lesson, append_stored, open_sheet, practise};
pub use crate::options::{
    MAX_PACK_BYTES, Options, OptionsFile, OptionsIntent, OptionsNotice, OptionsRequest,
};
pub use crate::path::{PathProjection, SkillShown, Start, UnitShown};
pub use crate::pens::{INK_DIAMETER, PracticePens, copy_brush, value_colour};
pub use crate::pictures::{FitJob, GROUND_SHOWN, PackShown, PictureImage, SHOWN_SIDE, grounded};
pub use crate::projection::{
    CopyRead, Greys, Pen, PracticeIntent, PracticeNotice, PracticeProjection, Probe, SHOWN,
    SheetView,
};
pub use crate::refused::Refused;
pub use crate::sheet::{PRACTICE_ROOM, PracticePick, PracticeSheet, Practising, ReadJob};

//! Practice's flow (G9, D112, S122): the open practice sheet owning the session it is drawn in
//! (`SheetSession`), its pens, Compare, a painted copy's reading, the eyedropper and the views it
//! glides to, a lesson's results and its end; the log of every practice sheet, the learner and the
//! path it tells of, the practice book; the practice options on the home; the Pomodoro's timer,
//! its chain of lessons and its blocks (S124, `pomodoro/`). A practice sheet is stored like an
//! artwork (its strokes and fills through the same history, autosave and library), but the
//! gallery never lists it: it belongs to its record, which `openpaint-practice-store` keeps in
//! the library's record space.
//!
//! Depends on core, practice and app. No GPU, no UI, no file system (`cargo repo-check`).
//!
//! The modules, each `//!` saying what it holds: `session.rs` (`SheetSession`, D119), `sheet/`
//! (the open sheet's flow), `refused.rs`, `pens.rs` (the practice pens on their own recipes in
//! `crates/practice-app/pens/`, the hard tip app's Hard Paint file), `layers.rs` (a whole copy's
//! layers), `copy.rs` (the copy read from the records); what the screens read: `projection.rs`
//! (`PracticeIntent`, `PracticeProjection`, `SheetView`), `shown.rs`, `lesson.rs`, `path.rs`
//! (`Start`, `PathProjection`), `copies.rs`; the log `log/`, the Pomodoro `pomodoro/` (its states
//! and rules in its `//!`, D117), the book `book.rs`, the pictures `pictures.rs`, the practice
//! options `options.rs`.
#![forbid(unsafe_code)]

mod book;
mod copies;
mod copy;
mod layers;
mod lesson;
mod log;
mod options;
mod pairs;
mod path;
mod pens;
mod pictures;
mod pomodoro;
mod projection;
mod refused;
mod session;
mod sheet;
mod shown;

pub use crate::book::{BookProjection, BookRecord, LineRead, Mark, SheetRead};
pub use crate::copies::{CopyShown, PictureShown};
pub use crate::layers::{
    BLOCKS_LAYER, DIVISION_DIMMED, DIVISION_LAYER, LAID_SIDE, LINES_LAYER, PICTURE_LAYER,
};
pub use crate::lesson::{End, Judged, LessonRow, LessonShown, OneMore, PairShown};
pub use crate::log::{PracticeLog, Refit};
pub use crate::options::{
    MAX_PACK_BYTES, Options, OptionsFile, OptionsIntent, OptionsNotice, OptionsRequest,
};
pub use crate::pairs::{CardMark, CardSource, CopyCard, UNDER, stored_card};
pub use crate::path::{OpenLesson, PathProjection, SkillShown, Start, UnitShown};
pub use crate::pens::{INK_DIAMETER, PracticePens, copy_brush, grey_colour, value_colour};
pub use crate::pictures::{FitJob, GROUND_SHOWN, PackShown, PictureImage, SHOWN_SIDE, grounded};
pub use crate::pomodoro::{BlockEnd, BlockShown, Timed, TimerIntent, TimerShown, TimerTold};
pub use crate::projection::{
    CopyRead, Pen, PracticeIntent, PracticeNotice, PracticeProjection, Probe, SHOWN, SheetView,
};
pub use crate::refused::Refused;
pub use crate::session::SheetSession;
pub use crate::sheet::{PRACTICE_ROOM, PracticePick, PracticeSheet, Practising, ReadJob};

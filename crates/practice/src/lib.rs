//! Practice (G9): the course's exercises as pure math. Generators lay out exercises that carry
//! their own answers, the same for the same seed; the alignment lays an original line onto a copy
//! for Compare's overlay; colour matches are measured and judged; records keep what a session
//! did. The app turns the strokes on a practice sheet into copies and the store keeps the
//! records.
//!
//! Pure by rule: no GPU, no UI, no file system, no threads, no clock (`cargo repo-check`).
#![forbid(unsafe_code)]

mod align;
mod colour;
mod colour_match;
mod judgement;
mod licence;
mod line;
mod metric;
mod pack;
mod path;
mod plan;
mod random;
mod record;

pub use align::{Fit, align};
pub use colour::{
    Difference, LEAST_CHROMA, Rgb8, chroma, delta_e, from_hsv, hsv, hue_name, hue_towards, oklab,
};
pub use colour_match::{ColourCopy, ColourScore, Lightness, Purity, Tone, score_colour};
pub use judgement::{Axis, DELTA_SPAN, HUE_SPAN, Judgement, NOTICEABLE, SV_SPAN, judgement};
pub use licence::Licence;
pub use line::{LineExercise, LineTier};
pub use metric::{Metric, Metrics, value};
pub use pack::{
    ItemRef, MAX_ID, MAX_PICTURE_SIDE, Pack, PackItem, PackProblem, PictureKind, Tier, valid_id,
};
pub use path::{Point, distance, length, nearest, resample};
pub use plan::{
    Exercise, FRAME, Frame, Kind, Original, Pair, STAGE_SESSIONS, STAGES, Sheet, stage_one,
};
pub use random::{Random, derived};
pub use record::{COLOUR_ROWS, ExerciseResult, KindSummary, SessionRecord, summary};

//! Practice (G9): the course's exercises as pure math. Generators lay out exercises that carry
//! their own answers, the same for the same seed; metrics compare a copy with its original from
//! the pen's own samples; records keep what a session did. The app turns the strokes on a
//! practice sheet into copies and the store keeps the records.
//!
//! Pure by rule: no GPU, no UI, no file system, no threads, no clock (`cargo repo-check`).
#![forbid(unsafe_code)]

mod colour;
mod colour_match;
mod line;
mod line_score;
mod metric;
mod path;
mod plan;
mod random;
mod record;

pub use colour::{Difference, LEAST_CHROMA, Rgb8, chroma, delta_e, from_hsv, hsv, hue_name, oklab};
pub use colour_match::{ColourCopy, ColourScore, Lightness, Purity, Tone, score_colour};
pub use line::{LineExercise, LineTier};
pub use line_score::{LineCopy, LineScore, key_point_matches, score_line};
pub use metric::{Metric, Metrics, value};
pub use path::{Point, distance, length, nearest};
pub use plan::{
    Exercise, FRAME, Frame, Kind, Original, Pair, STAGE_SESSIONS, STAGES, Sheet, stage_one,
};
pub use random::{Random, derived};
pub use record::{COLOUR_ROWS, ExerciseResult, KindSummary, LINE_ROWS, SessionRecord, summary};

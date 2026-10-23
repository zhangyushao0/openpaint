//! Practice (G9): the course's exercises as pure math. Generators lay out exercises that carry
//! their own answers, the same for the same seed; a session draws its warm-up from the kinds met
//! before; the alignment lays an original line or group onto a copy for Compare's overlay; colour
//! matches, colour combinations and value scales are measured and judged; records keep what a
//! session (or a long colour copy) did. The app turns the strokes and pixels on a practice sheet
//! into copies and the store keeps the records.
//!
//! Pure by rule: no GPU, no UI, no file system, no threads, no clock (`cargo repo-check`).
#![forbid(unsafe_code)]

mod align;
mod align_shape;
mod areas;
mod areas_edges;
mod areas_field;
mod areas_read;
mod colour;
mod colour_match;
mod combined;
mod judgement;
mod licence;
mod line;
mod metric;
mod outline;
mod outline_flood;
mod outline_ground;
mod outline_mask;
mod outline_matte;
mod outline_mesh;
mod outline_sides;
mod outline_trace;
mod pack;
mod path;
mod pictures;
mod plan;
mod random;
mod record;
mod scale;
mod session;
mod sheet;

pub use align::{Fit, align};
pub use align_shape::align_shape;
pub use areas::{Areas, Arrangement, GRID, STRONG, WEAK};
pub use areas_read::{AreasCopy, Contrast, MADE_STRONG};
pub use colour::{
    Difference, LEAST_CHROMA, Rgb8, chroma, delta_e, from_hsv, from_oklab, hsv, hue_name,
    hue_towards, oklab,
};
pub use colour_match::{ColourCopy, ColourScore, Lightness, Purity, Tone, score_colour};
pub use combined::{Combination, CombinedLines};
pub use judgement::{Axis, DELTA_SPAN, HUE_SPAN, Judgement, NOTICEABLE, SV_SPAN, judgement};
pub use licence::Licence;
pub use line::{LineExercise, LineTier};
pub use metric::{Metric, Metrics, value};
pub use outline::{Outline, OutlineRefusal, outline};
pub use outline_matte::{Matte, outline_of_matte};
pub use pack::{
    Credit, ItemRef, MAX_ID, MAX_OUTLINE_POINTS, MAX_PICTURE_SIDE, PERSONAL, Pack, PackItem,
    PackProblem, PictureKind, Tier, valid_id,
};
pub use path::{Point, apart, crossings, distance, length, nearest, resample};
pub use pictures::Pictures;
pub use plan::{Exercise, Family, Kind, Original, STAGE_SESSIONS, STAGES, STAGES_READY};
pub use random::{Random, derived};
pub use record::{
    AREAS_ROWS, COLOUR_ROWS, ExerciseResult, KindSummary, PROJECT, SCALE_ROWS, SessionRecord, met,
    summary, used,
};
pub use scale::{BARS, ROW, ScaleCheck, ScaleTone, ValueScale};
pub use session::{Before, Plan, WARM_UP, WARM_UP_TWO, plan, stage_one, stage_two};
pub use sheet::{FRAME, Frame, PICTURE_MARGIN, Pair, PictureFit, STUDY_SIDE, Sheet};

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
mod book;
mod colour;
mod colour_match;
mod combined;
mod demo;
mod drill;
mod edges;
mod ellipse;
mod ellipse_fit;
mod flat;
mod grid;
mod judgement;
mod licence;
mod line;
mod marks;
mod metric;
mod object;
mod object_answer;
mod object_body;
mod object_changes;
mod object_construction;
mod object_design;
mod object_hull;
mod object_picture;
mod object_rays;
mod object_rules;
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
mod perspective;
mod perspective_check;
mod perspective_eye;
mod perspective_free;
mod pictures;
mod plan;
mod random;
mod record;
mod scale;
mod seals;
mod session;
mod session_four;
mod session_three;
mod sheet;
mod solid;
mod solid_answer;
mod solid_check;
mod space;
mod space_check;
mod time;
mod vector;

pub use align::{Fit, align};
pub use align_shape::align_shape;
pub use areas::{Areas, Arrangement, GRID, STRONG, WEAK};
pub use areas_read::{AreasCopy, Contrast, MADE_STRONG};
pub use book::{Book, Day, LineShown, Road};
pub use colour::{
    Difference, HUELESS, Rgb8, delta_e, from_hsv, from_oklab, from_oklch, hsv, oklab, oklch,
};
pub use colour_match::{ColourCopy, ColourScore, Lightness, Purity, Tone, score_colour};
pub use combined::{Combination, CombinedLines};
pub use demo::{Demo, Drawn, Layout, Mark, Marker, On, WIDE_REACH};
pub use drill::{drill, kinds_of};
pub use edges::{Edge, SHORTEST, corrected, edges};
pub use ellipse::{Ellipse, EllipseGuide, Guide};
pub use ellipse_fit::{
    DEGREE, EllipseCheck, REACH, Reach, Roundness, Side, TILT, Tilt, fit_ellipse,
};
pub use flat::{Estimate, Flat, FlatAnswer, FlatCheck};
pub use judgement::{Axis, DELTA_SPAN, Judgement, NOTICEABLE, PART_SPAN, judgement};
pub use licence::Licence;
pub use line::{LineExercise, LineTier};
pub use marks::{MARK, last_edges, last_mark};
pub use metric::{Metric, Metrics, value};
pub use object::ObjectTask;
pub use object_construction::Stage;
pub use object_design::Carve;
pub use object_picture::{PAPER_SHADE, SHADE_STEP};
pub use object_rules::LEAST_EDGE;
pub use outline::{Outline, OutlineRefusal, outline};
pub use outline_matte::{Matte, outline_of_matte};
pub use pack::{
    Credit, ItemRef, MAX_ID, MAX_OUTLINE_POINTS, MAX_PICTURE_SIDE, PERSONAL, Pack, PackItem,
    PackProblem, PictureKind, Tier, valid_id,
};
pub use path::{Point, across_frame, apart, crossings, distance, length, meet, nearest, resample};
pub use perspective::{BoxView, Direction, EDGES, FAR, Horizon, PerspectiveBox};
pub use perspective_check::{BoxCheck, Checked, ON_ANGLE};
pub use perspective_free::{Corner, FreeBox, FreeCheck, RIGHT_ANGLE};
pub use pictures::Pictures;
pub use plan::{Exercise, Family, Kind, Original, STAGE_SESSIONS, STAGES, STAGES_READY};
pub use random::{Random, derived};
pub use record::{
    AREAS_ROWS, COLOUR_ROWS, DRILL, ExerciseResult, KindSummary, PROJECT, SCALE_ROWS,
    SessionRecord, met, summary, used,
};
pub use scale::{BARS, ROW, ScaleCheck, ScaleTone, ValueScale};
pub use seals::{Earned, Seal, Tally, earned, sealed};
pub use session::{Before, Plan, WARM_UP, WARM_UP_TWO, plan, stage_one, stage_two};
pub use session_four::stage_four;
pub use session_three::stage_three;
pub use sheet::{FRAME, Frame, PICTURE_MARGIN, Pair, PictureFit, STUDY_SIDE, Sheet};
pub use solid::{PEN_WIDTH, Pose, Solid, SolidTask, diagonals, middle_lines};
pub use solid_answer::{SolidAnswer, Stretch};
pub use solid_check::{COVERED, ReadCurve, SolidCheck};
pub use space::{SpaceAnswer, SpaceTask, Trick};
pub use space_check::SpaceCheck;
pub use time::{Date, PAUSE, Span, by_day, local_day, practised, seconds, spans_of, weekday};

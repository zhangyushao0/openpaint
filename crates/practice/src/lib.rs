//! Practice (G9): the course's exercises as pure math. Generators lay out exercises that carry
//! their own answers, the same for the same seed; a session draws its warm-up from the kinds met
//! before; the alignment lays an original line or group onto a copy for Compare's overlay; colour
//! matches, colour combinations and value scales are measured and judged; records keep what a
//! session (or a long colour copy) did. The app turns the strokes and pixels on a practice sheet
//! into copies and the store keeps the records.
//!
//! Pure by rule: no GPU, no UI, no file system, no threads, no clock (`cargo repo-check`).
#![forbid(unsafe_code)]

mod colour_areas;
mod compositions;
mod demos;
mod fits;
mod geometry;
mod kinds;
mod marks;
mod maths;
mod outlines;
mod packs;
mod records;
mod sessions;

pub use colour_areas::areas::{Areas, GRID, MOST_AREAS, STRONG, WEAK};
pub use colour_areas::areas_found::{Found, SQUINT};
pub use colour_areas::areas_generated::{Arrangement, GeneratedAreas};
pub use colour_areas::areas_read::{AreasCopy, Contrast, MADE_STRONG};
pub use colour_areas::areas_regions::ONE_STEP;
pub use colour_areas::main_hue::{HUE_STEP, MAIN_CHROMA, main_hue, turn};
pub use compositions::levels::{Level, LevelRow};
pub use compositions::masses::Masses;
pub use compositions::value_masses::{BLUR, ISLAND, MASSES_SIDE, RIM, Value, ValueMasses};
pub use demos::demo::{
    Demo, DemoLayout, Drawn, Mark, Marker, On, Pictured, Shaded, Strip, WIDE_REACH,
};
pub use fits::align::{Fit, align};
pub use fits::align_shape::align_shape;
pub use geometry::edges::{Edge, SHORTEST, corrected, edges};
pub use geometry::ellipse::{Ellipse, EllipseGuide, Guide};
pub use geometry::ellipse_fit::fit_ellipse;
pub use geometry::ellipse_sides::{Missed, REACH, Reach, Side};
pub use geometry::heading::{Checked, ON_ANGLE, PAST};
pub use geometry::marks::{MARK, last_edges, last_mark};
pub use geometry::off::{Heads, Off, PEN_WIDTH, ReadCurve, Straight, ringed};
pub use geometry::perspective::{BoxView, Direction, EDGES, FAR, Horizon, PerspectiveBox};
pub use kinds::boxes::marks::BoxCompared;
pub use kinds::boxes::perspective_check::{BoxCheck, EDGES_TO_ADD, NEAR_MISSED};
pub use kinds::colours::colour_match::{
    ColourCopy, ColourScore, Lightness, Purity, Tone, score_colour,
};
pub use kinds::colours::judgement::{
    Axis, DELTA_SPAN, Judgement, NOTICEABLE, PART_SPAN, judgement,
};
pub use kinds::colours::marks::ColourCompared;
pub use kinds::combinations::marks::{AREAS_CLEAN, AreasCompared, AreasRead, NEAR_STRONG};
pub use kinds::combinations::reading::AreasReading;
pub use kinds::combined_lines::combined::{Combination, CombinedLines};
pub use kinds::divisions::division::{
    DivisionCheck, DivisionLine, HEADING_ON, JOIN_REACH, MOST_MAIN, joined,
};
pub use kinds::divisions::marks::DivisionCompared;
pub use kinds::ellipses::ellipse_check::{
    DEGREE, EllipseCheck, NEAR_FINDINGS, Roundness, TILT, Tilt,
};
pub use kinds::ellipses::marks::EllipseCompared;
pub use kinds::estimates::flat::{Estimate, Flat, FlatAnswer, FlatCheck, NEAR_ANGLE, NEAR_OFF};
pub use kinds::estimates::marks::FlatCompared;
pub use kinds::free_boxes::marks::FreeCompared;
pub use kinds::free_boxes::perspective_free::{Corner, FreeBox, FreeCheck, RIGHT_ANGLE};
pub use kinds::generated_combinations::AREAS_SIDE;
pub use kinds::key_shifts::key_shift::{AWAY, GREY_CHROMA, Lean, PURE_CHROMA, Tendency};
pub use kinds::key_shifts::key_shift_check::{
    APART, AT, KeyCheck, LIGHTEST, LIGHTNESS_OFF, MERGED, MOVED, NEAR_AREAS, NEAR_SHARE, toward,
};
pub use kinds::key_shifts::marks::{KeyCompared, KeyRead};
pub use kinds::key_shifts::reading::KeyReading;
pub use kinds::lines::line::{LineExercise, LineTier};
pub use kinds::lines::marks::LinesCompared;
pub use kinds::objects::OBJECT_SIDE;
pub use kinds::objects::marks::ObjectCompared;
pub use kinds::objects::object::ObjectTask;
pub use kinds::objects::object_construction::Stage;
pub use kinds::objects::object_design::Carve;
pub use kinds::objects::object_picture::{PAPER_SHADE, SHADE_STEP};
pub use kinds::objects::object_rules::LEAST_EDGE;
pub use kinds::points_in_space::marks::SpaceCompared;
pub use kinds::points_in_space::space::{SpaceAnswer, SpaceTask, Trick};
pub use kinds::points_in_space::space_check::{NEAR_FACE, SpaceCheck};
pub use kinds::scales::marks::{ScaleCompared, ScaleReading};
pub use kinds::scales::scale::{
    CELLS, EVEN_ROW, FIRST_CELLS, FIRST_SCALES, NEAR_UNEVEN, NEAR_UNEVEN_LONG, ROW, ScaleCheck,
    ScaleTone, ValueScale, halving,
};
pub use kinds::shapes::ground::faded_ground;
pub use kinds::shapes::marks::{SHAPE_SHOWN, ShapeCompared};
pub use kinds::solids::marks::SolidCompared;
pub use kinds::solids::solid::{Pose, Solid, SolidTask, diagonals, middle_lines};
pub use kinds::solids::solid_answer::{SolidAnswer, Stretch};
pub use kinds::solids::solid_check::{COVERED, NEAR_MARKS, SolidCheck};
pub use kinds::value_masses::marks::{MassesCompared, MassesReading};
pub use kinds::value_masses::value_masses_read::{
    BARE_LEFT, HATCH_GAP, MassesCheck, NEAR_MATCHED, VALUE_PEN,
};
pub use kinds::{DrawnCopy, Facts, Kept, Kind, Needs, Original, Picks, Tools, Work};
pub use marks::asked::{Asked, Compare, Copied, Painted, Read, Reading, Reads, ToFit};
pub use marks::card::{Arg, CardItem, Say, WheelSize};
pub use marks::compared::{Compared, Comparison, Outcome, Seen, Verdict};
pub use marks::drawing::PictureCompared;
pub use marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
pub use marks::tile::{TILE, TileMark};
pub use maths::colour::{
    Difference, HUELESS, Rgb8, delta_e, from_hsv, from_oklab, from_oklch, hsv, in_gamut, in_grey,
    oklab, oklch,
};
pub use maths::path::{
    Point, across_frame, apart, crossings, distance, length, meet, nearest, outside, resample,
    within,
};
pub use maths::random::{Random, derived};
pub use outlines::outline::{Outline, OutlineRefusal, outline};
pub use outlines::outline_matte::{Matte, outline_of_matte};
pub use packs::crop::{
    BUSIEST, Crop, CropRefusal, DISTINCT, FEWEST_AREAS, GREYISH, LEAST_LONGER, Places, best,
    candidate, candidates, cut, rectangles, refusal, tier_of,
};
pub use packs::licence::Licence;
pub use packs::pack::{
    Credit, ItemRef, MAX_ID, MAX_OUTLINE_POINTS, MAX_PICTURE_SIDE, PERSONAL, Pack, PackItem,
    PackProblem, PictureKind, Tier, valid_id,
};
pub use packs::pictures::Pictures;
pub use records::book::{Book, Day, LineShown, Road};
pub use records::givens::{Given, Givens};
pub use records::metric::{Metric, Metrics, value};
pub use records::record::{
    AREAS_ROWS, COLOUR_ROWS, KindSummary, RECENT, SCALE_ROWS, met, recent, scales_met, summary,
    used,
};
pub use records::seals::{Earned, Seal, Tally, earned, sealed};
pub use records::settled::settled;
pub use records::sheet::{First, Judge, Laid, Lesson, Picked, Purpose, Sheet, number};
pub use records::time::{PAUSE, Span, by_day, local_day, practised, seconds};
pub use sessions::drill::{drill, kinds_of, stage_needs};
pub use sessions::layout::{FRAME, FRAME_REACH, Frame, Layout, Pair, PictureFit, STUDY_SIDE};
pub use sessions::plan::{Exercise, Family, STAGE_SESSIONS, STAGES, STAGES_READY};
pub use sessions::session::{Before, Plan, WARM_UP, WARM_UP_TWO, plan, stage_one, stage_two};
pub use sessions::session_five::stage_five;
pub use sessions::session_four::stage_four;
pub use sessions::session_three::stage_three;
pub use sessions::sheet_picture::{MassesShown, SheetPicture};

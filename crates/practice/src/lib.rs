//! Practice (G9): the course's exercises as pure math. Generators lay out exercises that carry
//! their own answers, the same for the same seed; the course is a path of units and skills, each
//! skill's lesson laid at the learner's level and the next lesson chosen from what the records
//! tell (`course/`); the alignment lays an original line or group onto a copy for Compare's
//! overlay; colour matches, colour combinations and value scales are measured and judged; records
//! keep what each lesson (or a long colour copy) did; each skill's progress is fitted from the
//! results, the measures and the learner's blind pairs (`progress/`). The app turns the strokes
//! and pixels on a practice sheet into copies and the store keeps the records.
//!
//! Pure by rule: no GPU, no UI, no file system, no threads, no clock (`cargo repo-check`).
#![forbid(unsafe_code)]

mod colour_areas;
mod compositions;
mod course;
mod demos;
mod fits;
mod geometry;
mod kinds;
mod marks;
mod maths;
mod outlines;
mod packs;
mod progress;
mod records;
mod sheets;

pub use colour_areas::areas::{Areas, GRID, MOST_AREAS, STRONG, WEAK};
pub use colour_areas::areas_found::{Found, SQUINT};
pub use colour_areas::areas_generated::{Arrangement, GeneratedAreas};
pub use colour_areas::areas_read::{AreasCopy, Contrast, MADE_STRONG};
pub use colour_areas::areas_regions::ONE_STEP;
pub use colour_areas::main_hue::{HUE_STEP, MAIN_CHROMA, main_hue, turn};
pub use compositions::levels::{Level, LevelRow};
pub use compositions::masses::Masses;
pub use compositions::value_masses::{BLUR, ISLAND, MASSES_SIDE, RIM, Value, ValueMasses};
pub use course::learner::{DOTS, Learner, Redo};
pub use course::retest;
pub use course::retest::Retest;
pub use course::schedule;
pub use course::schedule::{Offer, Planned, Standing};
pub use course::unit::{UNITS, Unit};
pub use demos::demo::Aimed;
pub use demos::demo::{
    Demo, DemoLayout, Drawn, Mark, Marker, On, Pictured, Shaded, Strip, WIDE_REACH,
};
pub use fits::align::{Fit, align};
pub use fits::align_shape::align_shape;
pub use geometry::edges::{Edge, SHORTEST, edges};
pub use geometry::ellipse::{Ellipse, EllipseGuide, Guide};
pub use geometry::heading::{ON_ANGLE, PAST};
pub use geometry::marks::{MARK, last_edges, last_mark};
pub use geometry::off::{Heads, Off, PEN_WIDTH, Straight, ringed};
pub use geometry::perspective::{BoxView, Direction, EDGES, FAR, Horizon, Look, PerspectiveBox};
pub use kinds::abstract_analysis::contrast::{
    CHROMA_CONTRAST, CONTRAST_REACH, ContrastView, Contrasts, HARD_EDGE, HUE_CONTRAST,
    LIGHTNESS_CONTRAST, SQUINTED, Threshold, VEILED,
};
pub use kinds::abstract_analysis::marks::{
    AnalysisCompared, AnalysisReading, CONTRASTS, DISTRIBUTIONS, VIEWS,
};
pub use kinds::arrows::answer::ArrowAnswer;
pub use kinds::arrows::marks::ArrowCompared;
pub use kinds::arrows::ribbon::Arrow;
pub use kinds::arrows::task::ArrowTask;
pub use kinds::boxes::marks::BoxCompared;
pub use kinds::cast_shadows::answer::{Corner, Shadows};
pub use kinds::cast_shadows::marks::ShadowCompared;
pub use kinds::cast_shadows::scene::{ShadowTask, Throw};
pub use kinds::cold_warm::leans::Leans;
pub use kinds::cold_warm::marks::{ShiftCompared, ShiftReading};
pub use kinds::colours::bands::{DELTA_SPAN, NOTICEABLE, PART_SPAN};
pub use kinds::colours::colour_match::{
    ColourCopy, ColourScore, Lightness, Purity, Tone, score_colour,
};
pub use kinds::colours::marks::ColourCompared;
pub use kinds::combinations::marks::{AREAS_CLEAN, AreasCompared, AreasRead, NEAR_STRONG};
pub use kinds::combinations::reading::AreasReading;
pub use kinds::combined_lines::combined::{Combination, CombinedLines};
pub use kinds::contours::marks::ContourCompared;
pub use kinds::contours::sections::Contour;
pub use kinds::contours::task::{Bend, ContourTask};
pub use kinds::contrast_thumbnails::marks::{
    Finding, ThumbnailCompared, ThumbnailRead, ThumbnailReading, WITHIN,
};
pub use kinds::contrast_thumbnails::rings::{Emphasis, MOVED_BY, RING, Ring, rings};
pub use kinds::contrast_thumbnails::steps::{CLEAR_WITHIN, Steps, VEIL_BELOW};
pub use kinds::curved_rotations::answer::{CurvedAnswer, CurvedLine, Owner};
pub use kinds::curved_rotations::curved::{CurvedRotation, CurvedTask, Handed};
pub use kinds::curved_rotations::design::{Attached, Attachment, Design, Segment, SegmentKind};
pub use kinds::curved_rotations::marks::CurvedCompared;
pub use kinds::curved_rotations::sections::{Rim, Section};
pub use kinds::curved_rotations::view::CurvedView;
pub use kinds::divisions::division::{
    DivisionCheck, DivisionLine, HEADING_ON, JOIN_REACH, MOST_MAIN, joined,
};
pub use kinds::divisions::marks::DivisionCompared;
pub use kinds::ellipses::funnel::Funnel;
pub use kinds::ellipses::marks::{EllipseCompared, FunnelCompared};
pub use kinds::estimates::flat::{Estimate, Flat, FlatAnswer, FlatCheck, NEAR_ANGLE, NEAR_OFF};
pub use kinds::estimates::marks::FlatCompared;
pub use kinds::free_boxes::FreePage;
pub use kinds::free_boxes::marks::{FreeCompared, guides};
pub use kinds::free_boxes::path::PathPage;
pub use kinds::gamut_masks::check::{MaskCheck, NEAR_FINDINGS, way};
pub use kinds::gamut_masks::marks::{MaskCompared, MaskRead};
pub use kinds::gamut_masks::mask::{
    INSIDE, Mask, MaskShape, distance as gamut_distance, nearest as gamut_nearest,
    span as gamut_span,
};
pub use kinds::gamut_masks::reading::MaskReading;
pub use kinds::ghosted_lines::ghosted::{Ghosted, GhostedTask};
pub use kinds::ghosted_lines::marks::GhostedCompared;
pub use kinds::ghosted_lines::plane::Plane;
pub use kinds::intersections::marks::IntersectionCompared;
pub use kinds::intersections::meeting::{IntersectionTask, Meeting, Way};
pub use kinds::key_shifts::key_shift::{AWAY, FAR_AWAY, GREY_CHROMA, Lean, PURE_CHROMA, Tendency};
pub use kinds::key_shifts::key_shift_check::{
    APART, AT, KeyCheck, LIGHTEST, LIGHTNESS_OFF, MERGED, MOVED, NEAR_SHARE, toward,
};
pub use kinds::key_shifts::marks::{KeyCompared, KeyRead};
pub use kinds::key_shifts::reading::KeyReading;
pub use kinds::light_on_form::answer::{
    CONTACT, CORE, CORE_STRIP, Grey, HALFTONE, LIGHT, Mass, REFLECTED, SOFT,
};
pub use kinds::light_on_form::marks::{LightCompared, LightReading};
pub use kinds::light_on_form::read::{LightCheck, NEAR};
pub use kinds::light_on_form::scene::{Form, LightTask, Lighting, Lit, Tones};
pub use kinds::light_on_form::trace::Parting;
pub use kinds::lines::line::{LineExercise, LineTier};
pub use kinds::lines::marks::LinesCompared;
pub use kinds::lit_colours::check::Rule;
pub use kinds::lit_colours::colours::Colours;
pub use kinds::lit_colours::marks::{LitCompared, LitRead, LitReading};
pub use kinds::lit_colours::regions::Region;
pub use kinds::lit_colours::scene::{ColourTask, LitColour, Palette};
pub use kinds::objects::OBJECT_SIDE;
pub use kinds::objects::marks::ObjectCompared;
pub use kinds::objects::object::ObjectTask;
pub use kinds::objects::object_construction::Stage;
pub use kinds::objects::object_design::Carve;
pub use kinds::objects::object_picture::{PAPER_SHADE, SHADE_STEP};
pub use kinds::objects::object_rules::LEAST_EDGE;
pub use kinds::observation::ObservedCompared;
pub use kinds::placements::answer::{Carried, Construction};
pub use kinds::placements::marks::PlacementCompared;
pub use kinds::placements::placement::{Placement, PlacementTask};
pub use kinds::placements::plan::{Footprint, Piece, Plan};
pub use kinds::points_in_space::marks::SpaceCompared;
pub use kinds::points_in_space::space::{SpaceAnswer, SpaceTask, Trick};
pub use kinds::points_in_space::space_check::{NEAR_FACE, SpaceCheck};
pub use kinds::rotations::marks::RotationCompared;
pub use kinds::rotations::rotation::{Rotation, RotationTask, Turning};
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
pub use kinds::textures::field::{Bumps, Texture};
pub use kinds::textures::marks::{TextureCompared, TextureReading};
pub use kinds::textures::read::TextureCheck;
pub use kinds::textures::shadows::{TextureShadows, TextureTask};
pub use kinds::value_masses::marks::{MassesCompared, MassesReading};
pub use kinds::value_masses::massing::Massing;
pub use kinds::value_masses::notan::{NotanCompared, NotanReading};
pub use kinds::value_masses::value_masses_read::{BARE_LEFT, HATCH_GAP, MassesCheck, VALUE_PEN};
pub use kinds::{
    Counted, DARKEST, DARKEST_PEN, DrawnCopy, Facts, Fill, Kept, Kind, Needs, Original, Pen,
    Picking, Skill, SkillLevel, ToolSet,
};
pub use marks::asked::{Asked, Compare, Copied, Painted, Pixels, Read, Reading, Reads, ToFit};
pub use marks::card::{Arg, CardItem, Choices, Say, WheelSize};
pub use marks::compared::{Compared, Comparison, Outcome, Seen, Verdict};
pub use marks::drawing::PictureCompared;
pub use marks::mark::{BESIDE_NAMES, BESIDE_SIDE, beside};
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
pub use progress::{
    Change, Curve, PairAsked, Progress, SkillFit, Within, ask, block_pairs, retest_pairs,
};
pub use records::blocks::{BREAK, Block, BlockTally, Blocks, Length, TimerWrite, Timing, UnderWay};
pub use records::book::{Book, Day, LineShown};
pub use records::givens::{Given, Givens};
pub use records::lessons::{OnSheet, lessons, number};
pub use records::metric::{Metric, Metrics, value};
pub use records::pairs::{Answer, BlindPair, CopyRef, older_left};
pub use records::record::{RECENT, recent, used};
pub use records::seals::{Earned, Seal, Tally, earned, sealed};
pub use records::sheet::{First, Judge, Laid, Lesson, Picked, Purpose, Sheet};
pub use records::time::{PAUSE, Span, by_day, local_day, practised, seconds};
pub use sheets::exercise::Exercise;
pub use sheets::layout::{FRAME, FRAME_REACH, Frame, Layout, Pair, PictureFit, ROWS, STUDY_SIDE};
pub use sheets::sheet_picture::{MassesShown, SheetPicture};

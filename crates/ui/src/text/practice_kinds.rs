//! Each kind's words, registered once by the key its kind's description gives its family
//! (`openpaint_practice::Facts::key`, S109): its name (its tile, its drill, the bar), its line in a
//! session's summary, its How card and, for a value scale, the steps made for the row in hand.
//! Registering a kind's words is its line in `KINDS`; the words themselves stay in the tables
//! beside this, English and Chinese together.

use openpaint_practice::{Estimate, Family, Trick};

use super::practice_demo::{COMBINED, How, LINE, SHAPE};
use super::practice_demo_colour as colour;
use super::practice_demo_points::{estimate, trick};
use super::{Language, Text};

texts! { PRACTICE_KINDS;
    LINES_COUNTED = "Lines", "线条";
    COMBINED_COUNTED = "Combined lines", "组合线条";
    COLOURS_COUNTED = "Colours", "单色";
    SHAPES_COUNTED = "Shapes", "单体图形";
    BOXES_COUNTED = "Boxes", "透视盒子";
    FREE_BOXES_COUNTED = "Free boxes", "自由盒子";
    ELLIPSES_COUNTED = "Ellipses", "椭圆";
    COMBINATIONS_COUNTED = "Colour combinations", "色彩组合";
    SCALES_COUNTED = "Value scales", "明度推移";
    COPIES_COUNTED = "Colour copies", "色彩临摹";
}

/// A kind's words.
pub(crate) struct KindWords {
    /// Its family's key (`Facts::key`).
    pub(crate) key: &'static str,
    /// Its name: "Line", "Centre".
    pub(crate) name: Text,
    /// What the summary counts: "Lines".
    pub(crate) counted: Text,
    pub(crate) how: How,
    /// Its How's steps made for the exercise in hand before its own (a value scale's row).
    pub(crate) made: Option<fn(Language, usize) -> Vec<String>>,
}

/// A kind's words, nothing made.
const fn words(key: &'static str, (name, counted): (Text, Text), how: How) -> KindWords {
    KindWords {
        key,
        name,
        counted,
        how,
        made: None,
    }
}

/// Every kind's words, in a warm-up's order.
const KINDS: [KindWords; 26] = [
    words("line", (super::LINE, LINES_COUNTED), LINE),
    words(
        "combined",
        (super::COMBINED_LINES, COMBINED_COUNTED),
        COMBINED,
    ),
    words("shape", (super::SHAPE_EXERCISE, SHAPES_COUNTED), SHAPE),
    words(
        "box",
        (super::BOX_EXERCISE, BOXES_COUNTED),
        super::practice_demo_space::BOX,
    ),
    words(
        "free-boxes",
        (super::FREE_BOXES_EXERCISE, FREE_BOXES_COUNTED),
        super::practice_demo_space::FREE,
    ),
    words(
        "ellipse",
        (super::ELLIPSE_EXERCISE, ELLIPSES_COUNTED),
        super::practice_demo_space::ELLIPSE,
    ),
    words(
        "point",
        (super::POINT_EXERCISE, super::POINTS_COUNTED),
        estimate(Estimate::Point),
    ),
    words(
        "proportion",
        (super::PROPORTION_EXERCISE, super::PROPORTIONS_COUNTED),
        estimate(Estimate::Proportion),
    ),
    words(
        "angle",
        (super::ANGLE_EXERCISE, super::ANGLES_COUNTED),
        estimate(Estimate::Angle),
    ),
    words(
        "centre",
        (super::CENTRE_EXERCISE, super::CENTRES_COUNTED),
        trick(Trick::Centre),
    ),
    words(
        "halves",
        (super::HALVES_EXERCISE, super::HALVES_COUNTED),
        trick(Trick::Halves),
    ),
    words(
        "thirds",
        (super::THIRDS_EXERCISE, super::THIRDS_COUNTED),
        trick(Trick::Thirds),
    ),
    words(
        "next-face",
        (super::STEP_EXERCISE, super::STEPS_COUNTED),
        trick(Trick::Step),
    ),
    words(
        "mirror",
        (super::MIRROR_EXERCISE, super::MIRRORS_COUNTED),
        trick(Trick::Mirror),
    ),
    words(
        "cylinder",
        (super::CYLINDER_EXERCISE, super::CYLINDERS_COUNTED),
        super::practice_demo_solids::CYLINDER,
    ),
    words(
        "cone",
        (super::CONE_EXERCISE, super::CONES_COUNTED),
        super::practice_demo_solids::CONE,
    ),
    words(
        "sphere",
        (super::SPHERE_EXERCISE, super::SPHERES_COUNTED),
        super::practice_demo_solids::SPHERE,
    ),
    words(
        "objects",
        (super::OBJECT_EXERCISE, super::OBJECTS_COUNTED),
        super::practice_demo_objects::OBJECT,
    ),
    words(
        "division",
        (super::DIVISION_EXERCISE, super::DIVISIONS_COUNTED),
        super::practice_demo_division::DIVISION,
    ),
    words(
        "whole-copy",
        (super::WHOLE_COPY_EXERCISE, super::WHOLE_COPIES_COUNTED),
        super::practice_demo_division::WHOLE_COPY,
    ),
    words(
        "colour",
        (super::COLOUR_EXERCISE, COLOURS_COUNTED),
        colour::COLOUR,
    ),
    words(
        "combination",
        (super::AREAS_EXERCISE, COMBINATIONS_COUNTED),
        colour::AREAS,
    ),
    KindWords {
        made: Some(colour::scale_steps),
        ..words(
            "scale",
            (super::SCALE_EXERCISE, SCALES_COUNTED),
            colour::SCALE,
        )
    },
    words(
        "study",
        (super::STUDY_EXERCISE, COPIES_COUNTED),
        colour::STUDY,
    ),
    words(
        "value-masses",
        (super::VALUE_MASSES_EXERCISE, super::MASSES_COUNTED),
        super::practice_masses::VALUE_MASSES,
    ),
    words(
        "key-shift",
        (super::KEY_SHIFT_EXERCISE, super::KEY_SHIFTS_COUNTED),
        super::practice_key::KEY_SHIFT,
    ),
];

/// `family`'s words, by its key.
pub(crate) fn kind_words(family: Family) -> &'static KindWords {
    let key = family.facts().key;
    (KINDS.iter())
        .find(|words| words.key == key)
        .unwrap_or(&KINDS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_has_its_words_under_its_key() {
        for family in Family::ALL {
            assert_eq!(kind_words(family).key, family.facts().key, "{family:?}");
        }
        assert_eq!(KINDS.len(), Family::ALL.len());
    }
}

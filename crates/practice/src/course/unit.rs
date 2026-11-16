//! The course's units (S122): V大's eight weeks, each with the skills it teaches in the course's
//! order. A skill is a kind's folder, named here by its stored code; the units whose kinds are not
//! built yet hold none. The order of the units and of the skills within them is the course order
//! the scheduler falls back on (`schedule::next`) and the path lays its tiles in.

use crate::kinds::Skill;

/// A unit of the course.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit {
    /// From 1, as V大's weeks.
    pub number: u8,
    /// Its skills in the course's order; none while its kinds are not built.
    pub skills: &'static [Skill],
}

/// The eight units.
pub const UNITS: [Unit; 8] = [
    // Lines, combined lines, colours.
    unit(1, &[Skill(1), Skill(3), Skill(2)]),
    // Single shapes, colour combinations, value scales, boxes, free boxes, ellipses.
    unit(
        2,
        &[Skill(4), Skill(18), Skill(6), Skill(8), Skill(9), Skill(10)],
    ),
    // Estimates on the flat, points in space, solids.
    unit(3, &[Skill(11), Skill(12), Skill(13)]),
    // Objects, divisions, value masses, whole copies.
    unit(4, &[Skill(14), Skill(15), Skill(17), Skill(16)]),
    // Key shifts, abstract analysis, structure rotation.
    unit(5, &[Skill(19), Skill(21), Skill(22)]),
    // Gamut masks, light on form, cast shadows.
    unit(6, &[Skill(24), Skill(26), Skill(27)]),
    // Curved rotation, placement, colour under light, texture.
    unit(7, &[Skill(28), Skill(25), Skill(29), Skill(30)]),
    unit(8, &[]),
];

const fn unit(number: u8, skills: &'static [Skill]) -> Unit {
    Unit { number, skills }
}

impl Unit {
    /// Unit `number`, from 1.
    #[must_use]
    pub fn of(number: u8) -> Option<&'static Self> {
        UNITS.iter().find(|unit| unit.number == number)
    }

    /// The unit that teaches `skill`.
    #[must_use]
    pub fn teaching(skill: Skill) -> Option<&'static Self> {
        UNITS.iter().find(|unit| unit.skills.contains(&skill))
    }

    /// Whether its kinds are built: it holds a skill.
    #[must_use]
    pub const fn built(&self) -> bool {
        !self.skills.is_empty()
    }
}

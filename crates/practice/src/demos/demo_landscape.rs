//! The landscape a division's and a whole copy's demonstrations share (S95): a few masses made
//! without a pack (`masses.rs`), its frames the picture's shape as the exercise's are (S99), and
//! how a hand draws its division in V大's three levels.

use crate::compositions::levels::Level;
use crate::compositions::masses::Masses;
use crate::demos::demo::{Drawn, Marker, On, Script, wobbled};
use crate::maths::path::Point;
use crate::sessions::sheet::PictureFit;

/// The example: a landscape four by three.
const SEED: u64 = 0xDE30_5EED_0095;
const ASPECT: f64 = 0.75;
/// How far a hand's line strays (frame units).
pub(crate) const STRAY: f64 = 0.004;
/// Lines drawn in the time of one, so the many short ones go at a hand's pace overall.
pub(crate) const TOGETHER: usize = 3;

/// The example and how a picture point lies in its frames, which are the picture's shape.
pub(crate) struct Landscape {
    pub(crate) masses: Masses,
    fit: PictureFit,
}

impl Landscape {
    pub(crate) fn new() -> Self {
        let masses = Masses::generate(SEED, ASPECT);
        let [width, height] = masses.size.map(|side| (side * 1000.0).round() as u32);
        Self {
            fit: PictureFit::of([width, height]),
            masses,
        }
    }

    /// The frame point of the picture point `p` (picture units, its width 1).
    pub(crate) fn at(&self, p: Point) -> Point {
        let side = self.fit.size[0];
        [p[0] * side, p[1] * side]
    }

    /// The picture's place, the whole frame: its top left and bottom right.
    pub(crate) fn rect(&self) -> [Point; 2] {
        [[0.0, 0.0], self.fit.size]
    }

    /// The picture in the original's frame.
    pub(crate) fn given(&self, script: &mut Script) {
        script.given(On::Original, Marker::Given, Drawn::Masses(self.rect()));
    }

    /// The division's lines of `level` in the frame.
    pub(crate) fn lines(&self, level: Level) -> Vec<Vec<Point>> {
        (self.masses.division.iter())
            .filter(|(held, _)| *held == level)
            .map(|(_, line)| line.iter().map(|p| self.at(*p)).collect())
            .collect()
    }

    /// The main lines: the turning point first, then each line passed in the air and drawn.
    pub(crate) fn main_lines(&self, script: &mut Script) {
        let turning = self.at(self.masses.turning);
        script.draw(On::Copy, Marker::Level(Level::Main), Drawn::Dot(turning));
        for (turn, line) in self.lines(Level::Main).into_iter().enumerate() {
            script.draw(On::Copy, Marker::Air, Drawn::Line(line.clone()));
            let drawn = wobbled(&line, STRAY, 0.4 + turn as f64);
            script.draw(On::Copy, Marker::Level(Level::Main), Drawn::Line(drawn));
        }
    }

    /// `level`'s lines a few at a time.
    pub(crate) fn level_lines(&self, script: &mut Script, level: Level) {
        let lines = self.lines(level);
        for (turn, group) in lines.chunks(TOGETHER).enumerate() {
            let marks = (group.iter())
                .map(|line| {
                    let drawn = wobbled(line, STRAY, 1.3 + turn as f64);
                    (On::Copy, Marker::Level(level), Drawn::Line(drawn))
                })
                .collect();
            script.together(marks);
        }
    }

    /// Compare's picture over the copy, at `pen`'s strength.
    pub(crate) fn laid(&self, pen: Marker) -> (On, Marker, Drawn) {
        (On::Copy, pen, Drawn::Masses(self.rect()))
    }
}

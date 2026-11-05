//! A division's and a whole copy's demonstrations (`demo.rs`, S95), V大's way on a landscape of a
//! few masses made without a pack (`masses.rs`), beside its picture, both frames the picture's
//! shape as the exercise's are (S99). A division: the ridge's turning point found first, the
//! two longest lines red right through the picture; from them each part's frame green; shorter
//! still blue; Compare lays the picture faint under the lines. A whole copy: the division red and
//! green; Draw, the division dimming under the big shapes' lines in ink; Compare lays the picture
//! over the copy, the division gone.

use super::{Demo, Drawn, Layout, Marker, On, Script, wobbled};
use crate::division::Level;
use crate::masses::Masses;
use crate::path::Point;
use crate::plan::Family;
use crate::sheet::PictureFit;

/// The example: a landscape four by three.
const SEED: u64 = 0xDE30_5EED_0095;
const ASPECT: f64 = 0.75;
/// How far a hand's line strays (frame units).
const STRAY: f64 = 0.004;
/// Lines drawn in the time of one, so the many short ones go at a hand's pace overall.
const TOGETHER: usize = 3;

/// The example and how a picture point lies in its frames, which are the picture's shape.
struct Example {
    masses: Masses,
    fit: PictureFit,
}

impl Example {
    fn new() -> Self {
        let masses = Masses::generate(SEED, ASPECT);
        let [width, height] = masses.size.map(|side| (side * 1000.0).round() as u32);
        Self {
            fit: PictureFit::of([width, height]),
            masses,
        }
    }

    /// The frame point of the picture point `p` (picture units, its width 1).
    fn at(&self, p: Point) -> Point {
        let side = self.fit.size[0];
        [p[0] * side, p[1] * side]
    }

    /// The picture's place, the whole frame: its top left and bottom right.
    fn rect(&self) -> [Point; 2] {
        [[0.0, 0.0], self.fit.size]
    }

    /// The picture in the original's frame.
    fn given(&self, script: &mut Script) {
        script.given(On::Original, Marker::Given, Drawn::Masses(self.rect()));
    }

    /// The division's lines of `level` in the frame.
    fn lines(&self, level: Level) -> Vec<Vec<Point>> {
        (self.masses.division.iter())
            .filter(|(held, _)| *held == level)
            .map(|(_, line)| line.iter().map(|p| self.at(*p)).collect())
            .collect()
    }

    /// The main lines: the turning point first, then each line passed in the air and drawn.
    fn main_lines(&self, script: &mut Script) {
        let turning = self.at(self.masses.turning);
        script.draw(On::Copy, Marker::Level(Level::Main), Drawn::Dot(turning));
        for (turn, line) in self.lines(Level::Main).into_iter().enumerate() {
            script.draw(On::Copy, Marker::Air, Drawn::Line(line.clone()));
            let drawn = wobbled(&line, STRAY, 0.4 + turn as f64);
            script.draw(On::Copy, Marker::Level(Level::Main), Drawn::Line(drawn));
        }
    }

    /// `level`'s lines a few at a time.
    fn level_lines(&self, script: &mut Script, level: Level) {
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
    fn laid(&self, pen: Marker) -> (On, Marker, Drawn) {
        (On::Copy, pen, Drawn::Masses(self.rect()))
    }
}

/// A division.
pub(super) fn division() -> Demo {
    let example = Example::new();
    let mut script = Script::new();
    example.given(&mut script);
    script.step();
    example.main_lines(&mut script);
    for level in [Level::Second, Level::Third] {
        script.step();
        example.level_lines(&mut script, level);
    }
    let mut compared = vec![example.laid(Marker::Faint)];
    for level in Level::ALL {
        let lines = example.lines(level).into_iter();
        compared.extend(lines.map(|line| (On::Copy, Marker::Level(level), Drawn::Line(line))));
    }
    let mut demo = script.compare(Family::Division, Layout::Pair, compared);
    demo.view = Some(example.rect());
    demo.masses = Some(Box::new(example.masses));
    demo
}

/// A whole copy.
pub(super) fn whole_copy() -> Demo {
    let example = Example::new();
    let mut script = Script::new();
    example.given(&mut script);
    script.step();
    example.main_lines(&mut script);
    example.level_lines(&mut script, Level::Second);
    script.step();
    let drawing: Vec<Vec<Point>> = (example.masses.drawing.iter())
        .map(|line| line.iter().map(|p| example.at(*p)).collect())
        .collect();
    for (turn, group) in drawing.chunks(TOGETHER).enumerate() {
        let marks = (group.iter())
            .map(|line| {
                let drawn = wobbled(line, STRAY, 2.1 + turn as f64);
                (On::Copy, Marker::Ink, Drawn::Line(drawn))
            })
            .collect();
        script.together(marks);
    }
    let mut demo = script.compare(
        Family::WholeCopy,
        Layout::Pair,
        vec![example.laid(Marker::Faint)],
    );
    demo.dimmed = match demo.steps[..] {
        [_, draw, compare] => Some([draw, compare]),
        _ => None,
    };
    demo.view = Some(example.rect());
    demo.masses = Some(Box::new(example.masses));
    demo
}

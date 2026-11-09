//! A result from each kind (S119): every comparison says who judges it. Where an exact answer
//! does, a steady hand copying it is `Clean`, one part off past the tolerance `Near`, several
//! `Redo`, and a copy stopped half-way `Incomplete`; where only the eye can, `Eye`. Boxes, solids
//! and objects are tried with their own hands (`boxes/verdict.rs`, `solids/`, `objects/`).

mod drawn;
mod eye;
mod painted;

use openpaint_practice::{
    Asked, Compare, Comparison, Copied, DrawnCopy, Exercise, Level, Outcome, Point, Random, Rgb8,
    SheetPicture, Verdict,
};

pub use Outcome::{Clean, Near, Redo};

/// A copy as the app would read it off the sheet: the colours laid and the lines drawn.
#[derive(Default)]
pub struct Laid {
    pub picks: Vec<Rgb8>,
    pub lines: Vec<Vec<Point>>,
    pub held: bool,
}

impl Copied for Laid {
    fn holds(&self) -> bool {
        self.held || !self.picks.is_empty() || !self.lines.is_empty()
    }

    fn picks(&self) -> Vec<Rgb8> {
        self.picks.clone()
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy {
            all: self.lines.clone(),
            corrected: None,
        }
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        (self.lines.iter())
            .map(|line| (Level::Main, line.clone()))
            .collect()
    }
}

/// How `exercise`'s copy `laid` is compared (its picture `picture`), if at all.
pub fn compare(
    exercise: &Exercise,
    picture: Option<&SheetPicture>,
    laid: &Laid,
) -> Option<Compare> {
    let original = exercise.original();
    exercise.compare(&Asked {
        exercise,
        original: &original,
        picture,
        copy: laid,
    })
}

/// The comparison shown first: a reading's until its pixels are read.
pub fn shown(compare: Compare) -> Comparison {
    match compare {
        Compare::Made(shown) | Compare::Fitted(shown, _) | Compare::Checked(shown) => shown,
        Compare::Read(reading) => reading.unread(),
    }
}

/// `exercise`'s copy drawn as `lines` checked as geometry, its verdict.
pub fn checked(exercise: &Exercise, lines: Vec<Vec<Point>>) -> Verdict {
    let copy = DrawnCopy {
        all: lines,
        corrected: None,
    };
    let compared = exercise.check(&exercise.original(), &copy);
    compared.expect("checked as geometry").verdict()
}

/// `strokes` as a steady hand draws them: each sample off by up to half a pixel of a 400-pixel
/// frame, seeded `seed`.
pub fn steady(strokes: Vec<Vec<Point>>, seed: u64) -> Vec<Vec<Point>> {
    let mut random = Random::new(seed);
    (strokes.into_iter())
        .map(|stroke| {
            (stroke.into_iter())
                .map(|[x, y]| {
                    let tremor = 0.0012;
                    [
                        x + random.range(-tremor, tremor),
                        y + random.range(-tremor, tremor),
                    ]
                })
                .collect()
        })
        .collect()
}

/// `stroke` moved by `by`.
pub fn moved(stroke: &[Point], by: Point) -> Vec<Point> {
    stroke
        .iter()
        .map(|p| [p[0] + by[0], p[1] + by[1]])
        .collect()
}

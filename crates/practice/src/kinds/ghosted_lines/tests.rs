//! Ghosted lines' tasks and answers (S156): every task keeps its rules seed after seed; the lines
//! laid join the given points exactly, a plane's middles through its centre toward where its
//! opposite edges meet, from edge to edge; Compare lays the same whatever the copy holds and never
//! before something is drawn; each level lays its own variant.

use super::GhostedLines;
use super::ghosted::{Ghosted, GhostedTask};
use super::plane::Plane;
use crate::compositions::levels::Level;
use crate::kinds::{Described, Kind, Original, SkillLevel, Turn};
use crate::marks::asked::{Asked, Compare, Copied};
use crate::marks::card::{CardItem, Choices};
use crate::marks::compared::{DrawnCopy, Seen, Verdict};
use crate::marks::mark::{Figure, In, LinePen, SheetMark};
use crate::maths::colour::Rgb8;
use crate::maths::path::{Point, distance};
use crate::packs::pictures::Pictures;
use crate::sheets::exercise::Exercise;

/// Seeds looked at a variant.
const SEEDS: u64 = 300;
/// How far a point may lie off a line it is on.
const ON: f64 = 1e-9;

/// How far `p` lies off the line through `a` and `b`.
fn off_line(p: Point, [a, b]: [Point; 2]) -> f64 {
    let [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / dx.hypot(dy)
}

/// Whether `p` lies on the segment from `a` to `b`, strictly between its ends.
fn between(p: Point, [a, b]: [Point; 2]) -> bool {
    let whole = distance(a, b);
    off_line(p, [a, b]) < ON && distance(a, p) < whole && distance(p, b) < whole
}

#[test]
fn every_task_keeps_its_rules_seed_after_seed() {
    for ghosted in Ghosted::ALL {
        for seed in 0..SEEDS {
            let task = GhostedTask::generate(ghosted, seed);
            assert!(task.keeps(), "{ghosted:?} {seed}");
            let (lines, points) = match ghosted {
                Ghosted::Superimposed => (1, 1),
                Ghosted::Pairs => (5, 10),
                Ghosted::Plane => (8, 4),
            };
            assert_eq!(
                (task.lines.len(), task.points().len()),
                (lines, points),
                "{ghosted:?} {seed}"
            );
        }
    }
    // The same task for the same seed, another for another.
    let task = |seed| GhostedTask::generate(Ghosted::Pairs, seed);
    assert_eq!(task(7), task(7));
    assert_ne!(task(7), task(8));
}

#[test]
fn the_lines_laid_join_the_given_points_exactly() {
    for seed in 0..SEEDS {
        let line = GhostedTask::generate(Ghosted::Superimposed, seed);
        assert_eq!(line.points(), [line.lines[0][0]], "its start dotted");
        let pairs = GhostedTask::generate(Ghosted::Pairs, seed);
        let ends: Vec<Point> = pairs.lines.iter().flatten().copied().collect();
        assert_eq!(pairs.points(), ends, "{seed}");
        let task = GhostedTask::generate(Ghosted::Plane, seed);
        let plane = task.plane.expect("a plane");
        planed(&plane, &task.lines, seed);
    }
}

/// `lines` are `plane`'s: its edges and diagonals corner to corner, its middles through its
/// centre toward where its opposite edges meet (along them where parallel), from edge to edge.
fn planed(plane: &Plane, lines: &[[Point; 2]], seed: u64) {
    let [a, b, c, d] = plane.corners;
    assert_eq!(lines[..6], [[a, b], [b, c], [c, d], [d, a], [a, c], [b, d]]);
    let centre = plane.centre();
    assert!(off_line(centre, [a, c]) < ON && off_line(centre, [b, d]) < ON);
    let pairs = [([a, d], [b, c], [a, b]), ([a, b], [d, c], [a, d])];
    for ((middle, (from, to, along)), vanishing) in
        lines[6..].iter().zip(pairs).zip(plane.vanishing())
    {
        assert!(off_line(centre, *middle) < ON, "{seed}: through the centre");
        assert!(between(middle[0], from) && between(middle[1], to), "{seed}");
        let heading = |[p, q]: [Point; 2]| [q[0] - p[0], q[1] - p[1]];
        match vanishing {
            Some(v) => assert!(off_line(v, *middle) < 1e-7 * distance(v, centre), "{seed}"),
            None => {
                let ([u0, u1], [w0, w1]) = (heading(*middle), heading(along));
                assert!((u0 * w1 - u1 * w0).abs() < ON, "{seed}: parallel");
            }
        }
    }
}

/// A copy of `lines`, nothing painted.
struct Lines(Vec<Vec<Point>>);

impl Copied for Lines {
    fn holds(&self) -> bool {
        !self.0.is_empty()
    }

    fn picks(&self) -> Vec<Rgb8> {
        Vec::new()
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy {
            all: self.0.clone(),
        }
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        Vec::new()
    }
}

/// What Compare lays over `copy` for `task` with Construction on: its marks, card and verdict.
fn laid(task: &GhostedTask, copy: &Lines) -> Option<(Vec<SheetMark>, Vec<CardItem>, Verdict)> {
    let exercise = Exercise::generated(Kind::Ghosted(task.ghosted), 5);
    let asked = Asked {
        exercise: &exercise,
        original: task,
        picture: None,
        copy,
        choices: Choices::default(),
    };
    let Compare::Checked(compared) = task.compare(&asked)? else {
        panic!("checked");
    };
    let seen = Seen {
        construction: true,
        shown: 1.0,
        chosen: 0,
        outline: &[],
        grey: false,
        view: 0,
        fit: None,
        probe: None,
    };
    Some((
        compared.marks(&seen),
        compared.card(&seen),
        compared.verdict(),
    ))
}

#[test]
fn compare_lays_the_exact_lines_whatever_the_copy_holds_and_waits_for_the_eye() {
    for ghosted in Ghosted::ALL {
        let task = GhostedTask::generate(ghosted, 11);
        assert!(laid(&task, &Lines(Vec::new())).is_none(), "nothing drawn");
        let exact = Lines(task.answer());
        let off = Lines(vec![vec![[0.1, 0.9], [0.8, 0.2]]]);
        let (marks, card, verdict) = laid(&task, &exact).expect("compared");
        assert_eq!(
            Some((marks.clone(), card.clone(), verdict)),
            laid(&task, &off)
        );
        assert_eq!(verdict, Verdict::Eye);
        let answered: Vec<Vec<Point>> = (marks.iter())
            .filter_map(|mark| match &mark.drawn {
                Figure::Line { points, pen, .. } if *pen == LinePen::Answer => Some(points.clone()),
                _ => None,
            })
            .collect();
        let lines: Vec<Vec<Point>> = task.lines.iter().map(|line| line.to_vec()).collect();
        assert_eq!(answered, lines, "{ghosted:?}: the exact lines, red");
        assert!(marks.iter().all(|mark| mark.on == In::Copy));
        let ringed = |mark: &SheetMark| matches!(mark.drawn, Figure::Ring(_));
        assert!(!marks.iter().any(ringed), "nothing ringed");
        // Only a plane has a construction: its edges run on toward where they meet.
        let plane = ghosted == Ghosted::Plane;
        assert_eq!(card == [CardItem::Construction], plane, "{ghosted:?}");
        let faint = |mark: &SheetMark| {
            matches!(
                mark.drawn,
                Figure::Line {
                    pen: LinePen::Faint,
                    ..
                }
            )
        };
        assert_eq!(marks.iter().any(faint), plane, "{ghosted:?}");
    }
}

#[test]
fn each_level_lays_its_own_variant() {
    let pictures = Pictures::default();
    let turn = Turn {
        turn: 4,
        seed: 3,
        pictures: &pictures,
        used: &[],
        recent: &[],
    };
    let laid = |level| -> Vec<Kind> {
        let lesson = GhostedLines.lesson(level, &turn);
        lesson.into_iter().map(|(kind, _)| kind).collect()
    };
    let of = |ghosted, count| vec![Kind::Ghosted(ghosted); count];
    assert_eq!(laid(SkillLevel::First), of(Ghosted::Superimposed, 3));
    assert_eq!(laid(SkillLevel::Second), of(Ghosted::Pairs, 2));
    assert_eq!(laid(SkillLevel::Third), of(Ghosted::Plane, 3));
}

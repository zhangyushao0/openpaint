//! Picture division (S95): a level's strokes joined into lines on known drawings (a line drawn in
//! three pieces is one, two crossing lines two, a part's frame of four strokes four), each level
//! counted and measured, the numbers kept by their codes; and the landscape of masses a How plays
//! on: its division and its line drawing lie where its picture changes.

use openpaint_practice::{
    DivisionCheck, JOIN_REACH, Level, MOST_MAIN, Masses, Metric, Point, distance, joined, length,
    oklab, resample, value,
};

/// A straight stroke from `a` to `b`, sampled as a pen's.
fn stroke(a: Point, b: Point) -> Vec<Point> {
    resample(&[a, b], 0.004)
}

#[test]
fn a_line_drawn_in_pieces_is_one_crossing_lines_two_and_a_frame_four() {
    // Three pieces along one line, gaps and an overlap of under two pen widths, one drawn back.
    let pieces = [
        stroke([0.05, 0.40], [0.35, 0.42]),
        stroke([0.62, 0.44], [0.37, 0.425]),
        stroke([0.60, 0.437], [0.95, 0.46]),
    ];
    let slices: Vec<&[Point]> = pieces.iter().map(Vec::as_slice).collect();
    let lines = joined(&slices);
    assert_eq!(lines.len(), 1, "one line");
    let whole = length(&lines[0]);
    assert!((whole - 0.92).abs() < 0.04, "{whole}");
    // Two crossing lines stay two: their ends are far apart.
    let crossing = [
        stroke([0.1, 0.1], [0.9, 0.9]),
        stroke([0.1, 0.9], [0.9, 0.1]),
    ];
    let slices: Vec<&[Point]> = crossing.iter().map(Vec::as_slice).collect();
    assert_eq!(joined(&slices).len(), 2);
    // A part's frame of four strokes meeting at its corners: they turn, so four lines.
    let corners = [[0.3, 0.3], [0.6, 0.3], [0.6, 0.7], [0.3, 0.7]];
    let frame: Vec<Vec<Point>> = (0..4)
        .map(|k| stroke(corners[k], corners[(k + 1) % 4]))
        .collect();
    let slices: Vec<&[Point]> = frame.iter().map(Vec::as_slice).collect();
    assert_eq!(joined(&slices).len(), 4);
    // A bend of 30 degrees where two strokes meet heads on; one of 60 does not.
    let bend = |degrees: f64| {
        let turn = degrees.to_radians();
        let middle = [0.5, 0.5];
        let end = [middle[0] + 0.3 * turn.cos(), middle[1] + 0.3 * turn.sin()];
        let pieces = [stroke([0.2, 0.5], middle), stroke(middle, end)];
        let slices: Vec<&[Point]> = pieces.iter().map(Vec::as_slice).collect();
        joined(&slices).len()
    };
    assert_eq!((bend(30.0), bend(60.0)), (1, 2));
    // Ends further apart than two pen widths stay apart; a dab is no line.
    let apart = [
        stroke([0.1, 0.5], [0.4, 0.5]),
        stroke([0.4 + 1.5 * JOIN_REACH, 0.5], [0.8, 0.5]),
    ];
    let slices: Vec<&[Point]> = apart.iter().map(Vec::as_slice).collect();
    assert_eq!(joined(&slices).len(), 2);
    let dab = [stroke([0.5, 0.5], [0.51, 0.5])];
    let slices: Vec<&[Point]> = dab.iter().map(Vec::as_slice).collect();
    assert!(joined(&slices).is_empty());
}

#[test]
fn each_level_is_counted_and_measured_against_the_pictures_longer_side() {
    let side = 0.9;
    let mut pieces = vec![
        (Level::Main, stroke([0.05, 0.3], [0.95, 0.35])),
        (Level::Main, stroke([0.05, 0.6], [0.5, 0.62])),
        (Level::Main, stroke([0.5, 0.62], [0.95, 0.64])),
        (Level::Second, stroke([0.2, 0.62], [0.2, 0.4])),
        (Level::Second, stroke([0.2, 0.4], [0.35, 0.4])),
        (Level::Second, stroke([0.35, 0.4], [0.35, 0.62])),
    ];
    let check = DivisionCheck::of(&pieces, side);
    assert_eq!(check.rows.map(|row| row.lines), [2, 3, 0]);
    let main = check.rows[0].length.expect("main lines");
    assert!((main - 0.9 / side).abs() < 0.03, "{main}");
    let second = check.rows[1].length.expect("second lines");
    assert!(
        (second - (0.22 + 0.15 + 0.22) / 3.0 / side).abs() < 0.01,
        "{second}"
    );
    assert_eq!(check.rows[2].length, None);
    assert_eq!(check.lines.len(), 5);
    assert!(!check.too_many_main());
    // The numbers kept: each level's count, its mean length where it has a line.
    let kept = |metric| value(&check.metrics, metric);
    assert_eq!(kept(Metric::DivisionLines(Level::Main)), Some(2.0));
    assert_eq!(kept(Metric::DivisionLines(Level::Third)), Some(0.0));
    assert_eq!(kept(Metric::DivisionLength(Level::Main)), Some(main));
    assert_eq!(kept(Metric::DivisionLength(Level::Third)), None);
    let codes: Vec<u8> = Level::ALL
        .iter()
        .flat_map(|l| [Metric::DivisionLines(*l), Metric::DivisionLength(*l)])
        .map(Metric::code)
        .collect();
    assert_eq!(codes, [49, 50, 51, 52, 53, 54]);
    for code in 49..=54 {
        assert_eq!(Metric::of_code(code).map(Metric::code), Some(code));
    }
    // V大's at most five main lines.
    for k in 0..4 {
        let y = 0.1 + 0.2 * f64::from(k);
        pieces.push((Level::Main, stroke([0.1, y], [0.3, y + 0.05])));
    }
    let many = DivisionCheck::of(&pieces, side);
    assert_eq!(many.rows[0].lines, 6);
    assert!(many.rows[0].lines > MOST_MAIN && many.too_many_main());
}

/// The picture's colour at `p` (picture units) on a picture `width` pixels wide.
fn at(picture: &([u32; 2], Vec<u8>), p: Point) -> [u8; 3] {
    let ([width, height], pixels) = picture;
    let scale = f64::from(*width);
    let x = ((p[0] * scale) as usize).min(*width as usize - 1);
    let y = ((p[1] * scale).max(0.0) as usize).min(*height as usize - 1);
    let k = 4 * (y * *width as usize + x);
    [pixels[k], pixels[k + 1], pixels[k + 2]]
}

#[test]
fn a_landscapes_lines_lie_where_its_picture_changes() {
    for seed in 0..12 {
        let aspect = [0.75, 1.0, 0.6][seed as usize % 3];
        let masses = Masses::generate(seed, aspect);
        let picture = masses.picture(400);
        assert_eq!(picture.0, [400, (400.0 * aspect).round() as u32]);
        assert!(picture.1.chunks(4).all(|p| p[3] == 255));
        // Across every line of its drawing, 4 pixels either way, the picture changes plainly
        // nearly everywhere (where the tree or the house stands over the ridge it may not).
        let (mut changed, mut all) = (0, 0);
        for line in &masses.drawing {
            let points = resample(line, 0.01);
            for pair in points.windows(2) {
                let d = [pair[1][0] - pair[0][0], pair[1][1] - pair[0][1]];
                let long = d[0].hypot(d[1]).max(1e-9);
                let n = [-d[1] / long * 0.01, d[0] / long * 0.01];
                let [a, b] = [
                    at(&picture, [pair[0][0] + n[0], pair[0][1] + n[1]]),
                    at(&picture, [pair[0][0] - n[0], pair[0][1] - n[1]]),
                ];
                let [p, q] = [oklab(a), oklab(b)];
                if distance([p[0], p[1]], [q[0], q[1]]).hypot(p[2] - q[2]) > 0.05 {
                    changed += 1;
                }
                all += 1;
            }
        }
        assert!(changed * 10 >= all * 8, "seed {seed}: {changed} of {all}");
        // The main lines run right through, from edge to edge, the ridge's peak their turning
        // point; at most five; the second and third levels shorter.
        let main: Vec<&Vec<Point>> = (masses.division.iter())
            .filter(|(level, _)| *level == Level::Main)
            .map(|(_, line)| line)
            .collect();
        assert!((1..=MOST_MAIN).contains(&main.len()));
        for line in &main {
            assert_eq!((line[0][0], line[line.len() - 1][0]), (0.0, 1.0));
        }
        assert!(main.iter().any(|line| line.contains(&masses.turning)));
        let longest = |level: Level| {
            (masses.division.iter())
                .filter(|(held, _)| *held == level)
                .map(|(_, line)| length(line))
                .fold(0.0, f64::max)
        };
        assert!(longest(Level::Second) < longest(Level::Main));
        assert!(longest(Level::Third) < longest(Level::Main));
    }
}

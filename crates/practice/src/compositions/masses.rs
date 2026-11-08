//! A picture of a few masses whose boundaries are known (S95): a small landscape of sky, a far
//! ridge, a field, a tree and a house, laid from a seed. A division's How plays V大's way on it with
//! no pack (the ridge and the field's edge run right through the picture: the main lines, the
//! ridge's peak its turning point; the tree's and the house's frames the second level; the roof,
//! the door, the window and the path the third), and the scenarios' composition pack is made of
//! such pictures. Its line drawing at low completion (the big shapes, no details) is the whole
//! copy's example. Everything is in picture units, its width 1 and its height `size[1]`; the
//! picture itself is `masses_picture.rs`.

use crate::compositions::levels::Level;
use crate::compositions::value_masses::Value;
use crate::maths::path::Point;
use crate::maths::random::Random;

/// What a part is painted as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Paint {
    Sky,
    Hills,
    Field,
    Path,
    Wall,
    Roof,
    Opening,
    Trunk,
    Crown,
}

impl Paint {
    /// Its value, as its colours keep it by day and in the evening (`masses_picture.rs`, S96).
    pub(crate) const fn value(self) -> Value {
        match self {
            Self::Sky | Self::Path | Self::Wall => Value::Light,
            Self::Hills | Self::Field => Value::Middle,
            Self::Roof | Self::Opening | Self::Trunk | Self::Crown => Value::Dark,
        }
    }
}

/// A part of the picture: its outline (a closed polygon) and what it is painted as.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Part {
    pub(crate) outline: Vec<Point>,
    pub(crate) paint: Paint,
}

/// A landscape of a few masses.
#[derive(Debug, Clone, PartialEq)]
pub struct Masses {
    /// Width and height, picture units (its width 1).
    pub size: Point,
    /// The parts in the order painted.
    pub(crate) parts: Vec<Part>,
    /// An evening's colours rather than a day's.
    pub(crate) evening: bool,
    /// The seed its grain is drawn from.
    pub(crate) seed: u64,
    /// Its division as V大 draws it: each line's level and points, the main lines first.
    pub division: Vec<(Level, Vec<Point>)>,
    /// The bent main line's turning point (the ridge's peak), found first.
    pub turning: Point,
    /// Its big shapes' lines: the line drawing a whole copy draws at low completion.
    pub drawing: Vec<Vec<Point>>,
}

impl Masses {
    /// The landscape `seed` lays in a picture `aspect` as high as it is wide.
    #[must_use]
    pub fn generate(seed: u64, aspect: f64) -> Self {
        let mut random = Random::new(seed);
        let h = aspect.clamp(0.5, 1.4);
        let ridge = [
            [0.0, h * random.range(0.40, 0.48)],
            [random.range(0.3, 0.7), h * random.range(0.17, 0.25)],
            [1.0, h * random.range(0.36, 0.46)],
        ];
        let ground = [
            [0.0, h * random.range(0.66, 0.72)],
            [random.range(0.38, 0.62), h * random.range(0.62, 0.7)],
            [1.0, h * random.range(0.66, 0.74)],
        ];
        let tree_left = random.unit() < 0.5;
        let evening = random.unit() < 0.4;
        let ridge_painted = bumped(&ridge, 0.012 * h, &mut random);
        let ground_painted = bumped(&ground, 0.006 * h, &mut random);
        let mut parts = vec![Part {
            outline: vec![[0.0, 0.0], [1.0, 0.0], [1.0, h], [0.0, h]],
            paint: Paint::Sky,
        }];
        parts.push(below(&ridge_painted, h, Paint::Hills));
        parts.push(below(&ground_painted, h, Paint::Field));
        let mut masses = Self {
            size: [1.0, h],
            parts,
            evening,
            seed,
            division: vec![
                (Level::Main, ridge.to_vec()),
                (Level::Main, ground.to_vec()),
            ],
            turning: ridge[1],
            drawing: vec![ridge_painted, ground_painted],
        };
        let house_x = if tree_left {
            random.range(0.56, 0.68)
        } else {
            random.range(0.32, 0.44)
        };
        let tree_x = if tree_left {
            random.range(0.15, 0.24)
        } else {
            random.range(0.76, 0.85)
        };
        masses.house(house_x, &ground, &mut random);
        masses.tree(tree_x, &ground, &mut random);
        masses
    }

    /// A house standing on the field at `x`, the path leading to its door.
    fn house(&mut self, x: f64, ground: &[Point; 3], random: &mut Random) {
        let h = self.size[1];
        let width = random.range(0.16, 0.21);
        let [left, right] = [x - width / 2.0, x + width / 2.0];
        let base = on(ground, left).max(on(ground, right)) + 0.012 * h;
        let eaves = base - h * random.range(0.13, 0.17);
        let apex = eaves - h * random.range(0.08, 0.11);
        let over = 0.018;
        let door = left + width * random.range(0.22, 0.36);
        let (door_width, door_top) = (0.032, base - 0.62 * (base - eaves));
        let window = [left + width * 0.62, (eaves + base) / 2.0 - 0.025];
        let side = 0.036;
        let window_square = rect(window, [window[0] + side, window[1] + side]);
        let door_line = vec![
            [door, base],
            [door, door_top],
            [door + door_width, door_top],
            [door + door_width, base],
        ];
        let roof = vec![[left - over, eaves], [x, apex], [right + over, eaves]];
        let path = [
            [[door - 0.13, h], [door - 0.004, base]],
            [
                [door + door_width + 0.09, h],
                [door + door_width + 0.004, base],
            ],
        ];
        self.parts.push(Part {
            outline: vec![path[0][0], path[0][1], path[1][1], path[1][0]],
            paint: Paint::Path,
        });
        let body = rect([left, eaves], [right, base]);
        self.parts.push(part(&body, Paint::Wall));
        self.parts.push(part(&roof, Paint::Roof));
        self.parts.push(part(&door_line, Paint::Opening));
        self.parts.push(part(&window_square, Paint::Opening));
        let frame = [[left - over, apex], [right + over, base]];
        self.framed(frame);
        self.division.extend(
            [
                vec![roof[0], roof[1]],
                vec![roof[1], roof[2]],
                vec![[left, eaves], [right, eaves]],
                door_line.clone(),
                closed(&window_square),
                path[0].to_vec(),
                path[1].to_vec(),
            ]
            .map(|line| (Level::Third, line)),
        );
        self.drawing.extend([
            closed(&body),
            roof,
            door_line,
            path[0].to_vec(),
            path[1].to_vec(),
        ]);
    }

    /// A tree standing on the field at `x`: its trunk and its crown, lumpy round.
    fn tree(&mut self, x: f64, ground: &[Point; 3], random: &mut Random) {
        let h = self.size[1];
        let foot = on(ground, x) + 0.01 * h;
        let trunk = h * random.range(0.06, 0.09);
        let [rx, ry] = [random.range(0.08, 0.11), h * random.range(0.17, 0.23)];
        let centre = [x, foot - trunk - ry * 0.85];
        let [a, b, c, d] = [0.0, 0.0, 0.0, 0.0].map(|_| random.range(0.0, std::f64::consts::TAU));
        let crown: Vec<Point> = (0..36)
            .map(|k| {
                let turn = std::f64::consts::TAU * f64::from(k) / 36.0;
                let lump = 1.0 + 0.07 * (5.0 * turn + a).sin() + 0.04 * (9.0 * turn + b).sin();
                let lean = 0.03 * (turn + c).cos() + 0.02 * (2.0 * turn + d).sin();
                [
                    centre[0] + rx * lump * turn.cos() + lean,
                    centre[1] + ry * lump * turn.sin(),
                ]
            })
            .collect();
        let stem = 0.012;
        let trunk_part = rect([x - stem, centre[1]], [x + stem, foot]);
        self.parts.push(part(&trunk_part, Paint::Trunk));
        self.parts.push(part(&crown, Paint::Crown));
        let [low, high] = bounds(&crown);
        self.framed([[low[0], low[1]], [high[0], foot]]);
        self.drawing.push(closed(&crown));
        self.drawing
            .push(vec![[x - stem, crown_bottom(&crown, x)], [x - stem, foot]]);
        self.drawing
            .push(vec![[x + stem, crown_bottom(&crown, x)], [x + stem, foot]]);
    }

    /// A part's frame from its top left to its bottom right, standing on the field: its left side,
    /// its top and its right side, the second level.
    fn framed(&mut self, [[left, top], [right, bottom]]: [Point; 2]) {
        self.division.extend(
            [
                vec![[left, bottom], [left, top]],
                vec![[left, top], [right, top]],
                vec![[right, top], [right, bottom]],
            ]
            .map(|line| (Level::Second, line)),
        );
    }
}

/// The height of the bent line `line` (left to right, three points) at `x`.
fn on(line: &[Point; 3], x: f64) -> f64 {
    let [a, b] = if x <= line[1][0] {
        [line[0], line[1]]
    } else {
        [line[1], line[2]]
    };
    let t = ((x - a[0]) / (b[0] - a[0]).max(f64::MIN_POSITIVE)).clamp(0.0, 1.0);
    a[1] + (b[1] - a[1]) * t
}

/// `line` (left to right) as painted: 48 points along it, its small turns (`amount` either way)
/// laid on in two slow waves, the ends kept.
fn bumped(line: &[Point; 3], amount: f64, random: &mut Random) -> Vec<Point> {
    let [p, q] = [random.range(0.0, 6.0), random.range(0.0, 6.0)];
    (0..=48)
        .map(|k| {
            let x = f64::from(k) / 48.0;
            let fade = (std::f64::consts::PI * x).sin();
            let turn = (11.0 * x + p).sin() + 0.6 * (23.0 * x + q).sin();
            [x, on(line, x) + amount * fade * turn]
        })
        .collect()
}

/// The part under `line` (left to right) down to the picture's foot `h`.
fn below(line: &[Point], h: f64, paint: Paint) -> Part {
    let mut outline = line.to_vec();
    outline.extend([[1.0, h], [0.0, h]]);
    Part { outline, paint }
}

fn part(outline: &[Point], paint: Paint) -> Part {
    Part {
        outline: outline.to_vec(),
        paint,
    }
}

/// The rectangle from `[left, top]` to `[right, bottom]`, clockwise.
fn rect([left, top]: Point, [right, bottom]: Point) -> Vec<Point> {
    vec![[left, top], [right, top], [right, bottom], [left, bottom]]
}

/// `points` with the first again at the end.
fn closed(points: &[Point]) -> Vec<Point> {
    points.iter().chain(points.first()).copied().collect()
}

/// The top left and bottom right of `points`.
pub(crate) fn bounds(points: &[Point]) -> [Point; 2] {
    points.iter().fold(
        [[f64::MAX, f64::MAX], [f64::MIN, f64::MIN]],
        |[low, high], p| {
            [
                [low[0].min(p[0]), low[1].min(p[1])],
                [high[0].max(p[0]), high[1].max(p[1])],
            ]
        },
    )
}

/// Where the crown's underside lies above `x`: its lowest point near the trunk.
fn crown_bottom(crown: &[Point], x: f64) -> f64 {
    (crown.iter())
        .filter(|p| (p[0] - x).abs() < 0.03)
        .map(|p| p[1])
        .fold(f64::MIN, f64::max)
        .max(crown.iter().map(|p| p[1]).fold(f64::MAX, f64::min))
}

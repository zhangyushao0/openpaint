//! The Drawing Guide's symmetry (Procreate's Drawing Guide > Symmetry, mirrored rather than
//! rotational): while it is on, every stroke the pen draws (brush, eraser, smudge) is drawn
//! again mirrored about the guide's centre, live; each copy is a stroke record of its own,
//! committed with the stroke as one history step. Fills and selections are not mirrored. The
//! guide is the session's state: it starts off with every artwork opened and is not stored.

use openpaint_core::CanvasSize;

/// Which mirrors the guide draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Symmetry {
    /// Mirrored across the upright line through the centre: left and right.
    #[default]
    Vertical,
    /// Mirrored across the level line through the centre: top and bottom.
    Horizontal,
    /// Both: four quarters.
    Quadrant,
    /// Eight segments: mirrored across four lines through the centre, 45 degrees apart.
    Radial,
}

impl Symmetry {
    pub const ALL: [Self; 4] = [
        Self::Vertical,
        Self::Horizontal,
        Self::Quadrant,
        Self::Radial,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vertical => "Vertical",
            Self::Horizontal => "Horizontal",
            Self::Quadrant => "Quadrant",
            Self::Radial => "Radial",
        }
    }

    /// The linear part of every copy besides the stroke itself, as rows `[[a, b], [c, d]]`:
    /// reflections across the guide's lines and, for several lines, the turns they compose to.
    fn copies(self) -> Vec<[[f32; 2]; 2]> {
        const FLIP_X: [[f32; 2]; 2] = [[-1.0, 0.0], [0.0, 1.0]];
        const FLIP_Y: [[f32; 2]; 2] = [[1.0, 0.0], [0.0, -1.0]];
        const HALF_TURN: [[f32; 2]; 2] = [[-1.0, 0.0], [0.0, -1.0]];
        match self {
            Self::Vertical => vec![FLIP_X],
            Self::Horizontal => vec![FLIP_Y],
            Self::Quadrant => vec![FLIP_X, FLIP_Y, HALF_TURN],
            Self::Radial => {
                // Quarter turns, and reflections across lines at 0, 45, 90 and 135 degrees.
                let turns = (1..4u8).map(|quarter| turn(f32::from(quarter) * 90.0));
                let lines = (0..4u8).map(|line| reflection(f32::from(line) * 45.0));
                turns.chain(lines).collect()
            }
        }
    }
}

/// A turn by `degrees` (screen y grows downwards, so positive turns clockwise).
fn turn(degrees: f32) -> [[f32; 2]; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [[round(cos), round(-sin)], [round(sin), round(cos)]]
}

/// The reflection across the line through the origin at `degrees` from the x axis.
fn reflection(degrees: f32) -> [[f32; 2]; 2] {
    let (sin, cos) = (2.0 * degrees.to_radians()).sin_cos();
    [[round(cos), round(sin)], [round(sin), round(-cos)]]
}

/// Exact zeros and ones where trigonometry leaves a few ulps (`cos 90` is not 0 in floats), so
/// a quarter turn maps whole pixels to whole pixels.
fn round(value: f32) -> f32 {
    let nearest = value.round();
    if (value - nearest).abs() < 1e-6 {
        nearest
    } else {
        value
    }
}

/// One mirrored copy: `centre + rows x (point - centre)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mirror {
    rows: [[f32; 2]; 2],
    centre: [f32; 2],
}

impl Mirror {
    /// Where `point` lands in this copy.
    #[must_use]
    pub fn apply(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        let [cx, cy] = self.centre;
        let [[a, b], [c, d]] = self.rows;
        let (dx, dy) = (x - cx, y - cy);
        [cx + a * dx + b * dy, cy + c * dx + d * dy]
    }
}

/// The Drawing Guide: whether it is on, its symmetry and its centre in canvas pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    pub on: bool,
    pub symmetry: Symmetry,
    pub centre: [f32; 2],
}

impl Default for Guide {
    /// Off, vertical, at the canvas's origin (no artwork open).
    fn default() -> Self {
        Self {
            on: false,
            symmetry: Symmetry::default(),
            centre: [0.0, 0.0],
        }
    }
}

/// What the Drawing Guide's controls ask for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuideIntent {
    /// Turns the guide on or off; it keeps its symmetry and centre.
    Show(bool),
    /// Mirrors with `symmetry` from now on, and turns the guide on.
    Symmetry(Symmetry),
    /// Moves the centre to this canvas point, kept on the canvas.
    Centre([f32; 2]),
}

impl Guide {
    /// Off, vertical, in the middle of `canvas`.
    #[must_use]
    pub fn new(canvas: CanvasSize) -> Self {
        Self {
            on: false,
            symmetry: Symmetry::default(),
            centre: [canvas.width() as f32 / 2.0, canvas.height() as f32 / 2.0],
        }
    }

    pub(crate) fn handle(&mut self, intent: GuideIntent, canvas: CanvasSize) {
        match intent {
            GuideIntent::Show(on) => self.on = on,
            GuideIntent::Symmetry(symmetry) => {
                self.symmetry = symmetry;
                self.on = true;
            }
            GuideIntent::Centre([x, y]) => {
                let (width, height) = (canvas.width() as f32, canvas.height() as f32);
                if x.is_finite() && y.is_finite() {
                    self.centre = [x.clamp(0.0, width), y.clamp(0.0, height)];
                }
            }
        }
    }

    /// The copies a stroke begun now is drawn with: none while the guide is off.
    #[must_use]
    pub fn mirrors(&self) -> Vec<Mirror> {
        if !self.on {
            return Vec::new();
        }
        let centre = self.centre;
        self.symmetry
            .copies()
            .into_iter()
            .map(|rows| Mirror { rows, centre })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guide(symmetry: Symmetry, centre: [f32; 2]) -> Guide {
        Guide {
            on: true,
            symmetry,
            centre,
        }
    }

    fn images(guide: &Guide, point: [f32; 2]) -> Vec<[f32; 2]> {
        guide
            .mirrors()
            .iter()
            .map(|mirror| mirror.apply(point))
            .collect()
    }

    #[test]
    fn vertical_and_horizontal_mirror_across_the_centre_lines() {
        let centre = [100.0, 60.0];
        assert_eq!(
            images(&guide(Symmetry::Vertical, centre), [130.0, 10.0]),
            [[70.0, 10.0]]
        );
        assert_eq!(
            images(&guide(Symmetry::Horizontal, centre), [130.0, 10.0]),
            [[130.0, 110.0]]
        );
        let quarters = images(&guide(Symmetry::Quadrant, centre), [130.0, 10.0]);
        assert_eq!(quarters, [[70.0, 10.0], [130.0, 110.0], [70.0, 110.0]]);
    }

    #[test]
    fn radial_makes_eight_segments_each_once() {
        let centre = [0.0, 0.0];
        let guide = guide(Symmetry::Radial, centre);
        let mut all = images(&guide, [3.0, 1.0]);
        all.push([3.0, 1.0]);
        let mut expected = vec![
            [3.0, 1.0],
            [-3.0, 1.0],
            [3.0, -1.0],
            [-3.0, -1.0],
            [1.0, 3.0],
            [-1.0, 3.0],
            [1.0, -3.0],
            [-1.0, -3.0],
        ];
        let key = |p: &[f32; 2]| (p[0] as i32, p[1] as i32);
        all.sort_by_key(key);
        expected.sort_by_key(key);
        assert_eq!(all, expected, "the dihedral group of the square, exactly");
        // A point on a guide line has a copy on itself.
        assert!(images(&guide, [2.0, 2.0]).contains(&[2.0, 2.0]));
    }

    #[test]
    fn every_copy_keeps_distances_and_mirroring_twice_is_the_point() {
        for symmetry in Symmetry::ALL {
            let guide = guide(symmetry, [37.5, 81.25]);
            for mirror in guide.mirrors() {
                let [a, b] = [[10.0, 20.0], [55.0, -4.0]].map(|p| mirror.apply(p));
                let length = (b[0] - a[0]).hypot(b[1] - a[1]);
                assert!((length - 45.0f32.hypot(24.0)).abs() < 1e-3, "{symmetry:?}");
                assert_eq!(mirror.apply(guide.centre), guide.centre);
            }
        }
        let vertical = guide(Symmetry::Vertical, [37.5, 0.0]).mirrors()[0];
        assert_eq!(vertical.apply(vertical.apply([3.0, 4.0])), [3.0, 4.0]);
    }

    #[test]
    fn the_guide_is_off_at_first_and_keeps_its_centre_on_the_canvas() {
        let canvas = CanvasSize::new(200, 120).expect("size");
        let mut guide = Guide::new(canvas);
        assert!(guide.mirrors().is_empty());
        assert_eq!(guide.centre, [100.0, 60.0]);
        guide.handle(GuideIntent::Symmetry(Symmetry::Quadrant), canvas);
        assert!(guide.on);
        assert_eq!(guide.mirrors().len(), 3);
        guide.handle(GuideIntent::Centre([-5.0, 300.0]), canvas);
        assert_eq!(guide.centre, [0.0, 120.0]);
        guide.handle(GuideIntent::Show(false), canvas);
        assert!(guide.mirrors().is_empty());
        assert_eq!(guide.symmetry, Symmetry::Quadrant);
    }
}

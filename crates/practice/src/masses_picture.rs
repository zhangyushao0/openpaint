//! A landscape of masses as a picture (`masses.rs`): each part painted in order over the one
//! before, four samples a pixel where parts meet; each in its colour shading from its top to its
//! foot (the sky lightening to the horizon, the far hills hazed, the field darker near), with a
//! slow mottle and a fine grain so it reads as paint rather than flat fills, the crown's mottle
//! stronger (leaves). A day's colours or an evening's.

use crate::colour::Rgb8;
use crate::masses::{Masses, Paint, Part, bounds};
use crate::path::Point;
use crate::random::Random;

/// Each paint's colour at its top and at its foot, by day and in the evening.
const fn colours(paint: Paint, evening: bool) -> [[f64; 3]; 2] {
    match (paint, evening) {
        (Paint::Sky, false) => [[118.0, 158.0, 204.0], [206.0, 220.0, 226.0]],
        (Paint::Sky, true) => [[206.0, 150.0, 124.0], [246.0, 214.0, 168.0]],
        (Paint::Hills, false) => [[138.0, 154.0, 170.0], [112.0, 128.0, 122.0]],
        (Paint::Hills, true) => [[150.0, 120.0, 132.0], [118.0, 96.0, 104.0]],
        (Paint::Field, false) => [[150.0, 158.0, 92.0], [88.0, 102.0, 56.0]],
        (Paint::Field, true) => [[132.0, 118.0, 74.0], [84.0, 72.0, 52.0]],
        (Paint::Path, false) => [[196.0, 178.0, 138.0], [176.0, 154.0, 112.0]],
        (Paint::Path, true) => [[190.0, 158.0, 120.0], [160.0, 126.0, 92.0]],
        (Paint::Wall, false) => [[232.0, 220.0, 192.0], [208.0, 194.0, 166.0]],
        (Paint::Wall, true) => [[236.0, 196.0, 154.0], [204.0, 162.0, 126.0]],
        (Paint::Roof, false) => [[150.0, 74.0, 58.0], [118.0, 58.0, 46.0]],
        (Paint::Roof, true) => [[120.0, 56.0, 52.0], [96.0, 44.0, 42.0]],
        (Paint::Opening, _) => [[62.0, 56.0, 58.0], [52.0, 46.0, 46.0]],
        (Paint::Trunk, _) => [[86.0, 66.0, 48.0], [70.0, 52.0, 38.0]],
        (Paint::Crown, false) => [[70.0, 104.0, 60.0], [40.0, 68.0, 42.0]],
        (Paint::Crown, true) => [[72.0, 74.0, 50.0], [46.0, 46.0, 36.0]],
    }
}

/// How strongly a part is mottled and grained, 8-bit levels either way.
const fn texture(paint: Paint) -> (f64, f64) {
    match paint {
        Paint::Crown => (16.0, 6.0),
        Paint::Field | Paint::Hills => (8.0, 4.0),
        Paint::Sky => (4.0, 1.5),
        _ => (5.0, 3.0),
    }
}

impl Masses {
    /// The picture `width` pixels wide (and as high as its aspect says), 8-bit sRGB, opaque.
    #[must_use]
    pub fn picture(&self, width: u32) -> ([u32; 2], Vec<u8>) {
        let width = width.max(1);
        let scale = f64::from(width);
        let height = ((self.size[1] * scale).round() as u32).max(1);
        let laid: Vec<Laid> = self.parts.iter().map(Laid::of).collect();
        let waves = Waves::new(self.seed);
        let mut grain = Random::new(self.seed ^ 0x6A41_4E5E);
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for row in 0..height {
            for column in 0..width {
                let mut sum = [0.0; 3];
                for [dx, dy] in [[0.25, 0.25], [0.75, 0.25], [0.25, 0.75], [0.75, 0.75]] {
                    let p = [
                        (f64::from(column) + dx) / scale,
                        (f64::from(row) + dy) / scale,
                    ];
                    let colour = self.colour_at(&laid, &waves, p);
                    for (total, channel) in sum.iter_mut().zip(colour) {
                        *total += channel / 4.0;
                    }
                }
                let fine = grain.range(-1.0, 1.0);
                let rgb: Rgb8 = sum.map(|c| (c + fine * 2.0).round().clamp(0.0, 255.0) as u8);
                pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        ([width, height], pixels)
    }

    /// The colour the topmost part over `p` paints there.
    fn colour_at(&self, laid: &[Laid], waves: &Waves, p: Point) -> [f64; 3] {
        let Some(part) = laid.iter().rev().find(|part| part.holds(p)) else {
            return [255.0; 3];
        };
        let [top, foot] = colours(part.paint, self.evening);
        let span = (part.bottom - part.top).max(f64::MIN_POSITIVE);
        let t = ((p[1] - part.top) / span).clamp(0.0, 1.0);
        let (mottle, grain) = texture(part.paint);
        let shade = mottle * waves.at(p) + grain * waves.fine(p);
        [0, 1, 2].map(|k| top[k] + (foot[k] - top[k]) * t + shade)
    }
}

/// A part ready to sample: its outline, its bounds' top and foot, and its box.
struct Laid {
    outline: Vec<Point>,
    paint: Paint,
    top: f64,
    bottom: f64,
    low: Point,
    high: Point,
}

impl Laid {
    fn of(part: &Part) -> Self {
        let [low, high] = bounds(&part.outline);
        Self {
            outline: part.outline.clone(),
            paint: part.paint,
            top: low[1],
            bottom: high[1],
            low,
            high,
        }
    }

    /// Whether `p` lies inside (even-odd).
    fn holds(&self, p: Point) -> bool {
        if p[0] < self.low[0] || p[0] > self.high[0] || p[1] < self.low[1] || p[1] > self.high[1] {
            return false;
        }
        let mut inside = false;
        let count = self.outline.len();
        for k in 0..count {
            let [a, b] = [self.outline[k], self.outline[(k + 1) % count]];
            if (a[1] > p[1]) != (b[1] > p[1]) {
                let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                if p[0] < x {
                    inside = !inside;
                }
            }
        }
        inside
    }
}

/// A slow mottle and a finer one, each a few waves at the seed's phases, about -1 to 1.
struct Waves {
    phases: [f64; 6],
}

impl Waves {
    fn new(seed: u64) -> Self {
        let mut random = Random::new(seed ^ 0x003A_55E5);
        Self {
            phases: [0.0; 6].map(|_| random.range(0.0, std::f64::consts::TAU)),
        }
    }

    fn at(&self, [x, y]: Point) -> f64 {
        let p = &self.phases;
        0.5 * (7.0 * x + 3.0 * y + p[0]).sin()
            + 0.3 * (5.0 * y - 9.0 * x + p[1]).sin()
            + 0.2 * (13.0 * x + 11.0 * y + p[2]).sin()
    }

    fn fine(&self, [x, y]: Point) -> f64 {
        let p = &self.phases;
        0.5 * (61.0 * x + 37.0 * y + p[3]).sin()
            + 0.3 * (47.0 * y - 71.0 * x + p[4]).sin()
            + 0.2 * (97.0 * x - 89.0 * y + p[5]).sin()
    }
}

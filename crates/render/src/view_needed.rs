//! Which view tiles a view needs (S32): the tiles the screen's canvas area shows, turned with
//! the view, then a ring of one tile round them, and a second ring kept but not filled.

use crate::view_atlas::Tier;
use crate::view_tier::ViewFrame;
use crate::view_tiles::{TIER_TILE, TileMap};

/// Tiles at most this many rings out are kept once evaluated; beyond, dropped.
const KEPT_RINGS: u8 = 2;

pub(crate) struct Needed {
    tiles: [u32; 2],
    /// Per view tile, row-major: 0 shown, 1 or 2 its ring, `u8::MAX` beyond.
    rings: Vec<u8>,
    /// The shown tiles, nearest the screen's centre first, then the first ring's.
    ordered: Vec<[u32; 2]>,
    shown: usize,
}

impl Needed {
    /// The tiles of `map`'s tier that `frame` needs.
    pub(crate) fn of(map: &TileMap, frame: &ViewFrame) -> Self {
        let tiles = map.tiles();
        let quad = corners(frame, map.scale() as f32);
        let mut needed = Self {
            tiles,
            rings: vec![u8::MAX; (tiles[0] * tiles[1]) as usize],
            ordered: Vec::new(),
            shown: 0,
        };
        let span = |axis: usize| {
            let (low, high) = quad
                .iter()
                .fold((f32::MAX, f32::MIN), |(low, high), corner| {
                    (low.min(corner[axis]), high.max(corner[axis]))
                });
            let side = TIER_TILE as f32;
            let first = (low / side).floor().max(0.0) as u32;
            let end = ((high / side).floor() + 1.0).clamp(0.0, tiles[axis] as f32) as u32;
            first..end
        };
        let (columns, rows) = (span(0), span(1));
        let mut shown = Vec::new();
        for row in rows {
            for column in columns.clone() {
                if meets(&quad, [column, row]) {
                    shown.push([column, row]);
                }
            }
        }
        for tile in &shown {
            needed.mark(*tile);
        }
        let centre = quad.iter().fold([0.0, 0.0], |sum, corner| {
            [sum[0] + corner[0] / 4.0, sum[1] + corner[1] / 4.0]
        });
        let far = |tile: &[u32; 2]| {
            let middle = tile.map(|at| (at as f32 + 0.5) * TIER_TILE as f32);
            (middle[0] - centre[0]).powi(2) + (middle[1] - centre[1]).powi(2)
        };
        let mut ring: Vec<[u32; 2]> = (0..tiles[1])
            .flat_map(|row| (0..tiles[0]).map(move |column| [column, row]))
            .filter(|tile| needed.ring(*tile) == 1)
            .collect();
        shown.sort_by(|a, b| far(a).total_cmp(&far(b)));
        ring.sort_by(|a, b| far(a).total_cmp(&far(b)));
        needed.shown = shown.len();
        needed.ordered = shown;
        needed.ordered.extend(ring);
        needed
    }

    /// Marks `tile` shown and its rings.
    fn mark(&mut self, tile: [u32; 2]) {
        let reach = i64::from(KEPT_RINGS);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let [x, y] = [i64::from(tile[0]) + dx, i64::from(tile[1]) + dy];
                if x < 0 || y < 0 || x >= i64::from(self.tiles[0]) || y >= i64::from(self.tiles[1])
                {
                    continue;
                }
                let at = (y * i64::from(self.tiles[0]) + x) as usize;
                let ring = dx.abs().max(dy.abs()) as u8;
                self.rings[at] = self.rings[at].min(ring);
            }
        }
    }

    fn ring(&self, [column, row]: [u32; 2]) -> u8 {
        self.rings
            .get((row * self.tiles[0] + column) as usize)
            .copied()
            .unwrap_or(u8::MAX)
    }

    /// Shown and first-ring tiles: how many.
    pub(crate) fn count(&self) -> usize {
        self.ordered.len()
    }

    /// Shown tiles nearest the centre first, then the first ring's.
    pub(crate) fn ordered(&self) -> impl Iterator<Item = [u32; 2]> + '_ {
        self.ordered.iter().copied()
    }

    /// `tile` is shown.
    pub(crate) fn shown(&self, tile: [u32; 2]) -> bool {
        self.ring(tile) == 0
    }

    /// `tile` is shown or in the first ring.
    pub(crate) fn needs(&self, tile: [u32; 2]) -> bool {
        self.ring(tile) <= 1
    }

    /// `tile` is kept once evaluated.
    pub(crate) fn keeps(&self, tile: [u32; 2]) -> bool {
        self.ring(tile) <= KEPT_RINGS
    }

    /// Needed tiles `tier` does not have ready.
    pub(crate) fn waiting(&self, tier: &Tier) -> usize {
        self.ordered().filter(|tile| !tier.is_ready(*tile)).count()
    }

    /// `tier` has every shown tile ready.
    pub(crate) fn shows(&self, tier: &Tier) -> bool {
        self.ordered[..self.shown]
            .iter()
            .all(|tile| tier.is_ready(*tile))
    }
}

/// The canvas area's corners in output pixels at `scale`.
fn corners(frame: &ViewFrame, scale: f32) -> [[f32; 2]; 4] {
    let [a, b, c, d, e, f] = frame.to_canvas;
    let [x0, y0] = frame.origin.map(|at| at as f32);
    let [x1, y1] = [x0 + frame.size[0] as f32, y0 + frame.size[1] as f32];
    [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
        .map(|[x, y]| [(a * x + b * y + c) * scale, (d * x + e * y + f) * scale])
}

/// Whether view tile `tile` meets the convex `quad`: no axis of the square's sides or the
/// quad's edges separates them.
fn meets(quad: &[[f32; 2]; 4], tile: [u32; 2]) -> bool {
    let side = TIER_TILE as f32;
    let [x0, y0] = tile.map(|at| at as f32 * side);
    let square = [
        [x0, y0],
        [x0 + side, y0],
        [x0 + side, y0 + side],
        [x0, y0 + side],
    ];
    let mut axes = vec![[1.0, 0.0], [0.0, 1.0]];
    for (i, from) in quad.iter().enumerate() {
        let to = quad[(i + 1) % 4];
        axes.push([from[1] - to[1], to[0] - from[0]]);
    }
    axes.iter().all(|axis| {
        let project = |points: &[[f32; 2]; 4]| {
            points.iter().fold((f32::MAX, f32::MIN), |(low, high), p| {
                let along = p[0] * axis[0] + p[1] * axis[1];
                (low.min(along), high.max(along))
            })
        };
        let ((a0, a1), (b0, b1)) = (project(quad), project(&square));
        a1 > b0 && b1 > a0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Output;

    fn frame(to_canvas: [f32; 6]) -> ViewFrame {
        ViewFrame {
            to_canvas,
            origin: [0, 0],
            size: [1000, 600],
        }
    }

    #[test]
    fn an_upright_view_needs_the_tiles_it_shows_and_a_ring() {
        // 4 screen pixels per canvas pixel at tier 4: output = screen, from canvas 100, 50.
        let map = TileMap::new([1000, 800], Output::at(4, [0, 0]).expect("4x"), (1, 1));
        let needed = Needed::of(&map, &frame([0.25, 0.0, 100.0, 0.0, 0.25, 50.0]));
        // Output 400..1400 x 200..800: columns 1..=5, rows 0..=3 shown.
        assert_eq!(needed.shown, 5 * 4);
        assert_eq!(needed.ordered().next(), Some([3, 1]), "the middle first");
        assert!(needed.needs([0, 0]) && needed.needs([6, 4]) && !needed.needs([7, 4]));
        assert!(needed.keeps([7, 5]) && !needed.keeps([8, 0]));
        assert_eq!(needed.count(), 7 * 5);
    }

    #[test]
    fn a_turned_view_needs_no_tile_outside_its_corners() {
        let map = TileMap::new([4000, 4000], Output::at(2, [0, 0]).expect("2x"), (1, 1));
        let (cos, sin) = (
            std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2,
        );
        // Turned 45 degrees, half a canvas pixel per screen pixel, about canvas 2000, 2000.
        let s = 0.5;
        let to_canvas = [s * cos, -s * sin, 2000.0, s * sin, s * cos, 1700.0];
        let needed = Needed::of(&map, &frame(to_canvas));
        let quad = corners(&frame(to_canvas), 2.0);
        let side = TIER_TILE as f32;
        let span = |axis: usize| {
            let low = quad
                .iter()
                .map(|corner| corner[axis])
                .fold(f32::MAX, f32::min);
            let high = quad
                .iter()
                .map(|corner| corner[axis])
                .fold(f32::MIN, f32::max);
            (high / side).floor() - (low / side).floor() + 1.0
        };
        let boxed = (span(0) * span(1)) as usize;
        assert!(needed.shown < boxed * 3 / 4, "{} of {boxed}", needed.shown);
        assert!(meets(&quad, needed.ordered().next().expect("one")));
        assert!(needed.ordered().all(|tile| needed.needs(tile)));
    }
}

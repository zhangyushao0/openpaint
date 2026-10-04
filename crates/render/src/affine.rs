//! `Affine`: a 2D affine map in doubles, for composing the placements a transform record sees
//! its earlier content through (S31) and mapping op bounds through them. Placements stay in
//! core; what the evaluator reads is these maps rounded to floats once.

use openpaint_core::Placement;

use crate::geom::Bounds;

/// `(x, y)` goes to `(a x + b y + c, d x + e y + f)`, the rows `[a, b, c, d, e, f]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Affine([f64; 6]);

impl Default for Affine {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine {
    pub(crate) const IDENTITY: Self = Self([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);

    /// Where `placement` puts a point of the content as it was.
    pub(crate) fn of(placement: Placement) -> Self {
        let [a, b, d, e] = placement.matrix().map(f64::from);
        let [c, f] = placement.translation().map(f64::from);
        Self([a, b, c, d, e, f])
    }

    /// This map, then `next`.
    pub(crate) fn then(self, next: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [na, nb, nc, nd, ne, nf] = next.0;
        Self([
            na * a + nb * d,
            na * b + nb * e,
            na * c + nb * f + nc,
            nd * a + ne * d,
            nd * b + ne * e,
            nd * c + ne * f + nf,
        ])
    }

    /// The map back. Placements never collapse content (core checks their determinant).
    pub(crate) fn inverse(self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let det = a * e - b * d;
        let [ia, ib, id, ie] = [e / det, -b / det, -d / det, a / det];
        Self([ia, ib, -(ia * c + ib * f), id, ie, -(id * c + ie * f)])
    }

    pub(crate) fn apply(self, [x, y]: [f64; 2]) -> [f64; 2] {
        let [a, b, c, d, e, f] = self.0;
        [a * x + b * y + c, d * x + e * y + f]
    }

    /// How many pixels of the image one pixel of the source covers along a side (the square
    /// root of the area ratio; placements scale evenly).
    pub(crate) fn scale(self) -> f64 {
        let [a, b, _, d, e, _] = self.0;
        (a * e - b * d).abs().sqrt()
    }

    pub(crate) fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    /// The rows as the evaluator reads them.
    pub(crate) fn rows(self) -> [f32; 6] {
        self.0.map(|value| value as f32)
    }

    /// The bounds of `bounds` mapped, widened by a pixel: the four corners' extent, and a pixel
    /// for the antialiased fringe, which in the image is never wider than half a pixel of it.
    pub(crate) fn bounds(self, Bounds([x0, y0, x1, y1]): Bounds) -> Bounds {
        let corners = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
        let mut out = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for [x, y] in corners {
            let [px, py] = self.apply([f64::from(x), f64::from(y)]);
            out = [
                out[0].min(px),
                out[1].min(py),
                out[2].max(px),
                out[3].max(py),
            ];
        }
        Bounds([out[0] - 1.0, out[1] - 1.0, out[2] + 1.0, out[3] + 1.0].map(|v| v as f32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
    }

    #[test]
    fn maps_compose_invert_and_keep_the_identity_exact() {
        let turned = Placement::around([10.0, 20.0], 3.0, 0.7, [true, false]).expect("map");
        let moved = Placement::moved([4.0, -2.5]).expect("map");
        let both = Affine::of(turned).then(Affine::of(moved));
        let p = [7.25, -3.0];
        let expected = moved.apply(turned.apply([7.25, -3.0]));
        let [x, y] = both.apply(p);
        assert!((x - f64::from(expected[0])).abs() < 1e-4);
        assert!((y - f64::from(expected[1])).abs() < 1e-4);
        assert!(close(both.inverse().apply(both.apply(p)), p));
        assert!((both.scale() - 3.0).abs() < 1e-6);
        assert!(Affine::of(Placement::IDENTITY).is_identity());
        assert_eq!(Affine::IDENTITY.rows(), [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let bounds = Affine::of(moved).bounds(Bounds([0.0, 0.0, 2.0, 1.0]));
        assert_eq!(bounds, Bounds([3.0, -3.5, 7.0, -0.5]));
    }
}

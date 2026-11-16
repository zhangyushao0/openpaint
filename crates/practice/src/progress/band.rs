//! The linear algebra of the progress fit (`fit.rs`): a symmetric positive definite system whose
//! unknowns are a skill's practice days, coupled only within a band of each other, and two more
//! (the levels' difficulties) coupled with all of them, a border.
//!
//! Whole-History Rating (Coulom 2008) solves one player's Newton step on a tridiagonal Hessian by
//! its LU decomposition (his Appendix B) and reads the posterior's variances off the inverse's
//! diagonal and sub-diagonal (B.2). Here a blind pair links the two days of its copies, which may
//! lie weeks apart, so the days' block is a band rather than three diagonals: it is factored by
//! Cholesky over the band (the tridiagonal its width one), the inverse's entries within the band
//! found by Takahashi's recurrence, and the border folded in by its Schur complement.

/// A symmetric matrix of `n` unknowns coupled within `width` of each other, held as its lower
/// band: entry `(i, i - k)` at `i * (width + 1) + k`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Band {
    n: usize,
    width: usize,
    lower: Vec<f64>,
}

impl Band {
    /// `n` unknowns coupled within `width`, every entry naught.
    pub(crate) fn new(n: usize, width: usize) -> Self {
        Self {
            n,
            width,
            lower: vec![0.0; n * (width + 1)],
        }
    }

    pub(crate) const fn len(&self) -> usize {
        self.n
    }

    /// Entry `(i, j)`, naught outside the band.
    pub(crate) fn get(&self, i: usize, j: usize) -> f64 {
        let (i, j) = if i >= j { (i, j) } else { (j, i) };
        if i - j > self.width {
            return 0.0;
        }
        self.lower[i * (self.width + 1) + (i - j)]
    }

    fn set(&mut self, i: usize, j: usize, value: f64) {
        self.lower[i * (self.width + 1) + (i - j)] = value;
    }

    /// Adds `value` at `(i, j)` and `(j, i)`; `i` and `j` lie within the band.
    pub(crate) fn add(&mut self, i: usize, j: usize, value: f64) {
        let (i, j) = if i >= j { (i, j) } else { (j, i) };
        debug_assert!(
            i - j <= self.width,
            "({i}, {j}) outside a band of {}",
            self.width
        );
        self.lower[i * (self.width + 1) + (i - j)] += value;
    }

    /// The first column of the band that row `i` reaches.
    fn from(&self, i: usize) -> usize {
        i.saturating_sub(self.width)
    }

    /// Its Cholesky factor `L` (`A = L Lᵀ`, lower, within the same band); none when it is not
    /// positive definite.
    fn cholesky(&self) -> Option<Self> {
        let mut l = Self::new(self.n, self.width);
        for i in 0..self.n {
            for j in self.from(i)..=i {
                let mut sum = self.get(i, j);
                for k in self.from(i).max(self.from(j))..j {
                    sum -= l.get(i, k) * l.get(j, k);
                }
                if i == j {
                    if sum <= 0.0 || !sum.is_finite() {
                        return None;
                    }
                    l.set(i, i, sum.sqrt());
                } else {
                    l.set(i, j, sum / l.get(j, j));
                }
            }
        }
        Some(l)
    }

    /// `x` with `L Lᵀ x = b`, it the factor `L`.
    fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = self.n;
        let mut y = b.to_vec();
        for i in 0..n {
            let from = self.from(i);
            let before: f64 = (y[from..i].iter().enumerate())
                .map(|(at, value)| self.get(i, from + at) * value)
                .sum();
            y[i] = (y[i] - before) / self.get(i, i);
        }
        for i in (0..n).rev() {
            let last = n.min(i + self.width + 1);
            let after: f64 = (y[i + 1..last].iter().enumerate())
                .map(|(at, value)| self.get(i + 1 + at, i) * value)
                .sum();
            y[i] = (y[i] - after) / self.get(i, i);
        }
        y
    }

    /// The entries of `(L Lᵀ)⁻¹` within the band, it the factor `L` (Takahashi's recurrence from
    /// `Lᵀ Z = L⁻¹`, the last row first).
    fn inverse_band(&self) -> Self {
        let (n, width) = (self.n, self.width);
        let mut z = Self::new(n, width);
        for j in (0..n).rev() {
            let last = (n - 1).min(j + width);
            for i in (j..=last).rev() {
                let mut sum = if i == j { 1.0 / self.get(j, j) } else { 0.0 };
                for k in j + 1..=last {
                    sum -= self.get(k, j) * z.get(k, i);
                }
                z.set(i, j, sum / self.get(j, j));
            }
        }
        z
    }
}

/// The whole system: the days' band `A`, the border `B` (each day's coupling with the two levels)
/// and the corner `C` (the levels' own).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct System {
    pub(crate) band: Band,
    pub(crate) border: Vec<[f64; 2]>,
    pub(crate) corner: [[f64; 2]; 2],
}

/// The system factored: `A`'s Cholesky factor, `W = A⁻¹ B` and the inverse of the Schur
/// complement `S = C − Bᵀ W`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Factored {
    l: Band,
    border: Vec<[f64; 2]>,
    w: Vec<[f64; 2]>,
    schur: [[f64; 2]; 2],
}

impl System {
    /// `n` days coupled within `width`, every entry naught.
    pub(crate) fn new(n: usize, width: usize) -> Self {
        Self {
            band: Band::new(n, width),
            border: vec![[0.0; 2]; n],
            corner: [[0.0; 2]; 2],
        }
    }

    /// It factored; none when it is not positive definite.
    pub(crate) fn factor(&self) -> Option<Factored> {
        let l = self.band.cholesky()?;
        let column = |k: usize| {
            let b: Vec<f64> = self.border.iter().map(|row| row[k]).collect();
            l.solve(&b)
        };
        let (w0, w1) = (column(0), column(1));
        let w: Vec<[f64; 2]> = w0.into_iter().zip(w1).map(|(a, b)| [a, b]).collect();
        let mut s = self.corner;
        for (row, wi) in self.border.iter().zip(&w) {
            for a in 0..2 {
                for b in 0..2 {
                    s[a][b] -= row[a] * wi[b];
                }
            }
        }
        let det = s[0][0] * s[1][1] - s[0][1] * s[1][0];
        if det <= 0.0 || s[0][0] <= 0.0 || !det.is_finite() {
            return None;
        }
        let schur = [
            [s[1][1] / det, -s[0][1] / det],
            [-s[1][0] / det, s[0][0] / det],
        ];
        Some(Factored {
            l,
            border: self.border.clone(),
            w,
            schur,
        })
    }
}

impl Factored {
    /// `(x, y)` with `A x + B y = g`, `Bᵀ x + C y = h`.
    pub(crate) fn solve(&self, g: &[f64], h: [f64; 2]) -> (Vec<f64>, [f64; 2]) {
        let z = self.l.solve(g);
        let mut rest = h;
        for (row, zi) in self.border.iter().zip(&z) {
            rest[0] -= row[0] * zi;
            rest[1] -= row[1] * zi;
        }
        let y = times(self.schur, rest);
        let x = (z.iter().zip(&self.w))
            .map(|(zi, wi)| zi - wi[0] * y[0] - wi[1] * y[1])
            .collect();
        (x, y)
    }

    /// `uᵀ Σ u` for `u` naught but at the days it names, `Σ` the inverse of the whole system:
    /// the posterior variance of that combination of days.
    pub(crate) fn variance(&self, u: &[(usize, f64)]) -> f64 {
        let n = self.l.len();
        let mut dense = vec![0.0; n];
        for (day, weight) in u {
            dense[*day] += weight;
        }
        let z = self.l.solve(&dense);
        let own: f64 = dense.iter().zip(&z).map(|(a, b)| a * b).sum();
        let mut across = [0.0; 2];
        for (day, weight) in u {
            across[0] += weight * self.w[*day][0];
            across[1] += weight * self.w[*day][1];
        }
        own + quadratic(self.schur, across)
    }

    /// For every day `i`, the variance of its ability less the first day's: `Σ_ii + Σ_00 −
    /// 2 Σ_i0`, from the band of `A⁻¹`, its first column and the border's share.
    pub(crate) fn spread_from_first(&self) -> Vec<f64> {
        let n = self.l.len();
        if n == 0 {
            return Vec::new();
        }
        let z = self.l.inverse_band();
        let mut first = vec![0.0; n];
        first[0] = 1.0;
        let column = self.l.solve(&first);
        (0..n)
            .map(|i| {
                let own = z.get(i, i) + z.get(0, 0) - 2.0 * column[i];
                let d = [self.w[i][0] - self.w[0][0], self.w[i][1] - self.w[0][1]];
                (own + quadratic(self.schur, d)).max(0.0)
            })
            .collect()
    }
}

fn times(m: [[f64; 2]; 2], v: [f64; 2]) -> [f64; 2] {
    [
        m[0][0] * v[0] + m[0][1] * v[1],
        m[1][0] * v[0] + m[1][1] * v[1],
    ]
}

fn quadratic(m: [[f64; 2]; 2], v: [f64; 2]) -> f64 {
    let mv = times(m, v);
    v[0] * mv[0] + v[1] * mv[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dense inverse by Gauss-Jordan, for checking.
    fn inverse(mut a: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
        let n = a.len();
        let mut inv: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n).map(|j| f64::from(u8::from(i == j))).collect())
            .collect();
        for c in 0..n {
            let p = a[c][c];
            for j in 0..n {
                a[c][j] /= p;
                inv[c][j] /= p;
            }
            for r in 0..n {
                if r != c {
                    let f = a[r][c];
                    for j in 0..n {
                        a[r][j] -= f * a[c][j];
                        inv[r][j] -= f * inv[c][j];
                    }
                }
            }
        }
        inv
    }

    /// A chain of 7 days, a link from day 1 to day 4 (width 3), a border on days 2, 5, 6.
    fn system() -> (System, Vec<Vec<f64>>) {
        let (n, width) = (7, 3);
        let mut system = System::new(n, width);
        system.band.add(0, 0, 0.25);
        for i in 1..n {
            let v = 1.0 / (0.02 * (i as f64 + 1.0));
            system.band.add(i, i, v);
            system.band.add(i - 1, i - 1, v);
            system.band.add(i, i - 1, -v);
            system.band.add(i, i, 0.3);
        }
        system.band.add(1, 1, 0.2);
        system.band.add(4, 4, 0.2);
        system.band.add(4, 1, -0.2);
        for (day, k, v) in [(2, 0, -0.4), (5, 0, -0.3), (5, 1, -0.3), (6, 1, -0.5)] {
            system.border[day][k] += v;
            system.band.add(day, day, 0.4);
        }
        system.corner = [[2.5, 0.3], [0.3, 2.2]];
        let entry = |i: usize, j: usize| match (i < n, j < n) {
            (true, true) => system.band.get(i, j),
            (true, false) => system.border[i][j - n],
            (false, true) => system.border[j][i - n],
            (false, false) => system.corner[i - n][j - n],
        };
        let dense = (0..n + 2)
            .map(|i| (0..n + 2).map(|j| entry(i, j)).collect())
            .collect();
        (system, dense)
    }

    #[test]
    fn the_band_and_its_border_solve_and_invert_as_the_dense_system_does() {
        let (system, dense) = system();
        let inv = inverse(dense);
        let factored = system.factor().expect("positive definite");
        let g = [0.3, -1.0, 0.5, 2.0, 0.0, -0.7, 1.1];
        let h = [0.4, -0.2];
        let (x, y) = factored.solve(&g, h);
        let rhs: Vec<f64> = g.iter().copied().chain(h).collect();
        for (i, row) in inv.iter().enumerate() {
            let want: f64 = row.iter().zip(&rhs).map(|(a, b)| a * b).sum();
            let got = if i < 7 { x[i] } else { y[i - 7] };
            assert!((want - got).abs() < 1e-9, "{i}: {want} {got}");
        }
        let spread = factored.spread_from_first();
        for i in 0..7 {
            let want = inv[i][i] + inv[0][0] - 2.0 * inv[i][0];
            assert!((want - spread[i]).abs() < 1e-9, "{i}: {want} {}", spread[i]);
            let asked = factored.variance(&[(i, 1.0), (0, -1.0)]);
            assert!((want.max(0.0) - asked.max(0.0)).abs() < 1e-9);
        }
        let far = factored.variance(&[(6, 1.0), (1, -1.0)]);
        let want = inv[6][6] + inv[1][1] - 2.0 * inv[6][1];
        assert!((far - want).abs() < 1e-9);
    }

    #[test]
    fn a_matrix_not_positive_definite_is_refused() {
        let mut system = System::new(2, 1);
        system.band.add(0, 0, 1.0);
        system.band.add(1, 1, 1.0);
        system.band.add(1, 0, 2.0);
        system.corner = [[1.0, 0.0], [0.0, 1.0]];
        assert!(system.factor().is_none());
    }
}

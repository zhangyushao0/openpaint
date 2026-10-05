use super::*;
use openpaint_core::CanvasPoint;

/// Fast turns from the author's library (`stroke-stats`: the largest sagittas above 4 px/ms),
/// `[x, y, pressure, ms]`; the samples with the pressure before them on the chord are Android's
/// resamples.
pub(crate) const FAST_TURNS: [[[f32; 4]; 9]; 4] = [
    [
        [1981.9099, 403.98126, 0.5590282, 0.0],
        [1986.7085, 399.0952, 0.5590282, 1.0],
        [1998.2478, 387.34418, 0.56733, 3.0],
        [2011.1385, 376.10248, 0.5747772, 6.0],
        [2020.7327, 369.95697, 0.5794164, 9.0],
        [2021.3418, 369.90332, 0.5794164, 9.0],
        [2027.4766, 369.35718, 0.5803931, 13.0],
        [2031.9744, 372.50507, 0.581736, 16.0],
        [2033.3167, 377.11957, 0.581736, 17.0],
    ],
    [
        [997.1242, 470.08148, 0.61298984, 0.0],
        [995.7266, 452.53778, 0.61298984, 3.0],
        [994.7249, 439.9538, 0.61408865, 4.0],
        [990.678, 417.1712, 0.6208033, 8.0],
        [984.6828, 402.3321, 0.6219021, 10.0],
        [982.84503, 400.38843, 0.6219021, 11.0],
        [976.8882, 394.088, 0.6309364, 13.0],
        [967.446, 391.53992, 0.6330118, 17.0],
        [960.01794, 393.88092, 0.6330118, 19.0],
    ],
    [
        [1991.053, 266.68457, 0.5424246, 0.0],
        [2010.3363, 245.44995, 0.5424246, 2.0],
        [2014.736, 240.60535, 0.5435234, 3.0],
        [2033.3229, 222.76862, 0.5448663, 6.0],
        [2046.5128, 212.87537, 0.5448663, 9.0],
        [2051.1733, 211.35046, 0.5448663, 11.0],
        [2054.7568, 210.17847, 0.5435234, 12.0],
        [2058.8037, 214.37415, 0.5413258, 16.0],
        [2058.954, 224.56659, 0.5413258, 19.0],
    ],
    [
        [1422.965, 380.1494, 0.5618361, 0.0],
        [1442.1088, 376.92938, 0.5618361, 2.0],
        [1455.0425, 374.7525, 0.56903917, 3.0],
        [1487.8687, 374.7525, 0.5781956, 6.0],
        [1521.1443, 379.99902, 0.5919912, 9.0],
        [1530.4147, 383.03345, 0.5919912, 10.0],
        [1554.5702, 390.9416, 0.5934562, 13.0],
        [1588.5945, 407.42828, 0.59577584, 16.0],
        [1616.7188, 425.8495, 0.59577584, 18.0],
    ],
];

pub(crate) fn samples(rows: &[[f32; 4]]) -> Vec<StrokeSample> {
    rows.iter()
        .map(|[x, y, pressure, ms]| {
            let position = CanvasPoint::new(*x, *y).expect("point");
            StrokeSample::new(position, *pressure, 0.0, 0.0, *ms as u32).expect("sample")
        })
        .collect()
}

fn positions(samples: &[StrokeSample], knots: &[usize]) -> Vec<[f32; 2]> {
    knots.iter().map(|k| point(&samples[*k])).collect()
}

fn spans(points: &[[f32; 2]]) -> Vec<Cubic> {
    (0..points.len().saturating_sub(1).max(1))
        .map(|j| Cubic::span(points, j))
        .collect()
}

fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let ab = sub(b, a);
    let h = dot(ab, ab);
    let t = if h > 0.0 {
        (dot(sub(p, a), ab) / h).clamp(0.0, 1.0)
    } else {
        0.0
    };
    len(sub(p, add(a, scaled(ab, t))))
}

/// The largest distance from the curve to the chord of its piece, over `n` pieces of equal
/// parameter.
fn flattening_error(cubic: &Cubic, n: u32) -> f32 {
    let mut worst: f32 = 0.0;
    for k in 0..n {
        let (t0, t1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
        let (a, b) = (cubic.at(t0), cubic.at(t1));
        for i in 0..=32 {
            let t = t0 + (t1 - t0) * i as f32 / 32.0;
            worst = worst.max(segment_distance(cubic.at(t), a, b));
        }
    }
    worst
}

#[test]
fn resamples_are_no_knots_and_the_rule_is_settled_one_sample_later() {
    for rows in &FAST_TURNS {
        let samples = samples(rows);
        let knots = knots(&samples);
        assert_eq!(knots.first(), Some(&0));
        assert_eq!(knots.last(), Some(&(samples.len() - 1)));
        let skipped: Vec<usize> = (0..samples.len()).filter(|i| !knots.contains(i)).collect();
        assert!(
            !skipped.is_empty(),
            "every window holds a resample: {knots:?}"
        );
        for i in skipped {
            assert_eq!(samples[i].pressure(), samples[i - 1].pressure());
            assert!(knots.contains(&(i - 1)) && knots.contains(&(i + 1)));
        }
        // A prefix decides every sample but its last the same way.
        for end in 2..=samples.len() {
            let prefix = super::knots(&samples[..end]);
            let settled: Vec<usize> = knots.iter().copied().filter(|k| *k < end - 1).collect();
            assert_eq!(prefix[..prefix.len() - 1], settled[..], "prefix {end}");
        }
    }
    // Moving off the chord, or a new pressure, keeps a sample.
    let mut rows = FAST_TURNS[0];
    rows[1][1] += 0.05;
    assert!(super::knots(&samples(&rows)).contains(&1));
    let mut rows = FAST_TURNS[0];
    rows[1][2] = 0.6;
    assert!(super::knots(&samples(&rows)).contains(&1));
}

#[test]
fn the_curve_passes_through_its_knots_smoothly() {
    for rows in &FAST_TURNS {
        let samples = samples(rows);
        let points = positions(&samples, &knots(&samples));
        let spans = spans(&points);
        for (j, span) in spans.iter().enumerate() {
            assert_eq!(span.0[0], points[j]);
            assert_eq!(span.0[3], points[j + 1]);
            assert_eq!(span.at(0.0), points[j]);
            let end = span.at(1.0);
            assert!(len(sub(end, points[j + 1])) < 1e-3, "{end:?}");
        }
        for pair in spans.windows(2) {
            let [before, after] = [pair[0].0, pair[1].0];
            let (u, v) = (sub(before[3], before[2]), sub(after[1], after[0]));
            let angle = cross(u, v).atan2(dot(u, v)).abs();
            assert!(angle < 1e-3, "a kink of {angle} rad at a knot");
        }
    }
}

#[test]
fn flattening_stays_within_the_tolerance_at_every_scale() {
    for rows in &FAST_TURNS {
        let samples = samples(rows);
        for span in spans(&positions(&samples, &knots(&samples))) {
            for scale in [1.0, 2.0, 4.0, 8.0] {
                let n = pieces(span.flatness(), span.bend(), scale);
                let error = flattening_error(&span, n) * scale;
                assert!(
                    error <= TOLERANCE * 1.01,
                    "{n} pieces at {scale}x err {error}"
                );
                if n == 1 {
                    assert!(
                        span.flatness() * scale <= TOLERANCE || span.bend() * scale <= TOLERANCE
                    );
                }
            }
        }
    }
}

#[test]
fn a_straight_span_is_one_piece_at_every_scale() {
    let points = [[3.0, 4.0], [40.0, 4.0]];
    let span = Cubic::span(&points, 0);
    assert_eq!(span.0[0], points[0]);
    assert_eq!(span.0[3], points[1]);
    for scale in [1.0, 2.0, 4.0, 8.0, 64.0] {
        assert_eq!(pieces(span.flatness(), span.bend(), scale), 1);
    }
    // Collinear knots unevenly spaced, and a diagonal.
    let line = [
        [0.0, 0.0],
        [1.0, 2.0],
        [7.0, 14.0],
        [8.0, 16.0],
        [30.0, 60.0],
    ];
    for span in spans(&line) {
        assert!(span.flatness() * 8.0 <= TOLERANCE, "{}", span.flatness());
    }
    // A single knot is a point.
    let dot = Cubic::span(&[[5.0, 6.0]], 0);
    assert_eq!(dot.0, [[5.0, 6.0]; 4]);
    assert_eq!(pieces(dot.flatness(), dot.bend(), 8.0), 1);
}

#[test]
fn reversals_and_near_coincident_knots_neither_loop_nor_overshoot() {
    let cases: [[[f32; 2]; 4]; 6] = [
        // A hairpin: back along the way it came.
        [[0.0, 0.0], [40.0, 0.0], [0.0, 1.0], [40.0, 2.0]],
        // A sharp reversal after a long span, into a short one.
        [[0.0, 0.0], [90.0, 0.0], [88.0, 0.5], [0.0, 1.0]],
        // Knots almost on top of each other before, inside and after.
        [[10.0, 10.0], [10.001, 10.0], [30.0, 12.0], [30.0, 12.0005]],
        [[0.0, 0.0], [20.0, 0.0], [20.002, 0.001], [40.0, 0.0]],
        // A long span between two short ones turning hard.
        [[0.0, 0.0], [1.0, 0.0], [60.0, 30.0], [60.5, 31.0]],
        // A right angle.
        [[0.0, 0.0], [30.0, 0.0], [30.0, 30.0], [30.0, 60.0]],
    ];
    for points in cases {
        let span = Cubic::centripetal(points);
        let (a, b) = (points[1], points[2]);
        let chord = len(sub(b, a)).max(1e-6);
        let curve: Vec<[f32; 2]> = (0..=256).map(|i| span.at(i as f32 / 256.0)).collect();
        let off = curve
            .iter()
            .map(|p| segment_distance(*p, a, b))
            .fold(0.0, f32::max);
        assert!(
            off <= 0.25 * chord + 1e-4,
            "{points:?}: {off} off a chord of {chord}"
        );
        // No loop: no two pieces of the flattened curve cross.
        for i in 0..curve.len() - 1 {
            for k in i + 2..curve.len() - 1 {
                assert!(
                    !crosses([curve[i], curve[i + 1]], [curve[k], curve[k + 1]]),
                    "{points:?} loops at {i} and {k}"
                );
            }
        }
        assert!(span.0.iter().flatten().all(|v| v.is_finite()));
    }
}

fn crosses([a, b]: [[f32; 2]; 2], [c, d]: [[f32; 2]; 2]) -> bool {
    let side = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| cross(sub(q, p), sub(r, p));
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

#[test]
fn pieces_grow_with_the_square_root_of_the_scale_and_stop_at_the_cap() {
    let span = Cubic::centripetal([[0.0, 0.0], [30.0, 0.0], [60.0, 20.0], [70.0, 60.0]]);
    let counts: Vec<u32> = [1.0, 2.0, 4.0, 8.0]
        .map(|scale| pieces(span.flatness(), span.bend(), scale))
        .into();
    assert!(
        counts.windows(2).all(|pair| pair[1] > pair[0]),
        "{counts:?}"
    );
    let ratio = counts[3] as f32 / counts[0] as f32;
    assert!((2.0..=3.5).contains(&ratio), "{counts:?}");
    assert_eq!(pieces(1.0e6, 1.0e6, 8.0), MAX_PIECES);
}

#[test]
fn predicted_samples_past_the_last_two_are_no_knots() {
    // Injected on the tablet at 125 Hz: Android adds a sample 1 to 2 ms after each one, on the
    // line through it and the one before (`device-input pen`, S34).
    let rows = [
        [1146.52, 486.80, 0.8, 85.0],
        [1172.61, 469.96, 0.8, 102.0],
        [1222.61, 436.26, 0.8, 118.0],
        [1229.00, 431.95, 0.8, 120.0],
        [1276.96, 409.09, 0.8, 135.0],
        [1282.43, 406.35, 0.8, 136.0],
        [1334.57, 390.61, 0.8, 152.0],
    ];
    assert_eq!(knots(&samples(&rows)), [0, 1, 2, 4, 6]);
    // Before the line it lies on, or with a new pressure, a sample stays a knot.
    let mut back = rows;
    back[3] = [1216.22, 440.57, 0.8, 120.0];
    assert!(knots(&samples(&back)).contains(&3));
    let mut pressed = rows;
    pressed[3][2] = 0.81;
    assert!(knots(&samples(&pressed)).contains(&3));
}

//! The store's upgrade of a value scale as 1.5.0 to 1.14.0 stored it (S97): kind (6, tone), its
//! cells (5 to 7) its seed's first draw, its ends the draws after.

use openpaint_practice::{Arrangement, Original, Plan, Random, ScaleTone};

use super::*;

/// A record of a combination and a value scale as this version writes it.
fn session() -> SessionRecord {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Areas(Arrangement::Blocks), 3),
            Exercise::generated(Kind::Scale(ScaleTone::Hue, 9), 4),
        ],
        warm_up: 0,
    };
    let mut session = SessionRecord::begun(9, (2, 4), (11, 12), plan);
    let scale = [(Metric::ScaleOff, 0.03), (Metric::ScaleReversed, 2.0)];
    (session.results).push(ExerciseResult::first(1, 14, 9, scale.to_vec()));
    session.time.push(Span { from: 12, to: 300 });
    session
}

/// The scale the old generator made of `seed` and `tone`: its cells, hue, saturation, dark, light.
fn as_it_was(seed: u64, tone: u8) -> (usize, f64, f64, f64, f64) {
    let mut random = Random::new(seed);
    let cells = 5 + ((random.unit() * 3.0) as usize).min(2);
    let (hue, saturation) = if tone == 0 {
        (0.0, 0.0)
    } else {
        (random.range(0.0, 360.0), random.range(0.2, 0.55))
    };
    (
        cells,
        hue,
        saturation,
        random.range(0.12, 0.3),
        random.range(0.78, 0.95),
    )
}

/// Read as this version keeps it, owing the upgrade its scale: the same scale per seed as the old
/// generator gave, so its strokes lie in its cells; written again as this version writes it, it
/// owes nothing.
#[test]
fn a_value_scale_as_1_14_0_stored_it_reads_as_the_same_scale() {
    let session = session();
    for (seed, tone) in (0..60u64).zip([0u8, 1].into_iter().cycle()) {
        let mut old = encode_session(&session);
        // The second exercise's kind, variant and seed, after the fixed fields, the count and the
        // first exercise.
        let at = 1 + 4 + 8 * 4 + 4 + 4 + EXERCISE_BYTES;
        old[at..at + 2].copy_from_slice(&[6, tone]);
        old[at + 2..at + 10].copy_from_slice(&seed.to_le_bytes());
        let (upgraded, owed) = decode_session(&practice_key(9), &old).expect("read");
        let scales = Owed {
            time: false,
            scales: true,
        };
        assert_eq!(owed, scales);
        let Original::Scale(scale) = upgraded.exercises[1].original() else {
            panic!("a scale");
        };
        let now = (scale.cells, scale.hue, scale.saturation);
        assert_eq!((now, scale.dark, scale.light), {
            let (cells, hue, saturation, dark, light) = as_it_was(seed, tone);
            ((cells, hue, saturation), dark, light)
        });
        let cells = u8::try_from(scale.cells).expect("cells");
        let kind = Kind::Scale(ScaleTone::ALL[usize::from(tone)], cells);
        assert_eq!(upgraded.exercises[1].kind, kind);
        assert_eq!(upgraded.results, session.results, "the rest as it was");
        let again = encode_session(&upgraded);
        let read = decode_session(&practice_key(9), &again);
        assert_eq!(read, Ok((upgraded, Owed::default())));
    }
}

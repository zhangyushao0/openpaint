use super::*;

fn phases(batches: &[MotionBatch]) -> Vec<MotionPhase> {
    batches.iter().map(|batch| batch.phase).collect()
}

#[test]
fn a_tap_goes_down_and_up_at_one_spot() {
    let batches = tap(Tool::Finger, [10.0, 20.0], 100);
    assert_eq!(phases(&batches), [MotionPhase::Down, MotionPhase::Up]);
    let up = &batches[1].pointers[0];
    assert_eq!(up.tool, ToolKind::Finger);
    assert_eq!(up.samples[0].position, [10.0, 20.0]);
    assert_eq!(end_ms(&batches, 0), 100 + TAP_MS);
}

#[test]
fn a_path_is_resampled_and_batched_per_frame() {
    let batches = path(Tool::Stylus, &[[0.0, 0.0, 0.2], [40.0, 0.0, 0.6]], 1000);
    let samples: Vec<PointerSample> = batches
        .iter()
        .flat_map(|batch| batch.pointers[0].samples.clone())
        .collect();
    assert_eq!(samples.len(), 11, "every 4 px over 40 px, both ends");
    assert_eq!(samples[5].position, [20.0, 0.0]);
    assert!((samples[5].pressure - 0.4).abs() < 1e-6);
    assert_eq!(samples[10].time_ms, 1000 + 10 * SAMPLE_MS);
    assert_eq!(batches.first().map(|b| b.phase), Some(MotionPhase::Down));
    assert_eq!(batches.last().map(|b| b.phase), Some(MotionPhase::Up));
    assert!(
        batches
            .iter()
            .all(|b| b.pointers[0].samples.len() <= PER_BATCH)
    );
}

#[test]
fn a_lift_ends_the_stroke_with_one_zero_pressure_sample() {
    let batches = stroke(
        (&[[0.0, 0.0, 0.5], [8.0, 0.0, 0.5]], false, 0),
        Some([12.0, 3.0]),
        None,
        0,
    );
    let up = batches.last().expect("up");
    assert_eq!(up.phase, MotionPhase::Up);
    assert_eq!(up.pointers[0].samples.len(), 1);
    let lift = up.pointers[0].samples[0];
    assert_eq!((lift.position, lift.pressure), ([12.0, 3.0], 0.0));
    assert_eq!(lift.time_ms, 3 * SAMPLE_MS);
}

#[test]
fn a_held_stroke_rests_at_its_end_before_the_lift() {
    let batches = stroke(
        (&[[0.0, 0.0, 0.5], [8.0, 0.0, 0.5]], false, 600),
        None,
        None,
        0,
    );
    let samples: Vec<PointerSample> = batches
        .iter()
        .flat_map(|batch| batch.pointers[0].samples.clone())
        .collect();
    assert_eq!(
        samples.len(),
        3 + 150,
        "three samples, then one every 4 ms for 600 ms"
    );
    assert!(samples[2..].iter().all(|s| s.position == [8.0, 0.0]));
    assert_eq!(samples.last().map(|s| s.time_ms), Some(8 + 600));
    assert_eq!(batches.last().map(|b| b.phase), Some(MotionPhase::Up));
}

#[test]
fn a_palm_lands_before_the_pen_or_during_the_stroke() {
    let line = [[0.0, 0.0, 0.5], [40.0, 0.0, 0.5]];
    let hand = |lands| Palm {
        at: [5.0, 5.0],
        lands,
    };
    let who = |batches: &[MotionBatch]| -> Vec<(i32, MotionPhase)> {
        batches
            .iter()
            .map(|b| (b.pointers[0].id, b.phase))
            .collect()
    };
    let before = who(&stroke(
        (&line, false, 0),
        None,
        Some(hand(Lands::Before)),
        0,
    ));
    assert_eq!(before.first(), Some(&(PALM_ID, MotionPhase::Down)));
    assert_eq!(before[1], (0, MotionPhase::Down));
    assert_eq!(before.last(), Some(&(PALM_ID, MotionPhase::Up)));
    let during = who(&stroke(
        (&line, false, 0),
        None,
        Some(hand(Lands::During)),
        0,
    ));
    assert_eq!(
        during[..2],
        [(0, MotionPhase::Down), (PALM_ID, MotionPhase::Down)]
    );
    let n = during.len();
    assert_eq!(
        during[n - 2..],
        [(PALM_ID, MotionPhase::Up), (0, MotionPhase::Up)]
    );
}

#[test]
fn fingers_land_one_by_one_move_together_and_lift_with_the_last_move() {
    let batches = fingers(
        &[[0.0, 0.0], [100.0, 0.0]],
        &[[0.0, 0.0], [200.0, 0.0]],
        64,
        10,
    );
    let phases = phases(&batches);
    assert_eq!(
        phases[..2],
        [MotionPhase::Down, MotionPhase::PointerDown { index: 1 }]
    );
    let n = phases.len();
    assert_eq!(
        phases[n - 2..],
        [MotionPhase::PointerUp { index: 1 }, MotionPhase::Up]
    );
    assert_eq!(
        n,
        2 + 3 + 2,
        "four frames of 16 ms: three moves, the last with the lift"
    );
    let lift = &batches[n - 2];
    assert_eq!(lift.pointers.len(), 2);
    let last = lift.pointers[1].samples.last().expect("sample");
    assert_eq!((last.position, last.time_ms), ([200.0, 0.0], 10 + 64));
    assert_eq!(batches[n - 1].pointers[0].samples[0].position, [0.0, 0.0]);
}

#[test]
fn a_press_moves_by_time_and_holds_where_keys_share_a_place() {
    let keys = [
        [0.0, 0.0, 0.0],
        [40.0, 0.0, 40.0],
        [40.0, 0.0, 440.0],
        [80.0, 0.0, 480.0],
    ];
    let samples: Vec<PointerSample> = press(Tool::Finger, &keys, 1.0, 100)
        .iter()
        .flat_map(|batch| batch.pointers[0].samples.clone())
        .collect();
    assert_eq!(samples.len(), 121, "every 4 ms over 480 ms");
    assert_eq!(samples[5].position, [20.0, 0.0]);
    assert!(
        samples[10..=110].iter().all(|s| s.position == [40.0, 0.0]),
        "held"
    );
    assert_eq!(
        (samples[120].position, samples[120].time_ms),
        ([80.0, 0.0], 580)
    );
}

#[test]
fn a_single_point_is_a_dot() {
    let batches = path(Tool::Stylus, &[[5.0, 5.0, 1.0]], 0);
    assert_eq!(phases(&batches), [MotionPhase::Down, MotionPhase::Up]);
}

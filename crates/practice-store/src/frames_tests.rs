//! An old copy's move (S99): the picture's place in its square onto the frame of its shape, every
//! point at the same share of the picture.

use openpaint_practice::{Frame, Sheet};

use super::Move;

#[test]
fn a_picture_s_place_in_its_square_goes_onto_its_frame_point_for_point() {
    for size in [[480, 300], [300, 400], [500, 500], [1, 2048]] {
        let square = Sheet::of(&[None; 4]).pairs[3].copy;
        let frame = Sheet::of(&[None, None, None, Some(size)]).pairs[3].copy;
        let by = Move::of(square, frame, size);
        let [w, h] = size.map(f64::from);
        let long = w.max(h);
        // As 1.16.0 laid it: 90 % of the square along the longer side, centred.
        let old = [w, h].map(|side| 360.0 * side / long);
        let from = [0, 1].map(|axis| square.origin[axis] + (400.0 - old[axis]) / 2.0);
        for share in [[0.0, 0.0], [1.0, 1.0], [0.25, 0.6], [1.0, 0.0]] {
            let p = [0, 1].map(|axis| from[axis] + share[axis] * old[axis]);
            let q = by.at(p);
            let expected = [0, 1].map(|axis| frame.origin[axis] + share[axis] * frame.size[axis]);
            for axis in 0..2 {
                assert!(
                    (q[axis] - expected[axis]).abs() < 1e-9,
                    "{size:?} {share:?}"
                );
            }
            let back = by.back(q);
            assert!((back[0] - p[0]).abs() < 1e-9 && (back[1] - p[1]).abs() < 1e-9);
        }
    }
}

#[test]
fn a_stroke_belongs_to_the_square_it_began_in_or_near() {
    let square = Frame::square([520.0, 80.0], 400.0);
    let by = Move::of(square, square, [400, 400]);
    assert!(
        by.square.holds([500.0, 100.0]),
        "within a tenth of the frame"
    );
    assert!(!by.square.holds([470.0, 100.0]));
}

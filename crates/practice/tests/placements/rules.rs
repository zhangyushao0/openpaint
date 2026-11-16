//! A placement's rules, seed after seed: its plan's (pieces on the grid, apart, no column shared,
//! heights of the level's and not all one; at 熟练 one turned and one round) and its sight's
//! (everything inside the frame, the horizon above the pieces, two or three points as the level
//! says, the vanishing points at sane distances), every piece carried.

use openpaint_practice::{FAR, Footprint, Kind, Placement, PlacementTask, Point, distance};

use crate::laid;

/// Whether `p` lies inside the frame by `margin`.
fn inside(p: Point, margin: f64) -> bool {
    p.iter().all(|v| (margin..=1.0 - margin).contains(v))
}

#[test]
fn every_plan_keeps_its_rules_seed_after_seed() {
    for task in laid() {
        let plan = &task.plan;
        let (count, cells, heights): (usize, usize, &[f64]) = match task.placement {
            Placement::TwoBoxes => (2, 4, &[0.5, 1.0]),
            Placement::ThreeBoxes => (3, 4, &[0.5, 1.0]),
            Placement::Turned => (4, 4, &[1.0 / 3.0, 2.0 / 3.0, 1.0, 4.0 / 3.0]),
            Placement::Above => (5, 8, &[0.5, 1.0, 1.5]),
        };
        assert_eq!((plan.pieces.len(), plan.cells), (count, cells), "{task:?}");
        let first = plan.pieces[0].height;
        assert!(plan.pieces.iter().any(|piece| piece.height != first));
        assert!(
            plan.pieces
                .iter()
                .all(|piece| heights.contains(&piece.height))
        );
        let shapes = |turned: bool| {
            (plan.pieces.iter())
                .filter(|piece| match piece.footprint {
                    Footprint::Turned { .. } => turned,
                    Footprint::Round { .. } => !turned,
                    Footprint::Cells { .. } => false,
                })
                .count()
        };
        let turned = usize::from(task.placement == Placement::Turned);
        assert_eq!((shapes(true), shapes(false)), (turned, turned));
        for (k, piece) in plan.pieces.iter().enumerate() {
            let [[c0, c1], [r0, r1]] = piece.footprint.cells();
            assert!(c0 < c1 && c1 <= cells && r0 < r1 && r1 <= cells);
            // Every corner on the ground; a box's on its grid's lines.
            for [u, v] in plan.corners(piece) {
                assert!((0.0..=plan.ground[0] + 1e-12).contains(&u));
                assert!((0.0..=plan.ground[1] + 1e-12).contains(&v));
            }
            for other in &plan.pieces[k + 1..] {
                let [[d0, d1], [s0, s1]] = other.footprint.cells();
                assert!(c1 <= d0 || d1 <= c0, "a column shared");
                if c1 == d0 || d1 == c0 {
                    assert!(r1 < s0 || s1 < r0, "side by side without a row between");
                }
            }
        }
    }
}

#[test]
fn every_sight_keeps_its_rules_and_every_piece_is_carried() {
    for task in laid() {
        let down = task.placement == Placement::Above;
        assert_eq!(task.vanishing[2].is_some(), down, "three points at 稳 only");
        assert_eq!(
            task.along.is_some(),
            down,
            "the unit along the edge at 稳 only"
        );
        assert!(inside([0.5, task.horizon], 0.06));
        let highest = (0..task.plan.pieces.len())
            .flat_map(|k| task.tops(k))
            .chain(task.ground)
            .fold(f64::INFINITY, |least, p| least.min(p[1]));
        assert!(task.horizon < highest, "the eye above every piece");
        for k in 0..task.plan.pieces.len() {
            for p in task.feet(k).into_iter().chain(task.tops(k)) {
                assert!(inside(p, 0.06), "{p:?}");
            }
        }
        assert!(task.ground.iter().all(|p| inside(*p, 0.06)));
        assert!(inside(task.upright[1], 0.06));
        for v in task.vanishing[..2].iter() {
            let v = v.expect("the ground's vanishing points");
            assert!(distance(v, [0.5, 0.5]) <= FAR);
            assert!((v[1] - task.horizon).abs() < 1e-9, "on the horizon");
        }
        let built = &task.construction;
        assert_eq!(built.carried.len(), task.plan.pieces.len());
        assert_eq!(built.tops.len(), task.plan.pieces.len());
        let lines = 2 + 2 * (task.plan.cells - 1);
        assert_eq!(
            built.grid.len(),
            lines,
            "the diagonals and the cells' lines"
        );
        assert!(!task.answer.sides.is_empty() && !task.hidden.is_empty());
        let round = (task.plan.pieces.iter())
            .any(|piece| matches!(piece.footprint, Footprint::Round { .. }));
        assert_eq!(task.answer.ellipses.len(), if round { 2 } else { 0 });
    }
}

#[test]
fn a_placement_is_its_variant_and_seed() {
    for placement in Placement::ALL {
        let a = PlacementTask::generate(placement, 7);
        assert_eq!(a, PlacementTask::generate(placement, 7));
        assert_ne!(a, PlacementTask::generate(placement, 8));
    }
    for (place, placement) in Placement::ALL.iter().enumerate() {
        let kind = Kind::Placement(*placement);
        assert_eq!(kind.code(), (25, place as u8));
        assert_eq!(Kind::of_code(25, place as u8), Some(kind));
    }
    assert_eq!(Kind::of_code(25, 4), None);
}

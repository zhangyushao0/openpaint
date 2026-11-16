//! A placement's answer on its construction (S153): V大's construction, found on the copy's frame
//! from what it gives alone (the ground's corners, the horizon, the upright), lands on the pieces
//! in space as the eye sees them within 1e-7: the ground's diagonals cross at its middle and the
//! halving lines run through it, every footprint's corner and every top's lies where the
//! construction finds it, each carried height's lines meet at the horizon.

use openpaint_practice::{Footprint, PlacementTask, Point, distance, meet};

use crate::laid;

const EXACT: f64 = 1e-7;

/// How far `p` lies off the line through `line`.
fn off(p: Point, [a, b]: [Point; 2]) -> f64 {
    ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs() / distance(a, b)
}

#[test]
fn the_ground_s_diagonals_cross_at_its_middle_and_halve_it() {
    for task in laid() {
        let [width, depth] = task.plan.ground;
        let grid = &task.construction.grid;
        let crossed = meet(grid[0], grid[1]).expect("the diagonals cross");
        let middle = task.at([width / 2.0, depth / 2.0], 0.0);
        assert!(distance(crossed, middle) < EXACT, "{crossed:?} {middle:?}");
        // The cells' lines run through the ground's points they say, the halves through its middle.
        let cells = task.plan.cells;
        for k in 1..cells {
            let share = k as f64 / cells as f64;
            let across = grid[2 * k];
            let away = grid[2 * k + 1];
            assert!(off(task.at([share * width, depth * 0.3], 0.0), across) < EXACT);
            assert!(off(task.at([width * 0.7, share * depth], 0.0), away) < EXACT);
            if 2 * k == cells {
                assert!(off(middle, across) < EXACT && off(middle, away) < EXACT);
            }
        }
    }
}

#[test]
fn every_corner_and_top_lies_where_the_construction_finds_it() {
    for task in laid() {
        let built = &task.construction;
        for (k, piece) in task.plan.pieces.iter().enumerate() {
            let (feet, tops) = (task.feet(k), task.tops(k));
            for (found, exact) in built.footprints[k].iter().zip(feet) {
                assert!(distance(*found, exact) < EXACT, "{task:?}");
            }
            for (found, exact) in built.tops[k].iter().zip(tops) {
                assert!(distance(*found, exact) < EXACT, "{found:?} {exact:?}");
            }
            let carried = &built.carried[k];
            assert!(distance(carried.top, tops[carried.corner]) < EXACT);
            assert!(distance(carried.foot, feet[carried.corner]) < EXACT);
            // The nearest corner carried: the lowest on the frame.
            assert!(feet.iter().all(|p| p[1] <= carried.foot[1] + 1e-12));
            // The two lines that carry a height meet on the horizon (or run along each other).
            if let [from_foot, from_top] = carried.lines[..] {
                assert!(off(carried.foot, from_foot) < EXACT);
                assert!(off(carried.top, from_top) < EXACT);
                if let Some(v) = meet(from_foot, from_top) {
                    assert!((v[1] - task.horizon).abs() < EXACT, "{v:?}");
                }
            }
            if let Footprint::Turned { .. } = piece.footprint {
                assert_eq!(built.runs.len(), 2, "its own vanishing points");
                for [from, to] in &built.runs {
                    assert!(distance(*from, carried.foot) < EXACT);
                    assert!((to[1] - task.horizon).abs() < EXACT);
                }
            }
        }
    }
}

#[test]
fn every_height_on_the_upright_is_the_eye_s_own_at_three_points_too() {
    // Each height on the upright at the near corner, evenly along it at two points, by the
    // cross-ratio at three: the point that height above the near corner, exactly.
    for task in laid() {
        let built = &task.construction;
        let up = |height: f64| task.at([0.0, 0.0], height);
        assert!(distance(task.upright[0], up(0.0)) < EXACT);
        assert!(distance(task.upright[1], up(1.0)) < EXACT);
        let tallest = (task.plan.pieces.iter()).fold(0.0, |most: f64, p| most.max(p.height));
        assert!(distance(built.upright[1], up(tallest)) < EXACT, "{task:?}");
        for (piece, carried) in task.plan.pieces.iter().zip(&built.carried) {
            let from = carried.lines.get(1).map_or(carried.top, |line| line[0]);
            assert!(distance(from, up(piece.height)) < EXACT, "{piece:?}");
        }
    }
}

/// A placement's answer is no plain one: its construction is laid.
#[test]
fn no_seed_tested_falls_back_to_the_plain_placement() {
    for task in laid() {
        let task: &PlacementTask = &task;
        assert!(!task.construction.grid.is_empty());
    }
}

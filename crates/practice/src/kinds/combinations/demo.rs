//! A colour combination's demonstration (`demos/demo.rs`) on a generated example, its pens in
//! their order (S100): Fill frame's overall tone, Big's largest area and each neighbour against
//! it, Small keeping the weak contrast weak, Soft only on the soft edges; then compared.

use crate::colour_areas::areas::Areas;
use crate::colour_areas::areas_generated::{Arrangement, GeneratedAreas};
use crate::demos::demo::{Demo, Drawn, Layout, Marker, On, Script, clearest};
use crate::maths::colour::{Rgb8, delta_e, from_oklab, oklab};
use crate::sessions::plan::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const AREAS_SEED: u64 = 0xDE30_5EED_0005;

/// The areas in `colours`, their edges soft where the picture's are (`soft`) or all hard, as the
/// hard tip lays them.
const fn painted(colours: Vec<Rgb8>, soft: bool) -> Drawn {
    Drawn::Areas { colours, soft }
}

/// A colour combination, painted with its pens in their order (S100): Fill frame with the overall
/// tone; Big for the largest area, then each neighbour against it; Small along the weak contrast's
/// edge, keeping it weak; Soft only on the edges soft in the picture (the copy's edges hard until
/// then, as the hard tip lays them); then Compare.
pub(crate) fn areas() -> Demo {
    // Four or five bands of middle lightness, so each area and the weak contrast read small; some
    // edges soft and some hard, so Soft shows where it belongs and where it does not.
    let generated = clearest(
        (AREAS_SEED, 8),
        |seed| GeneratedAreas::generate(Arrangement::Strata, seed),
        |generated| {
            let areas = &generated.areas;
            let count = areas.count() as f64;
            let light = areas.colours.iter().map(|c| oklab(*c)[0]).sum::<f64>() / count;
            let soft = soft_edges(areas).len();
            let mixed = soft > 0 && soft < areas.neighbours.len();
            -(light - 0.66).abs()
                - if count > 5.0 { 0.2 } else { 0.0 }
                - if mixed { 0.0 } else { 0.5 }
        },
    );
    let areas = &generated.areas;
    let sizes = areas.sizes();
    let (order, weak, last) = painting(areas, &sizes);
    let mut script = Script::new();
    script.given(
        On::Original,
        Marker::Given,
        painted(areas.colours.clone(), true),
    );
    copy_areas(
        &mut script,
        areas,
        (&order, weak, last),
        overall(areas, &sizes),
    );
    let mut compared = vec![(
        On::Copy,
        Marker::Faint,
        painted(areas.colours.clone(), true),
    )];
    for on in [On::Original, On::Copy] {
        for edge in areas.edges(order[0], None) {
            compared.push((on, Marker::Given, Drawn::Line(edge)));
        }
    }
    let mut demo = script.compare(Family::Areas, Layout::Pair, compared);
    demo.areas = Some(generated.clone());
    demo
}

/// The hand's copy of `areas`, a step a pen: Fill frame's `tone`; Big for the areas in `order`, the
/// largest a step of its own, the others the next; Small keeping the `weak` pair weak, its `last`
/// area painted against its partner along their pencilled edge; Soft on the soft edges, pencilled,
/// then softened. Each area a little lighter or darker than the picture's, as a hand copies it.
fn copy_areas(
    script: &mut Script,
    areas: &Areas,
    (order, weak, last): (&[usize], Option<(usize, usize)>, Option<usize>),
    tone: Rgb8,
) {
    let copied: Vec<Rgb8> = (0..areas.count())
        .map(|a| {
            let [l, x, y] = oklab(areas.colours[a]);
            let off = if a % 2 == 0 { 0.012 } else { -0.01 };
            from_oklab([l + off, x, y])
        })
        .collect();
    let mut shown = vec![tone; areas.count()];
    script.step();
    script.draw(On::Copy, Marker::Ink, painted(shown.clone(), false));
    // Big: the largest in a step of its own, then its neighbours two at a time.
    let (largest, neighbours) = order.split_at(order.len().min(1));
    for (step, turns) in [(largest, 1), (neighbours, 2)] {
        script.step();
        for turn in step.chunks(turns) {
            turn.iter().for_each(|area| shown[*area] = copied[*area]);
            script.draw(On::Copy, Marker::Ink, painted(shown.clone(), false));
        }
    }
    script.step();
    if let (Some((a, b)), Some(last)) = (weak, last) {
        let edges = areas.edges(a, Some(b)).into_iter();
        script.together(
            edges
                .map(|e| (On::Copy, Marker::Pencil, Drawn::Line(e)))
                .collect(),
        );
        shown[last] = areas.colours[last];
        script.draw(On::Copy, Marker::Ink, painted(shown.clone(), false));
    }
    script.step();
    let looked: Vec<(On, Marker, Drawn)> = (soft_edges(areas).into_iter())
        .flat_map(|(a, b)| areas.edges(a, Some(b)))
        .map(|edge| (On::Copy, Marker::Pencil, Drawn::Line(edge)))
        .collect();
    script.together(looked);
    script.draw(On::Copy, Marker::Ink, painted(shown, true));
}

/// The neighbouring areas whose shared edge is soft in the picture (either area's edge soft).
fn soft_edges(areas: &Areas) -> Vec<(usize, usize)> {
    (areas.neighbours.iter().copied())
        .filter(|(a, b)| areas.soft[*a].max(areas.soft[*b]) > 0.0)
        .collect()
}

/// The overall tone: the areas' colours by how much of the frame each holds.
fn overall(areas: &Areas, sizes: &[usize]) -> Rgb8 {
    let total: usize = sizes.iter().sum::<usize>().max(1);
    std::array::from_fn(|k| {
        let sum: usize = (0..areas.count())
            .map(|a| usize::from(areas.colours[a][k]) * sizes[a])
            .sum();
        u8::try_from(sum / total).unwrap_or(u8::MAX)
    })
}

/// The areas in the order painted, the largest first (the weak pair's second left out); the
/// weak pair; and that second, painted last against its partner.
fn painting(areas: &Areas, sizes: &[usize]) -> (Vec<usize>, Option<(usize, usize)>, Option<usize>) {
    let weak = (areas.neighbours.iter())
        .min_by(|x, y| {
            let apart = |(a, b): &&(usize, usize)| delta_e(areas.colours[*a], areas.colours[*b]);
            apart(x).total_cmp(&apart(y))
        })
        .copied();
    let mut order: Vec<usize> = (0..areas.count()).collect();
    order.sort_by(|a, b| sizes[*b].cmp(&sizes[*a]));
    let last = weak.map(|(a, b)| {
        let at = |x: usize| order.iter().position(|o| *o == x).unwrap_or(0);
        if at(a) > at(b) { a } else { b }
    });
    let largest = order[0];
    order.retain(|a| Some(*a) != last || *a == largest);
    (order, weak, last)
}

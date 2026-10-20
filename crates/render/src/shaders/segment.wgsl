// A stroke's curve spans (S34): each `Segment` op is one cubic Bézier span between two knots,
// drawn as tapered capsules of equal parameter, as many as `curve.rs` flattens it into at the
// output pixel's footprint (`raster.wgsl`'s `piece_of`). Radius, flow and contact change evenly
// from one knot to the next. A covering stroke keeps the largest coverage of the pieces, an
// accumulating one sums their densities (deposit.wgsl's `piece_density`).

// One straight piece: ends, radii, flows and paper contacts at both ends.
struct Piece {
    a: vec2<f32>,
    b: vec2<f32>,
    radii: vec2<f32>,
    flows: vec2<f32>,
    contacts: vec2<f32>,
}

// The span's chord with the knots' values.
fn chord_piece(op: u32) -> Piece {
    return Piece(
        vec2<f32>(word(op, 0), word(op, 1)),
        vec2<f32>(word(op, 2), word(op, 3)),
        vec2<f32>(word(op, 4), word(op, 5)),
        vec2<f32>(word(op, SEGMENT_FLOW), word(op, SEGMENT_FLOW + 1u)),
        vec2<f32>(word(op, SEGMENT_CONTACT), word(op, SEGMENT_CONTACT + 1u)),
    );
}

// The span's point at parameter `t` (Bernstein form, as `Cubic::at`).
fn span_point(op: u32, t: f32) -> vec2<f32> {
    let s = 1.0 - t;
    let p0 = vec2<f32>(word(op, 0), word(op, 1));
    let p1 = vec2<f32>(word(op, SEGMENT_CONTROLS), word(op, SEGMENT_CONTROLS + 1u));
    let p2 = vec2<f32>(word(op, SEGMENT_CONTROLS + 2u), word(op, SEGMENT_CONTROLS + 3u));
    let p3 = vec2<f32>(word(op, 2), word(op, 3));
    return s * s * s * p0 + 3.0 * s * s * t * p1 + 3.0 * s * t * t * p2 + t * t * t * p3;
}

// Coverage of a pixel whose centre lies `distance` canvas pixels outside the capsule, measured
// in output pixels: it fades in from half an output pixel outside the outline, over one output
// pixel for a hard edge (`edge` 1: box-filtered antialiasing, as sharp at 4x as at 1x) or over
// `1 / edge` canvas pixels for a soft one (smoothed; its look keeps its canvas width). It is
// exactly zero from half an output pixel out, inside the scene's bounds. Then times the flow,
// which changes evenly from `a` to `b`; equal flows give that flow exactly. At scale 1 both
// factors are 1, so the canvas is as before scales. Inside a transform record's frame the
// output pixel is measured in its content's pixels (`foot_scale`), so a scaled stroke's edge
// stays one output pixel wide.
fn piece_coverage(centre: vec2<f32>, piece: Piece, edge: f32) -> f32 {
    let a = piece.a;
    let b = piece.b;
    let distance = capsule_distance(centre, a, b, piece.radii.x, piece.radii.y);
    var ramp = edge;
    if edge < 1.0 {
        ramp = edge * foot_inverse;
    }
    var coverage = clamp((0.5 - distance * foot_scale) * ramp, 0.0, 1.0);
    if edge < 1.0 {
        coverage = coverage * coverage * (3.0 - 2.0 * coverage);
    }
    let ab = b - a;
    let h = dot(ab, ab);
    var t = 0.0;
    if h > 0.0 {
        t = clamp(dot(centre - a, ab) / h, 0.0, 1.0);
    }
    let flow_a = piece.flows.x;
    let flow_b = piece.flows.y;
    return coverage * (flow_a + (flow_b - flow_a) * t);
}

// Signed distance to the convex hull of the disc at `a` (radius `ra`) and the disc at `b`
// (radius `rb`): a capsule whose radius changes along its length. After Inigo Quilez's uneven
// capsule, rotated into the segment's frame.
fn capsule_distance(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, ra: f32, rb: f32) -> f32 {
    let ab = b - a;
    let h = dot(ab, ab);
    let dr = ra - rb;
    if h <= dr * dr {
        // One disc holds the other (or both sit on one point): the hull is the larger disc.
        return min(length(p - a) - ra, length(p - b) - rb);
    }
    let ap = p - a;
    // `ap` in units of the segment length: across (mirrored) and along.
    let q = vec2<f32>(abs(dot(ap, vec2<f32>(ab.y, -ab.x))), dot(ap, ab)) / h;
    let c = vec2<f32>(sqrt(h - dr * dr), dr);
    let k = c.x * q.y - c.y * q.x;
    if k < 0.0 {
        return sqrt(h * dot(q, q)) - ra;
    }
    if k > c.x {
        return sqrt(h * (dot(q, q) + 1.0 - 2.0 * q.y)) - rb;
    }
    return dot(c, q) - ra;
}

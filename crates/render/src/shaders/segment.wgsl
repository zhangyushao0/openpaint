// A stroke's curve spans (S34): each `Segment` op is one cubic Bézier span between two knots,
// evaluated as `span_pieces` tapered capsules of equal parameter, as `curve.rs` flattens it on
// the CPU: one while the chord lies within `TOLERANCE` output pixels of the curve at the output
// pixel's footprint (`foot_scale`), else enough that each piece does. Radius, flow and contact
// change evenly from one knot to the next. A covering stroke keeps the largest coverage of the
// pieces, an accumulating one sums their densities, so pieces join as segments always did.
// Appended to evaluate.wgsl after its declarations; deposit.wgsl holds `piece_density`.

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

// How many pieces the span is evaluated as here (`curve::pieces`).
fn span_pieces(op: u32) -> u32 {
    if word(op, SEGMENT_FLAT) * foot_scale <= TOLERANCE {
        return 1u;
    }
    let count = ceil(sqrt(word(op, SEGMENT_BEND) * foot_scale / TOLERANCE));
    return clamp(u32(min(count, f32(MAX_PIECES))), 1u, MAX_PIECES);
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

// What the span lays at `centre`: x, its coverage (covering) or density (accumulating); y, the
// paper contact there. A piece whose box widened by its reach misses `centre` lays nothing and
// is skipped. A stamped span (S45) is its stamps instead (stamp.wgsl), which read its stroke's
// paint op `STAMP_PAINT` ops from it.
fn span_laid(centre: vec2<f32>, op: u32) -> vec2<f32> {
    if word(op, STAMP_PAINT) != 0.0 {
        return stamps_laid(centre, op);
    }
    let cover = u32(word(op, SEGMENT_TIP)) == TIP_COVER;
    let whole = chord_piece(op);
    let count = span_pieces(op);
    if count == 1u {
        return piece_laid(centre, whole, op, cover, true);
    }
    let reach = max(whole.radii.x, whole.radii.y) + 0.5 * foot_inverse + 0.01;
    var laid = vec2<f32>(0.0);
    var start = whole.a;
    var t0 = 0.0;
    for (var k = 1u; k <= count; k++) {
        let t1 = f32(k) / f32(count);
        var end = whole.b;
        if k < count {
            end = span_point(op, t1);
        }
        let low = min(start, end) - vec2<f32>(reach);
        let high = max(start, end) + vec2<f32>(reach);
        if all(centre >= low) && all(centre <= high) {
            let piece = Piece(
                start,
                end,
                whole.radii.x + (whole.radii.y - whole.radii.x) * vec2<f32>(t0, t1),
                whole.flows.x + (whole.flows.y - whole.flows.x) * vec2<f32>(t0, t1),
                whole.contacts.x + (whole.contacts.y - whole.contacts.x) * vec2<f32>(t0, t1),
            );
            let one = piece_laid(centre, piece, op, cover, false);
            if cover {
                laid.x = max(laid.x, one.x);
            } else {
                laid = vec2<f32>(laid.x + one.x, max(laid.y, one.y));
            }
        }
        start = end;
        t0 = t1;
    }
    return laid;
}

// One piece's coverage, or its density and contact (a lone point is a dot only when `dot`: a
// whole span, never a piece of one).
fn piece_laid(centre: vec2<f32>, piece: Piece, op: u32, cover: bool, dot: bool) -> vec2<f32> {
    if cover {
        return vec2<f32>(piece_coverage(centre, piece, word(op, SEGMENT_EDGE)), 0.0);
    }
    return piece_density(centre, piece, u32(word(op, SEGMENT_TIP)) == TIP_BELL, dot);
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

// The raster core probe's stamps (S57): one quad per stamp of a stamped span, appended after
// raster.wgsl. The vertex stage places stamp `n` as `stamps_laid` and `stamp_laid` place it for
// every pixel (stamp.wgsl): on the span's chord holding its place along them, scattered, turned
// and sized by `stamp_jitter` of the stroke's seed and `n`; the fragment stage lays the shape
// there (`stamp_alpha`, its texture at the output pixel's footprint) times the flow.

struct StampOut {
    @builtin(position) position: vec4<f32>,
    // The stamp's place after its scatter, and its turn's cosine and sine.
    @location(0) @interpolate(flat) frame: vec4<f32>,
    // Its size (a radius), the flow there, its aspect and the stroke's edge.
    @location(1) @interpolate(flat) stamp: vec4<f32>,
    // Its place on the path, and how far from it the evaluator takes it at all.
    @location(2) @interpolate(flat) near: vec3<f32>,
    @location(3) @interpolate(flat) shape: u32,
    @location(4) @interpolate(flat) place: vec2<i32>,
}

// Where stamp `n` of span `op` lies on the span's chords, and how far through the span: the
// one chord holding its place along them (`stamps_laid`, `chord_stamps`).
fn stamp_on_path(op: u32, s: Stamping, n: u32) -> vec3<f32> {
    let chords = stamp_chords(op);
    var total = 0.0;
    var head = chord_point(op, 0u, chords);
    for (var k = 1u; k <= chords; k++) {
        let to = chord_point(op, k, chords);
        total += distance(head, to);
        head = to;
    }
    var ratio = 1.0;
    let chord = distance(vec2<f32>(word(op, 0), word(op, 1)), vec2<f32>(word(op, 2), word(op, 3)));
    if chords > 1u && chord > 0.0 {
        ratio = total / chord;
    }
    let along = (f32(n) * s.step - word(op, STAMP_ARC)) * ratio;
    head = chord_point(op, 0u, chords);
    var at = vec3<f32>(head, 0.0);
    var start = 0.0;
    for (var k = 0u; k < chords; k++) {
        let to = chord_point(op, k + 1u, chords);
        let extent = distance(head, to);
        let before = k > 0u && along < start;
        let after = k + 1u < chords && along >= start + extent;
        if !before && !after {
            var share = 0.0;
            if extent > 0.0 {
                share = (along - start) / extent;
            }
            at = vec3<f32>(head + (to - head) * share, (f32(k) + share) / f32(chords));
        }
        start += extent;
        head = to;
    }
    return at;
}

// Stamp `a.y` of span `a.x`: its quad, its shape's box turned with it and widened where its
// antialiased rim (half an output pixel) or a texture's mip levels reach.
@vertex
fn stamp_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> StampOut {
    footprint();
    let op = a.x;
    let n = a.y;
    let s = stamping(op + bitcast<u32>(i32(word(op, STAMP_PAINT))));
    let at = stamp_on_path(op, s, n);
    let t = at.z;
    let radius = word(op, 4) + (word(op, 5) - word(op, 4)) * t;
    let units = 2.0 * stamp_jitter(s.seed, n) - vec4<f32>(1.0);
    var path = 0.0;
    if s.scatter > 0.0 || s.rotation == ROTATE_DIRECTION {
        path = path_angle(op, t);
    }
    let along = vec2<f32>(cos(path), sin(path));
    let across = vec2<f32>(-along.y, along.x);
    let place = at.xy + radius * s.scatter * (units.x * along + units.y * across);
    var turn = s.angle + units.z * s.turn_jitter;
    if s.rotation == ROTATE_DIRECTION {
        turn += path;
    } else if s.rotation == ROTATE_AZIMUTH {
        turn += stamp_azimuth(op, t);
    }
    let size = radius * (1.0 + units.w * s.size_jitter);
    let x = vec2<f32>(cos(turn), sin(turn));
    let flow = word(op, SEGMENT_FLOW) + (word(op, SEGMENT_FLOW + 1u) - word(op, SEGMENT_FLOW)) * t;
    var half = vec2<f32>(s.aspect * size, size);
    if s.shape == STAMP_DISC {
        half = vec2<f32>(size);
    }
    var margin = 0.5 * foot_inverse + 0.01;
    if s.shape >= STAMP_TEXTURE {
        margin = 3.0 * foot_inverse + 0.01;
    }
    let side = corner_side(corner) * (half + vec2<f32>(margin));
    var out: StampOut;
    out.position = placed(place + x * side.x + vec2<f32>(-x.y, x.x) * side.y, b.xy, raster.atlas);
    out.frame = vec4<f32>(place, x);
    out.stamp = vec4<f32>(size, flow, s.aspect, word(op, SEGMENT_EDGE));
    out.near = vec3<f32>(at.xy, radius * s.reach + 3.0 * foot_inverse);
    out.shape = s.shape;
    out.place = b.xy;
    return out;
}

// What the stamp lays at the pixel: kept the largest (covering) or summed (accumulating).
@fragment
fn stamp_fragment(in: StampOut) -> @location(0) vec4<f32> {
    footprint();
    let centre = centre_of(pixel_of(in.position, in.place));
    if distance(centre, in.near.xy) > in.near.z {
        return vec4<f32>(0.0);
    }
    let offset = centre - in.frame.xy;
    let x = in.frame.zw;
    let q = vec2<f32>(dot(offset, x), dot(offset, vec2<f32>(-x.y, x.x)));
    let s = Stamping(0.0, in.shape, 0u, in.stamp.z, 0.0, 0.0, 0.0, 0.0, 0u, 0.0);
    let laid = stamp_alpha(q, in.stamp.x, s, in.stamp.w) * in.stamp.y;
    return vec4<f32>(laid, 0.0, 0.0, 0.0);
}

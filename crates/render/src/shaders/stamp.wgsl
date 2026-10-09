// Stamped spans (S45, DECISIONS D72; after Ciallo, Shen 2024): a stamped stroke is a row of
// stamps along its path, stamp n at n * step canvas pixels from its start, each its shape in a
// frame of its own (centre, radius, angle) jittered by a hash of n and the stroke's seed. A
// span holds the stamps whose places lie in it (`STAMP_FIRST` to `STAMP_LAST`, from the CPU,
// `stroke_stamp.rs`), laid along the span's chords: one straight chord, or up to STAMP_CHORDS
// of equal parameter on a curved span, at the canvas's tolerance whatever the output's scale, so
// every scale places them alike. A pixel takes only the stamps whose places lie within their
// reach of its projection onto a chord, and each from that chord alone, so no stamp counts
// twice. A covering stroke keeps the largest coverage of its stamps, an accumulating one sums
// their densities. Appended to evaluate.wgsl after segment.wgsl.

// A stroke's stamps, from its paint op (`protocol_stamp.rs`).
struct Stamping {
    step: f32,
    shape: u32,
    rotation: u32,
    aspect: f32,
    // In radii, across and along.
    scatter: f32,
    size_jitter: f32,
    // Radians.
    angle: f32,
    turn_jitter: f32,
    seed: u32,
    // How far a stamp reaches from its place on the path, in radii (`Shape::reach`).
    reach: f32,
}

// The op `entry` names: a scene op, or with `FRESH` one of this evaluation's.
fn op_at(entry: u32) -> Op {
    if entry >= FRESH {
        return fresh[entry - FRESH];
    }
    return ops[entry];
}

fn stamping(paint: Op) -> Stamping {
    let code = u32(paint.data[PAINT_STAMP + 1u]);
    let shape = code % ROTATIONS;
    let aspect = paint.data[PAINT_STAMP + 2u];
    let scatter = paint.data[PAINT_STAMP + 3u];
    let size_jitter = paint.data[PAINT_STAMP + 4u];
    var extent = 1.0;
    if shape == STAMP_SQUARE || shape >= STAMP_TEXTURE {
        extent = sqrt(aspect * aspect + 1.0);
    }
    let seed = u32(paint.data[PAINT_STAMP + 7u]) * 65536u + u32(paint.data[PAINT_STAMP + 8u]);
    return Stamping(
        paint.data[PAINT_STAMP],
        shape,
        code / ROTATIONS,
        aspect,
        scatter,
        size_jitter,
        paint.data[PAINT_STAMP + 5u],
        paint.data[PAINT_STAMP + 6u],
        seed,
        (1.0 + size_jitter) * extent + 1.4142135 * scatter,
    );
}

// How many chords a span's stamps lie along: the span's pieces at the canvas's own scale, at
// most STAMP_CHORDS.
fn stamp_chords(op: Op) -> u32 {
    if op.data[SEGMENT_FLAT] <= TOLERANCE {
        return 1u;
    }
    let count = ceil(sqrt(op.data[SEGMENT_BEND] / TOLERANCE));
    return clamp(u32(min(count, f32(STAMP_CHORDS))), 1u, STAMP_CHORDS);
}

// Chord k's start among `chords` of the span.
fn chord_point(op: Op, k: u32, chords: u32) -> vec2<f32> {
    if k == 0u {
        return vec2<f32>(op.data[0], op.data[1]);
    }
    if k == chords {
        return vec2<f32>(op.data[2], op.data[3]);
    }
    return span_point(op, f32(k) / f32(chords));
}

// What a stamped span lays at `centre`: x its coverage (covering) or density (accumulating).
fn stamps_laid(centre: vec2<f32>, op: Op, entry: u32) -> vec2<f32> {
    let first = op.data[STAMP_FIRST];
    let last = op.data[STAMP_LAST];
    if last < first {
        return vec2<f32>(0.0);
    }
    let s = stamping(op_at(entry + bitcast<u32>(i32(op.data[STAMP_PAINT]))));
    var margin = 0.5 * foot_inverse + 0.01;
    if s.shape >= STAMP_TEXTURE {
        margin += 2.0 * foot_inverse;
    }
    let reach = max(op.data[4], op.data[5]) * s.reach + margin;
    let chords = stamp_chords(op);
    // The chords' lengths and the share of the stroke's length per length along them.
    var total = 0.0;
    var head = chord_point(op, 0u, chords);
    for (var k = 1u; k <= chords; k++) {
        let to = chord_point(op, k, chords);
        total += distance(head, to);
        head = to;
    }
    var ratio = 1.0;
    let chord = distance(vec2<f32>(op.data[0], op.data[1]), vec2<f32>(op.data[2], op.data[3]));
    if chords > 1u && chord > 0.0 {
        ratio = total / chord;
    }
    var laid = 0.0;
    var taken = 0u;
    var start = 0.0;
    head = chord_point(op, 0u, chords);
    for (var k = 0u; k < chords; k++) {
        let to = chord_point(op, k + 1u, chords);
        let extent = distance(head, to);
        let low = min(head, to) - vec2<f32>(reach);
        let high = max(head, to) + vec2<f32>(reach);
        if all(centre >= low) && all(centre <= high) && taken < 2u * MAX_STAMPS {
            let chord_of = Chord(head, to, start, extent, k, chords, ratio);
            let one = chord_stamps(centre, op, s, chord_of, vec3<f32>(first, last, reach));
            taken += u32(one.y);
            if u32(op.data[SEGMENT_TIP]) == TIP_COVER {
                laid = max(laid, one.x);
            } else {
                laid += one.x;
            }
        }
        start += extent;
        head = to;
    }
    return vec2<f32>(laid, 0.0);
}

// One chord of a span: its ends, how far along the chords it starts and how long it is, which
// of how many it is, and the share of the stroke's length per length along the chords.
struct Chord {
    head: vec2<f32>,
    to: vec2<f32>,
    start: f32,
    length: f32,
    index: u32,
    count: u32,
    ratio: f32,
}

// The stamps of chord `c` near `centre`: x what they lay (their largest coverage, or their
// summed density), y how many were taken. `window` holds the span's first and last stamps and
// the reach.
fn chord_stamps(centre: vec2<f32>, op: Op, s: Stamping, c: Chord, window: vec3<f32>) -> vec2<f32> {
    var u = 0.0;
    if c.length > 0.0 {
        u = clamp(dot(centre - c.head, c.to - c.head) / (c.length * c.length), 0.0, 1.0);
    }
    let near = c.start + u * c.length;
    let arc = op.data[STAMP_ARC];
    // The stamps placed within the reach of `near`, along the chords.
    var low = max(window.x, floor((arc + (near - window.z) / c.ratio) / s.step));
    var high = min(window.y, ceil((arc + (near + window.z) / c.ratio) / s.step));
    if high - low + 1.0 > f32(MAX_STAMPS) {
        let middle = floor((arc + near / c.ratio) / s.step);
        low = max(low, middle - f32(MAX_STAMPS / 2u));
        high = min(high, low + f32(MAX_STAMPS - 1u));
    }
    let cover = u32(op.data[SEGMENT_TIP]) == TIP_COVER;
    var laid = 0.0;
    var taken = 0.0;
    for (var n = low; n <= high; n += 1.0) {
        // Where along the chords stamp n lies; it belongs to the chord holding that place.
        let along = (n * s.step - arc) * c.ratio;
        let before = c.index > 0u && along < c.start;
        let after = c.index + 1u < c.count && along >= c.start + c.length;
        if before || after {
            continue;
        }
        var share = 0.0;
        if c.length > 0.0 {
            share = (along - c.start) / c.length;
        }
        let place = c.head + (c.to - c.head) * share;
        let t = (f32(c.index) + share) / f32(c.count);
        let one = stamp_laid(centre, op, s, vec3<f32>(place, t), u32(n));
        taken += 1.0;
        if cover {
            laid = max(laid, one);
        } else {
            laid += one;
        }
    }
    return vec2<f32>(laid, taken);
}

// What stamp `n`, placed on the path at `at.xy`, `at.z` of the way through its span, lays at
// `centre`: its shape's coverage times the flow there.
fn stamp_laid(centre: vec2<f32>, op: Op, s: Stamping, at: vec3<f32>, n: u32) -> f32 {
    let t = at.z;
    let radius = op.data[4] + (op.data[5] - op.data[4]) * t;
    // Beyond its reach (and a texture's mip spread) it lays nothing.
    if distance(centre, at.xy) > radius * s.reach + 3.0 * foot_inverse {
        return 0.0;
    }
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
    let offset = centre - place;
    let x = vec2<f32>(cos(turn), sin(turn));
    let q = vec2<f32>(dot(offset, x), dot(offset, vec2<f32>(-x.y, x.x)));
    let flow = op.data[SEGMENT_FLOW] + (op.data[SEGMENT_FLOW + 1u] - op.data[SEGMENT_FLOW]) * t;
    return stamp_alpha(q, size, s, op.data[SEGMENT_EDGE]) * flow;
}

// Four numbers in 0..1 for stamp `index` of seed `seed`: `openpaint_core::jitter`, exactly.
fn stamp_jitter(seed: u32, index: u32) -> vec4<f32> {
    var hash = pcg(index ^ pcg(seed));
    var units = vec4<f32>(0.0);
    for (var k = 0u; k < 4u; k++) {
        units[k] = f32(hash >> 8u) * (1.0 / 16777216.0);
        hash = pcg(hash);
    }
    return units;
}

// The span's direction at parameter `t` as an angle: the curve's tangent, or its chord where
// the tangent vanishes (a span whose end knots coincide points along the canvas's x axis).
fn path_angle(op: Op, t: f32) -> f32 {
    let a = vec2<f32>(op.data[0], op.data[1]);
    let b = vec2<f32>(op.data[2], op.data[3]);
    let c1 = vec2<f32>(op.data[SEGMENT_CONTROLS], op.data[SEGMENT_CONTROLS + 1u]);
    let c2 = vec2<f32>(op.data[SEGMENT_CONTROLS + 2u], op.data[SEGMENT_CONTROLS + 3u]);
    let s = 1.0 - t;
    var tangent = (c1 - a) * s * s + 2.0 * (c2 - c1) * s * t + (b - c2) * t * t;
    if dot(tangent, tangent) <= 1e-12 {
        tangent = b - a;
    }
    if dot(tangent, tangent) <= 1e-12 {
        return 0.0;
    }
    return atan2(tangent.y, tangent.x);
}

// The pen's azimuth `t` of the way through the span, turning the short way between its knots'.
fn stamp_azimuth(op: Op, t: f32) -> f32 {
    let head = op.data[SEGMENT_CONTACT];
    var turn = op.data[SEGMENT_CONTACT + 1u] - head;
    turn -= 6.2831855 * round(turn / 6.2831855);
    return head + turn * t;
}

// A stamp's coverage at `q` (canvas pixels in its frame) for a stamp of radius `size`: an
// analytic shape's edge antialiased over one output pixel (or soft over its width, as a
// capsule's), a texture's alpha at the output pixel's footprint.
fn stamp_alpha(q: vec2<f32>, size: f32, s: Stamping, edge: f32) -> f32 {
    let half = vec2<f32>(s.aspect * size, size);
    if s.shape >= STAMP_TEXTURE {
        return texture_alpha(s.shape - STAMP_TEXTURE, q / max(half, vec2<f32>(1e-6)), half);
    }
    var outside = length(q) - size;
    if s.shape == STAMP_SQUARE {
        let d = abs(q) - half;
        outside = length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
    } else if s.shape == STAMP_ELLIPSE {
        let k0 = length(q / half);
        let k1 = length(q / (half * half));
        outside = select(-min(half.x, half.y), k0 * (k0 - 1.0) / k1, k1 > 0.0);
    }
    var ramp = edge;
    if edge < 1.0 {
        ramp = edge * foot_inverse;
    }
    var coverage = clamp((0.5 - outside * foot_scale) * ramp, 0.0, 1.0);
    if edge < 1.0 {
        coverage = coverage * coverage * (3.0 - 2.0 * coverage);
    }
    return coverage;
}

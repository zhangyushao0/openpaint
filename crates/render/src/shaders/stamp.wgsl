// Stamped spans (S45, DECISIONS D72; after Ciallo, Shen 2024): a stamped stroke is a row of
// stamps along its path, stamp n at n * step canvas pixels from its start, each its shape in a
// frame of its own (centre, radius, angle) jittered by a hash of n and the stroke's seed. A
// span holds the stamps whose places lie in it (`STAMP_FIRST` to `STAMP_LAST`, from the CPU,
// `stroke_stamp.rs`), laid along the span's chords: one straight chord, or up to STAMP_CHORDS
// of equal parameter on a curved span, at the canvas's tolerance whatever the output's scale, so
// every scale places them alike; each stamp belongs to the one chord holding its place along
// them (raster_stamp.wgsl draws one quad a stamp). A covering stroke keeps the largest coverage
// of its stamps, an accumulating one sums their densities.

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

fn stamping(paint: u32) -> Stamping {
    let code = u32(word(paint, PAINT_STAMP + 1u));
    let shape = code % ROTATIONS;
    let aspect = word(paint, PAINT_STAMP + 2u);
    let scatter = word(paint, PAINT_STAMP + 3u);
    let size_jitter = word(paint, PAINT_STAMP + 4u);
    var extent = 1.0;
    if shape == STAMP_SQUARE || shape >= STAMP_TEXTURE {
        extent = sqrt(aspect * aspect + 1.0);
    }
    let seed = u32(word(paint, PAINT_STAMP + 7u)) * 65536u + u32(word(paint, PAINT_STAMP + 8u));
    return Stamping(
        word(paint, PAINT_STAMP),
        shape,
        code / ROTATIONS,
        aspect,
        scatter,
        size_jitter,
        word(paint, PAINT_STAMP + 5u),
        word(paint, PAINT_STAMP + 6u),
        seed,
        (1.0 + size_jitter) * extent + 1.4142135 * scatter,
    );
}

// How many chords a span's stamps lie along: the span's pieces at the canvas's own scale, at
// most STAMP_CHORDS.
fn stamp_chords(op: u32) -> u32 {
    if word(op, SEGMENT_FLAT) <= TOLERANCE {
        return 1u;
    }
    let count = ceil(sqrt(word(op, SEGMENT_BEND) / TOLERANCE));
    return clamp(u32(min(count, f32(STAMP_CHORDS))), 1u, STAMP_CHORDS);
}

// Chord k's start among `chords` of the span.
fn chord_point(op: u32, k: u32, chords: u32) -> vec2<f32> {
    if k == 0u {
        return vec2<f32>(word(op, 0), word(op, 1));
    }
    if k == chords {
        return vec2<f32>(word(op, 2), word(op, 3));
    }
    return span_point(op, f32(k) / f32(chords));
}

// Four numbers in 0..1 for stamp `index` of seed `seed`: `openpaint_core::jitter`, exactly.
// Straight-line (S55): filled through a loop index it cost the Adreno 6 to 14 % more.
fn stamp_jitter(seed: u32, index: u32) -> vec4<f32> {
    let h0 = pcg(index ^ pcg(seed));
    let h1 = pcg(h0);
    let h2 = pcg(h1);
    let h3 = pcg(h2);
    return vec4<f32>(vec4<u32>(h0, h1, h2, h3) >> vec4<u32>(8u)) * (1.0 / 16777216.0);
}

// The span's direction at parameter `t` as an angle: the curve's tangent, or its chord where
// the tangent vanishes (a span whose end knots coincide points along the canvas's x axis).
fn path_angle(op: u32, t: f32) -> f32 {
    let a = vec2<f32>(word(op, 0), word(op, 1));
    let b = vec2<f32>(word(op, 2), word(op, 3));
    let c1 = vec2<f32>(word(op, SEGMENT_CONTROLS), word(op, SEGMENT_CONTROLS + 1u));
    let c2 = vec2<f32>(word(op, SEGMENT_CONTROLS + 2u), word(op, SEGMENT_CONTROLS + 3u));
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
fn stamp_azimuth(op: u32, t: f32) -> f32 {
    let head = word(op, SEGMENT_CONTACT);
    var turn = word(op, SEGMENT_CONTACT + 1u) - head;
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

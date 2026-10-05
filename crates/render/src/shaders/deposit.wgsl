// Accumulating strokes (DECISIONS D38, after draweave's ADR-0050): density laid per radius
// travelled, and coverage from density, through the paper when there is one. Appended to
// evaluate.wgsl, which binds `paper`.

// Density a single-sample dot lays at its centre, matching the share a full pass keeps.
const FLAT_DOT: f32 = 1.1;
const BELL_DOT: f32 = 0.52;
// Below this contact the valleys' dust fades out towards the tip's rim.
const DUST_CONTACT: f32 = 0.3;
// Gauss-Hermite nodes and weights over a standard normal distribution.
const NODES = array<f32, 5>(-2.85697, -1.3556262, 0.0, 1.3556262, 2.85697);
const WEIGHTS = array<f32, 5>(0.011257411, 0.22207592, 0.5333333, 0.22207592, 0.011257411);
const MOMENT_STEP: f32 = 1.0 / f32(MOMENT_ONE);

// The integral of the flat cross-section 1 - (p^2 + z^2)^4 along the path, up to z, at the
// squared distance p2 from it.
fn flat_antiderivative(z: f32, p2: f32) -> f32 {
    let p4 = p2 * p2;
    let z2 = z * z;
    return z - z * (p4 * p4 + z2 * (4.0 * p4 * p2 / 3.0
        + z2 * (6.0 * p4 / 5.0 + z2 * (4.0 * p2 / 7.0 + z2 / 9.0))));
}

// The integral of the bell (1 - p^2 - z^2)^2 along the path, up to z.
fn bell_antiderivative(z: f32, p2: f32) -> f32 {
    let falloff = 1.0 - p2;
    let z2 = z * z;
    return falloff * falloff * z - 2.0 * falloff * z * z2 / 3.0 + z * z2 * z2 / 5.0;
}

// What one accumulating piece (`segment.wgsl`) lays at `centre`: x, its tip's cross-section
// integrated over the stretch of the piece inside the tip, in radii (the radius the piece's
// mean), times the flow at the closest point; y, the paper contact there, which fades only at
// the rim. Path-length based: more samples or pieces along the same path lay the same density.
// A whole span whose ends meet is a dot (`dot`); a piece of a span never is.
fn piece_density(centre: vec2<f32>, piece: Piece, bell: bool, dot_allowed: bool) -> vec2<f32> {
    let a = piece.a;
    let b = piece.b;
    let radii = piece.radii;
    let radius = max((radii.x + radii.y) * 0.5, 0.001);
    let y = (centre - a) / radius;
    let v = (b - a) / radius;
    let vv = dot(v, v);
    var work = 0.0;
    var amount = 0.0;
    if distance(a, b) <= 0.001 {
        if !dot_allowed {
            return vec2<f32>(0.0);
        }
        let d2 = dot(y, y);
        let falloff = max(1.0 - d2, 0.0);
        if bell {
            work = falloff * falloff * BELL_DOT;
        } else {
            work = max(1.0 - d2 * d2 * d2 * d2, 0.0) * FLAT_DOT;
        }
    } else {
        let projection = dot(y, v) / vv;
        let p2 = max(dot(y, y) - dot(y, v) * projection, 0.0);
        amount = clamp(projection, 0.0, 1.0);
        if p2 < 1.0 {
            let half_span = sqrt((1.0 - p2) / vv);
            let start = clamp(projection - half_span, 0.0, 1.0);
            let end = clamp(projection + half_span, 0.0, 1.0);
            if end > start {
                let speed = sqrt(vv);
                let z0 = speed * (start - projection);
                let z1 = speed * (end - projection);
                if bell {
                    work = bell_antiderivative(z1, p2) - bell_antiderivative(z0, p2);
                } else {
                    work = flat_antiderivative(z1, p2) - flat_antiderivative(z0, p2);
                }
            }
        }
    }
    let flow_a = piece.flows.x;
    let flow = flow_a + (piece.flows.y - flow_a) * amount;
    let local = max(radii.x + (radii.y - radii.x) * amount, 0.001);
    let d = distance(centre, a + (b - a) * amount) / local;
    var touched = 0.0;
    if d < 1.0 {
        let contact_a = piece.contacts.x;
        let d2 = d * d;
        let reach = 1.0 - d2 * d2 * d2 * d2;
        touched = (contact_a + (piece.contacts.y - contact_a) * amount) * reach;
    }
    return vec2<f32>(max(work, 0.0) * max(flow, 0.0), touched);
}

// An accumulating stroke's coverage at `centre` (canvas pixels) from its density: max_density
// (1 - exp(-d)), or through the paper its expectation over the ranks under the output pixel
// (its footprint picks the mip level of the paper's moments). Exactly 0 without
// density, as the scene's bounds require.
fn deposit_coverage(density: f32, contact: f32, centre: vec2<f32>, op: Op) -> f32 {
    if density <= 0.0 {
        return 0.0;
    }
    let max_density = op.data[PAINT_DENSITY];
    let scale = op.data[PAINT_PAPER];
    if scale <= 0.0 {
        return max_density * (1.0 - exp(-density));
    }
    if contact <= 0.0 {
        return 0.0;
    }
    // An output pixel covers `scale / foot_scale` texels: the paper's level by that footprint
    // (one level finer per doubling, exactly the stroke's own level at scale 1; inside a
    // transform record's frame by its content's scale too).
    let moments = paper_moments(centre * scale, op.data[PAINT_PAPER + 1u] - foot_level);
    let deviation = sqrt(moments.y);
    let level = 1.0 - contact;
    let dust_share = op.data[PAINT_PAPER + 3u];
    let dust = dust_share * min(contact / DUST_CONTACT, 1.0);
    let depth = op.data[PAINT_PAPER + 2u];
    var expected = 0.0;
    for (var i = 0u; i < 5u; i++) {
        let rank = clamp(moments.x + deviation * NODES[i], 0.0, 1.0);
        let reach = clamp((rank - level) / max(contact, 0.001), 0.0, 1.0);
        var deposit = dust;
        if reach > 0.0 {
            deposit = dust + (1.0 - dust_share) * pow(reach, depth);
        }
        expected += WEIGHTS[i] * (1.0 - exp(-density * deposit));
    }
    return max_density * expected;
}

// Mean and variance of the paper's rank over 2^level texels around texel position `t`: wrapped
// bilinear on two levels, mixed by the fraction.
fn paper_moments(t: vec2<f32>, level: f32) -> vec2<f32> {
    let clamped = clamp(level, 0.0, f32(PAPER_LEVELS - 1u));
    let low = floor(clamped);
    let fine = paper_level(u32(low), t);
    let coarse = paper_level(min(u32(low) + 1u, PAPER_LEVELS - 1u), t);
    let mixed = fine + (coarse - fine) * (clamped - low);
    return vec2<f32>(mixed.x, max(mixed.y - mixed.x * mixed.x, 0.0));
}

fn paper_level(level: u32, t: vec2<f32>) -> vec2<f32> {
    let g = t / f32(1u << level) - vec2<f32>(0.5);
    let base = floor(g);
    let f = g - base;
    let c = vec2<i32>(base);
    let top = mix_moments(paper_texel(level, c), paper_texel(level, c + vec2<i32>(1, 0)), f.x);
    let bottom = mix_moments(
        paper_texel(level, c + vec2<i32>(0, 1)),
        paper_texel(level, c + vec2<i32>(1, 1)),
        f.x,
    );
    return mix_moments(top, bottom, f.y);
}

fn mix_moments(a: vec2<f32>, b: vec2<f32>, t: f32) -> vec2<f32> {
    return a + (b - a) * t;
}

// The moments of texel `c` of `level`, wrapping at the paper's edges.
fn paper_texel(level: u32, c: vec2<i32>) -> vec2<f32> {
    let size = vec2<u32>(max(PAPER_WIDTH >> level, 1u), max(PAPER_HEIGHT >> level, 1u));
    let at = bitcast<vec2<u32>>(c) & (size - vec2<u32>(1u));
    var offset = 0u;
    for (var k = 0u; k < level; k++) {
        offset += max(PAPER_WIDTH >> k, 1u) * max(PAPER_HEIGHT >> k, 1u);
    }
    let word = paper[offset + at.y * size.x + at.x];
    return vec2<f32>(f32(word & 0xffffu), f32(word >> 16u)) * MOMENT_STEP;
}

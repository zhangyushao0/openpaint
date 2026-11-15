// What the raster core's fragments read of an op and the scene's words (`raster/`): an op's
// operands, a stroke's paint at its coverage (grain, clip, density to coverage through the
// paper), a fill's colour, a selection's mask bits, a transform's patch and a moved content's
// share. Appended after raster_composite.wgsl.

// Operand `k` of op `op`: read where it is used, never a whole op (copying each op whole, 24
// words, held it in registers and made stamped spans and smudges up to twice as slow on the
// Adreno).
fn word(op: u32, k: u32) -> f32 {
    return ops[op].data[k];
}

// A stroke's colour at its coverage times the grain at canvas pixel `pixel` times its opacity
// (the grain and the clip are canvas pixels: nearest at a scale above 1). A covering
// stroke's coverage is `held` itself; an accumulating one's comes from its density
// (`deposit_coverage`). Without grain the factor is exactly 1.
fn stroke_paint(held: f32, contact: f32, centre: vec2<f32>, pixel: vec2<u32>, op: u32) -> vec4<f32> {
    var coverage = held;
    if word(op, PAINT_DENSITY) > 0.0 {
        coverage = deposit_coverage(held, contact, centre, op);
    }
    return paint_color(coverage * grain(pixel, op) * clip(pixel, op) * word(op, PAINT_OPACITY), op);
}

// 1 where a stroke drawn inside a selection may paint (the selection's mask holds `pixel`) or
// where it has no selection, else 0: zero paint, which leaves the layer exactly as it was.
fn clip(pixel: vec2<u32>, op: u32) -> f32 {
    let at = u32(word(op, PAINT_CLIP)) * 65536u + u32(word(op, PAINT_CLIP + 1u));
    if at == 0u {
        return 1.0;
    }
    return mask_bit(pixel, at - 1u);
}

// The op's colour (`word(op, 0..3)`) at `alpha`, premultiplied.
fn paint_color(alpha: f32, op: u32) -> vec4<f32> {
    return vec4<f32>(vec3<f32>(word(op, 0), word(op, 1), word(op, 2)) * alpha, alpha);
}

// 1 where the mask starting at word `at` of `masks` holds `pixel`, else 0. A mask is its
// origin, its size, then rows of whole words, one bit per pixel.
fn mask_bit(pixel: vec2<u32>, at: u32) -> f32 {
    let origin = vec2<u32>(masks[at], masks[at + 1u]);
    let size = vec2<u32>(masks[at + 2u], masks[at + 3u]);
    if any(pixel < origin) || any(pixel >= origin + size) {
        return 0.0;
    }
    let local = pixel - origin;
    let stride = (size.x + 31u) / 32u;
    let word = masks[at + MASK_HEADER + local.y * stride + local.x / 32u];
    return f32((word >> (local.x % 32u)) & 1u);
}

// A transform's patch where the pixel centre maps back to: bilinear between the four nearest
// patch pixels (premultiplied), transparent beyond the patch's edge. A centre that maps onto a
// pixel's centre gets that pixel exactly, so in place nothing changes.
fn patch_sample(centre: vec2<f32>, op: u32) -> vec4<f32> {
    let at = u32(word(op, PATCH_AT)) * 65536u + u32(word(op, PATCH_AT + 1u));
    let source = vec2<f32>(
        word(op, 0) * centre.x + word(op, 1) * centre.y + word(op, 2),
        word(op, 3) * centre.x + word(op, 4) * centre.y + word(op, 5),
    );
    let origin = vec2<f32>(f32(patches[at]), f32(patches[at + 1u]));
    let local = source - origin - vec2<f32>(0.5);
    let base = floor(local);
    let t = local - base;
    let cell = vec2<i32>(base);
    let top = mix(patch_texel(at, cell), patch_texel(at, cell + vec2<i32>(1, 0)), t.x);
    let bottom = mix(
        patch_texel(at, cell + vec2<i32>(0, 1)),
        patch_texel(at, cell + vec2<i32>(1, 1)),
        t.x,
    );
    return mix(top, bottom, t.y);
}

// The patch's pixel `cell` (from its top-left one), or transparent outside it.
fn patch_texel(at: u32, cell: vec2<i32>) -> vec4<f32> {
    let size = vec2<i32>(i32(patches[at + 2u]), i32(patches[at + 3u]));
    if any(cell < vec2<i32>(0)) || any(cell >= size) {
        return vec4<f32>(0.0);
    }
    return unpack4x8unorm(patches[at + MASK_HEADER + u32(cell.y * size.x + cell.x)]);
}

// The share of paint that lands at `pixel` through the paper's grain, fixed to the canvas: one
// minus the grain strength times the grain's height there. The height is the mean of a fine
// value per pixel and a coarse one, smoothly interpolated between cells of `grain_scale`
// pixels. Integers throughout and a power-of-two divisor at the end: exact on every backend.
fn grain(pixel: vec2<u32>, op: u32) -> f32 {
    let strength = u32(word(op, PAINT_GRAIN));
    if strength == 0u {
        return 1.0;
    }
    let scale = max(u32(word(op, PAINT_GRAIN_SCALE)), 1u);
    let fine = hash(pixel) >> 24u;
    let cell = pixel / scale;
    let f = pixel % scale;
    let g = scale - f;
    let corners = vec4<u32>(
        hash(cell + vec2<u32>(7919u, 0u)) >> 24u,
        hash(cell + vec2<u32>(7920u, 0u)) >> 24u,
        hash(cell + vec2<u32>(7919u, 1u)) >> 24u,
        hash(cell + vec2<u32>(7920u, 1u)) >> 24u,
    );
    let weighted = corners.x * g.x * g.y + corners.y * f.x * g.y + corners.z * g.x * f.y
        + corners.w * f.x * f.y;
    let coarse = weighted / (scale * scale);
    let height = (fine + coarse) / 2u;
    let reduce = (strength * height) / GRAIN_ONE;
    return f32(GRAIN_ONE - reduce) * GRAIN_STEP;
}

// One step of the grain factor: a power of two, so multiplying by it is exact.
const GRAIN_STEP: f32 = 1.0 / f32(GRAIN_ONE);

// A well-mixed 32-bit value per pixel (PCG).
fn hash(p: vec2<u32>) -> u32 {
    return pcg(p.x ^ pcg(p.y));
}

fn pcg(value: u32) -> u32 {
    let state = value * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// What of a transform record's moved content shows at `point` (where the centre maps back
// to): 1 inside its selection's mask, or for the whole layer inside the canvas, else 0: what
// moved came from the canvas only.
fn moved_share(point: vec2<f32>, cell: vec2<u32>, op: u32) -> f32 {
    let canvas = vec2<f32>(word(op, MOVE_CANVAS), word(op, MOVE_CANVAS + 1u));
    if any(point < vec2<f32>(0.0)) || any(point >= canvas) {
        return 0.0;
    }
    let at = u32(word(op, MOVE_MASK)) * 65536u + u32(word(op, MOVE_MASK + 1u));
    if at == 0u {
        return 1.0;
    }
    return mask_bit(cell, at - 1u);
}

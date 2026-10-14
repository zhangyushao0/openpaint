// A shape texture's alpha for a stamp (S45, DECISIONS D72; `shapes.rs`). Up to SHARP_LEVEL
// (an output pixel covering 8 texels: a stamp down to about 32 output pixels across) the rim's
// distance field is read (bilinear, its slope turned into output pixels), so the rim is
// antialiased over one output pixel at any size above that, however far the stamp is
// enlarged; from one level more, the coverage's mip level at the output pixel's footprint
// (trilinear, transparent beyond the texture's square), so a small stamp neither flickers nor
// aliases; between the two they blend. Appended to evaluate.wgsl after stamp.wgsl; the
// textures follow the paper in `textures` from SHAPES_AT.

const SHARP_LEVEL: f32 = 3.0;

// Texture `texture`'s alpha at `uv` (-1 to 1 across its square) for a stamp whose square has
// half sides of `half` canvas pixels.
fn texture_alpha(texture: u32, uv: vec2<f32>, half: vec2<f32>) -> f32 {
    let size = f32(SHAPE_SIZE);
    let texel = (uv + vec2<f32>(1.0)) * (0.5 * size);
    // Texels per output pixel along the texture's two axes.
    let per = 0.5 * size / half * foot_inverse;
    let level = log2(max(max(per.x, per.y), 1e-6));
    if level >= SHARP_LEVEL + 1.0 {
        return mip_coverage(texture, texel, level);
    }
    let sharp = rim_coverage(texture, texel, per);
    if level <= SHARP_LEVEL {
        return sharp;
    }
    return mix(sharp, mip_coverage(texture, texel, level), level - SHARP_LEVEL);
}

// The share of the output pixel inside the rim: the distance in texels at `texel`, bilinear,
// over how fast it changes per output pixel (`per` texels per output pixel on each axis).
fn rim_coverage(texture: u32, texel: vec2<f32>, per: vec2<f32>) -> f32 {
    let g = texel - vec2<f32>(0.5);
    let base = floor(g);
    let f = g - base;
    let c = vec2<i32>(base);
    let s00 = shape_distance(texture, c);
    let s10 = shape_distance(texture, c + vec2<i32>(1, 0));
    let s01 = shape_distance(texture, c + vec2<i32>(0, 1));
    let s11 = shape_distance(texture, c + vec2<i32>(1, 1));
    let top = mix(s00, s10, f.x);
    let bottom = mix(s01, s11, f.x);
    let d = mix(top, bottom, f.y);
    let slope = vec2<f32>(mix(s10 - s00, s11 - s01, f.y), bottom - top) * per;
    return clamp(0.5 + d / max(length(slope), 1e-6), 0.0, 1.0);
}

// The distance from texel `c`'s centre to the rim, in texels, positive inside; outside the
// texture's square, outside.
fn shape_distance(texture: u32, c: vec2<i32>) -> f32 {
    if any(c < vec2<i32>(0)) || any(c >= vec2<i32>(i32(SHAPE_SIZE))) {
        return -8.0;
    }
    let at = SHAPES_AT + texture * SHAPE_WORDS + u32(c.y) * SHAPE_SIZE + u32(c.x);
    return (f32((textures[at] >> 16u) & 0xffu) - 128.0) / 16.0;
}

// The coverage at `texel` (finest texels) over `2^level` texels: two levels mixed.
fn mip_coverage(texture: u32, texel: vec2<f32>, level: f32) -> f32 {
    let clamped = clamp(level, 0.0, f32(SHAPE_LEVELS - 1u));
    let low = floor(clamped);
    let fine = coverage_level(texture, u32(low), texel);
    let coarse = coverage_level(texture, min(u32(low) + 1u, SHAPE_LEVELS - 1u), texel);
    return mix(fine, coarse, clamped - low);
}

fn coverage_level(texture: u32, level: u32, texel: vec2<f32>) -> f32 {
    let g = texel / f32(1u << level) - vec2<f32>(0.5);
    let base = floor(g);
    let f = g - base;
    let c = vec2<i32>(base);
    let top = mix(
        shape_coverage(texture, level, c),
        shape_coverage(texture, level, c + vec2<i32>(1, 0)),
        f.x,
    );
    let bottom = mix(
        shape_coverage(texture, level, c + vec2<i32>(0, 1)),
        shape_coverage(texture, level, c + vec2<i32>(1, 1)),
        f.x,
    );
    return mix(top, bottom, f.y);
}

// Texel `c` of `level`'s coverage; transparent outside the square.
fn shape_coverage(texture: u32, level: u32, c: vec2<i32>) -> f32 {
    let side = SHAPE_SIZE >> level;
    if any(c < vec2<i32>(0)) || any(c >= vec2<i32>(i32(side))) {
        return 0.0;
    }
    let offset = mip_offset(SHAPE_BITS, level);
    let at = SHAPES_AT + texture * SHAPE_WORDS + offset + u32(c.y) * side + u32(c.x);
    return f32(textures[at] & 0xffffu) * (1.0 / 65535.0);
}

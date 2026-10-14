// The raster core probe's composites (S57), appended after raster_stamp.wgsl: each stroke's
// region of the atlas onto the layer through the evaluator's paint (`stroke_paint`: colour,
// opacity, grain, clip, density to coverage through the paper), one quad per tile its spans
// reach, combined by the paint op's blend in the blend unit; then the layer onto the paper,
// rounded as the evaluator rounds (`quantized`).

struct CompositeOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) paint: u32,
    @location(1) @interpolate(flat) contact: u32,
    // Where the stroke's coverage (and its contact) lie in the atlas from the target pixel.
    @location(2) @interpolate(flat) places: vec4<i32>,
}

// Tile `(a.y, a.z)` of the target for the stroke whose paint op is `a.x`.
@vertex
fn composite_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> CompositeOut {
    let side = corner_side(corner) * 0.5 + vec2<f32>(0.5);
    let p = vec2<f32>(f32(a.y), f32(a.z)) + side * RASTER_TILE;
    var out: CompositeOut;
    out.position = vec4<f32>(
        p.x / raster.size.x * 2.0 - 1.0,
        1.0 - p.y / raster.size.y * 2.0,
        0.0,
        1.0,
    );
    out.paint = a.x;
    out.contact = a.w;
    out.places = b;
    return out;
}

@fragment
fn composite_fragment(in: CompositeOut) -> @location(0) vec4<f32> {
    footprint();
    let pixel = vec2<i32>(floor(in.position.xy));
    let centre = centre_of(pixel);
    let held = textureLoad(source, pixel + in.places.xy, 0).r;
    var contact = 0.0;
    if in.contact == 1u {
        contact = textureLoad(source, pixel + in.places.zw, 0).r;
    }
    return stroke_paint(held, contact, centre, vec2<u32>(floor(centre)), in.paint);
}

// One triangle over the whole target.
@vertex
fn finish_vertex(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((corner << 1u) & 2u), f32(corner & 2u));
    return vec4<f32>(p * 2.0 - vec2<f32>(1.0), 0.0, 1.0);
}

// The layer at its opacity over the paper, rounded to 8 bits.
@fragment
fn finish_fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let layer = textureLoad(source, vec2<i32>(floor(position.xy)), 0);
    return quantized(over(layer * raster.opacity, raster.background));
}

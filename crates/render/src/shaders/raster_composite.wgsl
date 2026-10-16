// The raster core's draws onto the layer, its layer ends and its finish (S57's composites, S59),
// appended after raster_stamp.wgsl. A stroke's region of the atlas goes onto the layer through
// the evaluator's paint (`stroke_paint`: colour, opacity, grain, clip, density to coverage
// through the paper), one quad per tile its spans reach, combined by the paint op's blend in the
// blend unit; a fill paints its colour by its edge (`fill_coverage`), a patch is sampled through
// its placement (`patch_sample`), each over the pixels its bounds reach. A layer's end shows it
// through its mask (`masked`) at its opacity and lays it onto the canvas or into its chain by its
// mode: the blend unit's equations where they are the evaluator's (`composite`, `clipped`; the
// canvas is opaque), else written whole from a copy of what lies under it (Overlay). The finish
// rounds the canvas as the evaluator rounds (`quantized`, or `coded` for ten bits).

struct CompositeOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) paint: u32,
    @location(1) @interpolate(flat) contact: u32,
    // Where the stroke's coverage (and its contact) lie in the atlas from the region pixel.
    @location(2) @interpolate(flat) places: vec4<i32>,
}

// Tile `(a.y, a.z)` of the region for the stroke whose paint op is `a.x`.
@vertex
fn composite_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> CompositeOut {
    let side = corner_side(corner) * 0.5 + vec2<f32>(0.5);
    var out: CompositeOut;
    out.position = in_region(vec2<f32>(f32(a.y), f32(a.z)) + side * RASTER_TILE);
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

struct RectOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) op: u32,
}

// The rectangle `b` (region pixels, its corners) drawn for op `a.x`.
@vertex
fn rect_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> RectOut {
    let side = corner_side(corner) * 0.5 + vec2<f32>(0.5);
    var out: RectOut;
    out.position = in_region(vec2<f32>(b.xy) + side * vec2<f32>(b.zw - b.xy));
    out.op = a.x;
    return out;
}

// A fill's colour at the share of the pixel its edge covers (or its mask bit, hard).
@fragment
fn fill_fragment(in: RectOut) -> @location(0) vec4<f32> {
    footprint();
    let centre = centre_of(vec2<i32>(floor(in.position.xy)));
    let covered = fill_coverage(centre, vec2<u32>(floor(centre)), in.op);
    return paint_color(covered * word(in.op, PAINT_OPACITY), in.op);
}

// A patch sampled where the pixel's centre maps back to, laid over the layer.
@fragment
fn patch_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let centre = centre_of(vec2<i32>(floor(in.position.xy)));
    return patch_sample(centre, in.op);
}

// The layer's paint (or the chain) at `pixel` as its end shows it: through its mask.
fn shown_at(pixel: vec2<i32>, op: u32) -> vec4<f32> {
    let paint = textureLoad(source, pixel, 0);
    let masking = u32(word(op, LAYER_MASK));
    if masking == MASK_NONE {
        return paint;
    }
    return masked(textureLoad(second, pixel, 0), paint, masking);
}

// What a layer's end lays: the layer as shown, at its opacity (over, atop, Screen's equations).
@fragment
fn end_fragment(in: RectOut) -> @location(0) vec4<f32> {
    return shown_at(vec2<i32>(floor(in.position.xy)), in.op) * word(in.op, 0);
}

// Multiply as `under x (1 - alpha + colour)`: the blend unit multiplies what lies under by it.
@fragment
fn multiply_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let s = shown_at(vec2<i32>(floor(in.position.xy)), in.op) * word(in.op, 0);
    return vec4<f32>(vec3<f32>(1.0 - s.a) + s.rgb, s.a);
}

// A chain's base: the chain becomes the layer as shown (its opacity waits for the chain's end).
@fragment
fn base_fragment(in: RectOut) -> @location(0) vec4<f32> {
    return shown_at(vec2<i32>(floor(in.position.xy)), in.op);
}

// Overlay onto the copy of what lies under it, as the evaluator composites it.
@fragment
fn overlay_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let s = shown_at(pixel, in.op) * word(in.op, 0);
    let under = textureLoad(backdrop, pixel, 0);
    if u32(word(in.op, LAYER_INTO)) == INTO_CLIPPED {
        return clipped(s, under, MODE_OVERLAY);
    }
    return composite(s, under, MODE_OVERLAY);
}

struct FinishOut {
    @builtin(position) position: vec4<f32>,
    // The destination pixel holding the region's top-left one.
    @location(0) @interpolate(flat) place: vec2<i32>,
}

// One triangle over the viewport, the region's place in the destination from `b`.
@vertex
fn finish_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> FinishOut {
    let p = vec2<f32>(f32((corner << 1u) & 2u), f32(corner & 2u));
    var out: FinishOut;
    out.position = vec4<f32>(p * 2.0 - vec2<f32>(1.0), 0.0, 1.0);
    out.place = b.xy;
    return out;
}

// The canvas rounded to 8 bits as the evaluator rounds it.
@fragment
fn finish_eight(in: FinishOut) -> @location(0) vec4<f32> {
    return quantized(textureLoad(source, vec2<i32>(floor(in.position.xy)) - in.place, 0));
}

// The canvas as the ten-bit code the evaluator writes for the screen (S37).
@fragment
fn finish_ten(in: FinishOut) -> @location(0) vec4<f32> {
    return coded(textureLoad(source, vec2<i32>(floor(in.position.xy)) - in.place, 0));
}

// The raster core's draws onto the layer, its layer ends and its finish (S57's composites, S59),
// appended after raster_stamp.wgsl. A stroke's region of the atlas goes onto the layer through
// the evaluator's paint (`stroke_paint`: colour, opacity, grain, clip, density to coverage
// through the paper), one quad per tile its spans reach, combined by the paint op's blend in the
// blend unit; a fill paints its colour by its edge (`fill_coverage`), a patch is sampled through
// its placement (`patch_sample`), each over the pixels its bounds reach; a smudge writes the
// layer's copy moved by its residual (`smudge_applied`, S61). A layer's end shows it
// through its mask (`masked`) at its opacity and lays it onto the canvas or into its chain by its
// mode: the blend unit's equations where they are the evaluator's (`composite`, `clipped`; the
// canvas is opaque), else written whole from a copy of what lies under it (Overlay). The finish
// rounds the canvas as the evaluator rounds (`quantized`, or `coded` for ten bits). S65: every
// draw evaluates at the point its frame maps the pixel's centre to (`point_of`); an adjustment
// and a filter rewrite the layer from a copy of it, a moved content is cut and laid over the
// level under it, a whole layer's cut in place, and a group's end fades the canvas over a copy of
// what lay under the group.

struct CompositeOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) paint: u32,
    @location(1) @interpolate(flat) contact: u32,
    // Where the stroke's coverage (and its contact) lie in the atlas from the region pixel.
    @location(2) @interpolate(flat) places: vec4<i32>,
    // The frame its stroke lies in (S65).
    @location(3) @interpolate(flat) frame: u32,
}

// Tile `(a.y, a.z)` of the region for the stroke whose paint op is `a.x`; `a.w` its contact flag,
// then its frame.
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
    out.contact = a.w & 1u;
    out.places = b;
    out.frame = a.w >> 1u;
    return out;
}

@fragment
fn composite_fragment(in: CompositeOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let point = point_of(pixel, in.frame);
    let held = textureLoad(source, pixel + in.places.xy, 0).r;
    var contact = 0.0;
    if in.contact == 1u {
        contact = textureLoad(source, pixel + in.places.zw, 0).r;
    }
    return stroke_paint(held, contact, point, cell_of(point, in.frame), in.paint);
}

struct RectOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) op: u32,
    @location(1) @interpolate(flat) frame: u32,
}

// Corner `corner` of the rectangle `b` (region pixels, its corners).
fn rect_corner(corner: u32, b: vec4<i32>) -> vec4<f32> {
    let side = corner_side(corner) * 0.5 + vec2<f32>(0.5);
    return in_region(vec2<f32>(b.xy) + side * vec2<f32>(b.zw - b.xy));
}

// The rectangle `b` drawn for op `a.x` in frame `a.y`.
@vertex
fn rect_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> RectOut {
    var out: RectOut;
    out.position = rect_corner(corner, b);
    out.op = a.x;
    out.frame = a.y;
    return out;
}

struct ArenaOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) op: u32,
    @location(1) @interpolate(flat) base: u32,
    @location(2) @interpolate(flat) frame: u32,
}

// The rectangle `b` a smudge or (S65) a filter may change: op `a.x`, its block from word `a.y` of
// the arena, in frame `a.z`.
@vertex
fn arena_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> ArenaOut {
    var out: ArenaOut;
    out.position = rect_corner(corner, b);
    out.op = a.x;
    out.base = a.y;
    out.frame = a.z;
    return out;
}

// The layer (or mask) after the smudge: its copy (`source`) moved by the smudge's residual at
// the output pixel's centre where the smudge's clip lets it change the canvas pixel there, else
// the copy as it is; written whole. The smudge being drawn reads its own residual (S63).
@fragment
fn smudge_fragment(in: ArenaOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let before = textureLoad(source, pixel, 0);
    let point = point_of(pixel, in.frame);
    if clip(cell_of(point, in.frame), in.op) == 0.0 {
        return before;
    }
    let locked = word(in.op, SMUDGE_LOCKED) != 0.0;
    let drawing = word(in.op, SMUDGE_ACTIVE) != 0.0;
    return smudge_applied(before, residual_at(drawing, in.base, point), locked);
}

// A fill's colour at the share of the pixel its edge covers (or its mask bit, hard).
@fragment
fn fill_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let point = point_of(vec2<i32>(floor(in.position.xy)), in.frame);
    let covered = fill_coverage(point, cell_of(point, in.frame), in.op);
    return paint_color(covered * word(in.op, PAINT_OPACITY), in.op);
}

// A patch sampled where the pixel's centre maps back to, laid over the layer.
@fragment
fn patch_fragment(in: RectOut) -> @location(0) vec4<f32> {
    return patch_sample(point_of(vec2<i32>(floor(in.position.xy)), in.frame), in.op);
}

// S65: the layer (or mask) through a point adjustment (`adjusted`, adjust.wgsl), from its copy;
// written whole.
@fragment
fn adjust_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let point = point_of(pixel, in.frame);
    return adjusted(textureLoad(source, pixel, 0), cell_of(point, in.frame), in.op);
}

// S65: the layer (or mask) as a filter leaves it (`filtered_from`, filter_composite.wgsl): its
// result where the clip and its block hold the point, else its copy; written whole.
@fragment
fn filter_fragment(in: ArenaOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let point = point_of(pixel, in.frame);
    let before = textureLoad(source, pixel, 0);
    return filtered_from(before, point, cell_of(point, in.frame), in.op, in.base);
}

// S65: a moved selection's content (`source`, its level) cut to its selection where the pixel's
// centre maps back (`moved_share`), which the blend unit lays over the level under it.
@fragment
fn moved_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let point = point_of(pixel, in.frame);
    return textureLoad(source, pixel, 0) * moved_share(point, cell_of(point, in.frame), in.op);
}

// S65: a whole layer's moved content cut to the canvas where the pixel's centre maps back: the
// blend unit multiplies the layer by it, in place.
@fragment
fn cut_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let point = point_of(vec2<i32>(floor(in.position.xy)), in.frame);
    return vec4<f32>(moved_share(point, cell_of(point, in.frame), in.op));
}

// S65: a group's end, as the evaluator's `OP_END_GROUP`: the canvas (its copy, `backdrop`) at the
// group's opacity over what lay under the group (`source`); where the two are alike (none of its
// layers changed the pixel) the canvas as it is. Written whole.
@fragment
fn group_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let canvas = textureLoad(backdrop, pixel, 0);
    let under = textureLoad(source, pixel, 0);
    if all(canvas == under) {
        return canvas;
    }
    return canvas * word(in.op, 0) + under * (1.0 - word(in.op, 0));
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

// By its mode onto the copy of what lies under it, as the evaluator composites it: Overlay, and
// (S65) a merged layer's Multiply or Screen onto a level, which is not opaque.
@fragment
fn backdrop_fragment(in: RectOut) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let s = shown_at(pixel, in.op) * word(in.op, 0);
    let under = textureLoad(backdrop, pixel, 0);
    let mode = u32(word(in.op, LAYER_MODE));
    if u32(word(in.op, LAYER_INTO)) == INTO_CLIPPED {
        return clipped(s, under, mode);
    }
    return composite(s, under, mode);
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

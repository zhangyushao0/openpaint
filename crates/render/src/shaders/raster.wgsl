// The raster core probe (S57, measurement only; `raster.rs`): one layer's strokes drawn by the
// GPU's rasterizer instead of the evaluator's walk, at every pixel, over the ops binned to its
// tile. A continuous span is flattened as segment.wgsl flattens it at the output's scale and
// each piece is one quad around its capsule; a stamped span is one quad per stamp, placed,
// turned, sized and scattered by the same hashes (D72). Each piece's and stamp's coverage or
// density comes from the evaluator's own functions (segment.wgsl, stamp.wgsl,
// stamp_texture.wgsl, deposit.wgsl and helpers taken from evaluate.wgsl, appended after this
// file), combined within a stroke by the blend unit (max for coverage and contact, add for
// density) in the stroke's region of a scratch atlas; then each region is composited onto the
// layer by the evaluator's paint (`stroke_paint`) and the paint op's blend, and the layer onto
// the paper, rounded as the evaluator rounds. The protocol prelude is prepended.

struct Raster {
    // Output pixels per canvas pixel, its inverse, its base-2 log; the layer's opacity.
    scale: f32,
    inverse: f32,
    level: f32,
    opacity: f32,
    // The paper, premultiplied and opaque.
    background: vec4<f32>,
    // The output pixel of the target's top-left pixel.
    origin: vec2<i32>,
    // The target's and the atlas's sizes in pixels.
    size: vec2<f32>,
    atlas: vec2<f32>,
    unused: vec2<f32>,
}

@group(0) @binding(0) var<uniform> raster: Raster;
// The scene's ops, fill masks and the evaluator's textures (paper, then the shapes').
@group(0) @binding(1) var<storage, read> ops: array<Op>;
@group(0) @binding(2) var<storage, read> masks: array<u32>;
@group(0) @binding(3) var<storage, read> textures: array<u32>;
// What a composite reads (the atlas) or the finish reads (the layer).
@group(1) @binding(0) var source: texture_2d<f32>;

// The footprint of an output pixel in canvas pixels, as the evaluator holds it.
var<private> foot_scale: f32;
var<private> foot_inverse: f32;
var<private> foot_level: f32;

// Side of a composite tile, in target pixels (`raster_plan.rs`).
const RASTER_TILE: f32 = 32.0;

fn footprint() {
    foot_scale = raster.scale;
    foot_inverse = raster.inverse;
    foot_level = raster.level;
}

// Corner `corner` of a quad drawn as a strip: (-1, -1), (1, -1), (-1, 1), (1, 1).
fn corner_side(corner: u32) -> vec2<f32> {
    return vec2<f32>(f32(corner & 1u), f32(corner >> 1u)) * 2.0 - vec2<f32>(1.0);
}

// A point in canvas pixels as a position in an attachment of `size` pixels, whose pixel `p`
// holds target pixel `p - place`.
fn placed(canvas: vec2<f32>, place: vec2<i32>, size: vec2<f32>) -> vec4<f32> {
    let p = canvas * raster.scale - vec2<f32>(raster.origin) + vec2<f32>(place);
    return vec4<f32>(p.x / size.x * 2.0 - 1.0, 1.0 - p.y / size.y * 2.0, 0.0, 1.0);
}

// The canvas point the evaluator evaluates a target pixel at: its output pixel's centre.
fn centre_of(pixel: vec2<i32>) -> vec2<f32> {
    return (vec2<f32>(pixel + raster.origin) + vec2<f32>(0.5)) * raster.inverse;
}

// The target pixel an attachment's fragment holds.
fn pixel_of(position: vec4<f32>, place: vec2<i32>) -> vec2<i32> {
    return vec2<i32>(floor(position.xy)) - place;
}

struct PieceOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) op: u32,
    @location(1) @interpolate(flat) ends: vec4<f32>,
    @location(2) @interpolate(flat) radii: vec2<f32>,
    @location(3) @interpolate(flat) flows: vec2<f32>,
    @location(4) @interpolate(flat) contacts: vec2<f32>,
    @location(5) @interpolate(flat) place: vec2<i32>,
    @location(6) @interpolate(flat) dot_allowed: u32,
}

// Piece `k` of `count` of span `op`, as `span_laid` cuts it.
fn piece_of(op: u32, k: u32, count: u32) -> Piece {
    let whole = chord_piece(op);
    if count == 1u {
        return whole;
    }
    let t0 = f32(k) / f32(count);
    let t1 = f32(k + 1u) / f32(count);
    var start = whole.a;
    if k > 0u {
        start = span_point(op, t0);
    }
    var end = whole.b;
    if k + 1u < count {
        end = span_point(op, t1);
    }
    return Piece(
        start,
        end,
        whole.radii.x + (whole.radii.y - whole.radii.x) * vec2<f32>(t0, t1),
        whole.flows.x + (whole.flows.y - whole.flows.x) * vec2<f32>(t0, t1),
        whole.contacts.x + (whole.contacts.y - whole.contacts.x) * vec2<f32>(t0, t1),
    );
}

// One piece's quad: its capsule's box along it, widened by half an output pixel (where its
// coverage and density end) and a little more.
@vertex
fn piece_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> PieceOut {
    footprint();
    let piece = piece_of(a.x, a.y, a.z);
    let reach = max(piece.radii.x, piece.radii.y) + 0.5 * foot_inverse + 0.01;
    var along = vec2<f32>(1.0, 0.0);
    let length = distance(piece.a, piece.b);
    if length > 0.0 {
        along = (piece.b - piece.a) / length;
    }
    let side = corner_side(corner);
    let end = select(piece.a, piece.b, side.x > 0.0);
    let canvas = end + (along * side.x + vec2<f32>(-along.y, along.x) * side.y) * reach;
    var out: PieceOut;
    out.position = placed(canvas, b.xy, raster.atlas);
    out.op = a.x;
    out.ends = vec4<f32>(piece.a, piece.b);
    out.radii = piece.radii;
    out.flows = piece.flows;
    out.contacts = piece.contacts;
    out.place = b.xy;
    out.dot_allowed = a.w;
    return out;
}

fn piece_in(in: PieceOut) -> Piece {
    return Piece(in.ends.xy, in.ends.zw, in.radii, in.flows, in.contacts);
}

// A covering piece's coverage: the stroke keeps the largest (blend max).
@fragment
fn cover_fragment(in: PieceOut) -> @location(0) vec4<f32> {
    footprint();
    let centre = centre_of(pixel_of(in.position, in.place));
    let laid = piece_coverage(centre, piece_in(in), word(in.op, SEGMENT_EDGE));
    return vec4<f32>(laid, 0.0, 0.0, 0.0);
}

// An accumulating piece's density, summed (blend add), or its paper contact, the largest kept.
@fragment
fn density_fragment(in: PieceOut) -> @location(0) vec4<f32> {
    footprint();
    let centre = centre_of(pixel_of(in.position, in.place));
    let bell = u32(word(in.op, SEGMENT_TIP)) == TIP_BELL;
    let laid = piece_density(centre, piece_in(in), bell, in.dot_allowed == 1u);
    return vec4<f32>(laid.x, 0.0, 0.0, 0.0);
}

@fragment
fn contact_fragment(in: PieceOut) -> @location(0) vec4<f32> {
    footprint();
    let centre = centre_of(pixel_of(in.position, in.place));
    let bell = u32(word(in.op, SEGMENT_TIP)) == TIP_BELL;
    let laid = piece_density(centre, piece_in(in), bell, in.dot_allowed == 1u);
    return vec4<f32>(laid.y, 0.0, 0.0, 0.0);
}

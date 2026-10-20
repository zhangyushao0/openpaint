// The raster core (`raster.rs`, DECISIONS D93): a scene's region drawn by the GPU's rasterizer.
// A continuous span is flattened at the output's scale as `curve.rs` flattens it and each piece
// is one quad around its capsule; a stamped span is one quad per stamp, placed, turned, sized and
// scattered by the stroke's hashes (D72). Each piece's and stamp's coverage or density
// (segment.wgsl, stamp.wgsl, stamp_texture.wgsl, deposit.wgsl, appended after these files) is
// combined within a stroke by the blend unit (max for coverage and contact, add for density) in
// the stroke's region of a scratch atlas; then each region is composited onto the layer by the
// stroke's paint (`stroke_paint`, raster_paint.wgsl) and the paint op's blend, fills
// (fill_edge.wgsl) and patches drawn there as quads, a smudge applied to the layer from a copy of
// it by its residual (smudge_composite.wgsl), each layer's end composited onto the canvas by its
// mode, and the canvas rounded into its destination (raster_composite.wgsl). A draw inside a
// transform record's content names its frame (its op's index plus one; 0 on the canvas), whose
// rows map a pixel's centre into the content and whose map back places the quads; adjustments,
// filters, moves' and merged layers' ends and groups' ends rewrite targets from copies
// (raster_composite.wgsl). The protocol prelude is prepended.

struct Raster {
    // Output pixels per canvas pixel, its inverse, its base-2 log.
    scale: f32,
    inverse: f32,
    level: f32,
    unused: f32,
    // The paper, premultiplied and opaque.
    background: vec4<f32>,
    // The output pixel of the region's top-left pixel.
    origin: vec2<i32>,
    // The region's and the atlas's sizes in pixels.
    size: vec2<f32>,
    atlas: vec2<f32>,
    unused_too: vec2<f32>,
    // S65: where the centres lie on the canvas (a material seen from a transform record; else the
    // identity): rows, one over its canvas pixels per pixel, that one's base-2 log; and the rows
    // mapping canvas points back.
    base_x: vec4<f32>,
    base_y: vec4<f32>,
    unbase_x: vec4<f32>,
    unbase_y: vec4<f32>,
}

@group(0) @binding(0) var<uniform> raster: Raster;
// The scene's ops, fill masks, the textures (paper, then the shapes') and patches.
@group(0) @binding(1) var<storage, read> ops: array<Op>;
@group(0) @binding(2) var<storage, read> masks: array<u32>;
@group(0) @binding(3) var<storage, read> textures: array<u32>;
@group(0) @binding(4) var<storage, read> patches: array<u32>;
// The smudge residuals (smudge_composite.wgsl, S61): the finished
// ones' arena, which a smudge's instance names its block in, and the in-progress one's.
@group(0) @binding(5) var<storage, read> arena: array<u32>;
@group(0) @binding(6) var<storage, read> active_residual: array<u32>;
// What a draw reads: the atlas (a stroke's tiles); a layer's paint (or the chain), its mask and
// a copy of what lies under it (an end); the copy of the layer or mask a smudge applies to; the
// canvas (the finish).
@group(1) @binding(0) var source: texture_2d<f32>;
@group(1) @binding(1) var second: texture_2d<f32>;
@group(1) @binding(2) var backdrop: texture_2d<f32>;

// The footprint of an output pixel in canvas pixels (`footprint`, `foot_of`).
var<private> foot_scale: f32;
var<private> foot_inverse: f32;
var<private> foot_level: f32;

// Side of a composite tile, in region pixels (`raster_lay.rs`).
const RASTER_TILE: f32 = 32.0;

fn footprint() {
    foot_scale = raster.scale * raster.base_x.w;
    foot_inverse = raster.inverse / raster.base_x.w;
    foot_level = raster.level + raster.base_y.w;
}

// The footprint in frame `frame` (0: on the canvas).
fn foot_of(frame: u32) {
    if frame == 0u {
        footprint();
        return;
    }
    let op = frame - 1u;
    let foot = raster.base_x.w * word(op, FRAME_FOOT);
    foot_scale = raster.scale * foot;
    foot_inverse = raster.inverse / foot;
    foot_level = raster.level + raster.base_y.w + word(op, FRAME_LEVEL);
}

// Where frame `frame` (0: none) takes the canvas point `centre`: its content's point.
fn framed(centre: vec2<f32>, frame: u32) -> vec2<f32> {
    if frame == 0u {
        return centre;
    }
    let op = frame - 1u;
    return vec2<f32>(
        word(op, 0) * centre.x + word(op, 1) * centre.y + word(op, 2),
        word(op, 3) * centre.x + word(op, 4) * centre.y + word(op, 5),
    );
}

// The canvas pixel the sources on the grid are read at for `point` in frame `frame`.
fn cell_of(point: vec2<f32>, frame: u32) -> vec2<u32> {
    if frame == 0u {
        return vec2<u32>(floor(point));
    }
    return vec2<u32>(max(floor(point), vec2<f32>(0.0)));
}

// A point of frame `frame`'s content (0: the canvas) on the canvas: where its quads lie.
fn unframed(point: vec2<f32>, frame: u32) -> vec2<f32> {
    if frame == 0u {
        return point;
    }
    let op = frame - 1u;
    return vec2<f32>(
        word(op, FRAME_CANVAS) * point.x + word(op, FRAME_CANVAS + 1u) * point.y
            + word(op, FRAME_CANVAS + 2u),
        word(op, FRAME_CANVAS + 3u) * point.x + word(op, FRAME_CANVAS + 4u) * point.y
            + word(op, FRAME_CANVAS + 5u),
    );
}

// Corner `corner` of a quad drawn as a strip: (-1, -1), (1, -1), (-1, 1), (1, 1).
fn corner_side(corner: u32) -> vec2<f32> {
    return vec2<f32>(f32(corner & 1u), f32(corner >> 1u)) * 2.0 - vec2<f32>(1.0);
}

// A point in canvas pixels as a position in an attachment of `size` pixels, whose pixel `p`
// holds region pixel `p - place`.
fn placed(canvas: vec2<f32>, place: vec2<i32>, size: vec2<f32>) -> vec4<f32> {
    let back = vec2<f32>(
        raster.unbase_x.x * canvas.x + raster.unbase_x.y * canvas.y + raster.unbase_x.z,
        raster.unbase_y.x * canvas.x + raster.unbase_y.y * canvas.y + raster.unbase_y.z,
    );
    let p = back * raster.scale - vec2<f32>(raster.origin) + vec2<f32>(place);
    return vec4<f32>(p.x / size.x * 2.0 - 1.0, 1.0 - p.y / size.y * 2.0, 0.0, 1.0);
}

// A point in region pixels as a position in the region's targets.
fn in_region(p: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(p.x / raster.size.x * 2.0 - 1.0, 1.0 - p.y / raster.size.y * 2.0, 0.0, 1.0);
}

// The canvas point a region pixel is drawn at: its output pixel's centre, where
// the base puts it.
fn centre_of(pixel: vec2<i32>) -> vec2<f32> {
    let centre = (vec2<f32>(pixel + raster.origin) + vec2<f32>(0.5)) * raster.inverse;
    return vec2<f32>(
        raster.base_x.x * centre.x + raster.base_x.y * centre.y + raster.base_x.z,
        raster.base_y.x * centre.x + raster.base_y.y * centre.y + raster.base_y.z,
    );
}

// The point a draw in frame `frame` draws region pixel `pixel` at, its footprint set.
fn point_of(pixel: vec2<i32>, frame: u32) -> vec2<f32> {
    foot_of(frame);
    return framed(centre_of(pixel), frame);
}

// The region pixel an attachment's fragment holds.
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
    @location(7) @interpolate(flat) frame: u32,
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
// coverage and density end) and a little more, in its frame's content (`b.z`), placed through it.
@vertex
fn piece_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> PieceOut {
    let frame = bitcast<u32>(b.z);
    foot_of(frame);
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
    out.position = placed(unframed(canvas, frame), b.xy, raster.atlas);
    out.op = a.x;
    out.ends = vec4<f32>(piece.a, piece.b);
    out.radii = piece.radii;
    out.flows = piece.flows;
    out.contacts = piece.contacts;
    out.place = b.xy;
    out.dot_allowed = a.w;
    out.frame = frame;
    return out;
}

fn piece_in(in: PieceOut) -> Piece {
    return Piece(in.ends.xy, in.ends.zw, in.radii, in.flows, in.contacts);
}

// A covering piece's coverage: the stroke keeps the largest (blend max).
@fragment
fn cover_fragment(in: PieceOut) -> @location(0) vec4<f32> {
    let centre = point_of(pixel_of(in.position, in.place), in.frame);
    let laid = piece_coverage(centre, piece_in(in), word(in.op, SEGMENT_EDGE));
    return vec4<f32>(laid, 0.0, 0.0, 0.0);
}

// An accumulating piece's density, summed (blend add), or its paper contact, the largest kept.
@fragment
fn density_fragment(in: PieceOut) -> @location(0) vec4<f32> {
    let centre = point_of(pixel_of(in.position, in.place), in.frame);
    let bell = u32(word(in.op, SEGMENT_TIP)) == TIP_BELL;
    let laid = piece_density(centre, piece_in(in), bell, in.dot_allowed == 1u);
    return vec4<f32>(laid.x, 0.0, 0.0, 0.0);
}

@fragment
fn contact_fragment(in: PieceOut) -> @location(0) vec4<f32> {
    let centre = point_of(pixel_of(in.position, in.place), in.frame);
    let bell = u32(word(in.op, SEGMENT_TIP)) == TIP_BELL;
    let laid = piece_density(centre, piece_in(in), bell, in.dot_allowed == 1u);
    return vec4<f32>(laid.y, 0.0, 0.0, 0.0);
}

struct KeptOut {
    @builtin(position) position: vec4<f32>,
    // The page pixel of an atlas pixel, less the atlas pixel; the 32-px cells of its slot's tile
    // holding kept coverage (8 a row, two words).
    @location(0) @interpolate(flat) shift: vec2<i32>,
    @location(1) @interpolate(flat) cells: vec2<u32>,
}

// S64: the atlas rectangle `b` (its corners) of a stroke's kept coverage in a tile, read from its
// slot on the page bound (`a.zw`: the shift to it, `a.xy`: the cells it holds).
@vertex
fn kept_vertex(
    @builtin(vertex_index) corner: u32,
    @location(0) a: vec4<u32>,
    @location(1) b: vec4<i32>,
) -> KeptOut {
    let side = corner_side(corner) * 0.5 + vec2<f32>(0.5);
    let p = (vec2<f32>(b.xy) + side * vec2<f32>(b.zw - b.xy)) / raster.atlas;
    var out: KeptOut;
    out.position = vec4<f32>(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
    out.shift = bitcast<vec2<i32>>(a.zw);
    out.cells = a.xy;
    return out;
}

// The kept coverage, written whole: as the slot holds it in the cells it kept, else nothing.
@fragment
fn kept_fragment(in: KeptOut) -> @location(0) vec4<f32> {
    let at = vec2<i32>(floor(in.position.xy)) + in.shift;
    let cell = vec2<u32>(at % vec2<i32>(256)) / 32u;
    let index = cell.y * 8u + cell.x;
    let bits = select(in.cells.x, in.cells.y, index >= 32u);
    if ((bits >> (index % 32u)) & 1u) == 0u {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(textureLoad(source, at, 0).r, 0.0, 0.0, 0.0);
}

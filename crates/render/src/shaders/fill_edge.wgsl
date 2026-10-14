// Fill edges (S30, D56): a fill's coverage of an output pixel at every scale, from its mask's
// bits and the edge values the scene derived from them (`fill_edge.rs`). Appended to the
// evaluator.
//
// The mask's signed distance field (at each pixel centre the distance to the nearest centre of
// the other side less half a pixel, positive inside) is 1/2 or sqrt(2) - 1/2 at the centres
// with one of the other side among their 8 neighbours, the edge's, and only its side counts
// further away (`EDGE_FAR`). The scene keeps, per edge centre, that field averaged along the
// edge; bilinear between centres, its zero line is the fill's edge. A pixel's coverage is the
// share of its footprint inside that line, from 4 x 4 subsamples each ramped over its own
// width: antialiased over one output pixel at every scale. A cell whose corners lie on both
// sides has only edge centres for corners, so where all nine centres around a pixel lie inside
// (outside) the coverage is exactly 1 (0): a fill's inside is exactly its colour. Past the
// canvas's edge the mask continues as its edge pixels, so a fill reaching it keeps its corners.

const EDGE_SAMPLES: u32 = 4u;
// The bits of the 3 x 3 centres around the pixel in the 5 x 5 window (bit `5 y + x`).
const EDGE_INNER: u32 = (7u << 6u) | (7u << 11u) | (7u << 16u);
const EDGE_FAR: f32 = 1.5;
const EDGE_STEPS: f32 = 8192.0;

// A mask in the mask words: where it starts, its rectangle and its row stride in words.
struct MaskRect {
    at: u32,
    origin: vec2<i32>,
    size: vec2<i32>,
    stride: u32,
}

// A fill's coverage at the output pixel whose centre is `centre` (canvas pixels): its smoothed
// edge, or with `FILL_EDGE` 0 (a transform's lift) its mask bit at `cell`.
fn fill_coverage(centre: vec2<f32>, cell: vec2<u32>, op: u32) -> f32 {
    let at = u32(word(op, FILL_MASK)) * 65536u + u32(word(op, FILL_MASK + 1u));
    let canvas = vec2<i32>(i32(word(op, FILL_EDGE)), i32(word(op, FILL_EDGE + 1u)));
    if canvas.x == 0 {
        return mask_bit(cell, at);
    }
    let rect = MaskRect(
        at,
        vec2<i32>(i32(masks[at]), i32(masks[at + 1u])),
        vec2<i32>(i32(masks[at + 2u]), i32(masks[at + 3u])),
        (masks[at + 2u] + 31u) / 32u,
    );
    // Centres at whole numbers; `first` is the centre of value 0 of the 3 x 3 the footprint
    // lies in; the window's centres run from `first - 1` to `first + 3`.
    let grid = centre - vec2<f32>(0.5);
    let first = vec2<i32>(floor(grid - vec2<f32>(0.5)));
    var bits = 0u;
    for (var k = 0u; k < 25u; k++) {
        if ((EDGE_INNER >> k) & 1u) == 1u {
            bits |= edge_bit(rect, first + vec2<i32>(i32(k % 5u) - 1, i32(k / 5u) - 1), canvas) << k;
        }
    }
    if bits == EDGE_INNER {
        return 1.0;
    }
    if bits == 0u {
        return 0.0;
    }
    for (var k = 0u; k < 25u; k++) {
        if ((EDGE_INNER >> k) & 1u) == 0u {
            bits |= edge_bit(rect, first + vec2<i32>(i32(k % 5u) - 1, i32(k / 5u) - 1), canvas) << k;
        }
    }
    var values: array<f32, 9>;
    for (var k = 0u; k < 9u; k++) {
        let point = first + vec2<i32>(i32(k % 3u), i32(k / 3u));
        values[k] = edge_value(rect, bits, (k / 3u + 1u) * 5u + k % 3u + 1u, point, canvas);
    }
    return edge_share(values, grid - vec2<f32>(first));
}

// The mask's bit at canvas pixel `pixel`, past the canvas's edge its nearest edge pixel's.
fn edge_bit(rect: MaskRect, pixel: vec2<i32>, canvas: vec2<i32>) -> u32 {
    let local = clamp(pixel, vec2<i32>(0), canvas - vec2<i32>(1)) - rect.origin;
    if any(local < vec2<i32>(0)) || any(local >= rect.size) {
        return 0u;
    }
    let x = u32(local.x);
    let word = masks[rect.at + MASK_HEADER + u32(local.y) * rect.stride + x / 32u];
    return (word >> (x % 32u)) & 1u;
}

// The edge field at centre `point`, window bit `c` (one of the inner 3 x 3): `EDGE_FAR` on its
// side without one of the other side among its 8 neighbours, else the scene's value there.
fn edge_value(rect: MaskRect, bits: u32, c: u32, point: vec2<i32>, canvas: vec2<i32>) -> f32 {
    let inside = (bits >> c) & 1u;
    var other = bits;
    if inside == 1u {
        other = ~bits;
    }
    let side = (other >> (c - 1u)) | (other >> (c + 1u)) | (other >> (c - 5u)) | (other >> (c + 5u));
    let corner = (other >> (c - 6u)) | (other >> (c - 4u)) | (other >> (c + 4u)) | (other >> (c + 6u));
    if ((side | corner) & 1u) == 0u {
        return select(-EDGE_FAR, EDGE_FAR, inside == 1u);
    }
    return edge_lookup(rect, clamp(point, vec2<i32>(0), canvas - vec2<i32>(1)));
}

// The scene's value at edge centre `point`: after the mask's rows, per row of the rectangle
// widened by one pixel where its values start (and the next row's, where they end), then the
// values, each its column in the widened rectangle times 2^16 plus its value in `EDGE_STEPS`
// per pixel about 2^15, in column order.
fn edge_lookup(rect: MaskRect, point: vec2<i32>) -> f32 {
    let local = point - rect.origin + vec2<i32>(1);
    let table = rect.at + MASK_HEADER + rect.stride * u32(rect.size.y);
    let values = table + u32(rect.size.y) + 3u;
    let column = u32(local.x);
    var low = masks[table + u32(local.y)];
    var high = masks[table + u32(local.y) + 1u];
    while low < high {
        let middle = (low + high) / 2u;
        let word = masks[values + middle];
        let found = word >> 16u;
        if found == column {
            return (f32(word & 0xffffu) - 32768.0) / EDGE_STEPS;
        }
        if found < column {
            low = middle + 1u;
        } else {
            high = middle;
        }
    }
    // Every edge centre has its value; unreachable.
    return 0.0;
}

// The share of the output pixel at `local` (from centre `first` of the 3 x 3 `values`) inside
// the edge: each subsample's field over its width, from half outside to half inside.
fn edge_share(values: array<f32, 9>, local: vec2<f32>) -> f32 {
    var field = values;
    // The output pixel in the fill's pixels; shrunk below one (a transform record shrinking
    // it) the window holds one pixel's footprint, so the edge ramps over a fill pixel.
    let half = 0.5 * min(foot_inverse, 1.0);
    let ramp = f32(EDGE_SAMPLES) * max(foot_scale, 1.0);
    var sum = 0.0;
    for (var k = 0u; k < EDGE_SAMPLES * EDGE_SAMPLES; k++) {
        let step = vec2<f32>(f32(k % EDGE_SAMPLES), f32(k / EDGE_SAMPLES));
        let offset = ((2.0 * step + vec2<f32>(1.0)) / f32(EDGE_SAMPLES) - vec2<f32>(1.0)) * half;
        let p = local + offset;
        let cell = min(vec2<u32>(floor(p)), vec2<u32>(1u));
        let t = p - vec2<f32>(cell);
        let i = cell.y * 3u + cell.x;
        let top = mix(field[i], field[i + 1u], t.x);
        let bottom = mix(field[i + 3u], field[i + 4u], t.x);
        sum += clamp(0.5 + mix(top, bottom, t.y) * ramp, 0.0, 1.0);
    }
    return sum / f32(EDGE_SAMPLES * EDGE_SAMPLES);
}

// A filter applied to its layer at one pixel (S33, D59): inside its clip the layer so far becomes
// the filter's result (`filter.wgsl` wrote it into a block of the arena: its rectangle, then one
// premultiplied RGBA8 word per canvas pixel) at `point`, which is a canvas pixel's centre on the
// canvas itself (that pixel's word exactly) and between centres at 2x and up (bilinear: a
// result at canvas resolution, enlarged). Pixels past the canvas take its edge pixel, as the
// filter read them. Outside its block the layer was transparent before the filter and stays.

fn filtered(layer: vec4<f32>, point: vec2<f32>, cell: vec2<u32>, op: Op) -> vec4<f32> {
    if clip(cell, op) == 0.0 {
        return layer;
    }
    let base = masks[u32(op.data[FILTER_AT]) * 65536u + u32(op.data[FILTER_AT + 1u])];
    let origin = vec2<i32>(i32(arena[base]), i32(arena[base + 1u]));
    let size = vec2<i32>(i32(arena[base + 2u]), i32(arena[base + 3u]));
    let at = vec2<i32>(cell);
    if any(at < origin) || any(at >= origin + size) {
        return layer;
    }
    let last = vec2<i32>(i32(op.data[FILTER_CANVAS]), i32(op.data[FILTER_CANVAS + 1u])) - 1;
    let local = point - vec2<f32>(0.5);
    let corner = floor(local);
    let t = local - corner;
    let first = vec2<i32>(corner);
    var value = vec4<f32>(0.0);
    for (var k = 0u; k < 4u; k++) {
        let step = vec2<i32>(i32(k & 1u), i32(k >> 1u));
        let wx = select(1.0 - t.x, t.x, step.x == 1);
        let wy = select(1.0 - t.y, t.y, step.y == 1);
        let weight = wx * wy;
        if weight > 0.0 {
            let node = clamp(first + step, vec2<i32>(0), last) - origin;
            if all(node >= vec2<i32>(0)) && all(node < size) {
                let word = arena[base + MASK_HEADER + u32(node.y * size.x + node.x)];
                value += weight * unpack4x8unorm(word);
            }
        }
    }
    return value;
}

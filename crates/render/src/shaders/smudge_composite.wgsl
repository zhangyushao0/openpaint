// A smudge applied to its layer at one pixel (DECISIONS D46): the layer becomes A * layer + R
// from the smudge's residual (smudge.wgsl evaluated it) at the pixel's centre. The residual's
// nodes sit at canvas pixel centres; between them A and R are bilinear (S29: an output pixel
// at 2x or 4x), and at a node, as on the canvas itself, that node alone, exactly. A finished
// smudge's residual is a block of the arena, whose base its op finds in a mask word; the
// in-progress one's is the evaluation's own. Where the residual holds no node, or the node is
// still the identity, the layer stays exactly as it was, as the scene's bounds require; so it
// does outside the selection the smudge was drawn inside.

fn residual_word(drawing: bool, at: u32) -> u32 {
    if drawing {
        return active_residual[at];
    }
    return arena[at];
}

// A residual node's A and R; the identity (1, 0) where the residual holds none.
struct Node {
    a: f32,
    r: vec4<f32>,
}

fn residual_node(drawing: bool, base: u32, at: vec2<i32>) -> Node {
    let identity = Node(1.0, vec4<f32>(0.0));
    if any(at < vec2<i32>(0)) {
        return identity;
    }
    let node = vec2<u32>(at);
    let tile = node / RESIDUAL_TILE;
    let origin = vec2<u32>(residual_word(drawing, base), residual_word(drawing, base + 1u));
    let size = vec2<u32>(residual_word(drawing, base + 2u), residual_word(drawing, base + 3u));
    if any(tile < origin) || any(tile >= origin + size) {
        return identity;
    }
    let local_tile = tile - origin;
    let slot = residual_word(drawing, base + RESIDUAL_HEADER + local_tile.y * size.x + local_tile.x);
    if slot == 0u {
        return identity;
    }
    let local = node % RESIDUAL_TILE;
    let index = (slot - 1u) * TILE_NODES + local.y * RESIDUAL_TILE + local.x;
    let w = base + RESIDUAL_HEADER + size.x * size.y + index * NODE_WORDS;
    return Node(
        bitcast<f32>(residual_word(drawing, w)),
        vec4<f32>(
            bitcast<f32>(residual_word(drawing, w + 1u)),
            bitcast<f32>(residual_word(drawing, w + 2u)),
            bitcast<f32>(residual_word(drawing, w + 3u)),
            bitcast<f32>(residual_word(drawing, w + 4u)),
        ),
    );
}

// `centre` in canvas pixels; `cell` the canvas pixel holding it, for the clip.
fn smudged(before: vec4<f32>, centre: vec2<f32>, cell: vec2<u32>, op: Op) -> vec4<f32> {
    if clip(cell, op) == 0.0 {
        return before;
    }
    // The in-progress smudge reads its own residual from word 0.
    let drawing = op.data[SMUDGE_ACTIVE] != 0.0;
    var base = 0u;
    if !drawing {
        base = masks[u32(op.data[SMUDGE_AT]) * 65536u + u32(op.data[SMUDGE_AT + 1u])];
    }
    // The four nodes around the centre, each by its bilinear weight; a zero weight reads none.
    let local = centre - vec2<f32>(0.5);
    let corner = floor(local);
    let t = local - corner;
    let first = vec2<i32>(corner);
    var a = 0.0;
    var r = vec4<f32>(0.0);
    for (var k = 0u; k < 4u; k++) {
        let step = vec2<u32>(k & 1u, k >> 1u);
        let wx = select(1.0 - t.x, t.x, step.x == 1u);
        let wy = select(1.0 - t.y, t.y, step.y == 1u);
        let weight = wx * wy;
        if weight > 0.0 {
            let node = residual_node(drawing, base, first + vec2<i32>(step));
            a += weight * node.a;
            r += weight * node.r;
        }
    }
    if a == 1.0 && all(r == vec4<f32>(0.0)) {
        return before;
    }
    var after = a * before + r;
    if op.data[SMUDGE_LOCKED] != 0.0 {
        // Alpha Lock keeps the layer's alpha and moves only its colour.
        var rgb = before.rgb;
        if after.a > 1e-6 {
            rgb = after.rgb / after.a * before.a;
        }
        after = vec4<f32>(rgb, before.a);
    }
    return after;
}

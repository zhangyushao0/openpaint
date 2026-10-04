// A smudge applied to its layer at one pixel (DECISIONS D46): the layer becomes A * layer + R
// from the smudge's residual node at the pixel (smudge.wgsl evaluated it). A finished smudge's
// residual is a block of the arena, whose base its op finds in a mask word; the in-progress
// one's is the evaluation's own. Where the residual holds no node, or the node is still the
// identity, the layer stays exactly as it was, as the scene's bounds require; so it does
// outside the selection the smudge was drawn inside.

fn residual_word(drawing: bool, at: u32) -> u32 {
    if drawing {
        return active_residual[at];
    }
    return arena[at];
}

fn smudged(before: vec4<f32>, pixel: vec2<u32>, op: Op) -> vec4<f32> {
    // The in-progress smudge reads its own residual from word 0.
    let drawing = op.data[SMUDGE_ACTIVE] != 0.0;
    var base = 0u;
    if !drawing {
        base = masks[u32(op.data[SMUDGE_AT]) * 65536u + u32(op.data[SMUDGE_AT + 1u])];
    }
    let tile = pixel / RESIDUAL_TILE;
    let origin = vec2<u32>(residual_word(drawing, base), residual_word(drawing, base + 1u));
    let size = vec2<u32>(residual_word(drawing, base + 2u), residual_word(drawing, base + 3u));
    if any(tile < origin) || any(tile >= origin + size) {
        return before;
    }
    let at = tile - origin;
    let slot = residual_word(drawing, base + RESIDUAL_HEADER + at.y * size.x + at.x);
    if slot == 0u {
        return before;
    }
    let local = pixel % RESIDUAL_TILE;
    let node = (slot - 1u) * TILE_NODES + local.y * RESIDUAL_TILE + local.x;
    let w = base + RESIDUAL_HEADER + size.x * size.y + node * NODE_WORDS;
    let a = bitcast<f32>(residual_word(drawing, w));
    let r = vec4<f32>(
        bitcast<f32>(residual_word(drawing, w + 1u)),
        bitcast<f32>(residual_word(drawing, w + 2u)),
        bitcast<f32>(residual_word(drawing, w + 3u)),
        bitcast<f32>(residual_word(drawing, w + 4u)),
    );
    if (a == 1.0 && all(r == vec4<f32>(0.0))) || clip(pixel, op) == 0.0 {
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

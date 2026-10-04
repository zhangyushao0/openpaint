// Smudge contacts on a residual being evaluated (smudge/residual.rs, DECISIONS D46), after
// draweave's smudge.wgsl at one node per canvas pixel. A node at a pixel's centre holds A, the
// share of the layer's colour before the smudge (`material`) still there, and R, the colour
// carried in: its colour is A * material + R. One contact moves the share w of the colour a step
// back to where it is: `stamp` reads only the residual before the contact and writes the new
// values to `scratch`, `commit` stores them, so every node sees the same old values.
//
// The protocol's constants (RESIDUAL_TILE, TILE_NODES, NODE_WORDS, RESIDUAL_HEADER) are
// prepended. A residual's words: the header (first tile column and row, table width and height
// in tiles), the tile table (slot + 1 per tile, 0: none), then the nodes, slot by slot.

struct Contact {
    // Nodes x, y, width, height: those the contact changes (a reset: one tile).
    rect: vec4<u32>,
    centre: vec2<f32>,
    step: vec2<f32>,
    radius: f32,
    strength: f32,
    kernel: u32,
    pad: u32,
    canvas: vec2<u32>,
    pad2: vec2<u32>,
}

@group(0) @binding(0) var<uniform> contact: Contact;
@group(0) @binding(1) var<storage, read_write> words: array<u32>;
@group(0) @binding(2) var<storage, read_write> scratch: array<f32>;
@group(0) @binding(3) var material: texture_2d<f32>;

// The node at `c` as an index among the residual's nodes, or -1 outside the canvas or in a tile
// the residual does not hold.
fn node_index(c: vec2<i32>) -> i32 {
    if any(c < vec2<i32>(0)) || any(vec2<u32>(c) >= contact.canvas) {
        return -1;
    }
    let tile = vec2<u32>(c) / RESIDUAL_TILE;
    let origin = vec2<u32>(words[0], words[1]);
    let size = vec2<u32>(words[2], words[3]);
    if any(tile < origin) || any(tile >= origin + size) {
        return -1;
    }
    let at = tile - origin;
    let slot = words[RESIDUAL_HEADER + at.y * size.x + at.x];
    if slot == 0u {
        return -1;
    }
    let local = vec2<u32>(c) % RESIDUAL_TILE;
    return i32((slot - 1u) * TILE_NODES + local.y * RESIDUAL_TILE + local.x);
}

fn node_word(i: i32) -> u32 {
    return RESIDUAL_HEADER + words[2] * words[3] + u32(i) * NODE_WORDS;
}

fn kept(i: i32) -> f32 {
    return bitcast<f32>(words[node_word(i)]);
}

fn carried(i: i32) -> vec4<f32> {
    let w = node_word(i);
    return vec4<f32>(
        bitcast<f32>(words[w + 1u]),
        bitcast<f32>(words[w + 2u]),
        bitcast<f32>(words[w + 3u]),
        bitcast<f32>(words[w + 4u]),
    );
}

fn store(i: i32, a: f32, r: vec4<f32>) {
    let w = node_word(i);
    words[w] = bitcast<u32>(a);
    words[w + 1u] = bitcast<u32>(r.x);
    words[w + 2u] = bitcast<u32>(r.y);
    words[w + 3u] = bitcast<u32>(r.z);
    words[w + 4u] = bitcast<u32>(r.w);
}

// The colour now at node `c`; transparent beyond the canvas.
fn color_at(c: vec2<i32>) -> vec4<f32> {
    let i = node_index(c);
    if i < 0 {
        return vec4<f32>(0.0);
    }
    return kept(i) * textureLoad(material, c, 0) + carried(i);
}

// The colour at point `q` in pixels, bilinear between the four nodes around it.
fn field(q: vec2<f32>) -> vec4<f32> {
    let g = q - vec2<f32>(0.5);
    let a = vec2<i32>(floor(g));
    let t = g - floor(g);
    let x = vec2<i32>(1, 0);
    let y = vec2<i32>(0, 1);
    return mix(
        mix(color_at(a), color_at(a + x), t.x),
        mix(color_at(a + y), color_at(a + x + y), t.x),
        t.y,
    );
}

// The contact's weight at `p`: its tip's profile over the squared distance in radii, times its
// strength; 0 from the radius on.
fn weight(p: vec2<f32>) -> f32 {
    let delta = p - contact.centre;
    let q = dot(delta, delta) / (contact.radius * contact.radius);
    if q >= 1.0 {
        return 0.0;
    }
    var k: f32;
    switch contact.kernel {
        case 0u: { k = 1.0 - smoothstep(0.64, 1.0, q); }
        case 1u: { k = (1.0 - q) * (1.0 - q); }
        case 2u: { k = (1.0 - q) * (1.0 - q) * (1.0 - q); }
        default: { k = 1.0 - q * q * q * q; }
    }
    return contact.strength * k;
}

// A tile just allocated: every node keeps all of the material and carries nothing.
@compute @workgroup_size(8, 8)
fn reset(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= contact.rect.zw) {
        return;
    }
    let i = node_index(vec2<i32>(contact.rect.xy + id.xy));
    if i >= 0 {
        store(i, 1.0, vec4<f32>(0.0));
    }
}

@compute @workgroup_size(8, 8)
fn stamp(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= contact.rect.zw) {
        return;
    }
    let c = vec2<i32>(contact.rect.xy + id.xy);
    let s = (id.y * contact.rect.z + id.x) * NODE_WORDS;
    let i = node_index(c);
    if i < 0 {
        return;
    }
    let old_a = kept(i);
    let old_r = carried(i);
    let p = vec2<f32>(c) + vec2<f32>(0.5);
    let w = weight(p);
    var a = old_a;
    var r = old_r;
    if w > 0.0 {
        a = (1.0 - w) * old_a;
        r = (1.0 - w) * old_r + w * field(p - contact.step);
    }
    scratch[s] = a;
    scratch[s + 1u] = r.x;
    scratch[s + 2u] = r.y;
    scratch[s + 3u] = r.z;
    scratch[s + 4u] = r.w;
}

@compute @workgroup_size(8, 8)
fn commit(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= contact.rect.zw) {
        return;
    }
    let i = node_index(vec2<i32>(contact.rect.xy + id.xy));
    if i < 0 {
        return;
    }
    let s = (id.y * contact.rect.z + id.x) * NODE_WORDS;
    store(i, scratch[s], vec4<f32>(scratch[s + 1u], scratch[s + 2u], scratch[s + 3u], scratch[s + 4u]));
}

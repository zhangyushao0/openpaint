// The gallery thumbnail: each thumbnail pixel is the rounded mean of the canvas pixels in its
// box, in 8-bit integers, so the CPU can reproduce it exactly (`thumbnail.rs`). A ten-bit
// canvas's texel rounds to the 8-bit value its code stands for (S37, `readback::eight_bits`).
//
// Box of thumbnail pixel (x, y): canvas columns floor(x * W / w) .. floor((x + 1) * W / w),
// rows likewise; never empty because w <= W and h <= H.

struct Sizes {
    // Canvas width and height, thumbnail width and height.
    canvas: vec2<u32>,
    thumbnail: vec2<u32>,
}

@group(0) @binding(0) var canvas: texture_2d<f32>;
@group(0) @binding(1) var<uniform> sizes: Sizes;
@group(0) @binding(2) var<storage, read_write> pixels: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn thumbnail(@builtin(global_invocation_id) id: vec3<u32>) {
    let at = id.xy;
    if (at.x >= sizes.thumbnail.x || at.y >= sizes.thumbnail.y) {
        return;
    }
    let first = at * sizes.canvas / sizes.thumbnail;
    let end = (at + vec2<u32>(1u, 1u)) * sizes.canvas / sizes.thumbnail;
    var sum = vec4<u32>(0u, 0u, 0u, 0u);
    for (var y = first.y; y < end.y; y++) {
        for (var x = first.x; x < end.x; x++) {
            let texel = textureLoad(canvas, vec2<i32>(i32(x), i32(y)), 0);
            sum += vec4<u32>(round(texel * 255.0));
        }
    }
    let count = (end.x - first.x) * (end.y - first.y);
    let mean = (sum + vec4<u32>(count / 2u)) / count;
    pixels[at.y * sizes.thumbnail.x + at.x] =
        mean.r | (mean.g << 8u) | (mean.b << 16u) | (mean.a << 24u);
}

// The replay video's scale pass (`video.rs`): each video pixel is the mean of the canvas pixels
// it covers, each weighted by how much of it the video pixel covers, in integers, so the CPU
// reproduces it exactly (`video_reference`) and a video of the canvas's own size is the canvas.
// A ten-bit canvas's texel rounds to the 8-bit value its code stands for, as the thumbnail's.
//
// Along an axis of n canvas and m video pixels, in units of 1/m of a canvas pixel: video pixel o
// spans [o n, (o + 1) n), canvas pixel i spans [i m, (i + 1) m); their overlap is i's weight, and
// o's weights sum to n. A row's weighted sum is kept in 1/256 of a level before the column's sum:
// every sum stays below 2^32 for sides up to 16384 (`MAX_CANVAS_SIDE`).

struct Sizes {
    canvas: vec2<u32>,
    video: vec2<u32>,
}

@group(0) @binding(0) var canvas: texture_2d<f32>;
@group(0) @binding(1) var<uniform> sizes: Sizes;
@group(0) @binding(2) var<storage, read_write> pixels: array<u32>;

// How much of canvas pixel `i` video pixel `o` covers along an axis of `n` canvas and `m` video
// pixels, in units of 1/m of a canvas pixel.
fn overlap(o: u32, i: u32, n: u32, m: u32) -> u32 {
    let start = max(o * n, i * m);
    let end = min((o + 1u) * n, (i + 1u) * m);
    return select(0u, end - start, end > start);
}

@compute @workgroup_size(8, 8, 1)
fn scale(@builtin(global_invocation_id) id: vec3<u32>) {
    let at = id.xy;
    let n = sizes.canvas;
    let m = sizes.video;
    if (at.x >= m.x || at.y >= m.y) {
        return;
    }
    let first = at * n / m;
    let end = ((at + vec2<u32>(1u, 1u)) * n + m - vec2<u32>(1u, 1u)) / m;
    var total = vec4<u32>(0u, 0u, 0u, 0u);
    for (var y = first.y; y < end.y; y++) {
        var row = vec4<u32>(0u, 0u, 0u, 0u);
        for (var x = first.x; x < end.x; x++) {
            let texel = textureLoad(canvas, vec2<i32>(vec2<u32>(x, y)), 0);
            row += vec4<u32>(round(texel * 255.0)) * overlap(at.x, x, n.x, m.x);
        }
        let fixed = (row * 256u + vec4<u32>(n.x / 2u)) / n.x;
        total += fixed * overlap(at.y, y, n.y, m.y);
    }
    let mean = (total + vec4<u32>(n.y * 128u)) / (n.y * 256u);
    pixels[at.y * m.x + at.x] = mean.r | (mean.g << 8u) | (mean.b << 16u) | (mean.a << 24u);
}

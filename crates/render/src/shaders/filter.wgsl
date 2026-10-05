// Gaussian Blur and Sharpen (S33, D59): a filter's result from its layer's pixels before it
// (`material`, premultiplied, at canvas resolution) into its block of the arena, one band of
// rows at a time. `across` sums each row the band reads (the kernel's reach beyond its output
// rows each way) into `band`; `down` sums the columns of that and writes the band's rows of the
// block, premultiplied RGBA8, rounded here so every backend stores the same words. A blur is
// that sum; sharpening adds `strength` times the pixel's difference from it and keeps the
// result valid premultiplied colour. Reads past the canvas take its edge pixel.

struct Job {
    // The block: x, y, width, height in canvas pixels.
    block: vec4<u32>,
    // The first row the band holds, its row count, the first row written, the rows written.
    rows: vec4<u32>,
    canvas: vec2<u32>,
    reach: u32,
    // Where the block's pixels start in the arena, after the header `Filters` writes.
    base: u32,
    strength: f32,
}

@group(0) @binding(0) var<uniform> job: Job;
@group(0) @binding(1) var<storage, read> weights: array<f32>;
@group(0) @binding(2) var material: texture_2d<f32>;
@group(0) @binding(3) var<storage, read_write> band: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> arena: array<u32>;

@compute @workgroup_size(16, 16)
fn across(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= job.block.z || id.y >= job.rows.y {
        return;
    }
    let y = i32(job.rows.x + id.y);
    let x = i32(job.block.x + id.x);
    let last = i32(job.canvas.x) - 1;
    let reach = i32(job.reach);
    var sum = vec4<f32>(0.0);
    for (var k = -reach; k <= reach; k++) {
        let at = vec2<i32>(clamp(x + k, 0, last), y);
        sum += weights[u32(k + reach)] * textureLoad(material, at, 0);
    }
    band[id.y * job.block.z + id.x] = sum;
}

@compute @workgroup_size(16, 16)
fn down(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= job.block.z || id.y >= job.rows.w {
        return;
    }
    let y = i32(job.rows.z + id.y);
    let last = i32(job.canvas.y) - 1;
    let reach = i32(job.reach);
    var blurred = vec4<f32>(0.0);
    for (var k = -reach; k <= reach; k++) {
        let row = u32(clamp(y + k, 0, last) - i32(job.rows.x));
        blurred += weights[u32(k + reach)] * band[row * job.block.z + id.x];
    }
    var value = blurred;
    if job.strength > 0.0 {
        let here = textureLoad(material, vec2<i32>(i32(job.block.x + id.x), y), 0);
        value = here + job.strength * (here - blurred);
    }
    let alpha = clamp(value.a, 0.0, 1.0);
    let kept = vec4<f32>(clamp(value.rgb, vec3<f32>(0.0), vec3<f32>(alpha)), alpha);
    let local = (u32(y) - job.block.y) * job.block.z + id.x;
    arena[job.base + local] = packed(kept);
}


// The 8-bit value nearest each channel, low byte red.
fn packed(colour: vec4<f32>) -> u32 {
    let k = vec4<u32>(floor(colour * 255.0 + vec4<f32>(0.5)));
    return k.r | (k.g << 8u) | (k.b << 16u) | (k.a << 24u);
}

// Gaussian Blur and Sharpen (S33, D59; S35): a filter's result from its layer's pixels before it
// (`material`, premultiplied, at canvas resolution) into its block of the arena. A blur of a
// large deviation is evaluated on cells of `factor` x `factor` canvas pixels (their mean), its
// deviation reduced to match, and enlarged bilinearly onto the block's pixels (`enlarge`); at
// factor 1 the cells are the pixels and `down` writes the block itself. Per band of cell rows
// `across` sums each row the band reads (the kernel's reach beyond its rows each way) into
// `band`; `down` sums its columns. Both read their neighbours from workgroup memory, loaded once
// per workgroup. Results are premultiplied RGBA8, rounded here so every backend stores the same
// words; sharpening adds `strength` times the pixel's difference from the blur and keeps the
// result valid premultiplied colour. Reads past the canvas take its edge pixel or cell.

struct Job {
    // The block: x, y, width, height in canvas pixels.
    block: vec4<u32>,
    // The cells the block's result needs: first column, first row, columns, rows.
    cells: vec4<u32>,
    // The first cell row the band holds, its row count, the first row written, the rows written.
    rows: vec4<u32>,
    // The canvas in pixels, then in cells.
    canvas: vec4<u32>,
    // The kernel's reach in cells, where the block's pixels start in the arena, the sharpening
    // strength's bits (0: a blur), canvas pixels per cell.
    kernel: vec4<u32>,
}

@group(0) @binding(0) var<uniform> job: Job;
@group(0) @binding(1) var<storage, read> weights: array<f32>;
@group(0) @binding(2) var material: texture_2d<f32>;
@group(0) @binding(3) var<storage, read_write> band: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> arena: array<u32>;
// At a factor above 1: the blurred cells, `cells.z` to a row.
@group(0) @binding(5) var<storage, read_write> blurred_cells: array<vec4<f32>>;

// The longest reach in cells; the generated constant is prepended.
const SPAN: u32 = 16u + 2u * MAX_CELL_REACH;
var<workgroup> line: array<vec4<f32>, 16u * SPAN>;

// The mean of the cell's pixels, the cell taken inside the canvas.
fn cell(x: i32, y: i32) -> vec4<f32> {
    let factor = i32(job.kernel.w);
    let at = vec2<i32>(clamp(x, 0, i32(job.canvas.z) - 1), clamp(y, 0, i32(job.canvas.w) - 1));
    if factor == 1 {
        return textureLoad(material, at, 0);
    }
    let last = vec2<i32>(job.canvas.xy) - 1;
    var sum = vec4<f32>(0.0);
    for (var j = 0; j < factor; j++) {
        for (var i = 0; i < factor; i++) {
            sum += textureLoad(material, min(at * factor + vec2<i32>(i, j), last), 0);
        }
    }
    return sum / f32(factor * factor);
}

@compute @workgroup_size(16, 16)
fn across(
    @builtin(workgroup_id) group: vec3<u32>,
    @builtin(local_invocation_id) local: vec3<u32>,
) {
    let reach = job.kernel.x;
    let x = i32(job.cells.x + group.x * 16u + local.x);
    let row = group.y * 16u + local.y;
    let y = i32(job.rows.x + min(row, job.rows.y - 1u));
    let first = i32(job.cells.x + group.x * 16u) - i32(reach);
    for (var i = local.x; i < 16u + 2u * reach; i += 16u) {
        line[local.y * SPAN + i] = cell(first + i32(i), y);
    }
    workgroupBarrier();
    if u32(x) >= job.cells.x + job.cells.z || row >= job.rows.y {
        return;
    }
    var sum = vec4<f32>(0.0);
    for (var k = 0u; k <= 2u * reach; k++) {
        sum += weights[k] * line[local.y * SPAN + local.x + k];
    }
    band[row * job.cells.z + u32(x) - job.cells.x] = sum;
}

@compute @workgroup_size(16, 16)
fn down(
    @builtin(workgroup_id) group: vec3<u32>,
    @builtin(local_invocation_id) local: vec3<u32>,
) {
    let reach = job.kernel.x;
    let column = min(group.x * 16u + local.x, job.cells.z - 1u);
    let first = i32(job.rows.z + group.y * 16u) - i32(reach);
    let lowest = i32(job.canvas.w) - 1;
    for (var i = local.y; i < 16u + 2u * reach; i += 16u) {
        let at = u32(clamp(first + i32(i), 0, lowest)) - job.rows.x;
        line[i * 16u + local.x] = band[min(at, job.rows.y - 1u) * job.cells.z + column];
    }
    workgroupBarrier();
    let written = group.y * 16u + local.y;
    if group.x * 16u + local.x >= job.cells.z || written >= job.rows.w {
        return;
    }
    var sum = vec4<f32>(0.0);
    for (var k = 0u; k <= 2u * reach; k++) {
        sum += weights[k] * line[(local.y + k) * 16u + local.x];
    }
    let y = job.rows.z + written;
    if job.kernel.w > 1u {
        blurred_cells[(y - job.cells.y) * job.cells.z + column] = sum;
        return;
    }
    let x = job.cells.x + column;
    var value = sum;
    let strength = bitcast<f32>(job.kernel.z);
    if strength > 0.0 {
        let here = textureLoad(material, vec2<u32>(x, y), 0);
        value = here + strength * (here - sum);
    }
    arena[job.kernel.y + (y - job.block.y) * job.block.z + x - job.block.x] = packed(kept(value));
}

// The blurred cells enlarged onto the block's pixels: bilinear between cell centres, the cells
// taken inside those the block needs.
@compute @workgroup_size(16, 16)
fn enlarge(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= job.block.z || id.y >= job.block.w {
        return;
    }
    let pixel = job.block.xy + id.xy;
    let at = (vec2<f32>(pixel) + vec2<f32>(0.5)) / f32(job.kernel.w) - vec2<f32>(0.5);
    let corner = floor(at);
    let t = at - corner;
    let low = vec2<i32>(job.cells.xy);
    let high = low + vec2<i32>(job.cells.zw) - 1;
    var value = vec4<f32>(0.0);
    for (var k = 0u; k < 4u; k++) {
        let step = vec2<i32>(i32(k & 1u), i32(k >> 1u));
        let w = select(1.0 - t, t, step == vec2<i32>(1));
        let node = vec2<u32>(clamp(vec2<i32>(corner) + step, low, high) - low);
        value += w.x * w.y * blurred_cells[node.y * job.cells.z + node.x];
    }
    let local = id.y * job.block.z + id.x;
    arena[job.kernel.y + local] = packed(kept(value));
}

// Valid premultiplied colour: alpha in 0..1, each channel within it.
fn kept(value: vec4<f32>) -> vec4<f32> {
    let alpha = clamp(value.a, 0.0, 1.0);
    return vec4<f32>(clamp(value.rgb, vec3<f32>(0.0), vec3<f32>(alpha)), alpha);
}

// The 8-bit value nearest each channel, low byte red.
fn packed(colour: vec4<f32>) -> u32 {
    let k = vec4<u32>(floor(colour * 255.0 + vec4<f32>(0.5)));
    return k.r | (k.g << 8u) | (k.b << 16u) | (k.a << 24u);
}

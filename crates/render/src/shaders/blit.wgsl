// Shows the canvas on screen at one screen pixel per canvas pixel. The pass sets the viewport to
// the canvas rectangle on screen; one triangle covers it, and each pixel loads its texel
// unfiltered, so the screen shows the evaluated bytes unchanged.

@group(0) @binding(0) var canvas: texture_2d<f32>;

struct Corner {
    @builtin(position) position: vec4<f32>,
    // 0 to 1 across the viewport.
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Corner {
    // uv (0, 0), (2, 0), (0, 2): a triangle that holds the whole viewport.
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return Corner(vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0), uv);
}

@fragment
fn fragment(corner: Corner) -> @location(0) vec4<f32> {
    // Pixel centres sit half a texel inside, so rounding error never reaches a neighbour.
    let size = vec2<f32>(textureDimensions(canvas));
    return textureLoad(canvas, vec2<i32>(floor(corner.uv * size)), 0);
}

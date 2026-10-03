// Shows the canvas on screen through the view: the pass clips to the canvas area; one triangle
// covers the screen; each pixel maps its centre to canvas pixels with the view's affine map and
// samples the canvas there, bilinearly filtered. Outside the canvas the surround the pass was
// cleared to shows. At one screen pixel per canvas pixel, upright and on whole pixels, every
// sample lands on a texel centre, so the screen shows the evaluated bytes unchanged.

@group(0) @binding(0) var canvas: texture_2d<f32>;
@group(0) @binding(1) var filtering: sampler;
// Rows of the map from screen pixels to canvas pixels: canvas = (a x + b y + c, d x + e y + f).
struct View {
    first: vec4<f32>,
    second: vec4<f32>,
}
@group(0) @binding(2) var<uniform> view: View;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    // (0, 0), (2, 0), (0, 2) in 0..1 of the viewport: a triangle that holds all of it.
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) screen: vec4<f32>) -> @location(0) vec4<f32> {
    let at = vec3<f32>(screen.xy, 1.0);
    let point = vec2<f32>(dot(view.first.xyz, at), dot(view.second.xyz, at));
    let size = vec2<f32>(textureDimensions(canvas));
    if any(point < vec2<f32>(0.0)) || any(point >= size) {
        discard;
    }
    return textureSampleLevel(canvas, filtering, point / size, 0.0);
}

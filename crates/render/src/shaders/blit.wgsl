// Shows the canvas on screen through the view: the pass clips to the canvas area; one triangle
// covers the screen; each pixel maps its centre to canvas pixels with the view's affine map and
// samples the canvas there, bilinearly filtered. Outside the canvas the surround the pass was
// cleared to shows. At one screen pixel per canvas pixel, upright and on whole pixels, every
// sample lands on a texel centre, so the screen shows the evaluated bytes unchanged.
// The pen's hover outline is one thin circle drawn here, where what lies under it is known: each
// of its pixels is black over light and white over dark (draweave's adaptive cursor).

@group(0) @binding(0) var canvas: texture_2d<f32>;
@group(0) @binding(1) var filtering: sampler;
// Rows of the map from screen pixels to canvas pixels: canvas = (a x + b y + c, d x + e y + f);
// the outline (centre x, y, radius, half its width; no outline when the width is 0), all in
// screen pixels; the surround's colour.
struct View {
    first: vec4<f32>,
    second: vec4<f32>,
    cursor: vec4<f32>,
    surround: vec4<f32>,
}
@group(0) @binding(2) var<uniform> view: View;

// How much of the outline covers the pixel at `at`, from 0 to 1.
fn outline(at: vec2<f32>) -> f32 {
    let half = view.cursor.w;
    if half <= 0.0 {
        return 0.0;
    }
    let off = abs(distance(at, view.cursor.xy) - view.cursor.z);
    return clamp(half + 0.5 - off, 0.0, 1.0);
}

// `under` with the outline over it, in black or white, whichever stands out. The canvas holds
// encoded values, so the split lies a little under the encoded middle grey.
fn with_outline(under: vec4<f32>, cover: f32) -> vec4<f32> {
    let luminance = dot(under.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let ink = select(1.0, 0.0, luminance >= 0.4);
    return vec4<f32>(mix(under.rgb, vec3<f32>(ink), cover), under.a);
}

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
    let cover = outline(screen.xy);
    if any(point < vec2<f32>(0.0)) || any(point >= size) {
        if cover <= 0.0 {
            discard;
        }
        return with_outline(view.surround, cover);
    }
    let shown = textureSampleLevel(canvas, filtering, point / size, 0.0);
    if cover <= 0.0 {
        return shown;
    }
    return with_outline(shown, cover);
}

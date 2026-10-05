// Shows the canvas on screen through the view: the pass clips to the canvas area; one triangle
// covers the screen; each pixel maps its centre to canvas pixels with the view's affine map and
// samples the canvas there (10 bits per channel where the device has them, S37), bilinearly
// filtered. Outside the canvas the surround the pass was cleared to shows. At one screen pixel
// per canvas pixel, upright and on whole pixels, every sample lands on a texel centre, so the
// screen shows the evaluated values unchanged.
// The pen's hover outline is one thin circle drawn here, where what lies under it is known: each
// of its pixels is black over light and white over dark (draweave's adaptive cursor). So is the
// selection's outline, just outside its pixels, and outside it the canvas carries light diagonal
// stripes (Procreate's, still): ink lines on paper, never moving.

@group(0) @binding(0) var canvas: texture_2d<f32>;
@group(0) @binding(1) var filtering: sampler;
// Rows of the map from screen pixels to canvas pixels: canvas = (a x + b y + c, d x + e y + f);
// the outline (centre x, y, radius, half its width; no outline when the width is 0), all in
// screen pixels; the surround's colour; the selection's rectangle in canvas pixels (x, y,
// width, height); whether a selection shows (1 or 0) and screen pixels per point.
struct View {
    first: vec4<f32>,
    second: vec4<f32>,
    cursor: vec4<f32>,
    surround: vec4<f32>,
    chosen: vec4<f32>,
    marks: vec4<f32>,
}
@group(0) @binding(2) var<uniform> view: View;
// The selection's mask rows, one word per texel, bit `x % 32` of texel `x / 32`.
@group(0) @binding(3) var selection: texture_2d<u32>;
// The view tiers (S32): the current tier's atlas of tiles and its index (per view tile, its
// slot plus one while ready, else 0), the previous tier's, and their scales (output pixels per
// canvas pixel; 0: none). Where a tier has the four texels a pixel needs, it shows, the
// current one first; elsewhere the canvas.
@group(1) @binding(0) var tier_atlas: texture_2d<f32>;
@group(1) @binding(1) var tier_index: texture_2d<u32>;
@group(1) @binding(2) var earlier_atlas: texture_2d<f32>;
@group(1) @binding(3) var earlier_index: texture_2d<u32>;
@group(1) @binding(4) var<uniform> tiers: vec4<f32>;

// Output pixels along a view tile's side (`view_tiles.rs`).
const TIER_TILE: i32 = 256;

// A tier's texel `at` of the canvas's output (clamped to it) from the slot holding its tile;
// alpha -1 when the tile is not ready.
fn tier_texel(atlas: texture_2d<f32>, index: texture_2d<u32>, at: vec2<i32>, size: vec2<i32>) -> vec4<f32> {
    let texel = clamp(at, vec2<i32>(0), size - vec2<i32>(1));
    let tile = texel / TIER_TILE;
    let held = textureLoad(index, tile, 0).r;
    if held == 0u {
        return vec4<f32>(0.0, 0.0, 0.0, -1.0);
    }
    let across = i32(textureDimensions(atlas).x) / TIER_TILE;
    let slot = i32(held - 1u);
    let origin = vec2<i32>(slot % across, slot / across) * TIER_TILE;
    return textureLoad(atlas, origin + texel % TIER_TILE, 0);
}

// The tier at `scale` at canvas point `point`, bilinear between its four nearest texels as the
// canvas's sampler would be; alpha -1 when one of them is not ready.
fn tier_sample(atlas: texture_2d<f32>, index: texture_2d<u32>, scale: f32, point: vec2<f32>) -> vec4<f32> {
    let size = vec2<i32>(vec2<f32>(textureDimensions(canvas)) * scale);
    let at = point * scale - vec2<f32>(0.5);
    let base = floor(at);
    let t = at - base;
    let cell = vec2<i32>(base);
    let a = tier_texel(atlas, index, cell, size);
    let b = tier_texel(atlas, index, cell + vec2<i32>(1, 0), size);
    let c = tier_texel(atlas, index, cell + vec2<i32>(0, 1), size);
    let d = tier_texel(atlas, index, cell + vec2<i32>(1, 1), size);
    if min(min(a.a, b.a), min(c.a, d.a)) < 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, -1.0);
    }
    return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
}

// What the screen shows of the canvas at canvas point `point`: the current tier, the previous
// one, or the canvas target, whichever is first ready there.
fn canvas_at(point: vec2<f32>, size: vec2<f32>) -> vec4<f32> {
    if tiers.x > 0.0 {
        let shown = tier_sample(tier_atlas, tier_index, tiers.x, point);
        if shown.a >= 0.0 {
            return shown;
        }
    }
    if tiers.y > 0.0 {
        let shown = tier_sample(earlier_atlas, earlier_index, tiers.y, point);
        if shown.a >= 0.0 {
            return shown;
        }
    }
    return textureSampleLevel(canvas, filtering, point / size, 0.0);
}

// The selection's outline and stripes, in points: the outline's width, the stripes' spacing and
// width, and how strongly ink lies on the stripes and between them.
const OUTLINE_POINTS: f32 = 1.5;
const STRIPE_SPACING: f32 = 9.0;
const STRIPE_POINTS: f32 = 1.0;
const STRIPE_INK: f32 = 0.28;
const DIM_INK: f32 = 0.06;

// How much of the outline covers the pixel at `at`, from 0 to 1.
fn outline(at: vec2<f32>) -> f32 {
    let half = view.cursor.w;
    if half <= 0.0 {
        return 0.0;
    }
    let off = abs(distance(at, view.cursor.xy) - view.cursor.z);
    return clamp(half + 0.5 - off, 0.0, 1.0);
}

// `under` with ink over it at `cover`, in black or white, whichever stands out. The canvas holds
// encoded values, so the split lies a little under the encoded middle grey.
fn with_outline(under: vec4<f32>, cover: f32) -> vec4<f32> {
    let luminance = dot(under.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let ink = select(1.0, 0.0, luminance >= 0.4);
    return vec4<f32>(mix(under.rgb, vec3<f32>(ink), cover), under.a);
}

// Whether the canvas pixel under canvas point `point` is selected.
fn selected(point: vec2<f32>) -> bool {
    let local = vec2<i32>(floor(point)) - vec2<i32>(view.chosen.xy);
    if any(local < vec2<i32>(0)) || any(local >= vec2<i32>(view.chosen.zw)) {
        return false;
    }
    let word = textureLoad(selection, vec2<i32>(local.x / 32, local.y), 0).r;
    return ((word >> u32(local.x % 32)) & 1u) == 1u;
}

// `shown` (the canvas at canvas point `point`, screen pixel `screen`) with the selection's
// marks: unchanged inside; on the outline, where a pixel an outline's width away on screen is
// selected, ink; elsewhere outside, the stripes.
fn with_selection(shown: vec4<f32>, point: vec2<f32>, screen: vec2<f32>) -> vec4<f32> {
    if view.marks.x <= 0.0 || selected(point) {
        return shown;
    }
    let reach = max(round(OUTLINE_POINTS * view.marks.y), 1.0);
    let across = vec2<f32>(view.first.x, view.second.x) * reach;
    let down = vec2<f32>(view.first.y, view.second.y) * reach;
    if selected(point + across) || selected(point - across) || selected(point + down)
        || selected(point - down) {
        return with_outline(shown, 1.0);
    }
    let spacing = STRIPE_SPACING * view.marks.y;
    let phase = (screen.x + screen.y) - floor((screen.x + screen.y) / spacing) * spacing;
    let on_stripe = phase < STRIPE_POINTS * view.marks.y * 1.41;
    return with_outline(shown, select(DIM_INK, STRIPE_INK, on_stripe));
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
    let shown = with_selection(canvas_at(point, size), point, screen.xy);
    if cover <= 0.0 {
        return shown;
    }
    return with_outline(shown, cover);
}

// The chrome's paper (D44): islands and the panels growing out of them as signed distance
// fields, filled with paper and outlined with one anti-aliased ink isoline. Each panel and its
// island form one silhouette by a smooth union, so they share one outline; a panel exists only
// below its island's middle, so it leaves through the island's lower edge and never swells its
// top. Positions are in points; distances turn into pixels along their real gradient.

struct Chrome {
    // width and height in points, pixels per point, linear framebuffer (0 or 1)
    screen: vec4<f32>,
    // rule width, ring width (points), island count, panel count
    widths: vec4<f32>,
    paper: vec4<f32>,
    rule: vec4<f32>,
    ring: vec4<f32>,
    // per island: centre xy, half size zw; corner radius in x of `island_corners`
    islands: array<vec4<f32>, 4>,
    island_corners: array<vec4<f32>, 4>,
    // per panel, bottom first: centre xy, half size zw
    panels: array<vec4<f32>, 3>,
    // per panel: corner radius, join radius, ring-to-rule (0..1), its island's index
    shapes: array<vec4<f32>, 3>,
};

@group(0) @binding(0) var<uniform> u: Chrome;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn rounded_rect(p: vec2<f32>, b: vec4<f32>, radius: f32) -> f32 {
    let q = abs(p - b.xy) - b.zw + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Polynomial smooth minimum: within `k` of each other two shapes flow together.
fn smin(a: f32, b: f32, k: f32) -> f32 {
    if (k <= 0.0) {
        return min(a, b);
    }
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}

// Paper inside `d` < 0 (pixels) with an ink line of `width` pixels along d = 0, over `under`;
// premultiplied alpha.
fn paint(under: vec4<f32>, d: f32, width: f32, ink: vec3<f32>) -> vec4<f32> {
    let fill = clamp(0.5 - d, 0.0, 1.0);
    let line = clamp(0.5 - (abs(d) - 0.5 * width), 0.0, 1.0);
    var shape = vec4<f32>(u.paper.rgb * fill, fill);
    shape = shape * (1.0 - line) + vec4<f32>(ink, 1.0) * line;
    return shape + under * (1.0 - shape.a);
}

// Distance (points) to panel `i`, kept below the middle of island `k`.
fn panel(p: vec2<f32>, i: u32, k: u32) -> f32 {
    let d = rounded_rect(p, u.panels[i], u.shapes[i].x);
    let below = u.islands[k].y - p.y;
    return -smin(-d, -below, 8.0);
}

fn to_pixels(d: f32) -> f32 {
    let g = length(vec2<f32>(dpdx(d), dpdy(d)));
    return d / max(g, 1e-4);
}

@fragment
fn fragment(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let ppp = u.screen.z;
    let p = frag.xy / ppp;
    let islands = u32(u.widths.z);
    let panels = u32(u.widths.w);
    var color = vec4<f32>(0.0);
    for (var k = 0u; k < 4u; k++) {
        var d = rounded_rect(p, u.islands[k], u.island_corners[k].x);
        var ink = u.rule.rgb;
        var width = u.widths.x;
        for (var i = 0u; i < 3u; i++) {
            let mine = i < panels && u32(u.shapes[i].w) == k;
            let b = panel(p, i, k);
            // The outline takes the panel's ink where the panel is the nearer surface: the ring
            // it grew from turns into the panel's rule.
            let near = smoothstep(-1.5, 1.5, d - b) * f32(mine);
            let panel_ink = mix(u.ring.rgb, u.rule.rgb, u.shapes[i].z);
            let panel_width = mix(u.widths.y, u.widths.x, u.shapes[i].z);
            ink = mix(ink, panel_ink, near);
            width = mix(width, panel_width, near);
            d = select(d, smin(d, b, u.shapes[i].y), mine);
        }
        let painted = paint(color, to_pixels(d), width * ppp, ink);
        color = select(color, painted, k < islands);
    }
    if (u.screen.w > 0.5 && color.a > 0.0) {
        let straight = color.rgb / color.a;
        let linear = select(pow((straight + 0.055) / 1.055, vec3<f32>(2.4)), straight / 12.92,
            straight <= vec3<f32>(0.04045));
        color = vec4<f32>(linear * color.a, color.a);
    }
    return color;
}

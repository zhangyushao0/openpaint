// Adjustments (S28, D54): hue, saturation and brightness, or tone curves, on the current layer
// at the op's place, inside its clip; elsewhere the layer stays exactly as it was. Paint is
// adjusted as straight sRGB-encoded colour and keeps its alpha; a transparent pixel stays. A
// mask's grey over transparent shows `1 - (a - r)`; it is adjusted as the grey it shows (turned
// over on an inverted mask), as a colour whose luma is kept, and stored again as black at the
// hidden share, which shows that grey.

// `cell`: the canvas pixel holding the evaluated centre, for the clip. A point operation, so
// the same at every output scale.
fn adjusted(layer: vec4<f32>, cell: vec2<u32>, op: u32) -> vec4<f32> {
    if clip(cell, op) == 0.0 {
        return layer;
    }
    let on = u32(word(op, ADJUST_ON));
    if on == ADJUST_ON_PAINT {
        if layer.a <= 0.0 {
            return layer;
        }
        let straight = clamp(layer.rgb / layer.a, vec3<f32>(0.0), vec3<f32>(1.0));
        return vec4<f32>(adjust_colour(straight, op) * layer.a, layer.a);
    }
    var shown = clamp(1.0 - (layer.a - layer.r), 0.0, 1.0);
    if on == ADJUST_ON_INVERTED_MASK {
        shown = 1.0 - shown;
    }
    var grey = clamp(luma(adjust_colour(vec3<f32>(shown), op)), 0.0, 1.0);
    if on == ADJUST_ON_INVERTED_MASK {
        grey = 1.0 - grey;
    }
    return vec4<f32>(0.0, 0.0, 0.0, 1.0 - grey);
}

fn adjust_colour(rgb: vec3<f32>, op: u32) -> vec3<f32> {
    if u32(word(op, ADJUST_KIND)) == ADJUST_KIND_CURVES {
        return curved(rgb, op);
    }
    return hue_saturation_brightness(
        rgb,
        word(op, ADJUST_HSB),
        word(op, ADJUST_HSB + 1u),
        word(op, ADJUST_HSB + 2u),
    );
}

// Rec. 601 weights, as a mask's grey.
fn luma(rgb: vec3<f32>) -> f32 {
    return 0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b;
}

// Turned round the HSV hue circle by `hue - 0.5` of a turn, then the distance from its grey
// scaled by `2 saturation`, then towards black (brightness below 0.5) or white (above).
fn hue_saturation_brightness(rgb: vec3<f32>, hue: f32, saturation: f32, brightness: f32) -> vec3<f32> {
    var colour = rgb;
    if hue != 0.5 {
        colour = turned(colour, hue - 0.5);
    }
    let grey = vec3<f32>(luma(colour));
    colour = clamp(grey + (colour - grey) * (2.0 * saturation), vec3<f32>(0.0), vec3<f32>(1.0));
    let t = 2.0 * brightness - 1.0;
    if t < 0.0 {
        return colour * (1.0 + t);
    }
    return colour + (vec3<f32>(1.0) - colour) * t;
}

// `rgb` turned by `turns` of the HSV hue circle, its HSV saturation and value kept.
fn turned(rgb: vec3<f32>, turns: f32) -> vec3<f32> {
    let high = max(rgb.r, max(rgb.g, rgb.b));
    let low = min(rgb.r, min(rgb.g, rgb.b));
    let delta = high - low;
    if delta <= 0.0 {
        return rgb;
    }
    var sector = 0.0;
    if high == rgb.r {
        sector = sixths((rgb.g - rgb.b) / delta);
    } else if high == rgb.g {
        sector = (rgb.b - rgb.r) / delta + 2.0;
    } else {
        sector = (rgb.r - rgb.g) / delta + 4.0;
    }
    let hue = sixths(sector + 6.0 * turns);
    let middle = low + delta * (1.0 - abs(hue - 2.0 * floor(hue / 2.0) - 1.0));
    switch u32(hue) {
        case 0u: {
            return vec3<f32>(high, middle, low);
        }
        case 1u: {
            return vec3<f32>(middle, high, low);
        }
        case 2u: {
            return vec3<f32>(low, high, middle);
        }
        case 3u: {
            return vec3<f32>(low, middle, high);
        }
        case 4u: {
            return vec3<f32>(middle, low, high);
        }
        default: {
            return vec3<f32>(high, low, middle);
        }
    }
}

// `value` taken round to `0..6`.
fn sixths(value: f32) -> f32 {
    return value - 6.0 * floor(value / 6.0);
}

// Each channel through its table (S43, D70): linearly between the two entries around the
// colour's place among the 256, by its fraction, so a smooth gradient through a steep curve
// stays smooth (rounding to the nearest entry first stepped by the curve's slope).
fn curved(rgb: vec3<f32>, op: u32) -> vec3<f32> {
    let at = u32(word(op, ADJUST_TABLES)) * 65536u + u32(word(op, ADJUST_TABLES + 1u));
    let scaled = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * 255.0;
    let below = min(floor(scaled), vec3<f32>(254.0));
    let index = vec3<u32>(below);
    let share = scaled - below;
    return vec3<f32>(
        between(at, index.r, share.r),
        between(at + 64u, index.g, share.g),
        between(at + 128u, index.b, share.b),
    );
}

// The table at `at` between entries `index` and `index + 1`, `share` of the way.
fn between(at: u32, index: u32, share: f32) -> f32 {
    return mix(table(at, index), table(at, index + 1u), share);
}

fn table(at: u32, index: u32) -> f32 {
    let word = masks[at + index / 4u];
    return f32((word >> (8u * (index % 4u))) & 255u) / 255.0;
}

// The gallery picture enlarged over a whole canvas target (S54, `underlay.rs`): each target
// pixel samples the picture bilinearly at its centre. What a canvas opening without a display
// cache shows under its tiles until they are evaluated, never the truth. TEXEL_FORMAT is the
// target's storage format, put in by `underlay.rs`.

@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var bilinear: sampler;
@group(0) @binding(2) var canvas: texture_storage_2d<TEXEL_FORMAT, write>;

@compute @workgroup_size(8, 8, 1)
fn underlay(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(canvas);
    if (id.x >= size.x || id.y >= size.y) {
        return;
    }
    let at = (vec2<f32>(id.xy) + vec2<f32>(0.5, 0.5)) / vec2<f32>(size);
    let colour = textureSampleLevel(picture, bilinear, at, 0.0);
    textureStore(canvas, vec2<i32>(id.xy), vec4<f32>(colour.rgb, 1.0));
}

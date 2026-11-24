// The Render Studio's picture: the running sum divided by its samples, exposed, through ACES's
// filmic fit (Narkowicz) as the viewport's Realistic style, and sRGB-encoded.

struct Show {
    // samples, exposure (a factor), 0, 0
    at: vec4f,
}

@group(0) @binding(0) var<uniform> S: Show;
@group(0) @binding(1) var<storage, read> sum: array<vec4f>;
@group(0) @binding(2) var out: texture_storage_2d<rgba8unorm, write>;

fn filmic(x: vec3f) -> vec3f {
    return saturate((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14));
}

fn srgb(c: vec3f) -> vec3f {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3f(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3f(0.0031308));
}

@compute @workgroup_size(8, 8)
fn show_main(@builtin(global_invocation_id) id: vec3u) {
    let size = textureDimensions(out);
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    let v = sum[id.y * size.x + id.x] / max(S.at.x, 1.0);
    let c = srgb(filmic(max(v.rgb, vec3f(0.0)) * S.at.y));
    textureStore(out, vec2i(id.xy), vec4f(c, v.a));
}

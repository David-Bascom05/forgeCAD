// The Render Studio's denoiser: the edge-avoiding a-trous wavelet filter (Dammertz, Sewtz,
// Hanika & Lensch 2010, "Edge-Avoiding A-Trous Wavelet Transform for fast Global Illumination
// Filtering"), written from the paper. The light is divided by the first hit's albedo first, so
// texture and colour edges survive, filtered in five passes of growing step (1, 2, 4, 8, 16),
// each weighting a 5x5 B3-spline by how alike the neighbours' light, normal and depth are, then
// multiplied by the albedo again. The light's tolerance is each pixel's own standard error, carried
// and filtered through the passes (as Schied et al. 2017, "Spatiotemporal Variance-Guided
// Filtering", do), so noise is smoothed where it is and edges are kept where it isn't.

struct Pass {
    // width, height, the step in pixels, samples summed
    size: vec4u,
    // the light's tolerance in standard errors, the normal's power, the depth's tolerance, 0
    sigma: vec4f,
}

@group(0) @binding(0) var<uniform> D: Pass;
@group(0) @binding(1) var<storage, read> accum: array<vec4f>;
@group(0) @binding(2) var<storage, read> aov: array<vec4f>;
@group(0) @binding(3) var<storage, read> src: array<vec4f>;
@group(0) @binding(4) var<storage, read_write> dst: array<vec4f>;

fn samples() -> f32 {
    return max(f32(D.size.w), 1.0);
}

fn albedo(p: u32) -> vec3f {
    return max(aov[p * 2u].xyz / samples(), vec3f(0.02));
}

fn depth(p: u32) -> f32 {
    return aov[p * 2u].w / samples();
}

fn normal(p: u32) -> vec3f {
    let n = aov[p * 2u + 1u].xyz / samples();
    let l = length(n);
    return select(vec3f(0.0), n / l, l > 1e-6);
}

fn lum(c: vec3f) -> f32 {
    return dot(c, vec3f(0.2126, 0.7152, 0.0722));
}

@compute @workgroup_size(8, 8)
fn prepare(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= D.size.x || id.y >= D.size.y {
        return;
    }
    let p = id.y * D.size.x + id.x;
    let mean = accum[p] / samples();
    var c = mean.rgb / albedo(p);
    // a firefly: far brighter than every neighbour is brought down to the brightest of them
    var most = 0.0;
    for (var j = -1; j <= 1; j += 1) {
        for (var i = -1; i <= 1; i += 1) {
            let x = i32(id.x) + i;
            let y = i32(id.y) + j;
            if (i == 0 && j == 0) || x < 0 || y < 0 || x >= i32(D.size.x) || y >= i32(D.size.y) {
                continue;
            }
            let q = u32(y) * D.size.x + u32(x);
            let n = accum[q].rgb / samples() / albedo(q);
            most = max(most, lum(n));
        }
    }
    let l = lum(c);
    if l > 4.0 * most + 0.05 {
        c *= (4.0 * most + 0.05) / l;
    }
    // the variance of the mean's luminance, in the albedo-divided light's units
    let m = lum(mean.rgb);
    let var_mean = max(aov[p * 2u + 1u].w / samples() - m * m, 0.0) / samples();
    let a = max(lum(albedo(p)), 0.02);
    dst[p] = vec4f(c, var_mean / (a * a));
}

@compute @workgroup_size(8, 8)
fn atrous(@builtin(global_invocation_id) id: vec3u) {
    let w = i32(D.size.x);
    let h = i32(D.size.y);
    if i32(id.x) >= w || i32(id.y) >= h {
        return;
    }
    let p = id.y * D.size.x + id.x;
    let c = src[p];
    let lp = lum(c.rgb);
    let tol = D.sigma.x * sqrt(c.a) + 1e-4;
    let np = normal(p);
    let zp = depth(p);
    // the depth's slope here, for how far a neighbour's depth may stray along the surface
    let xr = min(i32(id.x) + 1, w - 1);
    let yd = min(i32(id.y) + 1, h - 1);
    let dz = max(abs(depth(u32(i32(id.y) * w + xr)) - zp), abs(depth(u32(yd * w + i32(id.x))) - zp));
    let kernel = array<f32, 5>(1.0 / 16.0, 1.0 / 4.0, 3.0 / 8.0, 1.0 / 4.0, 1.0 / 16.0);
    let step = i32(D.size.z);
    var sum = vec3f(0.0);
    var weights = 0.0;
    var var_sum = 0.0;
    for (var j = -2; j <= 2; j += 1) {
        for (var i = -2; i <= 2; i += 1) {
            let x = i32(id.x) + i * step;
            let y = i32(id.y) + j * step;
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let q = u32(y * w + x);
            let sq = src[q];
            let cq = sq.rgb;
            let wc = exp(-abs(lum(cq) - lp) / tol);
            let nq = normal(q);
            // a miss (no normal) only blends with misses
            var wn = pow(max(dot(np, nq), 0.0), D.sigma.y);
            if dot(np, np) < 0.5 && dot(nq, nq) < 0.5 {
                wn = 1.0;
            }
            let reach = D.sigma.z * (dz * length(vec2f(f32(i), f32(j))) * f32(step) + 1e-3 * zp) + 1e-6;
            let wz = exp(-abs(depth(q) - zp) / reach);
            let k = kernel[i + 2] * kernel[j + 2] * wc * wn * wz;
            sum += cq * k;
            weights += k;
            var_sum += k * k * sq.a;
        }
    }
    let wt = max(weights, 1e-12);
    dst[p] = vec4f(sum / wt, var_sum / (wt * wt));
}

@compute @workgroup_size(8, 8)
fn finish(@builtin(global_invocation_id) id: vec3u) {
    if id.x >= D.size.x || id.y >= D.size.y {
        return;
    }
    let p = id.y * D.size.x + id.x;
    let c = src[p];
    dst[p] = vec4f(c.rgb * albedo(p), accum[p].a / samples());
}

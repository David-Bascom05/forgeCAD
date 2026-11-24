// The Render Studio's path tracer: one sample per pixel per dispatch, added to a running sum.
// Its BSDF, sampling and tables are `bsdf.rs`'s, line for line where it matters; the hierarchy
// and the environment's tables are `scene.rs`'s and `env.rs`'s arrays as they are.

struct Params {
    // width, height, the sample's index, the seed
    size: vec4u,
    // the placements' root node, the map's width and height, the most bounces
    counts: vec4u,
    // where the tables start in `materials` (vec4s), placements, flags (1: AOVs), 0
    misc: vec4u,
    // the eye; w: half the height seen, for a parallel view (0: perspective)
    eye: vec4f,
    // the view's right and up, scaled by tan of half the field (perspective); w unused
    right: vec4f,
    up: vec4f,
    // forward; w the lens's radius (0: a pinhole)
    fwd: vec4f,
    // the distance in focus, the map's turn, its strength, the brightest a sample may add
    lens: vec4f,
    // the ground: its height, on (1), its material, the backdrop (0 the map, 1 colour, 2 none)
    ground: vec4f,
    // the backdrop's colour
    back: vec4f,
}

@group(0) @binding(0) var<uniform> P: Params;
@group(0) @binding(1) var<storage, read> nodes: array<vec4u>;
@group(0) @binding(2) var<storage, read> tris: array<vec4f>;
@group(0) @binding(3) var<storage, read> instances: array<vec4f>;
@group(0) @binding(4) var<storage, read> materials: array<vec4f>;
@group(0) @binding(5) var<storage, read> env: array<f32>;
@group(0) @binding(6) var<storage, read_write> accum: array<vec4f>;
@group(0) @binding(7) var<storage, read_write> aov: array<vec4f>;

const PI: f32 = 3.14159265358979;
const ALPHA_MIN: f32 = 1.0e-3;
const LUT_N: u32 = 32u;
const BIG: f32 = 3.0e38;

// ------------------------------------------------------------------ randomness (v3.rs)

fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand(pixel: u32, sample: u32, dim: u32, seed: u32) -> f32 {
    let h = pcg(pixel ^ pcg(sample ^ pcg(dim ^ pcg(seed))));
    return f32(h >> 8u) * (1.0 / 16777216.0);
}

struct Rng {
    pixel: u32,
    sample: u32,
    dim: u32,
}

fn next(r: ptr<function, Rng>) -> f32 {
    let x = rand((*r).pixel, (*r).sample, (*r).dim, P.size.w);
    (*r).dim += 1u;
    return x;
}

// ------------------------------------------------------------------ the scene (scene.rs)

struct Hit {
    t: f32,
    tri: i32,
    u: f32,
    v: f32,
    inst: u32,
}

fn hit_box(o: vec3f, inv: vec3f, lo: vec3f, hi: vec3f, t_max: f32) -> f32 {
    let t0 = (lo - o) * inv;
    let t1 = (hi - o) * inv;
    let a = min(t0, t1);
    let b = max(t0, t1);
    let near = max(max(a.x, a.y), max(a.z, 0.0));
    let far = min(min(b.x, b.y), min(b.z, t_max));
    return select(BIG, near, near <= far);
}

fn safe_inv(d: vec3f) -> vec3f {
    let s = select(vec3f(-1.0), vec3f(1.0), d >= vec3f(0.0));
    return select(1.0 / d, s * 1e30, abs(d) < vec3f(1e-30));
}

// a part's hierarchy from its root: the nearest triangle before `best.t` (or any, `any`)
fn trace_part(root: u32, o: vec3f, d: vec3f, inst: u32, best: ptr<function, Hit>, any: bool) {
    let inv = safe_inv(d);
    var stack: array<u32, 48>;
    var sp = 0u;
    var at = root;
    let r0 = nodes[root * 2u];
    let r1 = nodes[root * 2u + 1u];
    if hit_box(o, inv, bitcast<vec3f>(r0.xyz), bitcast<vec3f>(r1.xyz), (*best).t) >= BIG {
        return;
    }
    loop {
        let a = nodes[at * 2u];
        let b = nodes[at * 2u + 1u];
        if b.w > 0u {
            for (var k = a.w; k < a.w + b.w; k += 1u) {
                let p0 = tris[k * 3u].xyz;
                let e1 = tris[k * 3u + 1u].xyz;
                let e2 = tris[k * 3u + 2u].xyz;
                let p = cross(d, e2);
                let det = dot(e1, p);
                if abs(det) < 1e-20 {
                    continue;
                }
                let id = 1.0 / det;
                let s = o - p0;
                let u = dot(s, p) * id;
                if u < 0.0 || u > 1.0 {
                    continue;
                }
                let q = cross(s, e1);
                let v = dot(d, q) * id;
                if v < 0.0 || u + v > 1.0 {
                    continue;
                }
                let t = dot(e2, q) * id;
                if t > 0.0 && t < (*best).t {
                    *best = Hit(t, i32(k), u, v, inst);
                    if any {
                        return;
                    }
                }
            }
        } else {
            let ca = a.w;
            let cb = a.w + 1u;
            let ta = hit_box(o, inv, bitcast<vec3f>(nodes[ca * 2u].xyz), bitcast<vec3f>(nodes[ca * 2u + 1u].xyz), (*best).t);
            let tb = hit_box(o, inv, bitcast<vec3f>(nodes[cb * 2u].xyz), bitcast<vec3f>(nodes[cb * 2u + 1u].xyz), (*best).t);
            if ta < BIG && tb < BIG {
                if sp < 48u {
                    stack[sp] = select(ca, cb, ta <= tb);
                    sp += 1u;
                }
                at = select(cb, ca, ta <= tb);
                continue;
            }
            if ta < BIG {
                at = ca;
                continue;
            }
            if tb < BIG {
                at = cb;
                continue;
            }
        }
        if sp == 0u {
            break;
        }
        sp -= 1u;
        at = stack[sp];
    }
}

fn row3(k: u32, j: u32) -> vec4f {
    return instances[k * 4u + j];
}

// the nearest hit in the scene before `t_max`; with `any`, the first found
fn trace(o: vec3f, d: vec3f, t_max: f32, any: bool) -> Hit {
    var best = Hit(t_max, -1, 0.0, 0.0, 0u);
    if P.misc.y == 0u {
        return best;
    }
    let inv = safe_inv(d);
    var stack: array<u32, 32>;
    var sp = 0u;
    var at = P.counts.x;
    let r0 = nodes[at * 2u];
    let r1 = nodes[at * 2u + 1u];
    if hit_box(o, inv, bitcast<vec3f>(r0.xyz), bitcast<vec3f>(r1.xyz), t_max) >= BIG {
        return best;
    }
    loop {
        let a = nodes[at * 2u];
        let b = nodes[at * 2u + 1u];
        if b.w > 0u {
            for (var k = a.w; k < a.w + b.w; k += 1u) {
                let m0 = row3(k, 0u);
                let m1 = row3(k, 1u);
                let m2 = row3(k, 2u);
                let lo = vec3f(dot(m0.xyz, o) + m0.w, dot(m1.xyz, o) + m1.w, dot(m2.xyz, o) + m2.w);
                let ld = vec3f(dot(m0.xyz, d), dot(m1.xyz, d), dot(m2.xyz, d));
                let root = bitcast<u32>(row3(k, 3u).x);
                trace_part(root, lo, ld, k, &best, any);
                if any && best.tri >= 0 {
                    return best;
                }
            }
        } else {
            let ca = a.w;
            let cb = a.w + 1u;
            let ta = hit_box(o, inv, bitcast<vec3f>(nodes[ca * 2u].xyz), bitcast<vec3f>(nodes[ca * 2u + 1u].xyz), best.t);
            let tb = hit_box(o, inv, bitcast<vec3f>(nodes[cb * 2u].xyz), bitcast<vec3f>(nodes[cb * 2u + 1u].xyz), best.t);
            if ta < BIG && tb < BIG {
                if sp < 32u {
                    stack[sp] = select(ca, cb, ta <= tb);
                    sp += 1u;
                }
                at = select(cb, ca, ta <= tb);
                continue;
            }
            if ta < BIG {
                at = ca;
                continue;
            }
            if tb < BIG {
                at = cb;
                continue;
            }
        }
        if sp == 0u {
            break;
        }
        sp -= 1u;
        at = stack[sp];
    }
    return best;
}

fn unoct(w: u32) -> vec3f {
    let x = f32(i32(w << 16u) >> 16u) / 32767.0;
    let y = f32(i32(w) >> 16u) / 32767.0;
    let z = 1.0 - abs(x) - abs(y);
    let t = max(-z, 0.0);
    let xx = x + select(t, -t, x >= 0.0);
    let yy = y + select(t, -t, y >= 0.0);
    return normalize(vec3f(xx, yy, z));
}

// a direction by the inverse transpose: world-to-local rows, transposed
fn to_world_normal(k: u32, n: vec3f) -> vec3f {
    let m0 = row3(k, 0u).xyz;
    let m1 = row3(k, 1u).xyz;
    let m2 = row3(k, 2u).xyz;
    return normalize(m0 * n.x + m1 * n.y + m2 * n.z);
}

// ------------------------------------------------------------------ the environment (env.rs)

fn env_w() -> u32 {
    return P.counts.y;
}
fn env_h() -> u32 {
    return P.counts.z;
}
fn marginal(i: u32) -> f32 {
    return env[i];
}
fn conditional(y: u32, i: u32) -> f32 {
    return env[env_h() + 1u + y * (env_w() + 1u) + i];
}
fn env_rgb(x: u32, y: u32) -> vec3f {
    let at = env_h() + 1u + env_h() * (env_w() + 1u) + (y * env_w() + x) * 3u;
    return vec3f(env[at], env[at + 1u], env[at + 2u]);
}

fn env_pixel(d: vec3f) -> vec2u {
    var u = (atan2(d.y, d.x) - P.lens.y) / (2.0 * PI);
    u -= floor(u);
    let v = acos(clamp(d.z, -1.0, 1.0)) / PI;
    return vec2u(min(u32(u * f32(env_w())), env_w() - 1u), min(u32(v * f32(env_h())), env_h() - 1u));
}

fn env_radiance(d: vec3f) -> vec3f {
    let p = env_pixel(d);
    return env_rgb(p.x, p.y) * P.lens.z;
}

fn env_pdf_at(x: u32, y: u32) -> f32 {
    let row = marginal(y + 1u) - marginal(y);
    let col = conditional(y, x + 1u) - conditional(y, x);
    let s = sin(PI * (f32(y) + 0.5) / f32(env_h()));
    return row * col * f32(env_w() * env_h()) / (2.0 * PI * PI * max(s, 1e-6));
}

fn env_pdf(d: vec3f) -> f32 {
    let p = env_pixel(d);
    return env_pdf_at(p.x, p.y);
}

struct EnvSample {
    d: vec3f,
    rgb: vec3f,
    pdf: f32,
}

fn env_sample(u1: f32, u2: f32) -> EnvSample {
    let h = env_h();
    let w = env_w();
    var lo = 0u;
    var hi = h;
    while lo + 1u < hi {
        let mid = (lo + hi) / 2u;
        if marginal(mid) <= u1 { lo = mid; } else { hi = mid; }
    }
    let y = min(lo, h - 1u);
    lo = 0u;
    hi = w;
    while lo + 1u < hi {
        let mid = (lo + hi) / 2u;
        if conditional(y, mid) <= u2 { lo = mid; } else { hi = mid; }
    }
    let x = min(lo, w - 1u);
    let fy = (u1 - marginal(y)) / max(marginal(y + 1u) - marginal(y), 1e-30);
    let fx = (u2 - conditional(y, x)) / max(conditional(y, x + 1u) - conditional(y, x), 1e-30);
    let u = (f32(x) + clamp(fx, 0.0, 0.999)) / f32(w) + P.lens.y / (2.0 * PI);
    let v = (f32(y) + clamp(fy, 0.0, 0.999)) / f32(h);
    let phi = 2.0 * PI * u;
    let theta = PI * v;
    let d = vec3f(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));
    return EnvSample(d, env_rgb(x, y) * P.lens.z, env_pdf_at(x, y));
}

// ------------------------------------------------------------------ the BSDF (bsdf.rs)

struct Mat {
    base: vec3f,
    metal: f32,
    f82: vec3f,
    rough: f32,
    ior: f32,
    transmission: f32,
    coat: f32,
    coat_rough: f32,
}

fn material(k: u32) -> Mat {
    let a = materials[k * 4u];
    let b = materials[k * 4u + 1u];
    let c = materials[k * 4u + 2u];
    return Mat(a.xyz, a.w, b.xyz, b.w, c.x, c.y, c.z, c.w);
}

fn alpha_of(rough: f32) -> f32 {
    return max(rough * rough, ALPHA_MIN);
}

fn ggx_d(m: vec3f, a: f32) -> f32 {
    if m.z <= 0.0 {
        return 0.0;
    }
    let a2 = a * a;
    let d = m.z * m.z * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

fn lambda(w: vec3f, a: f32) -> f32 {
    let c2 = w.z * w.z;
    let s2 = max(1.0 - c2, 0.0);
    if s2 <= 0.0 {
        return 0.0;
    }
    return 0.5 * (-1.0 + sqrt(1.0 + a * a * s2 / c2));
}

fn g1(w: vec3f, a: f32) -> f32 {
    return 1.0 / (1.0 + lambda(w, a));
}

fn g2(wo: vec3f, wi: vec3f, a: f32) -> f32 {
    return 1.0 / (1.0 + lambda(wo, a) + lambda(wi, a));
}

fn sample_vndf(wo: vec3f, a: f32, u1: f32, u2: f32) -> vec3f {
    let ws = normalize(vec3f(a * wo.x, a * wo.y, wo.z));
    let phi = 2.0 * PI * u1;
    let z = (1.0 - u2) * (1.0 + ws.z) - ws.z;
    let s = sqrt(clamp(1.0 - z * z, 0.0, 1.0));
    let h = vec3f(s * cos(phi), s * sin(phi), z) + ws;
    return normalize(vec3f(a * h.x, a * h.y, max(h.z, 0.0)));
}

fn vndf_pdf(wo: vec3f, m: vec3f, a: f32) -> f32 {
    let om = dot(wo, m);
    if om <= 0.0 || wo.z <= 0.0 {
        return 0.0;
    }
    return g1(wo, a) * om * ggx_d(m, a) / wo.z;
}

fn reflect_about(w: vec3f, m: vec3f) -> vec3f {
    return m * (2.0 * dot(w, m)) - w;
}

fn schlick(f0: f32, c: f32) -> f32 {
    let m = clamp(1.0 - c, 0.0, 1.0);
    let m2 = m * m;
    return f0 + (1.0 - f0) * m2 * m2 * m;
}

fn schlick3(f0: vec3f, c: f32) -> vec3f {
    return vec3f(schlick(f0.x, c), schlick(f0.y, c), schlick(f0.z, c));
}

fn conductor_fresnel(f0: vec3f, f82: vec3f, c: f32) -> vec3f {
    let mb = 1.0 / 7.0;
    let k = 1.0 / (mb * pow(1.0 - mb, 6.0));
    let fs = schlick3(f0, mb);
    let a = (fs - f82 * fs) * k;
    let t = c * pow(1.0 - c, 6.0);
    return clamp(schlick3(f0, c) - a * t, vec3f(0.0), vec3f(1.0));
}

fn conductor_fresnel_avg(f0: vec3f, f82: vec3f) -> vec3f {
    let mb = 1.0 / 7.0;
    let k = 1.0 / (mb * pow(1.0 - mb, 6.0));
    let fs = schlick3(f0, mb);
    let a = (fs - f82 * fs) * k;
    return clamp(f0 + (vec3f(1.0) - f0) / 21.0 - a / 126.0, vec3f(0.0), vec3f(1.0));
}

fn dielectric_fresnel(ci_in: f32, eta: f32) -> f32 {
    let ci = min(abs(ci_in), 1.0);
    let s2 = (1.0 - ci * ci) / (eta * eta);
    if s2 >= 1.0 {
        return 1.0;
    }
    let ct = sqrt(1.0 - s2);
    let rs = (ci - eta * ct) / (ci + eta * ct);
    let rp = (eta * ci - ct) / (eta * ci + ct);
    return 0.5 * (rs * rs + rp * rp);
}

fn f0_of_ior(ior: f32) -> f32 {
    let r = (ior - 1.0) / (ior + 1.0);
    return r * r;
}

fn lutf(i: u32) -> f32 {
    let v = materials[P.misc.x + i / 4u];
    return v[i % 4u];
}

fn lut_at(table: u32, mu: f32, rough: f32) -> f32 {
    let n = LUT_N;
    let x = clamp(clamp(mu, 0.0, 1.0) * f32(n - 1u), 0.0, f32(n - 1u));
    let y = clamp(clamp(rough, 0.0, 1.0) * f32(n - 1u), 0.0, f32(n - 1u));
    let x0 = min(u32(x), n - 2u);
    let y0 = min(u32(y), n - 2u);
    let fx = x - f32(x0);
    let fy = y - f32(y0);
    let base = table * n * n;
    let lo = lutf(base + y0 * n + x0) * (1.0 - fx) + lutf(base + y0 * n + x0 + 1u) * fx;
    let hi = lutf(base + (y0 + 1u) * n + x0) * (1.0 - fx) + lutf(base + (y0 + 1u) * n + x0 + 1u) * fx;
    return lo * (1.0 - fy) + hi * fy;
}

fn lut_avg(table: u32, rough: f32) -> f32 {
    let n = LUT_N;
    let y = clamp(clamp(rough, 0.0, 1.0) * f32(n - 1u), 0.0, f32(n - 1u));
    let y0 = min(u32(y), n - 2u);
    let f = y - f32(y0);
    let base = 2u * n * n + table * n;
    return lutf(base + y0) * (1.0 - f) + lutf(base + y0 + 1u) * f;
}

fn e_spec(f0: f32, mu: f32, rough: f32) -> f32 {
    return min(f0 * lut_at(0u, mu, rough) + lut_at(1u, mu, rough), 1.0);
}

fn e_spec_avg(f0: f32, rough: f32) -> f32 {
    return min(f0 * lut_avg(0u, rough) + lut_avg(1u, rough), 1.0);
}

struct Spec {
    f: f32,
    pdf: f32,
    m: vec3f,
}

fn spec_reflect(wo: vec3f, wi: vec3f, a: f32) -> Spec {
    if wo.z <= 0.0 || wi.z <= 0.0 {
        return Spec(0.0, 0.0, vec3f(0.0));
    }
    let m = normalize(wo + wi);
    let d = ggx_d(m, a);
    return Spec(d * g2(wo, wi, a) / (4.0 * wo.z * wi.z), g1(wo, a) * d / (4.0 * wo.z), m);
}

fn max3(v: vec3f) -> f32 {
    return max(v.x, max(v.y, v.z));
}

fn lobe_odds(b: Mat, mu: f32) -> array<f32, 5> {
    let coat = b.coat * e_spec(f0_of_ior(1.5), mu, b.coat_rough);
    let under = 1.0 - coat;
    let glass = (1.0 - b.metal) * b.transmission;
    let plastic = (1.0 - b.metal) * (1.0 - b.transmission);
    let es = e_spec(f0_of_ior(b.ior), mu, b.rough);
    let diffuse = (1.0 - es) * max(max3(b.base), 0.02);
    let share = es / max(es + diffuse, 1e-6);
    var p = array<f32, 5>(coat, under * b.metal, under * plastic * share, under * plastic * (1.0 - share), under * glass);
    let sum = p[0] + p[1] + p[2] + p[3] + p[4];
    for (var k = 0; k < 5; k += 1) {
        p[k] /= max(sum, 1e-9);
    }
    return p;
}

struct Eval {
    f: vec3f,
    pdf: f32,
}

fn glass_eval(b: Mat, wo: vec3f, wi: vec3f, inside: bool, a: f32) -> Eval {
    let eta = select(b.ior, 1.0 / b.ior, inside);
    if wi.z > 0.0 {
        let s = spec_reflect(wo, wi, a);
        let fr = dielectric_fresnel(dot(wo, s.m), eta);
        return Eval(vec3f(fr * s.f), fr * s.pdf);
    }
    if wi.z == 0.0 {
        return Eval(vec3f(0.0), 0.0);
    }
    var m = normalize(wo + wi * eta);
    if m.z < 0.0 {
        m = -m;
    }
    let om = dot(wo, m);
    let im = dot(wi, m);
    if om <= 0.0 || im >= 0.0 {
        return Eval(vec3f(0.0), 0.0);
    }
    let fr = dielectric_fresnel(om, eta);
    let d = ggx_d(m, a);
    let den = om + eta * im;
    let den2 = den * den;
    let ft = (1.0 - fr) * d * g2(wo, wi, a) * om * (-im) / (wo.z * (-wi.z) * den2);
    let jac = eta * eta * (-im) / den2;
    return Eval(b.base * max(ft, 0.0), (1.0 - fr) * vndf_pdf(wo, m, a) * jac);
}

fn bsdf_eval(b: Mat, wo_in: vec3f, wi_in: vec3f) -> Eval {
    let flip = wo_in.z < 0.0;
    let wo = select(wo_in, -wo_in, flip);
    let wi = select(wi_in, -wi_in, flip);
    let mu_o = max(wo.z, 1e-6);
    let odds = lobe_odds(b, mu_o);
    var f = vec3f(0.0);
    var pdf = 0.0;
    let f0c = f0_of_ior(1.5);
    var under_o = 1.0;
    var under_i = 1.0;
    if b.coat > 0.0 {
        under_o = 1.0 - b.coat * e_spec(f0c, mu_o, b.coat_rough);
        under_i = 1.0 - b.coat * e_spec(f0c, abs(wi.z), b.coat_rough);
        let s = spec_reflect(wo, wi, alpha_of(b.coat_rough));
        if s.f > 0.0 {
            f += vec3f(b.coat * schlick(f0c, dot(wo, s.m)) * s.f);
            pdf += odds[0] * s.pdf;
        }
    }
    let under = under_o * under_i;
    let a = alpha_of(b.rough);
    if b.metal > 0.0 && wi.z > 0.0 {
        let s = spec_reflect(wo, wi, a);
        let fr = conductor_fresnel(b.base, b.f82, dot(wo, s.m));
        let e1o = e_spec(1.0, mu_o, b.rough);
        let e1i = e_spec(1.0, wi.z, b.rough);
        let eavg = e_spec_avg(1.0, b.rough);
        let favg = conductor_fresnel_avg(b.base, b.f82);
        let ms = (1.0 - e1o) * (1.0 - e1i) / (PI * max(1.0 - eavg, 1e-4));
        let fms = favg * favg * eavg / (vec3f(1.0) - favg * (1.0 - eavg));
        f += (fr * s.f + fms * ms) * (b.metal * under);
        pdf += odds[1] * s.pdf;
    }
    let plastic = (1.0 - b.metal) * (1.0 - b.transmission);
    if plastic > 0.0 && wi.z > 0.0 {
        let f0 = f0_of_ior(b.ior);
        let s = spec_reflect(wo, wi, a);
        let es_o = e_spec(f0, mu_o, b.rough);
        let es_i = e_spec(f0, wi.z, b.rough);
        let es_avg = e_spec_avg(f0, b.rough);
        let diffuse = b.base * ((1.0 - es_o) * (1.0 - es_i) / (PI * max(1.0 - es_avg, 1e-4)));
        f += (vec3f(schlick(f0, dot(wo, s.m)) * s.f) + diffuse) * (plastic * under);
        pdf += odds[2] * s.pdf + odds[3] * max(wi.z, 0.0) / PI;
    }
    let glass = (1.0 - b.metal) * b.transmission;
    if glass > 0.0 {
        let g = glass_eval(b, wo, wi, flip, a);
        f += g.f * (glass * under);
        pdf += odds[4] * g.pdf;
    }
    return Eval(f, pdf);
}

struct Sampled {
    wi: vec3f,
    f: vec3f,
    pdf: f32,
}

fn bsdf_sample(b: Mat, wo: vec3f, u: vec3f) -> Sampled {
    let flip = wo.z < 0.0;
    let wl = select(wo, -wo, flip);
    var odds = lobe_odds(b, max(wl.z, 1e-6));
    var x = u.x;
    var lobe = 0;
    while lobe < 4 && x >= odds[lobe] {
        x -= odds[lobe];
        lobe += 1;
    }
    var wi: vec3f;
    if lobe == 0 {
        wi = reflect_about(wl, sample_vndf(wl, alpha_of(b.coat_rough), u.y, u.z));
    } else if lobe <= 2 {
        wi = reflect_about(wl, sample_vndf(wl, alpha_of(b.rough), u.y, u.z));
    } else if lobe == 3 {
        let r = sqrt(u.y);
        let phi = 2.0 * PI * u.z;
        wi = vec3f(r * cos(phi), r * sin(phi), sqrt(max(1.0 - u.y, 0.0)));
    } else {
        let m = sample_vndf(wl, alpha_of(b.rough), u.y, u.z);
        let eta = select(b.ior, 1.0 / b.ior, flip);
        let fr = dielectric_fresnel(dot(wl, m), eta);
        let pick = clamp(x / max(odds[4], 1e-9), 0.0, 1.0);
        let c = dot(wl, m);
        let k = 1.0 - (1.0 - c * c) / (eta * eta);
        if pick < fr || k <= 0.0 {
            wi = reflect_about(wl, m);
        } else {
            wi = normalize(m * (c / eta - sqrt(k)) - wl / eta);
        }
    }
    wi = select(wi, -wi, flip);
    let e = bsdf_eval(b, wo, wi);
    return Sampled(wi, e.f, e.pdf);
}

// ------------------------------------------------------------------ the camera

fn concentric(u: vec2f) -> vec2f {
    let p = 2.0 * u - 1.0;
    if p.x == 0.0 && p.y == 0.0 {
        return vec2f(0.0);
    }
    var r: f32;
    var th: f32;
    if abs(p.x) > abs(p.y) {
        r = p.x;
        th = (PI / 4.0) * (p.y / p.x);
    } else {
        r = p.y;
        th = PI / 2.0 - (PI / 4.0) * (p.x / p.y);
    }
    return r * vec2f(cos(th), sin(th));
}

// ------------------------------------------------------------------ the path

fn basis(n: vec3f) -> mat3x3f {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = vec3f(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let bt = vec3f(b, s + n.y * n.y * a, -n.y);
    return mat3x3f(t, bt, n);
}

fn power(a: f32, b: f32) -> f32 {
    let a2 = a * a;
    return a2 / max(a2 + b * b, 1e-30);
}

@compute @workgroup_size(8, 8)
fn trace_main(@builtin(global_invocation_id) id: vec3u) {
    let w = P.size.x;
    let h = P.size.y;
    if id.x >= w || id.y >= h {
        return;
    }
    let pixel = id.y * w + id.x;
    var rng = Rng(pixel, P.size.z, 0u);
    let jx = next(&rng);
    let jy = next(&rng);
    let sx = 2.0 * (f32(id.x) + jx) / f32(w) - 1.0;
    let sy = 1.0 - 2.0 * (f32(id.y) + jy) / f32(h);
    var o: vec3f;
    var d: vec3f;
    let lu = vec2f(next(&rng), next(&rng));
    if P.eye.w > 0.0 {
        let half_w = P.eye.w * f32(w) / f32(h);
        o = P.eye.xyz + normalize(P.right.xyz) * sx * half_w + normalize(P.up.xyz) * sy * P.eye.w;
        d = P.fwd.xyz;
    } else {
        let dir = P.fwd.xyz + P.right.xyz * sx + P.up.xyz * sy;
        o = P.eye.xyz;
        d = normalize(dir);
        if P.fwd.w > 0.0 {
            let focus = P.eye.xyz + dir * P.lens.x;
            let l = concentric(lu) * P.fwd.w;
            o = P.eye.xyz + normalize(P.right.xyz) * l.x + normalize(P.up.xyz) * l.y;
            d = normalize(focus - o);
        }
    }

    var radiance = vec3f(0.0);
    var through = vec3f(1.0);
    var alpha = 1.0;
    var last_pdf = 0.0;
    var albedo = vec3f(0.0);
    var normal = vec3f(0.0);
    var depth = 0.0;
    let ground_on = P.ground.y > 0.5;
    let scale = max(length(P.eye.xyz), 1.0);
    for (var bounce = 0u; bounce < P.counts.w; bounce += 1u) {
        rng.dim = 8u + bounce * 16u;
        var hit = trace(o, d, BIG, false);
        var on_ground = false;
        if ground_on && d.z < 0.0 && o.z > P.ground.x {
            let tg = (P.ground.x - o.z) / d.z;
            if tg < hit.t {
                on_ground = true;
                hit.t = tg;
            }
        }
        if hit.tri < 0 && !on_ground {
            var le: vec3f;
            if bounce == 0u {
                if P.ground.w < 0.5 {
                    le = env_radiance(d);
                } else {
                    le = P.back.xyz;
                    alpha = select(1.0, 0.0, P.ground.w > 1.5);
                }
                radiance += through * le;
            } else {
                le = env_radiance(d);
                let mis = select(power(last_pdf, env_pdf(d)), 1.0, last_pdf <= 0.0);
                radiance += min(through * le * mis, vec3f(P.lens.w));
            }
            break;
        }
        let p = o + d * hit.t;
        var n: vec3f;
        var ng: vec3f;
        var mat: Mat;
        if on_ground {
            n = vec3f(0.0, 0.0, 1.0);
            ng = n;
            mat = material(u32(P.ground.z));
        } else {
            let k = u32(hit.tri);
            let i = hit.inst;
            let t0 = tris[k * 3u];
            let t1 = tris[k * 3u + 1u];
            let t2 = tris[k * 3u + 2u];
            let nl = unoct(bitcast<u32>(t0.w)) * (1.0 - hit.u - hit.v) + unoct(bitcast<u32>(t1.w)) * hit.u + unoct(bitcast<u32>(t2.w)) * hit.v;
            n = to_world_normal(i, nl);
            ng = to_world_normal(i, cross(t1.xyz, t2.xyz));
            mat = material(bitcast<u32>(row3(i, 3u).y));
        }
        let wo_w = -d;
        // opaque surfaces face the eye; glass keeps its outward normal to know inside from out
        if mat.transmission <= 0.0 || on_ground {
            if dot(ng, wo_w) < 0.0 {
                ng = -ng;
            }
            if dot(n, ng) < 0.0 {
                n = -n;
            }
        } else if dot(n, ng) < 0.0 {
            n = -n;
        }
        if bounce == 0u {
            albedo = select(mat.base, vec3f(1.0), mat.metal > 0.5);
            normal = n;
            depth = hit.t;
        }
        let frame = basis(n);
        let wo = wo_w * frame;
        let eps = 1e-4 * max(scale, length(p));
        // the light, by the map's sampler
        let ls = env_sample(next(&rng), next(&rng));
        let wi_l = ls.d * frame;
        let e = bsdf_eval(mat, wo, wi_l);
        if e.pdf >= 0.0 && max3(e.f) > 0.0 && ls.pdf > 0.0 {
            let side = select(-1.0, 1.0, dot(ls.d, ng) > 0.0);
            let shadow = trace(p + ng * eps * side, ls.d, BIG, true);
            var blocked = shadow.tri >= 0;
            if !blocked && ground_on && ls.d.z < 0.0 && !on_ground {
                blocked = true;
            }
            if !blocked {
                let mis = power(ls.pdf, e.pdf);
                radiance += min(through * e.f * abs(wi_l.z) * ls.rgb * (mis / ls.pdf), vec3f(P.lens.w));
            }
        }
        // the next direction, by the BSDF
        let s = bsdf_sample(mat, wo, vec3f(next(&rng), next(&rng), next(&rng)));
        if s.pdf <= 0.0 || max3(s.f) <= 0.0 {
            break;
        }
        through *= s.f * abs(s.wi.z) / s.pdf;
        last_pdf = s.pdf;
        let wi_w = frame * s.wi;
        let side = select(-1.0, 1.0, dot(wi_w, ng) > 0.0);
        o = p + ng * eps * side;
        d = normalize(wi_w);
        if bounce >= 3u {
            let q = clamp(max3(through), 0.05, 0.95);
            if next(&rng) > q {
                break;
            }
            through /= q;
        }
    }
    accum[pixel] += vec4f(radiance, alpha);
    if P.misc.z == 1u {
        aov[pixel * 2u] += vec4f(albedo, depth);
        aov[pixel * 2u + 1u] += vec4f(normal, 1.0);
    }
}

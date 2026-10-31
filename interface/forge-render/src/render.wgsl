struct Globals {
    view_proj: mat4x4f,
    eye: vec4f,
    // 2 / width, 2 / height, a line's half width in pixels
    px: vec4f,
    text: array<vec4u, 16>,
}
struct Placement {
    r0: vec4f,
    r1: vec4f,
    r2: vec4f,
}
@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var<storage, read> placements: array<Placement>;
@group(0) @binding(2) var<storage, read> segments: array<vec4u>;
@group(0) @binding(3) var<storage, read> tints: array<vec4f>;

fn place(pl: Placement, p: vec3f) -> vec3f {
    let h = vec4f(p, 1.0);
    return vec3f(dot(pl.r0, h), dot(pl.r1, h), dot(pl.r2, h));
}

fn turn(pl: Placement, n: vec3f) -> vec3f {
    return vec3f(dot(pl.r0.xyz, n), dot(pl.r1.xyz, n), dot(pl.r2.xyz, n));
}

fn unoct(v: vec2f) -> vec3f {
    let z = 1.0 - abs(v.x) - abs(v.y);
    let t = max(-z, 0.0);
    return vec3f(v.x - t * sign(v.x), v.y - t * sign(v.y), z);
}

struct Shaded {
    @builtin(position) clip: vec4f,
    @location(0) p: vec3f,
    @location(1) n: vec3f,
    @location(2) tint: vec3f,
}

@vertex
fn surface_vs(@location(0) p: vec3f, @location(1) n: vec2f, @location(2) at: u32) -> Shaded {
    let pl = placements[at];
    let w = place(pl, p);
    return Shaded(g.view_proj * vec4f(w, 1.0), w, turn(pl, unoct(n)), tints[at].rgb);
}

@fragment
fn surface_fs(in: Shaded) -> @location(0) vec4f {
    let n = normalize(in.n);
    let v = normalize(g.eye.xyz - in.p);
    let key = normalize(v + vec3f(0.25, 0.25, 0.75));
    let half_v = normalize(key + v);
    let up = n.z * 0.5 + 0.5;
    let sky = mix(vec3f(0.24, 0.23, 0.22), vec3f(0.50, 0.53, 0.58), up);
    let diffuse = max(dot(n, key), 0.0) * 0.62;
    let shine = pow(max(dot(n, half_v), 0.0), 48.0) * 0.28;
    return vec4f(in.tint * (sky + diffuse) + shine, 1.0);
}

struct Line {
    @builtin(position) clip: vec4f,
    @location(0) colour: vec3f,
}

// where a line's corner goes: along it, across it
const CORNERS = array<vec2f, 6>(
    vec2f(0.0, -1.0), vec2f(1.0, -1.0), vec2f(0.0, 1.0),
    vec2f(0.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
);
// nearer than the surface it lies on by this share of its distance
const LIFT = 1.0 + 3e-4;

fn signed_pair(w: u32) -> vec2f {
    return vec2f(f32(i32(w << 16u) >> 16u), f32(i32(w) >> 16u)) / 32767.0;
}

@vertex
fn line_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> Line {
    let none = Line(vec4f(0.0, 0.0, -1.0, 1.0), vec3f(0.0));
    let pl = placements[at];
    let s0 = segments[vi / 6u * 2u];
    let s1 = segments[vi / 6u * 2u + 1u];
    let wa = place(pl, bitcast<vec3f>(s0.xyz));
    let wb = place(pl, bitcast<vec3f>(s1.xyz));
    var colour = vec3f(0.05, 0.06, 0.08);
    if (s0.w & 0xffffu) == 0x8000u {
        if (s1.w & 0xffffu) != 0u {
            colour = vec3f(0.30, 0.32, 0.35);
        }
    } else {
        let v = g.eye.xyz - wa;
        let front_a = dot(turn(pl, unoct(signed_pair(s0.w))), v);
        let front_b = dot(turn(pl, unoct(signed_pair(s1.w))), v);
        if front_a * front_b > 0.0 {
            return none;
        }
    }
    var a = g.view_proj * vec4f(wa, 1.0);
    var b = g.view_proj * vec4f(wb, 1.0);
    // cut at the near plane: clip z is the near distance
    let near = a.z;
    if a.w < near && b.w < near {
        return none;
    }
    if a.w < near {
        a = mix(a, b, (near - a.w) / (b.w - a.w));
    } else if b.w < near {
        b = mix(b, a, (near - b.w) / (a.w - b.w));
    }
    let to_px = 1.0 / g.px.xy;
    let sa = a.xy / a.w * to_px;
    let sb = b.xy / b.w * to_px;
    let d = sb - sa;
    let len = length(d);
    if len < 1e-4 {
        return none;
    }
    let along = d / len;
    let across = vec2f(-along.y, along.x);
    let c = CORNERS[vi % 6u];
    let end = select(a, b, c.x > 0.5);
    let off = (along * (c.x * 2.0 - 1.0) + across * c.y) * g.px.z * g.px.xy;
    return Line(vec4f(end.xy + off * end.w, end.z * LIFT, end.w), colour);
}

@fragment
fn line_fs(in: Line) -> @location(0) vec4f {
    return vec4f(in.colour, 1.0);
}

// the numbers overlay: a strip of 3x5 glyphs at the top left, scaled up
const SCALE = 3.0;
const GLYPHS = array<u32, 40>(
    0x7B6Fu, 0x749Au, 0x73E7u, 0x79E7u, 0x49EDu, 0x79CFu, 0x7BCFu, 0x24A7u, 0x7BEFu, 0x79EFu,
    0x5BEAu, 0x3AEBu, 0x624Eu, 0x3B6Bu, 0x72CFu, 0x12CFu, 0x6B4Eu, 0x5BEDu, 0x7497u, 0x2B24u,
    0x5AEDu, 0x7249u, 0x5BFDu, 0x5B6Bu, 0x2B6Au, 0x12EBu, 0x676Au, 0x5AEBu, 0x388Eu, 0x2497u,
    0x7B6Du, 0x2B6Du, 0x5FEDu, 0x5AADu, 0x24ADu, 0x72A7u, 0x2000u, 0x0000u, 0x12A4u, 0x0410u,
);

@vertex
fn hud_vs(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4f {
    let c = array<vec2f, 3>(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    return vec4f(c[vi], 0.0, 1.0);
}

@fragment
fn hud_fs(@builtin(position) at: vec4f) -> @location(0) vec4f {
    let p = (at.xy - vec2f(8.0, 8.0)) / SCALE;
    if p.x < 0.0 || p.y < -1.0 || p.y >= 6.0 || p.x >= 1024.0 {
        discard;
    }
    let cell = u32(p.x) / 4u;
    let code = (g.text[cell / 16u][cell / 4u % 4u] >> (cell % 4u * 8u)) & 0xffu;
    if code == 255u {
        discard;
    }
    if p.y < 0.0 || p.y >= 5.0 {
        return vec4f(0.0, 0.0, 0.0, 0.6);
    }
    let x = u32(p.x) % 4u;
    if x < 3u && ((GLYPHS[code] >> (u32(p.y) * 3u + x)) & 1u) == 1u {
        return vec4f(1.0, 1.0, 1.0, 1.0);
    }
    return vec4f(0.0, 0.0, 0.0, 0.6);
}

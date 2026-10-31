struct Globals {
    view_proj: mat4x4f,
    eye: vec4f,
    // 2 / width, 2 / height, a line's half width in pixels
    px: vec4f,
    // the view's axes, for the cube
    right: vec4f,
    up: vec4f,
    forward: vec4f,
    // the cube's region under the cursor
    cube_hover: vec4f,
    // picked vertices: where (w: 0 none, 1 hovered, 2 selected)
    dots: array<vec4f, 8>,
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
// somewhere nothing draws
const NOWHERE = vec4f(0.0, 0.0, -1.0, 1.0);

fn signed_pair(w: u32) -> vec2f {
    return vec2f(f32(i32(w << 16u) >> 16u), f32(i32(w) >> 16u)) / 32767.0;
}

// a segment's corner in clip space, half_px wide either side
fn line_corner(vi: u32, pl: Placement, half_px: f32) -> vec4f {
    let s0 = segments[vi / 6u * 2u];
    let s1 = segments[vi / 6u * 2u + 1u];
    var a = g.view_proj * vec4f(place(pl, bitcast<vec3f>(s0.xyz)), 1.0);
    var b = g.view_proj * vec4f(place(pl, bitcast<vec3f>(s1.xyz)), 1.0);
    // cut at the near plane: clip z is the near distance
    let near = a.z;
    if a.w < near && b.w < near {
        return NOWHERE;
    }
    if a.w < near {
        a = mix(a, b, (near - a.w) / (b.w - a.w));
    } else if b.w < near {
        b = mix(b, a, (near - b.w) / (a.w - b.w));
    }
    let to_px = 1.0 / g.px.xy;
    let d = b.xy / b.w * to_px - a.xy / a.w * to_px;
    let len = length(d);
    if len < 1e-4 {
        return NOWHERE;
    }
    let along = d / len;
    let across = vec2f(-along.y, along.x);
    let c = CORNERS[vi % 6u];
    let end = select(a, b, c.x > 0.5);
    let off = (along * (c.x * 2.0 - 1.0) + across * c.y) * half_px * g.px.xy;
    return vec4f(end.xy + off * end.w, end.z * LIFT, end.w);
}

@vertex
fn line_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> Line {
    let pl = placements[at];
    let s0 = segments[vi / 6u * 2u];
    let s1 = segments[vi / 6u * 2u + 1u];
    var colour = vec3f(0.05, 0.06, 0.08);
    if (s0.w & 0xffffu) == 0x8000u {
        if (s1.w & 0xffffu) != 0u {
            colour = vec3f(0.30, 0.32, 0.35);
        }
    } else {
        let v = g.eye.xyz - place(pl, bitcast<vec3f>(s0.xyz));
        let front_a = dot(turn(pl, unoct(signed_pair(s0.w))), v);
        let front_b = dot(turn(pl, unoct(signed_pair(s1.w))), v);
        if front_a * front_b > 0.0 {
            return Line(NOWHERE, colour);
        }
    }
    return Line(line_corner(vi, pl, g.px.z), colour);
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

// what's under the cursor: a few pixels' worth of the same geometry, with who drew each pixel:
// the placement + 1, and a face number, or for a B-rep edge a segment's with the top bit set
struct Picked {
    @builtin(position) clip: vec4f,
    @location(0) @interpolate(flat) id: vec2u,
}

@vertex
fn surface_pick_vs(@location(0) p: vec3f, @location(2) at: u32, @location(3) face: u32) -> Picked {
    return Picked(g.view_proj * vec4f(place(placements[at], p), 1.0), vec2u(at + 1u, face));
}

@vertex
fn line_pick_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> Picked {
    return Picked(line_corner(vi, placements[at], g.px.z), vec2u(at + 1u, (vi / 6u) | 0x80000000u));
}

@fragment
fn pick_fs(in: Picked) -> @location(0) vec2u {
    return in.id;
}

// a picked entity drawn again on top; the top bit of a placement's number says selected
const HOVER = vec4f(0.20, 0.50, 0.95, 0.40);
const CHOSEN = vec4f(0.98, 0.55, 0.10, 0.50);
const SELECTED_BIT = 0x80000000u;
// just nearer than the same triangles drawn before, so it wins without fighting
const MARK_LIFT = 1.0 + 1e-5;

struct Mark {
    @builtin(position) clip: vec4f,
    @location(0) colour: vec4f,
}

fn mark_colour(selected: bool) -> vec4f {
    return select(HOVER, CHOSEN, selected);
}

@vertex
fn face_mark_vs(@location(0) p: vec3f, @location(2) at: u32) -> Mark {
    let clip = g.view_proj * vec4f(place(placements[at & ~SELECTED_BIT], p), 1.0);
    return Mark(vec4f(clip.xy, clip.z * MARK_LIFT, clip.w), mark_colour((at & SELECTED_BIT) != 0u));
}

@vertex
fn edge_mark_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> Mark {
    let colour = vec4f(mark_colour((at & SELECTED_BIT) != 0u).rgb, 1.0);
    return Mark(line_corner(vi, placements[at & ~SELECTED_BIT], g.px.w), colour);
}

@fragment
fn mark_fs(in: Mark) -> @location(0) vec4f {
    return in.colour;
}

struct Dot {
    @builtin(position) clip: vec4f,
    @location(0) colour: vec4f,
    @location(1) corner: vec2f,
}

@vertex
fn dot_vs(@builtin(vertex_index) vi: u32) -> Dot {
    let d = g.dots[vi / 6u];
    let c = CORNERS[vi % 6u];
    let corner = vec2f(c.x * 2.0 - 1.0, c.y);
    if d.w == 0.0 {
        return Dot(NOWHERE, vec4f(0.0), corner);
    }
    let clip = g.view_proj * vec4f(d.xyz, 1.0);
    let off = corner * 5.0 * g.px.xy;
    let colour = vec4f(mark_colour(d.w > 1.5).rgb, 1.0);
    return Dot(vec4f(clip.xy + off * clip.w, clip.z * LIFT * LIFT, clip.w), colour, corner);
}

@fragment
fn dot_fs(in: Dot) -> @location(0) vec4f {
    if dot(in.corner, in.corner) > 1.0 {
        discard;
    }
    return in.colour;
}

// the view cube: faces with their outward normal and label's right and up
const CUBE_N = array<vec3f, 6>(
    vec3f(0.0, 0.0, 1.0), vec3f(0.0, 0.0, -1.0), vec3f(0.0, -1.0, 0.0),
    vec3f(0.0, 1.0, 0.0), vec3f(1.0, 0.0, 0.0), vec3f(-1.0, 0.0, 0.0),
);
const CUBE_U = array<vec3f, 6>(
    vec3f(1.0, 0.0, 0.0), vec3f(1.0, 0.0, 0.0), vec3f(1.0, 0.0, 0.0),
    vec3f(-1.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0), vec3f(0.0, -1.0, 0.0),
);
const CUBE_V = array<vec3f, 6>(
    vec3f(0.0, 1.0, 0.0), vec3f(0.0, -1.0, 0.0), vec3f(0.0, 0.0, 1.0),
    vec3f(0.0, 0.0, 1.0), vec3f(0.0, 0.0, 1.0), vec3f(0.0, 0.0, 1.0),
);
// TOP, BOTTOM, FRONT, BACK, RIGHT, LEFT as glyph numbers
const CUBE_LABEL = array<array<u32, 6>, 6>(
    array<u32, 6>(29u, 24u, 25u, 37u, 37u, 37u),
    array<u32, 6>(11u, 24u, 29u, 29u, 24u, 22u),
    array<u32, 6>(15u, 27u, 24u, 23u, 29u, 37u),
    array<u32, 6>(11u, 10u, 12u, 20u, 37u, 37u),
    array<u32, 6>(27u, 18u, 16u, 17u, 29u, 37u),
    array<u32, 6>(21u, 14u, 15u, 29u, 37u, 37u),
);
const CUBE_LENGTH = array<u32, 6>(3u, 6u, 5u, 4u, 5u, 4u);
// a corner's reach in the viewport's half (CUBE_SCALE) and a face's rim (CUBE_RIM), as in nav.rs
const CUBE_SCALE = 1.1;
const CUBE_RIM = 0.17;

struct CubeFace {
    @builtin(position) clip: vec4f,
    @location(0) q: vec3f,
    @location(1) @interpolate(flat) face: u32,
    @location(2) uv: vec2f,
}

@vertex
fn cube_vs(@builtin(vertex_index) vi: u32) -> CubeFace {
    let face = vi / 6u;
    let c = CORNERS[vi % 6u];
    let uv = vec2f(c.x - 0.5, c.y * 0.5);
    let q = CUBE_N[face] * 0.5 + CUBE_U[face] * uv.x + CUBE_V[face] * uv.y;
    let at = vec2f(dot(q, g.right.xyz), dot(q, g.up.xyz)) / CUBE_SCALE;
    return CubeFace(vec4f(at, 0.5, 1.0), q, face, uv);
}

@fragment
fn cube_fs(in: CubeFace) -> @location(0) vec4f {
    let n = CUBE_N[in.face];
    let light = 0.65 + 0.35 * max(-dot(n, g.forward.xyz), 0.0);
    let rim = 0.5 - CUBE_RIM;
    let region = vec3f(step(rim, in.q.x) - step(rim, -in.q.x), step(rim, in.q.y) - step(rim, -in.q.y), step(rim, in.q.z) - step(rim, -in.q.z));
    var colour = vec3f(0.80, 0.82, 0.85) * light;
    if dot(g.cube_hover.xyz, g.cube_hover.xyz) > 0.5 && all(region == g.cube_hover.xyz) {
        colour = vec3f(0.30, 0.55, 0.90) * light;
    }
    if max(abs(in.uv.x), abs(in.uv.y)) > 0.485 {
        colour = vec3f(0.15, 0.16, 0.18);
    }
    let unit = 0.036;
    let length = CUBE_LENGTH[in.face];
    let text = vec2f(in.uv.x / unit + f32(length) * 2.0 - 0.5, -in.uv.y / unit + 2.5);
    if text.x >= 0.0 && text.y >= 0.0 && text.y < 5.0 && u32(text.x) / 4u < length {
        let x = u32(text.x) % 4u;
        let code = CUBE_LABEL[in.face][u32(text.x) / 4u];
        if x < 3u && ((GLYPHS[code] >> (u32(text.y) * 3u + x)) & 1u) == 1u {
            colour = vec3f(0.10, 0.11, 0.13);
        }
    }
    return vec4f(colour, 1.0);
}

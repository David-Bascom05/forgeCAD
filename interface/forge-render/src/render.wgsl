struct Globals {
    view_proj: mat4x4f,
    eye: vec4f,
    // 2 / width, 2 / height, a line's half width in pixels
    px: vec4f,
    // the view's axes, for the cube
    right: vec4f,
    up: vec4f,
    forward: vec4f,
    // the view's flat background
    back: vec4f,
    // picked vertices: where (w: 0 none, 1 hovered, 2 selected)
    dots: array<vec4f, 8>,
    // for occlusion: view x and y per clip unit at distance 1, the near plane, the reach in pixels
    ao: vec4f,
    // the view cube's square, which occlusion leaves alone: left, top, right, bottom
    cube_box: vec4f,
    // the cutting plane: what's on its plus side is cut away; all zero cuts nothing
    section: vec4f,
    // a second plane, and x: 0 none, 1 what both cut goes (three quarters left), 2 what either
    // cuts goes (a quarter left); y: 1 shows only the cut faces (the end section)
    section2: vec4f,
    section_mode: vec4f,
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
    @invariant @builtin(position) clip: vec4f,
    @location(0) p: vec3f,
    @location(1) n: vec3f,
    // rgb, and how metal
    @location(2) tint: vec4f,
}

@vertex
fn surface_vs(@location(0) p: vec3f, @location(1) n: vec2f, @location(2) at: u32) -> Shaded {
    let pl = placements[at];
    let w = place(pl, p);
    return Shaded(g.view_proj * vec4f(w, 1.0), w, turn(pl, unoct(n)), tints[at]);
}

// what a shiny surface reflects: a bright sky, a band of light low down, a darker floor
fn studio(r: vec3f) -> vec3f {
    let sky = mix(vec3f(0.42, 0.44, 0.48), vec3f(0.95, 0.97, 1.0), smoothstep(0.0, 0.8, r.z));
    let floor = mix(vec3f(0.24, 0.24, 0.26), vec3f(0.40, 0.40, 0.42), smoothstep(-1.0, 0.0, r.z));
    let band = smoothstep(0.10, 0.0, abs(r.z - 0.06)) * 0.55;
    return mix(floor, sky, smoothstep(-0.04, 0.04, r.z)) + band;
}

fn shade(in: Shaded) -> vec4f {
    let n = normalize(in.n);
    let v = normalize(g.eye.xyz - in.p);
    let nv = max(dot(n, v), 0.0);
    let base = in.tint.rgb;
    let metal = in.tint.a;
    // a key light over the shoulder of the eye, the room's light from above
    let key = normalize(v + normalize(g.up.xyz * 0.7 + g.right.xyz * 0.35));
    let nl = max(dot(n, key), 0.0);
    let room = mix(vec3f(0.26, 0.25, 0.24), vec3f(0.56, 0.58, 0.62), n.z * 0.5 + 0.5);
    let diffuse = base * (1.0 - metal) * (room + nl * 0.55);
    // Schlick's Fresnel, a Blinn lobe for the key, the studio for the rest
    let f0 = mix(vec3f(0.04), base, metal);
    let fresnel = f0 + (vec3f(1.0) - f0) * pow(1.0 - nv, 5.0);
    let h = normalize(key + v);
    let lobe = pow(max(dot(n, h), 0.0), mix(40.0, 90.0, metal)) * mix(0.18, 0.55, metal);
    let spec = fresnel * studio(reflect(-v, n)) * mix(0.35, 0.95, metal) + lobe * f0 * 2.0;
    return vec4f(diffuse + spec, 1.0);
}

@fragment
fn surface_fs(in: Shaded) -> @location(0) vec4f {
    return shade(in);
}

fn plus(pl: vec4f, p: vec3f) -> bool {
    return dot(pl.xyz, p) + pl.w > 0.0;
}

fn cut_away(p: vec3f) -> bool {
    let a = plus(g.section, p);
    if g.section_mode.x < 0.5 {
        return a;
    }
    let b = plus(g.section2, p);
    if g.section_mode.x < 1.5 {
        return a && b;
    }
    return a || b;
}

// where along the ray from the eye (0) to p (1) one plane's plus side is: lo..hi, empty if lo > hi
fn span_on(pl: vec4f, p: vec3f) -> vec2f {
    let se = dot(pl.xyz, g.eye.xyz) + pl.w;
    let sp = dot(pl.xyz, p) + pl.w;
    let t = se / (se - sp);
    if se > 0.0 && sp > 0.0 {
        return vec2f(0.0, 1.0);
    }
    if se > 0.0 {
        return vec2f(0.0, t);
    }
    if sp > 0.0 {
        return vec2f(t, 1.0);
    }
    return vec2f(1.0, 0.0);
}

@fragment
fn surface_cut_fs(in: Shaded) -> @location(0) vec4f {
    if cut_away(in.p) {
        discard;
    }
    return shade(in);
}

// the inside of a cut part, seen through the cut: the ray to it crosses the plane where the cut
// face is, so it colours itself as that, hatched with lines a steady size on screen. It stays at
// its own depth: only a surface in front of it can be seen instead, and then the cut isn't there.
@fragment
fn cap_fs(in: Shaded) -> @location(0) vec4f {
    if cut_away(in.p) {
        discard;
    }
    // the last place the ray to it leaves what's cut away, and the plane it leaves by
    let a = span_on(g.section, in.p);
    var t = select(-1.0, a.y, a.x <= a.y);
    var n = g.section.xyz;
    if g.section_mode.x > 0.5 {
        let b = span_on(g.section2, in.p);
        let b_ok = b.x <= b.y;
        if g.section_mode.x < 1.5 {
            // both: where both hold, its far end
            let lo = max(a.x, b.x);
            let hi = min(a.y, b.y);
            t = select(-1.0, hi, a.x <= a.y && b_ok && lo <= hi);
            if b.y < a.y {
                n = g.section2.xyz;
            }
        } else if b_ok && b.y > t {
            t = b.y;
            n = g.section2.xyz;
        }
    }
    if t <= 0.0 || t >= 1.0 {
        discard;
    }
    let ray = in.p - g.eye.xyz;
    let q = g.eye.xyz + ray * t;
    let px_mm = dot(g.forward.xyz, q - g.eye.xyz) * g.ao.y * g.px.y;
    let spacing = pow(2.0, ceil(log2(px_mm * 9.0)));
    let side = select(vec3f(1.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0), abs(n.x) > 0.5);
    let u = normalize(cross(n, side));
    let v = cross(n, u);
    let stripe = fract((dot(q, u) + dot(q, v)) / spacing);
    let line = 1.0 - smoothstep(0.0, 0.16, min(stripe, 1.0 - stripe));
    // the cut face bold against the part's own surfaces: the part's tint pulled to a warm red
    let cap = mix(in.tint.rgb, vec3f(0.86, 0.30, 0.18), 0.55);
    return vec4f(mix(cap, vec3f(0.10, 0.05, 0.04), line * 0.7), 1.0);
}

// behind everything: light at the top of the window, a little darker below
@fragment
fn back_fs(@builtin(position) at: vec4f) -> @location(0) vec4f {
    return vec4f(g.back.rgb, 1.0);
}

struct Line {
    @builtin(position) clip: vec4f,
    @location(0) colour: vec3f,
    // how far across the line, in pixels from its middle
    @location(1) across: f32,
}

// where a line's corner goes: along it, across it
const CORNERS = array<vec2f, 6>(
    vec2f(0.0, -1.0), vec2f(1.0, -1.0), vec2f(0.0, 1.0),
    vec2f(0.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
);
// nearer than the surface it lies on by this share of its distance, at the ordinary lens (a long
// lens's part is a smaller share of its distance: g.eye.w scales these), and no less than f32 can hold
const LIFT = 3e-4;
const LIFT_MIN = 3e-7;
// somewhere nothing draws
const NOWHERE = vec4f(0.0, 0.0, -1.0, 1.0);
// lines fade over this many pixels at their sides, past their width
const FEATHER = 0.75;
// a segment is four vertices (lines are drawn with a shared index pattern, so each is shaded once)
const LINE_CORNERS = array<vec2f, 4>(vec2f(0.0, -1.0), vec2f(1.0, -1.0), vec2f(0.0, 1.0), vec2f(1.0, 1.0));

fn signed_pair(w: u32) -> vec2f {
    return vec2f(f32(i32(w << 16u) >> 16u), f32(i32(w) >> 16u)) / 32767.0;
}

// a segment's corner in clip space, half_px wide either side
fn line_corner(vi: u32, pl: Placement, half_px: f32) -> vec4f {
    let s0 = segments[vi / 4u * 2u];
    let s1 = segments[vi / 4u * 2u + 1u];
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
    let c = LINE_CORNERS[vi % 4u];
    let end = select(a, b, c.x > 0.5);
    let off = (along * (c.x * 2.0 - 1.0) + across * c.y) * half_px * g.px.xy;
    return vec4f(end.xy + off * end.w, end.z * (1.0 + max(LIFT * g.eye.w, LIFT_MIN)), end.w);
}

// where in the world a segment's corner is, to the end its corner is at
fn line_end_at(vi: u32, pl: Placement) -> vec3f {
    let end = segments[vi / 4u * 2u + u32(LINE_CORNERS[vi % 4u].x > 0.5)];
    return place(pl, bitcast<vec3f>(end.xyz));
}

fn line_at(vi: u32, at: u32) -> Line {
    let pl = placements[at];
    let s0 = segments[vi / 4u * 2u];
    let s1 = segments[vi / 4u * 2u + 1u];
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
            return Line(NOWHERE, colour, 0.0);
        }
    }
    let half_px = g.px.z + FEATHER;
    return Line(line_corner(vi, pl, half_px), colour, LINE_CORNERS[vi % 4u].y * half_px);
}

@vertex
fn line_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> Line {
    return line_at(vi, at);
}

struct CutLine {
    @builtin(position) clip: vec4f,
    @location(0) colour: vec3f,
    @location(1) across: f32,
    @location(2) p: vec3f,
}

@vertex
fn line_cut_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> CutLine {
    let l = line_at(vi, at);
    return CutLine(l.clip, l.colour, l.across, line_end_at(vi, placements[at]));
}

@fragment
fn line_cut_fs(in: CutLine) -> @location(0) vec4f {
    if cut_away(in.p) {
        discard;
    }
    let half_px = g.px.z + FEATHER;
    return vec4f(in.colour, saturate((half_px - abs(in.across)) / FEATHER));
}

@fragment
fn line_fs(in: Line) -> @location(0) vec4f {
    let half_px = g.px.z + FEATHER;
    return vec4f(in.colour, saturate((half_px - abs(in.across)) / FEATHER));
}

// the numbers overlay: a strip of 3x5 glyphs at the top left, scaled up
const SCALE = 3.0;
const HUD_COLUMNS = 85.0;
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
    let p0 = (at.xy - vec2f(8.0, 8.0)) / SCALE;
    if p0.x < 0.0 || p0.y < -1.0 || p0.y >= 20.0 || p0.x >= 4.0 * HUD_COLUMNS {
        discard;
    }
    // rows of text, seven units apart
    let row = u32(p0.y + 1.0) / 7u;
    let p = vec2f(p0.x, p0.y - f32(row) * 7.0);
    let cell = row * u32(HUD_COLUMNS) + u32(p.x) / 4u;
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
    @location(1) p: vec3f,
}

@vertex
fn surface_pick_vs(@location(0) p: vec3f, @location(2) at: u32, @location(3) face: u32) -> Picked {
    let w = place(placements[at], p);
    return Picked(g.view_proj * vec4f(w, 1.0), vec2u(at + 1u, face), w);
}

@vertex
fn line_pick_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> Picked {
    let pl = placements[at];
    return Picked(line_corner(vi, pl, g.px.z), vec2u(at + 1u, (vi / 4u) | 0x80000000u), line_end_at(vi, pl));
}

@fragment
fn pick_fs(in: Picked) -> @location(0) vec2u {
    if cut_away(in.p) {
        discard;
    }
    return in.id;
}

// a picked entity drawn again on top; the top bit of a placement's number says selected
const HOVER = vec4f(0.20, 0.50, 0.95, 0.40);
// the theme's focus amber, in linear
const CHOSEN = vec4f(0.745, 0.361, 0.011, 0.45);
const SELECTED_BIT = 0x80000000u;
// just nearer than the same triangles drawn before, so it wins without fighting
const MARK_LIFT = 1e-5;
const MARK_LIFT_MIN = 1.5e-7;

struct Mark {
    @builtin(position) clip: vec4f,
    @location(0) colour: vec4f,
}

fn mark_colour(selected: bool) -> vec4f {
    return select(HOVER, CHOSEN, selected);
}

struct ExactMark {
    @invariant @builtin(position) clip: vec4f,
    @location(0) colour: vec4f,
}

// the very triangles the surface drew, at the very depth: the fill can't fight it
@vertex
fn face_mark_exact_vs(@location(0) p: vec3f, @location(2) at: u32) -> ExactMark {
    let w = place(placements[at & ~SELECTED_BIT], p);
    return ExactMark(g.view_proj * vec4f(w, 1.0), mark_colour((at & SELECTED_BIT) != 0u));
}

@fragment
fn exact_mark_fs(in: ExactMark) -> @location(0) vec4f {
    return in.colour;
}

@vertex
fn face_mark_vs(@location(0) p: vec3f, @location(2) at: u32) -> Mark {
    let clip = g.view_proj * vec4f(place(placements[at & ~SELECTED_BIT], p), 1.0);
    return Mark(vec4f(clip.xy, clip.z * (1.0 + max(MARK_LIFT * g.eye.w, MARK_LIFT_MIN)), clip.w), mark_colour((at & SELECTED_BIT) != 0u));
}

@vertex
fn edge_mark_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> Mark {
    let colour = vec4f(mark_colour((at & SELECTED_BIT) != 0u).rgb, 1.0);
    return Mark(line_corner(vi, placements[at & ~SELECTED_BIT], g.px.w), colour);
}

// a dark band behind a marked edge, so it reads on anything
@vertex
fn edge_halo_vs(@builtin(vertex_index) vi: u32, @location(0) at: u32) -> Mark {
    return Mark(line_corner(vi, placements[at & ~SELECTED_BIT], g.px.w * 2.5), vec4f(0.0, 0.0, 0.0, 0.75));
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

// the view cube: triangles in window pixels, built on the CPU (viewcube.rs)
struct CubeVert {
    @builtin(position) clip: vec4f,
    @location(0) colour: vec4f,
}

@vertex
fn cube_vs(@location(0) px: vec2f, @location(1) colour: vec4f) -> CubeVert {
    return CubeVert(vec4f(px.x * g.px.x - 1.0, 1.0 - px.y * g.px.y, 0.5, 1.0), colour);
}

// world triangles, translucent: the origin's planes, axes and point
@vertex
fn world_vs(@location(0) p: vec3f, @location(1) colour: vec4f) -> CubeVert {
    let clip = g.view_proj * vec4f(p, 1.0);
    return CubeVert(vec4f(clip.xy, clip.z * (1.0 + max(LIFT * g.eye.w, LIFT_MIN)), clip.w), colour);
}

@fragment
fn cube_fs(in: CubeVert) -> @location(0) vec4f {
    return in.colour;
}

// ambient occlusion: how much of the hemisphere above each pixel its neighbours on screen hide,
// worked out at half size from the scene's depth, smoothed keeping to surfaces, then multiplied in
@group(1) @binding(0) var scene_depth: texture_depth_multisampled_2d;
@group(1) @binding(1) var ao_in: texture_2d<f32>;
const AO_SAMPLES = 8;
const AO_STRENGTH = 2.6;
// a fragment's own slope, not occlusion
const AO_BIAS = 0.12;
const AO_TILE = array<u32, 16>(0u, 8u, 2u, 10u, 12u, 4u, 14u, 6u, 3u, 11u, 1u, 9u, 15u, 7u, 13u, 5u);

// a pixel's point in view space: x right, y up, z ahead; z zero where nothing was drawn
fn view_at(px: vec2i) -> vec3f {
    let d = textureLoad(scene_depth, px, 0);
    if d <= 0.0 {
        return vec3f(0.0);
    }
    let z = g.ao.z / d;
    let clip = (vec2f(px) + 0.5) * g.px.xy - 1.0;
    return vec3f(clip.x * g.ao.x * z, -clip.y * g.ao.y * z, z);
}

@fragment
fn ao_fs(@builtin(position) at: vec4f) -> @location(0) vec4f {
    let px = vec2i(at.xy * 2.0);
    let p = view_at(px);
    if p.z == 0.0 {
        return vec4f(1.0, 0.0, 0.0, 1.0);
    }
    // the surface's normal from the nearer neighbour either way, so an edge doesn't bend it
    let l = view_at(px - vec2i(2, 0));
    let r = view_at(px + vec2i(2, 0));
    let b = view_at(px - vec2i(0, 2));
    let t = view_at(px + vec2i(0, 2));
    let dx = select(r - p, p - l, abs(l.z - p.z) < abs(r.z - p.z) && l.z != 0.0);
    let dy = select(t - p, p - b, abs(b.z - p.z) < abs(t.z - p.z) && b.z != 0.0);
    var n = normalize(cross(dx, dy));
    if dot(n, p) > 0.0 {
        n = -n;
    }
    let reach = g.ao.w * p.z * g.ao.y * g.px.y;
    // a 4 x 4 tile of turns, which the smoothing below exactly averages out
    let tile = vec2u(at.xy) % 4u;
    let noise = f32(AO_TILE[tile.y * 4u + tile.x]) / 16.0 * 6.2831853;
    var hidden = 0.0;
    for (var i = 0; i < AO_SAMPLES; i += 1) {
        let u = (f32(i) + 0.5) / f32(AO_SAMPLES);
        let turn_by = f32(i) * 2.3999632 + noise;
        let off = vec2f(cos(turn_by), sin(turn_by)) * sqrt(u) * g.ao.w;
        let q = view_at(px + vec2i(off));
        if q.z == 0.0 {
            continue;
        }
        let d = q - p;
        let far2 = dot(d, d);
        // what's near counts in full, then less, and nothing far enough off to be a halo
        let fall = saturate(2.0 - far2 / (reach * reach * 2.25));
        hidden += max(dot(d, n) / sqrt(far2 + 1e-6) - AO_BIAS, 0.0) * fall;
    }
    let ao = saturate(1.0 - AO_STRENGTH * hidden / f32(AO_SAMPLES));
    return vec4f(ao, p.z, 0.0, 1.0);
}

// over the noise's 4 x 4 pattern, only from the same surface
@fragment
fn ao_blur_fs(@builtin(position) at: vec4f) -> @location(0) vec4f {
    let px = vec2i(at.xy);
    let centre = textureLoad(ao_in, px, 0);
    if centre.g == 0.0 {
        return centre;
    }
    var sum = 0.0;
    var weight = 0.0;
    for (var y = -2; y < 2; y += 1) {
        for (var x = -2; x < 2; x += 1) {
            let s = textureLoad(ao_in, px + vec2i(x, y), 0);
            let w = max(1.0 - abs(s.g - centre.g) / (centre.g * 0.01), 0.0);
            sum += s.r * w;
            weight += w;
        }
    }
    return vec4f(sum / max(weight, 1e-4), centre.g, 0.0, 1.0);
}

// back to full size, from the half-size pixels on this pixel's own surface; blended by multiplying
@fragment
fn ao_apply_fs(@builtin(position) at: vec4f) -> @location(0) vec4f {
    if at.x >= g.cube_box.x && at.y >= g.cube_box.y && at.x < g.cube_box.z && at.y < g.cube_box.w {
        return vec4f(1.0);
    }
    let px = vec2i(at.xy);
    let d = textureLoad(scene_depth, px, 0);
    if d <= 0.0 {
        return vec4f(1.0);
    }
    let z = g.ao.z / d;
    let half_px = (at.xy - 1.0) * 0.5;
    let base = vec2i(floor(half_px));
    let f = half_px - floor(half_px);
    var sum = 0.0;
    var weight = 0.0;
    for (var y = 0; y < 2; y += 1) {
        for (var x = 0; x < 2; x += 1) {
            let s = textureLoad(ao_in, base + vec2i(x, y), 0);
            let bilinear = abs(1.0 - f32(x) - f.x) * abs(1.0 - f32(y) - f.y);
            let w = bilinear * max(1.0 - abs(s.g - z) / (z * 0.01), 0.001);
            sum += s.r * w;
            weight += w;
        }
    }
    return vec4f(vec3f(sum / max(weight, 1e-5)), 1.0);
}

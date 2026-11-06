// A DWG scene in 2D: segments as screen-space quads (instanced, one quad a segment) and fills as
// triangles, both in the scene's coordinates less its origin. Pan and zoom are this uniform
// alone; a layer's switch and an item's selection are a word each in storage, read here.

struct U {
    // the canvas centre, in the scene's coordinates less its origin
    center: vec2<f32>,
    // half the canvas, in device pixels
    half: vec2<f32>,
    // device pixels a unit
    scale: f32,
    // device pixels a millimetre of lineweight; 0 draws every line 1 px (LWDISPLAY off)
    lw: f32,
    // a layout's paper: white (ACI 7) is drawn black
    paper: u32,
    pad: u32,
    selected: vec4<f32>,
    hovered: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: U;
// bit 0: the layer is shown (on and thawed)
@group(0) @binding(1) var<storage, read> layers: array<u32>;
// bit 0: the item's entity is selected; bit 1: under the pointer; bit 2: gone (edited since:
// drawn from the edit's own scene, or erased)
@group(0) @binding(2) var<storage, read> flags: array<u32>;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) colour: vec4<f32>,
};

fn to_px(p: vec2<f32>) -> vec2<f32> {
    return (p - u.center) * u.scale;
}

fn clip(px: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(px.x / u.half.x, px.y / u.half.y, 0.0, 1.0);
}

fn colour(rgb: u32, f: u32) -> vec4<f32> {
    if (f & 1u) != 0u {
        return u.selected;
    }
    if (f & 2u) != 0u {
        return u.hovered;
    }
    let c4 = unpack4x8unorm(rgb);
    // a mask (wipeout, text background): the canvas's own colour (model: dwgdoc_ui's MODEL_BG)
    if c4.a < 0.5 {
        if u.paper != 0u {
            return vec4<f32>(1.0, 1.0, 1.0, 1.0);
        }
        return vec4<f32>(0.1059, 0.1294, 0.1608, 1.0);
    }
    var c = c4.rgb;
    if u.paper != 0u && all(c > vec3<f32>(0.99)) {
        c = vec3<f32>(0.0);
    }
    return vec4<f32>(c, 1.0);
}

fn hidden() -> Out {
    var o: Out;
    o.pos = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    o.colour = vec4<f32>(0.0);
    return o;
}

// a 1 px line's ends at pixel centres, so a level or plumb line is one crisp row of pixels
fn snap(p: vec2<f32>) -> vec2<f32> {
    return floor(p + u.half) + vec2<f32>(0.5) - u.half;
}

@vertex
fn vs_line(
    @builtin(vertex_index) vi: u32,
    @location(0) a: vec2<f32>,
    @location(1) b: vec2<f32>,
    @location(2) rgb: u32,
    @location(3) weight: f32,
    @location(4) layer: u32,
    @location(5) item: u32,
) -> Out {
    if (layers[layer] & 1u) == 0u {
        return hidden();
    }
    let f = flags[item];
    if (f & 4u) != 0u {
        return hidden();
    }
    var w = 1.0;
    if u.lw > 0.0 {
        w = max(1.0, round(weight * u.lw));
    }
    if (f & 3u) != 0u {
        w = w + 1.0;
    }
    var pa = to_px(a);
    var pb = to_px(b);
    if w < 1.5 {
        pa = snap(pa);
        pb = snap(pb);
    }
    let d = pb - pa;
    let len = length(d);
    var dir = vec2<f32>(1.0, 0.0);
    if len > 1e-4 {
        dir = d / len;
    }
    let n = vec2<f32>(-dir.y, dir.x);
    var ends = array<f32, 6>(0.0, 1.0, 1.0, 0.0, 1.0, 0.0);
    var sides = array<f32, 6>(-1.0, -1.0, 1.0, -1.0, 1.0, 1.0);
    let t = ends[vi];
    let s = sides[vi];
    // a thick line's ends squared off past its points, so its corners close; a 1 px line's to
    // its far pixel, so a short one still shows
    let cap = max(w, 1.0) * 0.5;
    let p = mix(pa, pb, t) + dir * (t * 2.0 - 1.0) * cap + n * s * w * 0.5;
    var o: Out;
    o.pos = clip(p);
    o.colour = colour(rgb, f);
    return o;
}

@vertex
fn vs_fill(
    @location(0) p: vec2<f32>,
    @location(1) rgb: u32,
    @location(2) layer: u32,
    @location(3) item: u32,
) -> Out {
    if (layers[layer] & 1u) == 0u {
        return hidden();
    }
    let f = flags[item];
    if (f & 4u) != 0u {
        return hidden();
    }
    var o: Out;
    o.pos = clip(to_px(p));
    o.colour = colour(rgb, f);
    return o;
}

@fragment
fn fs(i: Out) -> @location(0) vec4<f32> {
    return i.colour;
}

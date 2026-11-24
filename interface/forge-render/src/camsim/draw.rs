//! The simulation's geometry for one frame, as world triangles (`origin::WorldVert`, linear
//! colour): the toolpath as screen-facing ribbons, the stock's columns merged into runs, the tool
//! revolved from its profile, the workholding's boxes and the markers. Light is baked into the
//! opaque pieces' colours (the renderer draws them as they come).

use super::timeline::{Kind, Seg, Timeline};
use super::zmap::{Compare, PartMap, ZMap, compare};
use crate::origin::WorldVert;
use crate::viewcube::rgb;
use forge_cam_geom::tool::{PSeg, Part, Shape};
use forge_math::dmath;

/// The simulation's colours (forgeCAD's own; linear).
pub mod look {
    use crate::viewcube::rgb;
    pub const RAPID: [f32; 3] = rgb(0xE8B13A);
    pub const CUT: [f32; 3] = rgb(0x3D8BEA);
    pub const ARC: [f32; 3] = rgb(0x35C6D8);
    pub const PLUNGE: [f32; 3] = rgb(0xD0508C);
    pub const LINK: [f32; 3] = rgb(0x4DB88A);
    pub const STOCK: [f32; 3] = rgb(0xB9C0C8);
    pub const UNDER: [f32; 3] = rgb(0x3B6FE0);
    pub const IN_TOL: [f32; 3] = rgb(0x3FAE5A);
    pub const GOUGE: [f32; 3] = rgb(0xE0453A);
    pub const CUTTER: [f32; 3] = rgb(0xD9A441);
    pub const SHANK: [f32; 3] = rgb(0x9097A0);
    pub const HOLDER: [f32; 3] = rgb(0x5D646E);
    pub const FIXTURE: [f32; 3] = rgb(0x6C7A89);
    pub const PART: [f32; 3] = rgb(0x8FA6BF);
    pub const LEFTOVER: [f32; 3] = rgb(0xF08A24);
    pub const FAIL: [f32; 3] = rgb(0xF03A3A);
    pub const NO_VALUE: [f32; 3] = rgb(0x6B7078);
    /// A measured value's bands: too little, right, too much.
    pub const LOW: [f32; 3] = rgb(0x4C8FE0);
    pub const GOOD: [f32; 3] = rgb(0x46B55A);
    pub const HIGH: [f32; 3] = rgb(0xE5A23A);
    pub const OVER: [f32; 3] = rgb(0xE0453A);
}

/// A piece's colour.
pub fn colour_of(g: &Seg) -> [f32; 3] {
    match g.kind {
        Kind::Rapid => look::RAPID,
        Kind::Plunge => look::PLUNGE,
        Kind::Link => look::LINK,
        Kind::Cut if g.arc.is_some() => look::ARC,
        Kind::Cut => look::CUT,
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn len(a: [f32; 3]) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn f3(p: [f64; 3]) -> [f32; 3] {
    [p[0] as f32, p[1] as f32, p[2] as f32]
}

/// How the view sees the world: the eye, whether it is parallel (and then its direction), and
/// the size of a pixel at distance 1 (perspective) or anywhere (parallel).
#[derive(Clone, Copy, Debug)]
pub struct Eye {
    pub at: [f32; 3],
    pub forward: [f32; 3],
    pub parallel: bool,
    /// mm per pixel at distance 1 from the eye (perspective) or everywhere (parallel).
    pub mm_px: f32,
}

impl Eye {
    fn px(&self, p: [f32; 3]) -> f32 {
        if self.parallel {
            self.mm_px
        } else {
            len(sub(p, self.at)) * self.mm_px
        }
    }
    fn view(&self, p: [f32; 3]) -> [f32; 3] {
        if self.parallel {
            self.forward
        } else {
            sub(p, self.at)
        }
    }
}

/// A line from a to b `w` pixels wide, facing the eye.
pub fn ribbon(eye: &Eye, a: [f32; 3], b: [f32; 3], w: f32, c: [f32; 4], out: &mut Vec<WorldVert>) {
    let d = sub(b, a);
    let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5];
    let n = cross(d, eye.view(mid));
    let l = len(n);
    if l < 1e-12 {
        return;
    }
    let h = 0.5 * w * eye.px(mid) / l;
    let n = [n[0] * h, n[1] * h, n[2] * h];
    let p = [
        [a[0] - n[0], a[1] - n[1], a[2] - n[2]],
        [b[0] - n[0], b[1] - n[1], b[2] - n[2]],
        [b[0] + n[0], b[1] + n[1], b[2] + n[2]],
        [a[0] + n[0], a[1] + n[1], a[2] + n[2]],
    ];
    for k in [0, 1, 2, 0, 2, 3] {
        out.push(WorldVert { at: p[k], colour: c });
    }
}

/// What of the toolpath to draw.
pub struct PathShow<'a> {
    /// Per op: shown or not.
    pub ops: &'a [bool],
    pub rapids: bool,
    /// The time played: pieces before it bright, after it faint (None: all bright).
    pub upto: Option<f64>,
    /// Only what was played (hide what's ahead).
    pub played_only: bool,
    pub width: f32,
    /// A piece drawn wider (the move list's selection).
    pub picked: Option<usize>,
    /// Colour cutting pieces by a measured value instead of by kind (`Values`).
    pub values: Option<&'a Values>,
}

/// A measured value per piece of the timeline (chip load, engagement, ..) and how it is
/// coloured: each piece's value as a fraction of its scale, mapped through the bands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Values {
    /// What it is, and its unit (the legend's words).
    pub name: String,
    pub unit: String,
    /// Per piece (`Timeline::segs`): the value and the scale it is read against (None: no
    /// value, drawn grey).
    pub per_piece: Vec<Option<(f32, f32)>>,
    /// Bands of value / scale: upper ends, with each band's colour and word.
    pub bands: Vec<(f32, [f32; 3], String)>,
}

impl Values {
    /// The colour of a value against its scale.
    pub fn colour(&self, v: f32, scale: f32) -> [f32; 3] {
        let f = if scale > 0.0 { v / scale } else { v };
        self.bands
            .iter()
            .find(|b| f <= b.0)
            .or(self.bands.last())
            .map_or(look::CUT, |b| b.1)
    }
}

/// The toolpath's ribbons.
pub fn toolpath(tl: &Timeline, eye: &Eye, show: &PathShow, out: &mut Vec<WorldVert>) {
    let mut pts = Vec::with_capacity(64);
    for (i, g) in tl.segs.iter().enumerate() {
        if !show.ops.get(g.op as usize).copied().unwrap_or(true) {
            continue;
        }
        if g.kind == Kind::Rapid && !show.rapids {
            continue;
        }
        let ahead = show.upto.is_some_and(|t| g.t0 >= t);
        if ahead && show.played_only {
            continue;
        }
        let c = match show.values.filter(|_| g.kind != Kind::Rapid) {
            Some(vals) => match vals.per_piece.get(i).copied().flatten() {
                Some((v, sc)) => vals.colour(v, sc),
                None => look::NO_VALUE,
            },
            None => colour_of(g),
        };
        let alpha = if ahead { 0.28 } else { 0.95 };
        let w = if show.picked == Some(i) {
            show.width * 3.0
        } else {
            show.width
        };
        // the piece now playing: bright up to the tool, faint after
        let split = show
            .upto
            .filter(|t| *t > g.t0 && *t < g.t1())
            .map(|t| (t - g.t0) / g.dt.max(1e-12));
        pts.clear();
        g.points(5f64.to_radians(), &mut pts);
        let n = pts.len() - 1;
        for k in 0..n {
            let (a, b) = (f3(pts[k]), f3(pts[k + 1]));
            let al = match split {
                Some(u) if (k as f64 + 0.5) / n as f64 > u => 0.28,
                _ => alpha,
            };
            ribbon(eye, a, b, w, [c[0], c[1], c[2], al], out);
        }
    }
}

/// Baked light: a face's brightness by its normal (a light over the viewer's shoulder, a little
/// from above).
fn shade(c: [f32; 3], n: [f32; 3]) -> [f32; 4] {
    let l = [0.35f32, -0.45, 0.82];
    let ll = len(l);
    let d = (n[0] * l[0] + n[1] * l[1] + n[2] * l[2]) / ll;
    let k = 0.38 + 0.62 * d.max(0.0) + 0.12 * (-d).max(0.0);
    [c[0] * k, c[1] * k, c[2] * k, 1.0]
}

/// The baked light, for other drawers (the machine's bodies).
pub fn shade_pub(c: [f32; 3], n: [f32; 3]) -> [f32; 4] {
    shade(c, n)
}

fn quad(p: [[f32; 3]; 4], c: [f32; 4], out: &mut Vec<WorldVert>) {
    for k in [0, 1, 2, 0, 2, 3] {
        out.push(WorldVert { at: p[k], colour: c });
    }
}

/// How the stock is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StockColour {
    Material,
    /// Against the part: under-cut, in tolerance, gouge (display classes, never a verdict).
    Compare,
}

/// A column's colour.
fn column_colour(g: &ZMap, part: Option<&PartMap>, k: usize, mode: StockColour, tol: f32) -> [f32; 3] {
    match (mode, part) {
        (StockColour::Compare, Some(pm)) => match compare(g, pm, k, tol) {
            Some((Compare::Gouge, _)) => look::GOUGE,
            Some((Compare::InTol, _)) => look::IN_TOL,
            Some((Compare::Under, d)) => {
                // deeper blue the more is left (to 5 mm)
                let f = (d / 5.0).clamp(0.0, 1.0) * 0.55 + 0.45;
                [look::UNDER[0] * f, look::UNDER[1] * f, look::UNDER[2] * f]
            }
            None => look::STOCK,
        },
        _ => look::STOCK,
    }
}

/// The stock's surface: each row's columns merged into runs of one height and colour (tops and
/// bottoms), and walls where neighbours differ.
pub fn stock(g: &ZMap, part: Option<&PartMap>, mode: StockColour, tol: f32, out: &mut Vec<WorldVert>) {
    let (nx, ny, c) = (g.nx, g.ny, g.cell as f32);
    let x = |i: usize| g.x0 as f32 + i as f32 * c;
    let y = |j: usize| g.y0 as f32 + j as f32 * c;
    let full = |k: usize| g.hi[k] > g.lo[k];
    let cols: Vec<[f32; 3]> = (0..nx * ny)
        .map(|k| column_colour(g, part, k, mode, tol))
        .collect();
    for j in 0..ny {
        let mut i = 0;
        while i < nx {
            let k = j * nx + i;
            if !full(k) {
                i += 1;
                continue;
            }
            let (h, l, col) = (g.hi[k], g.lo[k], cols[k]);
            let mut e = i + 1;
            while e < nx
                && full(j * nx + e)
                && g.hi[j * nx + e] == h
                && g.lo[j * nx + e] == l
                && cols[j * nx + e] == col
            {
                e += 1;
            }
            let (x0, x1, y0, y1) = (x(i), x(e), y(j), y(j + 1));
            quad(
                [[x0, y0, h], [x1, y0, h], [x1, y1, h], [x0, y1, h]],
                shade(col, [0.0, 0.0, 1.0]),
                out,
            );
            quad(
                [[x0, y0, l], [x0, y1, l], [x1, y1, l], [x1, y0, l]],
                shade(look::STOCK, [0.0, 0.0, -1.0]),
                out,
            );
            i = e;
        }
    }
    // walls: between each column and its +x / +y neighbour (or the grid's edge), the part of one
    // column's interval the other lacks
    let wall = |out: &mut Vec<WorldVert>,
                k: usize,
                other: Option<usize>,
                p0: [f32; 2],
                p1: [f32; 2],
                n: [f32; 3]| {
        let (l, h) = (g.lo[k], g.hi[k]);
        if h <= l {
            return;
        }
        let (ol, oh) = match other {
            Some(o) if full(o) => (g.lo[o], g.hi[o]),
            _ => (h, h),
        };
        let col = shade(look::STOCK, n);
        let mut band = |a: f32, b: f32| {
            if b > a {
                quad(
                    [
                        [p0[0], p0[1], a],
                        [p1[0], p1[1], a],
                        [p1[0], p1[1], b],
                        [p0[0], p0[1], b],
                    ],
                    col,
                    out,
                );
            }
        };
        if other.is_some_and(full) {
            band(oh.max(l), h);
            band(l, ol.min(h));
        } else {
            band(l, h);
        }
    };
    for j in 0..ny {
        for i in 0..nx {
            let k = j * nx + i;
            // +x side: this column's part not in the neighbour's, facing +x; and the neighbour's
            // part not in this one, facing −x
            let same = |o: usize| g.lo[o] == g.lo[k] && g.hi[o] == g.hi[k];
            let right = (i + 1 < nx).then_some(k + 1).filter(|&r| !same(r));
            let edge_x = i + 1 == nx;
            if right.is_some() || edge_x {
                wall(
                    out,
                    k,
                    right,
                    [x(i + 1), y(j)],
                    [x(i + 1), y(j + 1)],
                    [1.0, 0.0, 0.0],
                );
            }
            if let Some(r) = right {
                wall(
                    out,
                    r,
                    Some(k),
                    [x(i + 1), y(j + 1)],
                    [x(i + 1), y(j)],
                    [-1.0, 0.0, 0.0],
                );
            }
            if i == 0 {
                wall(out, k, None, [x(0), y(j + 1)], [x(0), y(j)], [-1.0, 0.0, 0.0]);
            }
            let up = (j + 1 < ny).then_some(k + nx).filter(|&u| !same(u));
            if up.is_some() || j + 1 == ny {
                wall(
                    out,
                    k,
                    up,
                    [x(i + 1), y(j + 1)],
                    [x(i), y(j + 1)],
                    [0.0, 1.0, 0.0],
                );
            }
            if let Some(u) = up {
                wall(
                    out,
                    u,
                    Some(k),
                    [x(i), y(j + 1)],
                    [x(i + 1), y(j + 1)],
                    [0.0, -1.0, 0.0],
                );
            }
            if j == 0 {
                wall(out, k, None, [x(i), y(0)], [x(i + 1), y(0)], [0.0, -1.0, 0.0]);
            }
        }
    }
}

/// The tool at tip position p: each body of its profile revolved (cutter, shank, holder).
pub fn tool(shape: &Shape, p: [f64; 3], fail: bool, out: &mut Vec<WorldVert>) {
    let seg = 32;
    for (bi, b) in shape.bodies.iter().enumerate() {
        let base = match b.part {
            Part::Cutter => look::CUTTER,
            Part::NonCutting if bi + 1 == shape.bodies.len() && shape.bodies.len() > 2 => look::HOLDER,
            Part::NonCutting => look::SHANK,
        };
        let base = if fail { look::FAIL } else { base };
        // the profile as a polyline in (ρ, z)
        let mut prof: Vec<[f64; 2]> = vec![];
        for s in &b.chain {
            match *s {
                PSeg::Line { a, b } => {
                    if prof.last() != Some(&a) {
                        prof.push(a);
                    }
                    prof.push(b);
                }
                PSeg::Arc { a, b, c, r, ccw } => {
                    let a0 = dmath::atan2(a[1] - c[1], a[0] - c[0]);
                    let mut a1 = dmath::atan2(b[1] - c[1], b[0] - c[0]);
                    let tau = std::f64::consts::TAU;
                    if ccw {
                        while a1 < a0 {
                            a1 += tau;
                        }
                    } else {
                        while a1 > a0 {
                            a1 -= tau;
                        }
                    }
                    if prof.last() != Some(&a) {
                        prof.push(a);
                    }
                    for k in 1..=8 {
                        let t = a0 + (a1 - a0) * k as f64 / 8.0;
                        prof.push(if k == 8 {
                            b
                        } else {
                            [c[0] + r * dmath::cos(t), c[1] + r * dmath::sin(t)]
                        });
                    }
                }
            }
        }
        for w in prof.windows(2) {
            let (q0, q1) = (w[0], w[1]);
            if q0 == q1 {
                continue;
            }
            // the band's outward normal in (ρ, z)
            let (dr, dz) = (q1[0] - q0[0], q1[1] - q0[1]);
            let nl = (dr * dr + dz * dz).sqrt();
            let (nr, nz) = (dz / nl, -dr / nl);
            for k in 0..seg {
                let (t0, t1) = (
                    std::f64::consts::TAU * k as f64 / seg as f64,
                    std::f64::consts::TAU * (k + 1) as f64 / seg as f64,
                );
                let at = |q: [f64; 2], t: f64| -> [f32; 3] {
                    [
                        (p[0] + q[0] * dmath::cos(t)) as f32,
                        (p[1] + q[0] * dmath::sin(t)) as f32,
                        (p[2] + q[1]) as f32,
                    ]
                };
                let tm = 0.5 * (t0 + t1);
                let n = [
                    (nr * dmath::cos(tm)) as f32,
                    (nr * dmath::sin(tm)) as f32,
                    nz as f32,
                ];
                quad(
                    [at(q0, t0), at(q0, t1), at(q1, t1), at(q1, t0)],
                    shade(base, n),
                    out,
                );
            }
        }
    }
}

/// A box, shaded.
pub fn solid_box(lo: [f64; 3], hi: [f64; 3], c: [f32; 3], out: &mut Vec<WorldVert>) {
    let (a, b) = (f3(lo), f3(hi));
    let v = |x: bool, y: bool, z: bool| {
        [
            if x { b[0] } else { a[0] },
            if y { b[1] } else { a[1] },
            if z { b[2] } else { a[2] },
        ]
    };
    let faces: [([[bool; 3]; 4], [f32; 3]); 6] = [
        (
            [
                [false, false, true],
                [true, false, true],
                [true, true, true],
                [false, true, true],
            ],
            [0.0, 0.0, 1.0],
        ),
        (
            [
                [false, false, false],
                [false, true, false],
                [true, true, false],
                [true, false, false],
            ],
            [0.0, 0.0, -1.0],
        ),
        (
            [
                [true, false, false],
                [true, true, false],
                [true, true, true],
                [true, false, true],
            ],
            [1.0, 0.0, 0.0],
        ),
        (
            [
                [false, false, false],
                [false, false, true],
                [false, true, true],
                [false, true, false],
            ],
            [-1.0, 0.0, 0.0],
        ),
        (
            [
                [false, true, false],
                [false, true, true],
                [true, true, true],
                [true, true, false],
            ],
            [0.0, 1.0, 0.0],
        ),
        (
            [
                [false, false, false],
                [true, false, false],
                [true, false, true],
                [false, false, true],
            ],
            [0.0, -1.0, 0.0],
        ),
    ];
    for (q, n) in faces {
        quad(q.map(|f| v(f[0], f[1], f[2])), shade(c, n), out);
    }
}

/// A box's twelve edges as ribbons (a leftover region, the stock outline).
pub fn box_edges(eye: &Eye, lo: [f64; 3], hi: [f64; 3], w: f32, c: [f32; 4], out: &mut Vec<WorldVert>) {
    let (a, b) = (f3(lo), f3(hi));
    let v = |x: bool, y: bool, z: bool| {
        [
            if x { b[0] } else { a[0] },
            if y { b[1] } else { a[1] },
            if z { b[2] } else { a[2] },
        ]
    };
    for (p, q) in [
        ((0, 0, 0), (1, 0, 0)),
        ((0, 1, 0), (1, 1, 0)),
        ((0, 0, 1), (1, 0, 1)),
        ((0, 1, 1), (1, 1, 1)),
        ((0, 0, 0), (0, 1, 0)),
        ((1, 0, 0), (1, 1, 0)),
        ((0, 0, 1), (0, 1, 1)),
        ((1, 0, 1), (1, 1, 1)),
        ((0, 0, 0), (0, 0, 1)),
        ((1, 0, 0), (1, 0, 1)),
        ((0, 1, 0), (0, 1, 1)),
        ((1, 1, 0), (1, 1, 1)),
    ] {
        let f = |t: (i32, i32, i32)| v(t.0 == 1, t.1 == 1, t.2 == 1);
        ribbon(eye, f(p), f(q), w, c, out);
    }
}

/// A marker: a small three-axis cross, `size` pixels.
pub fn marker(eye: &Eye, at: [f64; 3], size: f32, c: [f32; 4], out: &mut Vec<WorldVert>) {
    let p = f3(at);
    let h = 0.5 * size * eye.px(p);
    for d in [[h, 0.0, 0.0], [0.0, h, 0.0], [0.0, 0.0, h]] {
        let a = [p[0] - d[0], p[1] - d[1], p[2] - d[2]];
        let b = [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
        ribbon(eye, a, b, size * 0.25, c, out);
    }
}

/// The part's triangles, flat shaded by their normals; `alpha` < 1 for a ghost.
pub fn part(tris: &[[[f64; 3]; 3]], alpha: f32, out: &mut Vec<WorldVert>) {
    for t in tris {
        let [a, b, c] = t.map(f3);
        let n = cross(sub(b, a), sub(c, a));
        let l = len(n);
        if l < 1e-12 {
            continue;
        }
        let mut col = shade(look::PART, [n[0] / l, n[1] / l, n[2] / l]);
        col[3] = alpha;
        for p in [a, b, c] {
            out.push(WorldVert { at: p, colour: col });
        }
    }
}

/// An sRGB colour as the renderer's linear one, with alpha.
pub fn lin(hex: u32, a: f32) -> [f32; 4] {
    let c = rgb(hex);
    [c[0], c[1], c[2], a]
}

impl Values {
    /// Values per (op, path, move of the posted path) laid onto the timeline's pieces: every
    /// piece of that move takes it.
    pub fn of_moves(
        tl: &Timeline,
        name: &str,
        unit: &str,
        bands: Vec<(f32, [f32; 3], String)>,
        value: &dyn Fn(usize, usize, usize) -> Option<(f32, f32)>,
    ) -> Values {
        Values {
            name: name.into(),
            unit: unit.into(),
            per_piece: tl
                .segs
                .iter()
                .map(|g| value(g.op as usize, g.path as usize, g.mv as usize))
                .collect(),
            bands,
        }
    }

    /// Chip thickness against the op's target feed per tooth: rubbing below half, overload
    /// above 1.25 (the quality measurement's own thresholds).
    pub fn chip_bands() -> Vec<(f32, [f32; 3], String)> {
        vec![
            (0.5, look::LOW, "rubbing (< 0.5 fz)".into()),
            (1.0, look::GOOD, "0.5 – 1 fz".into()),
            (1.25, look::HIGH, "1 – 1.25 fz".into()),
            (f32::INFINITY, look::OVER, "overload (> 1.25 fz)".into()),
        ]
    }

    /// Engagement angle (degrees, scale 1): light, half, heavy, slotting.
    pub fn engagement_bands() -> Vec<(f32, [f32; 3], String)> {
        vec![
            (30.0, look::LOW, "< 30°".into()),
            (90.0, look::GOOD, "30 – 90°".into()),
            (150.0, look::HIGH, "90 – 150°".into()),
            (f32::INFINITY, look::OVER, "> 150° (slotting)".into()),
        ]
    }
}

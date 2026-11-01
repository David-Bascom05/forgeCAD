//! The view cube: a chamfered cube drawn from the live camera, whose 6 faces, 12 edges and 8 corners
//! are the 26 regions it can go to, with a ring flat on the screen that rolls the view, arrows that
//! step it by a quarter turn, home and roll. Geometry and hit testing read the same projection, so
//! what you see is what a click finds. All in pixels; no GPU here.

use crate::camera::{Camera, dot};

/// The square the widget fills, before the window's scale.
pub const WIDGET_PX: f32 = 160.0;
/// How much of a face's half each edge's chamfer takes.
const CHAMFER: f32 = 0.24;
/// A mild perspective, so it reads as a solid: the eye's distance in cube half sides.
const EYE: f32 = 9.0;
/// The ring round the cube, flat on the screen, and where the arrows are: in shares of the half side.
const RING_IN: f32 = 0.72;
const RING_OUT: f32 = 0.77;
const ARROW_AT: f32 = 0.84;
/// Pixels of widget half per cube half side: its corners fit inside the ring.
const SCALE: f32 = 0.42;
/// Looking this straight at a face shows the roll arrows and the arrows to its neighbours.
const SQUARE_ON: f32 = 0.9999;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    /// Each axis -1, 0 or 1: a face has one non-zero, an edge two, a corner three.
    Region([i8; 3]),
    Ring,
    Home,
    /// Clockwise on screen, or the other way.
    Roll(bool),
    /// A quarter turn to see what's that way on screen: x and y, y up.
    Step(i8, i8),
}

/// Where the widget is on the window, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub at: (f32, f32),
    pub side: f32,
    /// Device pixels per CSS pixel.
    pub scale: f32,
}

/// A triangle corner in window pixels, with linear colour and alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vert {
    pub px: [f32; 2],
    pub colour: [f32; 4],
}

/// sRGB hex to linear, for a target that encodes on write.
pub const fn rgb(hex: u32) -> [f32; 3] {
    [channel(hex >> 16), channel(hex >> 8), channel(hex)]
}

const fn channel(v: u32) -> f32 {
    LINEAR[(v & 255) as usize]
}

const LINEAR: [f32; 256] = linear_table();

const fn linear_table() -> [f32; 256] {
    let mut t = [0.0; 256];
    let mut i = 0;
    while i < 256 {
        let c = i as f64 / 255.0;
        t[i] = if c <= 0.04045 {
            (c / 12.92) as f32
        } else {
            // (c + 0.055) / 1.055 to the power 2.4, as x^2 * x^0.4 by repeated roots
            let x = (c + 0.055) / 1.055;
            let r = root5(x);
            (x * x * r * r) as f32
        };
        i += 1;
    }
    t
}

const fn root5(x: f64) -> f64 {
    let mut y = 1.0;
    let mut k = 0;
    while k < 24 {
        let y4 = y * y * y * y;
        y -= (y * y4 - x) / (5.0 * y4);
        k += 1;
    }
    y
}

/// Which way the part's axes are named: a rotation of the cube's labels. Named space has RIGHT at +x,
/// FRONT at -y and TOP at +z; this takes a direction in the world to its name. Always one of the 24
/// proper rotations, so opposite faces stay opposite and the set stays right-handed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orient {
    m: [[i8; 3]; 3],
}

impl Default for Orient {
    fn default() -> Orient {
        Orient {
            m: [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
        }
    }
}

fn cross_i(a: [i8; 3], b: [i8; 3]) -> [i8; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The names of the six faces, as the menu has them, with where each is in named space.
pub const FACE_NAMES: [(&str, [i8; 3]); 6] = [
    ("Front", [0, -1, 0]),
    ("Back", [0, 1, 0]),
    ("Top", [0, 0, 1]),
    ("Bottom", [0, 0, -1]),
    ("Left", [-1, 0, 0]),
    ("Right", [1, 0, 0]),
];

impl Orient {
    /// A world direction, as it's named.
    pub fn named(&self, w: [i8; 3]) -> [i8; 3] {
        [0, 1, 2].map(|r| (0..3).map(|k| self.m[r][k] * w[k]).sum())
    }

    /// A named direction, in the world.
    pub fn world(&self, n: [i8; 3]) -> [i8; 3] {
        [0, 1, 2].map(|k| (0..3).map(|r| self.m[r][k] * n[r]).sum())
    }

    fn world_f(&self, n: [f32; 3]) -> [f32; 3] {
        [0, 1, 2].map(|k| (0..3).map(|r| self.m[r][k] as f32 * n[r]).sum())
    }

    /// This world face gets this name, and the world axis `up` (square to the face) stays up: it's
    /// TOP, or on a TOP or BOTTOM face what reads as up on it.
    pub fn with_face(face: [i8; 3], name: [i8; 3], up: [i8; 3]) -> Orient {
        let target = match name[2] {
            0 => [0, 0, 1],
            s => [0, s, 0],
        };
        let (c, third) = (cross_i(face, up), cross_i(name, target));
        let mut m = [[0i8; 3]; 3];
        for (r, row) in m.iter_mut().enumerate() {
            for (k, v) in row.iter_mut().enumerate() {
                *v = name[r] * face[k] + target[r] * up[k] + third[r] * c[k];
            }
        }
        Orient { m }
    }

    /// As the view file holds it.
    pub fn words(&self) -> String {
        let w: Vec<String> = self.m.iter().flatten().map(i8::to_string).collect();
        w.join(" ")
    }

    /// Back from that, if it's a proper rotation of the axes.
    pub fn from_words(s: &str) -> Option<Orient> {
        let v: Vec<i8> = s
            .split_whitespace()
            .map(|w| w.parse().ok())
            .collect::<Option<_>>()?;
        let [a, b, c, d, e, f, g, h, i] = v[..] else {
            return None;
        };
        let o = Orient {
            m: [[a, b, c], [d, e, f], [g, h, i]],
        };
        // a unit vector in each row and column, and right-handed
        let axis = |x: [i8; 3]| x.iter().map(|q| q.abs() as i32).sum::<i32>() == 1;
        let ok = o.m.iter().all(|r| axis(*r))
            && (0..3).all(|k| axis([o.m[0][k], o.m[1][k], o.m[2][k]]))
            && cross_i(o.m[0], o.m[1]) == o.m[2];
        ok.then_some(o)
    }
}

/// The widget's colours, from the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub face: [f32; 3],
    pub hover: [f32; 3],
    pub hover_ink: [f32; 3],
    pub ink: [f32; 3],
    pub line: [f32; 3],
    pub focus: [f32; 3],
}

pub const DARK: Palette = Palette {
    face: rgb(0x2A313B),
    hover: rgb(0x16304F),
    hover_ink: rgb(0x9CC3EA),
    ink: rgb(0xE5E9EE),
    line: rgb(0xA1A9B3),
    focus: rgb(0xE0A21B),
};

pub const LIGHT: Palette = Palette {
    face: rgb(0xF7F8FA),
    hover: rgb(0xE3ECF5),
    hover_ink: rgb(0x1D4470),
    ink: rgb(0x1A1F1C),
    line: rgb(0x525A55),
    focus: rgb(0xE0A21B),
};

struct Poly {
    pts: Vec<[f32; 3]>,
    normal: [f32; 3],
    region: [i8; 3],
}

/// The 26 flat pieces of the chamfered cube.
fn polys() -> Vec<Poly> {
    let (a, b) = (1.0, 1.0 - CHAMFER);
    let mut out = Vec::with_capacity(26);
    let unit = |r: [i8; 3]| {
        let n = r.map(|v| v as f32);
        let l = dot(n, n).sqrt();
        n.map(|v| v / l)
    };
    for axis in 0..3 {
        let (i, j) = ((axis + 1) % 3, (axis + 2) % 3);
        for s in [-1.0f32, 1.0] {
            // a face
            let mut r = [0i8; 3];
            r[axis] = s as i8;
            let at = |p: f32, q: f32| {
                let mut v = [0.0; 3];
                v[axis] = s * a;
                v[i] = p;
                v[j] = q;
                v
            };
            out.push(Poly {
                pts: vec![at(-b, -b), at(b, -b), at(b, b), at(-b, b)],
                normal: unit(r),
                region: r,
            });
            // the edges along the axis
            for t in [-1.0f32, 1.0] {
                let mut r = [0i8; 3];
                r[i] = s as i8;
                r[j] = t as i8;
                let at = |along: f32, on_i: bool| {
                    let mut v = [0.0; 3];
                    v[axis] = along;
                    v[i] = s * if on_i { a } else { b };
                    v[j] = t * if on_i { b } else { a };
                    v
                };
                out.push(Poly {
                    pts: vec![at(-b, true), at(b, true), at(b, false), at(-b, false)],
                    normal: unit(r),
                    region: r,
                });
            }
        }
    }
    for x in [-1.0f32, 1.0] {
        for y in [-1.0f32, 1.0] {
            for z in [-1.0f32, 1.0] {
                let r = [x as i8, y as i8, z as i8];
                out.push(Poly {
                    pts: vec![
                        [x * a, y * b, z * b],
                        [x * b, y * a, z * b],
                        [x * b, y * b, z * a],
                    ],
                    normal: unit(r),
                    region: r,
                });
            }
        }
    }
    out
}

/// The camera, as the widget sees it.
#[derive(Clone, Copy)]
struct Eye {
    f: [f32; 3],
    r: [f32; 3],
    u: [f32; 3],
    centre: (f32, f32),
    /// Pixels per cube half side at the middle.
    k: f32,
}

impl Eye {
    fn new(camera: &Camera, place: &Place) -> Eye {
        let (f, r, u) = camera.axes();
        let half = place.side * 0.5;
        Eye {
            f,
            r,
            u,
            centre: (place.at.0 + half, place.at.1 + half),
            k: half * SCALE,
        }
    }

    fn px(&self, p: [f32; 3]) -> [f32; 2] {
        let s = EYE / (EYE + dot(p, self.f));
        [
            self.centre.0 + dot(p, self.r) * self.k * s,
            self.centre.1 - dot(p, self.u) * self.k * s,
        ]
    }

    /// Faces towards the eye.
    fn sees(&self, p: [f32; 3], n: [f32; 3]) -> bool {
        let to_eye = [0, 1, 2].map(|k| -self.f[k] * EYE - p[k]);
        dot(to_eye, n) > 0.0
    }

    fn square_on(&self) -> bool {
        self.f.iter().any(|v| v.abs() > SQUARE_ON)
    }
}

fn inside(pts: &[[f32; 2]], at: (f32, f32)) -> bool {
    let mut sign = 0.0f32;
    for k in 0..pts.len() {
        let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
        let c = (b[0] - a[0]) * (at.1 - a[1]) - (b[1] - a[1]) * (at.0 - a[0]);
        if c * sign < 0.0 {
            return false;
        }
        if c != 0.0 {
            sign = c;
        }
    }
    true
}

fn round(v: [f32; 3]) -> [i8; 3] {
    v.map(|c| c.round() as i8)
}

type Arrow = ([[f32; 2]; 3], (i8, i8));

/// The four arrows round the ring: their triangle, and which way each steps the view.
fn arrows(eye: &Eye, place: &Place) -> [Arrow; 4] {
    let half = place.side * 0.5;
    let (reach, wide, tall) = (half * ARROW_AT, 7.0 * place.scale, 8.0 * place.scale);
    [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)].map(|(x, y)| {
        let (fx, fy) = (x as f32, y as f32);
        let c = |out: f32, side: f32| {
            [
                eye.centre.0 + fx * out + fy * side,
                eye.centre.1 - fy * out + fx * side,
            ]
        };
        ([c(reach + tall, 0.0), c(reach, wide), c(reach, -wide)], (x, y))
    })
}

fn home_at(place: &Place) -> (f32, f32) {
    let m = 12.0 * place.scale;
    (place.at.0 + m, place.at.1 + m)
}

fn roll_at(place: &Place, clockwise: bool) -> (f32, f32) {
    let m = 12.0 * place.scale;
    let x = place.at.0 + place.side - m;
    let y = place.at.1 + m;
    if clockwise {
        (x, y)
    } else {
        (x - 22.0 * place.scale, y)
    }
}

fn near(a: (f32, f32), b: (f32, f32), r: f32) -> bool {
    (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) < r * r
}

/// What's under a pixel.
pub fn hit(camera: &Camera, place: &Place, px: (f32, f32)) -> Option<Hit> {
    let eye = Eye::new(camera, place);
    let (x, y) = (px.0 - place.at.0, px.1 - place.at.1);
    if x < 0.0 || y < 0.0 || x >= place.side || y >= place.side {
        return None;
    }
    let reach = 10.0 * place.scale;
    if near(px, home_at(place), reach) {
        return Some(Hit::Home);
    }
    if eye.square_on() {
        for clockwise in [true, false] {
            if near(px, roll_at(place, clockwise), reach) {
                return Some(Hit::Roll(clockwise));
            }
        }
    }
    for (tri, (x, y)) in arrows(&eye, place) {
        // a little more than the triangle, for the hand
        let mid = [
            (tri[0][0] + tri[1][0] + tri[2][0]) / 3.0,
            (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0,
        ];
        if near(px, (mid[0], mid[1]), reach) || inside(&tri, px) {
            return Some(Hit::Step(x, y));
        }
    }
    for p in polys() {
        if eye.sees(p.pts[0], p.normal) {
            let flat: Vec<[f32; 2]> = p.pts.iter().map(|&q| eye.px(q)).collect();
            if inside(&flat, px) {
                return Some(Hit::Region(p.region));
            }
        }
    }
    let (dx, dy) = (px.0 - eye.centre.0, px.1 - eye.centre.1);
    let r = (dx * dx + dy * dy).sqrt() / (place.side * 0.5);
    // easier to catch than it is to see
    (r > RING_IN - 0.03 && r < RING_OUT + 0.03).then_some(Hit::Ring)
}

struct Pen<'a> {
    out: &'a mut Vec<Vert>,
}

impl Pen<'_> {
    fn tri(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], colour: [f32; 4]) {
        for px in [a, b, c] {
            self.out.push(Vert { px, colour });
        }
    }

    fn fill(&mut self, pts: &[[f32; 2]], colour: [f32; 4]) {
        for k in 1..pts.len() - 1 {
            self.tri(pts[0], pts[k], pts[k + 1], colour);
        }
    }

    fn segment(&mut self, a: [f32; 2], b: [f32; 2], w: f32, colour: [f32; 4]) {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (ux, uy) = (dx / l * w * 0.5, dy / l * w * 0.5);
        // square ends half the width past each point, so polylines join solid
        let c = |p: [f32; 2], e: f32, s: f32| [p[0] + ux * e - uy * s, p[1] + uy * e + ux * s];
        self.fill(
            &[c(a, -1.0, 1.0), c(a, -1.0, -1.0), c(b, 1.0, -1.0), c(b, 1.0, 1.0)],
            colour,
        );
    }

    fn polyline(&mut self, pts: &[[f32; 2]], w: f32, colour: [f32; 4]) {
        for k in 1..pts.len() {
            self.segment(pts[k - 1], pts[k], w, colour);
        }
    }
}

/// A face's word, its normal, and which way the word runs and stands.
type FaceWord = (&'static str, [f32; 3], [f32; 3], [f32; 3]);

const LABELS: [FaceWord; 6] = [
    ("TOP", [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ("BOTTOM", [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
    ("FRONT", [0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ("BACK", [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ("RIGHT", [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    ("LEFT", [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
];

/// A face's word: painted on the face, in real type, by whoever draws text, through `on_face`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label {
    pub text: &'static str,
    /// The face's middle and normal, and which way the word runs and stands on it.
    pub n: [f32; 3],
    pub x: [f32; 3],
    pub y: [f32; 3],
    pub alpha: f32,
    pub hovered: bool,
}

/// A word's em on its face, in cube half sides: the same on every face, so BOTTOM just fits.
pub const LABEL_EM: f32 = 0.3;
/// How square-on a face must be for its word to start showing, and to show in full.
const LABEL_FADE: (f32, f32) = (0.22, 0.42);

/// The words on the cube, each a part of its face.
pub fn labels(camera: &Camera, place: &Place, hover: Option<Hit>, orient: &Orient) -> Vec<Label> {
    let eye = Eye::new(camera, place);
    LABELS
        .into_iter()
        .map(|(text, n, x, y)| (text, orient.world_f(n), orient.world_f(x), orient.world_f(y)))
        .filter(|(_, n, _, _)| eye.sees(*n, *n))
        .map(|(text, n, x, y)| Label {
            text,
            n,
            x,
            y,
            alpha: ((-dot(n, eye.f) - LABEL_FADE.0) / (LABEL_FADE.1 - LABEL_FADE.0)).clamp(0.0, 1.0),
            hovered: hover == Some(Hit::Region(round(n))),
        })
        .collect()
}

/// A place on a word's face, `a` along it and `b` up it in cube half sides from its middle, in the
/// widget's own projection: pixels, as the face itself is drawn.
pub fn on_face(camera: &Camera, place: &Place, l: &Label, a: f32, b: f32) -> [f32; 2] {
    Eye::new(camera, place).px([0, 1, 2].map(|k| l.n[k] + l.x[k] * a + l.y[k] * b))
}

/// Pixels of cube half side at the middle: what a word's type is sized from.
pub fn half_side_px(place: &Place) -> f32 {
    place.side * 0.5 * SCALE
}

/// The widget's triangles, for this camera, into `out`. A few thousand at most.
pub fn draw(camera: &Camera, place: &Place, hover: Option<Hit>, pal: &Palette, out: &mut Vec<Vert>) {
    let eye = Eye::new(camera, place);
    let mut pen = Pen { out };
    let line = place.scale;
    let near_it = hover.is_some();

    // the ring: flat on the screen, faint until the pointer's on the widget
    let ring_on = hover == Some(Hit::Ring);
    let ring_colour = if ring_on { pal.hover_ink } else { pal.line };
    let alpha = if ring_on {
        0.8
    } else if near_it {
        0.35
    } else {
        0.18
    };
    let half = place.side * 0.5;
    let steps = 96;
    let at = |k: usize, r: f32| {
        let a = k as f32 / steps as f32 * core::f32::consts::TAU;
        [
            eye.centre.0 + a.cos() * r * half,
            eye.centre.1 + a.sin() * r * half,
        ]
    };
    for k in 0..steps {
        let quad = [
            at(k, RING_IN),
            at(k + 1, RING_IN),
            at(k + 1, RING_OUT),
            at(k, RING_OUT),
        ];
        pen.fill(&quad, [ring_colour[0], ring_colour[1], ring_colour[2], alpha]);
    }

    for p in polys() {
        if !eye.sees(p.pts[0], p.normal) {
            continue;
        }
        let flat: Vec<[f32; 2]> = p.pts.iter().map(|&q| eye.px(q)).collect();
        let on = hover == Some(Hit::Region(p.region));
        let toward = -dot(p.normal, eye.f);
        // lit a little from above and the eye; flat colour, no sheen
        let shade = 0.72 + 0.2 * toward + 0.12 * p.normal[2];
        let base = if on { pal.hover } else { pal.face };
        let colour = [base[0] * shade, base[1] * shade, base[2] * shade, 1.0];
        pen.fill(&flat, colour);
        let mut outline = flat.clone();
        outline.push(flat[0]);
        let edge = if on { pal.hover_ink } else { pal.line };
        pen.polyline(&outline, line, [edge[0], edge[1], edge[2], 0.7]);
    }

    let stroke = |on: bool| {
        let c = if on { pal.hover_ink } else { pal.line };
        [c[0], c[1], c[2], 1.0]
    };

    // home: a small house
    let h = home_at(place);
    let s = 5.5 * place.scale;
    let house = [[h.0 - s, h.1 - 0.1 * s], [h.0, h.1 - s], [h.0 + s, h.1 - 0.1 * s]];
    let walls = [
        [h.0 - 0.7 * s, h.1 - 0.3 * s],
        [h.0 - 0.7 * s, h.1 + 0.85 * s],
        [h.0 + 0.7 * s, h.1 + 0.85 * s],
        [h.0 + 0.7 * s, h.1 - 0.3 * s],
    ];
    let c = stroke(hover == Some(Hit::Home));
    pen.polyline(&house, 1.4 * line, c);
    pen.polyline(&walls, 1.4 * line, c);

    if eye.square_on() {
        for clockwise in [true, false] {
            let r = roll_at(place, clockwise);
            let c = stroke(hover == Some(Hit::Roll(clockwise)));
            let radius = 5.5 * place.scale;
            let dir = if clockwise { 1.0 } else { -1.0 };
            let at = |t: f32| {
                let a = -core::f32::consts::FRAC_PI_2 + dir * t;
                [r.0 + a.cos() * radius, r.1 + a.sin() * radius]
            };
            let arc: Vec<[f32; 2]> = (0..=12).map(|k| at(k as f32 / 12.0 * 4.4)).collect();
            pen.polyline(&arc, 1.4 * line, c);
            let tip = at(4.4 + 0.5);
            let back = at(4.4);
            let (nx, ny) = (back[0] - r.0, back[1] - r.1);
            let w = 0.55;
            pen.fill(
                &[
                    tip,
                    [back[0] + nx * w, back[1] + ny * w],
                    [back[0] - nx * w, back[1] - ny * w],
                ],
                c,
            );
        }
    }
    // the steps show only when the pointer's on the widget, and stay quiet till one's meant
    if near_it {
        for (tri, (x, y)) in arrows(&eye, place) {
            let on = hover == Some(Hit::Step(x, y));
            let c = if on { pal.focus } else { pal.line };
            pen.fill(&tri, [c[0], c[1], c[2], if on { 0.9 } else { 0.4 }]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place() -> Place {
        Place {
            at: (0.0, 0.0),
            side: 280.0,
            scale: 2.0,
        }
    }

    fn cam(yaw: f32, pitch: f32) -> Camera {
        let mut c = Camera::fit([0.0; 3], 10.0);
        c.yaw = yaw;
        c.pitch = pitch;
        c
    }

    #[test]
    fn there_are_26_regions_and_each_one_differs() {
        let mut r: Vec<[i8; 3]> = polys().iter().map(|p| p.region).collect();
        r.sort();
        r.dedup();
        assert_eq!(r.len(), 26);
        assert!(r.iter().all(|v| *v != [0; 3]));
    }

    #[test]
    fn every_region_is_clickable_from_the_view_that_faces_it() {
        for p in polys() {
            let v = region_axis(p.region);
            let c = crate::nav::region_view(cam(0.3, 0.2), p.region);
            let e = Eye::new(&c, &place());
            let f = e.f;
            assert!(dot(f, v) < -0.99, "{:?} looks at it", p.region);
            let mid = e.px(p.normal.map(|n| {
                n * if p.pts.len() == 4 && p.region.iter().filter(|x| **x != 0).count() == 1 {
                    1.0
                } else {
                    1.02
                }
            }));
            let got = hit(&c, &place(), (mid[0], mid[1]));
            assert_eq!(got, Some(Hit::Region(p.region)));
        }
    }

    fn region_axis(r: [i8; 3]) -> [f32; 3] {
        let n = r.map(|v| v as f32);
        let l = dot(n, n).sqrt();
        n.map(|v| v / l)
    }

    #[test]
    fn a_pixel_finds_the_piece_drawn_there() {
        // walk a grid over several views: a click's region is a piece that faces the eye
        for (y, p) in [(0.6, 0.4), (-2.0, 0.9), (2.4, -0.5)] {
            let c = cam(y, p);
            let e = Eye::new(&c, &place());
            for gy in 0..28 {
                for gx in 0..28 {
                    let px = (gx as f32 * 10.0 + 5.0, gy as f32 * 10.0 + 5.0);
                    if let Some(Hit::Region(r)) = hit(&c, &place(), px) {
                        let q = polys().into_iter().find(|q| q.region == r).unwrap();
                        assert!(e.sees(q.pts[0], q.normal), "{r:?} at {px:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn square_on_shows_roll_and_only_then() {
        let front = crate::nav::region_view(cam(0.3, 0.2), [0, -1, 0]);
        let at = roll_at(&place(), true);
        assert_eq!(hit(&front, &place(), at), Some(Hit::Roll(true)));
        assert_ne!(hit(&cam(0.3, 0.2), &place(), at), Some(Hit::Roll(true)));
    }

    #[test]
    fn home_the_flat_ring_and_the_step_arrows_are_found_from_any_view() {
        for (y, p) in [
            (0.5, 0.5),
            (0.0, 0.0),
            (2.0, -1.2),
            (1.0, core::f32::consts::FRAC_PI_2),
        ] {
            let c = cam(y, p);
            let pl = place();
            assert_eq!(hit(&c, &pl, home_at(&pl)), Some(Hit::Home));
            let (cx, cy) = (pl.at.0 + pl.side / 2.0, pl.at.1 + pl.side / 2.0);
            let half = pl.side / 2.0;
            let r = (RING_IN + RING_OUT) / 2.0 * half;
            // the ring is a circle on the screen, whatever the view: any angle clear of the arrows
            for a in [0.6f32, 2.2, 3.9, 5.5] {
                let px = (cx + a.cos() * r, cy + a.sin() * r);
                assert_eq!(hit(&c, &pl, px), Some(Hit::Ring), "{y} {p} {a}");
            }
            let e = Eye::new(&c, &pl);
            for (tri, (x, yy)) in arrows(&e, &pl) {
                assert_eq!(hit(&c, &pl, (tri[0][0], tri[0][1])), Some(Hit::Step(x, yy)));
            }
        }
    }

    #[test]
    fn the_cube_fits_inside_the_ring_from_every_view() {
        let pl = place();
        let half = pl.side / 2.0;
        for yaw in (0..24).map(|k| k as f32 * 0.26) {
            for pitch in (-6..=6).map(|k| k as f32 * 0.26) {
                let e = Eye::new(&cam(yaw, pitch), &pl);
                for q in polys().iter().flat_map(|p| p.pts.clone()) {
                    let w = e.px(q);
                    let r = (w[0] - e.centre.0).hypot(w[1] - e.centre.1) / half;
                    assert!(r < RING_IN, "{r}");
                }
            }
        }
    }

    #[test]
    fn a_word_is_on_its_face_and_fades_as_the_face_turns_edge_on() {
        let p = place();
        // straight on: the word lies square, in full
        let front = crate::nav::region_view(cam(0.3, 0.2), [0, -1, 0]);
        let ls = labels(&front, &p, None, &Orient::default());
        let l = ls.iter().find(|l| l.text == "FRONT").unwrap();
        assert_eq!(l.alpha, 1.0);
        let (u, v) = (
            on_face(&front, &p, l, -0.5, 0.0),
            on_face(&front, &p, l, 0.5, 0.0),
        );
        assert!(
            (u[1] - v[1]).abs() < 1e-3 && v[0] > u[0],
            "left to right, level: {u:?} {v:?}"
        );
        // from a corner, all three read, foreshortened alike
        let corner = crate::nav::region_view(cam(0.3, 0.2), [1, -1, 1]);
        let ls = labels(&corner, &p, None, &Orient::default());
        assert_eq!(ls.len(), 3);
        assert!(ls.iter().all(|l| l.alpha == 1.0), "{ls:?}");
        // turned near edge-on, it's gone rather than a smear
        let grazing = cam(-core::f32::consts::FRAC_PI_2 + 1.45, 0.0);
        let f = labels(&grazing, &p, None, &Orient::default())
            .into_iter()
            .find(|l| l.text == "FRONT");
        assert!(f.is_none_or(|l| l.alpha == 0.0), "{f:?}");
        // and from every view some word reads in full
        for yaw in (0..40).map(|k| k as f32 * 0.157) {
            for pitch in (-8..=8).map(|k| k as f32 * 0.196) {
                let c = cam(yaw, pitch);
                assert!(
                    labels(&c, &p, None, &Orient::default())
                        .iter()
                        .any(|l| l.alpha >= 1.0),
                    "{yaw} {pitch}"
                );
            }
        }
    }

    /// Every face given every name, with every up that's square to it.
    fn all_orients() -> Vec<(Orient, [i8; 3], [i8; 3], [i8; 3])> {
        let axes: Vec<[i8; 3]> = FACE_NAMES.iter().map(|f| f.1).collect();
        let mut out = Vec::new();
        for &face in &axes {
            for &(_, name) in &FACE_NAMES {
                for &up in axes
                    .iter()
                    .filter(|u| (0..3).map(|k| u[k] * face[k]).sum::<i8>() == 0)
                {
                    out.push((Orient::with_face(face, name, up), face, name, up));
                }
            }
        }
        out
    }

    #[test]
    fn every_face_name_and_up_gives_one_of_the_24_rotations_consistently() {
        let all = all_orients();
        let mut kinds: Vec<Orient> = all.iter().map(|a| a.0).collect();
        kinds.sort_by_key(|o| o.words());
        kinds.dedup();
        assert_eq!(kinds.len(), 24);
        for (o, face, name, _) in &all {
            assert_eq!(o.named(*face), *name, "the chosen face has its name");
            for &(_, n) in &FACE_NAMES {
                let w = o.world(n);
                assert_eq!(o.named(w), n, "there and back");
                let opposite = n.map(|v| -v);
                assert_eq!(o.world(opposite), w.map(|v| -v), "opposites stay opposite");
            }
            assert_eq!(
                cross_i(o.world([1, 0, 0]), o.world([0, 1, 0])),
                o.world([0, 0, 1]),
                "right-handed"
            );
            assert_eq!(Orient::from_words(&o.words()), Some(*o));
        }
        assert_eq!(
            Orient::from_words("1 0 0 0 1 0 0 0 -1"),
            None,
            "a mirror isn't a view"
        );
    }

    #[test]
    fn the_up_stays_up_for_a_side_name_and_reads_upright_on_top() {
        let o = Orient::with_face([1, 0, 0], [0, -1, 0], [0, 0, 1]);
        assert_eq!(o.named([0, 0, 1]), [0, 0, 1], "z stays TOP");
        assert_eq!(o.named([1, 0, 0]), [0, -1, 0], "+x is FRONT");
        let o = Orient::with_face([1, 0, 0], [0, 0, 1], [0, 0, 1]);
        assert_eq!(o.named([1, 0, 0]), [0, 0, 1], "+x is TOP");
        assert_eq!(o.named([0, 0, 1]), [0, 1, 0], "what was up reads up on it");
    }

    #[test]
    fn a_click_on_a_named_face_looks_at_the_face_with_that_word() {
        for (o, ..) in all_orients() {
            for (word, n, _, _) in LABELS {
                let region = o.world(round(n));
                let c = crate::nav::region_view_as(cam(0.3, 0.2), region, &o);
                let e = Eye::new(&c, &place());
                let toward = region.map(|v| v as f32);
                assert!(dot(e.f, toward) < -0.999, "{word} {:?}", o.words());
                let shown = labels(&c, &place(), None, &o);
                let l = shown.iter().find(|l| l.alpha >= 1.0).expect("a word");
                assert_eq!(l.text, word, "{}", o.words());
                // upright on it, and not mirrored
                let (u, v) = (
                    on_face(&c, &place(), l, -0.5, 0.0),
                    on_face(&c, &place(), l, 0.5, 0.0),
                );
                let (t, b) = (
                    on_face(&c, &place(), l, 0.0, 0.5),
                    on_face(&c, &place(), l, 0.0, -0.5),
                );
                assert!(
                    v[0] > u[0] + 1.0 && (u[1] - v[1]).abs() < 1e-2,
                    "{word} {}: {u:?} {v:?}",
                    o.words()
                );
                assert!(t[1] < b[1], "{word}");
            }
        }
    }

    #[test]
    fn colours_convert_as_srgb_does() {
        let c = rgb(0xFFFFFF);
        assert!((c[0] - 1.0).abs() < 1e-5);
        let mid = rgb(0x808080)[0];
        assert!((mid - 0.2158).abs() < 1e-3, "{mid}");
    }

    #[test]
    fn it_draws_at_most_a_few_thousand_triangles() {
        let mut out = Vec::new();
        draw(&cam(0.7, 0.5), &place(), Some(Hit::Ring), &DARK, &mut out);
        assert!(
            out.len() % 3 == 0 && out.len() > 300 && out.len() < 3 * 3000,
            "{}",
            out.len()
        );
    }
}

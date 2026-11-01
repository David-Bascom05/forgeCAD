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

/// A word to set on the widget, in real type by whoever draws text. Pixels, radians clockwise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label {
    pub text: &'static str,
    pub at: [f32; 2],
    pub angle: f32,
    /// CSS pixels.
    pub size: f32,
    pub alpha: f32,
    pub hovered: bool,
}

/// How big face labels are at most, and where they're too small to read (CSS px).
const LABEL_PX: f32 = 10.0;
const LABEL_MIN: f32 = 6.0;
const LABEL_FULL: f32 = 8.0;
/// A capital's width and height in ems, near enough to fit a word to its face.
const EM_WIDE: f32 = 0.72;
const EM_TALL: f32 = 0.72;

/// The words on the widget. A face's word runs along that face's own edge, so it turns with the
/// cube, and always reads left to right; it's never squashed: it shrinks to fit what's visible of
/// its face, and fades out only when that's too small to read.
pub fn labels(camera: &Camera, place: &Place, hover: Option<Hit>) -> Vec<Label> {
    let eye = Eye::new(camera, place);
    let mut out = Vec::with_capacity(10);
    let reach = 1.0 - CHAMFER;
    for (word, n, x, y) in LABELS {
        if !eye.sees(n, n) {
            continue;
        }
        let mid = eye.px(n);
        let along = |d: [f32; 3], s: f32| eye.px([0, 1, 2].map(|k| n[k] + d[k] * s));
        let (x0, x1) = (along(x, -reach), along(x, reach));
        let (y0, y1) = (along(y, -reach), along(y, reach));
        let wide = (x1[0] - x0[0]).hypot(x1[1] - x0[1]) / place.scale;
        let tall = (y1[0] - y0[0]).hypot(y1[1] - y0[1]) / place.scale;
        let letters = word.len() as f32;
        let size = LABEL_PX
            .min(wide * 0.92 / (letters * EM_WIDE))
            .min(tall * 0.8 / EM_TALL);
        let mut angle = (x1[1] - x0[1]).atan2(x1[0] - x0[0]);
        if angle > core::f32::consts::FRAC_PI_2 + 0.02 {
            angle -= core::f32::consts::PI;
        } else if angle < -core::f32::consts::FRAC_PI_2 - 0.02 {
            angle += core::f32::consts::PI;
        }
        out.push(Label {
            text: word,
            at: mid,
            angle,
            size: size.max(LABEL_MIN),
            alpha: ((size - LABEL_MIN) / (LABEL_FULL - LABEL_MIN)).clamp(0.0, 1.0),
            hovered: hover == Some(Hit::Region(round(n))),
        });
    }
    out
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

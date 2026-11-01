//! OpenCascade's own text format, `.brep`, the shapes FreeCAD keeps in its documents. Written from
//! what the files hold. Solids become bodies through the same healing as a STEP file's; refusals
//! name the shape and say why; nothing in a file can panic this or make it loop.

use crate::heal::{PEdge, PFace, Proto};
use crate::nurbs::{self, Nurbs};
use crate::part21::Limits;
use crate::read::{
    ImportError, Imported, Net, Refusal, Solid, Stats, extruded, finish, net_of, net_of_curve, turned,
};
use forge_geom::spline::{Spline, SplineCurve};
use forge_geom::{
    Circle, Cone, Curve, CurveGeom, Cylinder, Ellipse, Hyperbola, Line, Parabola, Plane, Sphere, Surface,
    Torus,
};
use forge_math::{Frame, Interval, Point3, Vec3, dmath, tol};
use forge_topo::Name;
use std::collections::BTreeMap;

type R<T> = Result<T, Refusal>;

const MAX_WRAP: u32 = 16;
/// Faces one file may make, copies of a shape counted each time: a few bytes can place a shape
/// many times over, nested.
const MAX_FACES: usize = 1 << 20;

fn refuse<T>(entity: usize, kind: &str, why: impl Into<String>) -> R<T> {
    Err(Refusal {
        entity: entity as u32,
        kind: kind.to_string(),
        why: why.into(),
    })
}

/// A placement: rotation by rows, then translation. `rigid` is false for a scale or a mirror.
#[derive(Clone, Copy)]
struct Xf {
    r: [[f64; 3]; 3],
    t: [f64; 3],
    rigid: bool,
}

const IDENTITY: Xf = Xf {
    r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    t: [0.0; 3],
    rigid: true,
};

impl Xf {
    fn new(r: [[f64; 3]; 3], t: [f64; 3]) -> Xf {
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let orthonormal =
            (0..3).all(|i| (0..3).all(|j| (dot(r[i], r[j]) - if i == j { 1.0 } else { 0.0 }).abs() <= 1e-9));
        let cross = [
            r[0][1] * r[1][2] - r[0][2] * r[1][1],
            r[0][2] * r[1][0] - r[0][0] * r[1][2],
            r[0][0] * r[1][1] - r[0][1] * r[1][0],
        ];
        let rigid = orthonormal && dot(cross, r[2]) > 0.0;
        Xf { r, t, rigid }
    }

    fn vec(&self, v: Vec3) -> Vec3 {
        let f = |k: usize| self.r[k][0] * v.x + self.r[k][1] * v.y + self.r[k][2] * v.z;
        Vec3::new(f(0), f(1), f(2))
    }

    fn point(&self, p: Point3) -> Point3 {
        let v = self.vec(Vec3::new(p.x, p.y, p.z));
        Point3::new(v.x + self.t[0], v.y + self.t[1], v.z + self.t[2])
    }

    /// `self` after `o`.
    fn then(&self, o: &Xf) -> Xf {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = (0..3).map(|k| self.r[i][k] * o.r[k][j]).sum();
            }
        }
        let t = self.point(Point3::new(o.t[0], o.t[1], o.t[2]));
        Xf {
            r,
            t: [t.x, t.y, t.z],
            rigid: self.rigid && o.rigid,
        }
    }

    fn inverse(&self) -> Xf {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, x) in row.iter_mut().enumerate() {
                *x = self.r[j][i];
            }
        }
        let v = |k: usize| -(r[k][0] * self.t[0] + r[k][1] * self.t[1] + r[k][2] * self.t[2]);
        let t = [v(0), v(1), v(2)];
        Xf {
            r,
            t,
            rigid: self.rigid,
        }
    }

    /// Where two uses of a shape are one: the same shape at the same place, rounding aside.
    fn key(&self) -> [i64; 12] {
        let mut k = [0; 12];
        for i in 0..3 {
            for j in 0..3 {
                k[3 * i + j] = (self.r[i][j] * 1e9).round() as i64;
            }
            k[9 + i] = (self.t[i] * 1e7).round() as i64;
        }
        k
    }
}

struct Toks<'a> {
    t: Vec<&'a str>,
    at: usize,
}

impl<'a> Toks<'a> {
    fn next(&mut self) -> Option<&'a str> {
        let x = self.t.get(self.at).copied();
        self.at += 1;
        x
    }

    fn num(&mut self) -> Option<f64> {
        self.next()?.parse().ok().filter(|x: &f64| x.is_finite())
    }

    fn int(&mut self) -> Option<i64> {
        self.next()?.parse().ok()
    }

    fn count(&mut self) -> Option<usize> {
        self.int().and_then(|n| usize::try_from(n).ok())
    }

    fn vec(&mut self) -> Option<Vec3> {
        Some(Vec3::new(self.num()?, self.num()?, self.num()?))
    }

    fn point(&mut self) -> Option<Point3> {
        Some(Point3::new(self.num()?, self.num()?, self.num()?))
    }

    /// Past `n` numbers, or None if the file ends first (or `n` is absurd).
    fn skip(&mut self, n: usize) -> Option<()> {
        self.at = self.at.checked_add(n)?;
        (self.at <= self.t.len()).then_some(())
    }

    fn find(&mut self, word: &str) -> Option<()> {
        while self.next()? != word {}
        Some(())
    }

    fn skip_curve(&mut self, depth: u32) -> Option<()> {
        if depth > MAX_WRAP {
            return None;
        }
        match self.int()? {
            1 => self.skip(6),
            2 | 4 => self.skip(13),
            3 | 5 => self.skip(14),
            6 => {
                let (rat, deg) = (self.count()?, self.count()?);
                self.skip(deg.checked_add(1)?.checked_mul(3 + rat.min(1))?)
            }
            7 => {
                let rat = self.count()?;
                self.skip(2)?;
                let (np, nk) = (self.count()?, self.count()?);
                self.skip(np.checked_mul(3 + rat.min(1))?)?;
                self.skip(nk.checked_mul(2)?)
            }
            8 => {
                self.skip(2)?;
                self.skip_curve(depth + 1)
            }
            9 => {
                self.skip(4)?;
                self.skip_curve(depth + 1)
            }
            _ => None,
        }
    }

    fn skip_surface(&mut self, depth: u32) -> Option<()> {
        if depth > MAX_WRAP {
            return None;
        }
        match self.int()? {
            1 => self.skip(12),
            2 | 4 => self.skip(13),
            3 | 5 => self.skip(14),
            6 => {
                self.skip(3)?;
                self.skip_curve(depth + 1)
            }
            7 => {
                self.skip(6)?;
                self.skip_curve(depth + 1)
            }
            8 => {
                let rat = usize::from(self.count()? + self.count()? > 0);
                let (p, q) = (self.count()?, self.count()?);
                let n = p.checked_add(1)?.checked_mul(q.checked_add(1)?)?;
                self.skip(n.checked_mul(3 + rat)?)
            }
            9 => {
                let rat = usize::from(self.count()? + self.count()? > 0);
                self.skip(4)?;
                let (nu, nv, ku, kv) = (self.count()?, self.count()?, self.count()?, self.count()?);
                self.skip(nu.checked_mul(nv)?.checked_mul(3 + rat)?)?;
                self.skip(ku.checked_add(kv)?.checked_mul(2)?)
            }
            10 => {
                self.skip(4)?;
                self.skip_surface(depth + 1)
            }
            11 => {
                self.skip(1)?;
                self.skip_surface(depth + 1)
            }
            _ => None,
        }
    }
}

/// How a shape is used: + forward, - reversed, i internal, e external.
#[derive(Clone, Copy, PartialEq)]
enum Ori {
    F,
    R,
    I,
    E,
}

impl Ori {
    fn with(self, o: Ori) -> Ori {
        match (self, o) {
            (Ori::F, x) => x,
            (Ori::R, Ori::F) => Ori::R,
            (Ori::R, Ori::R) => Ori::F,
            (_, x @ (Ori::I | Ori::E)) => x,
            (x, _) => x,
        }
    }
}

#[derive(Clone, Copy)]
struct Child {
    ori: Ori,
    shape: usize,
    loc: usize,
}

enum Kind {
    Vertex(Point3),
    Edge {
        degenerate: bool,
        /// Its 3D curve: curve, location, first, last.
        curve: Option<(usize, usize, f64, f64)>,
    },
    Face {
        surface: usize,
        loc: usize,
    },
    /// Wi, Sh, So, CS, Co
    Group(&'static str),
}

struct Shape {
    kind: Kind,
    children: Vec<Child>,
}

struct File<'a> {
    toks: Toks<'a>,
    locs: Vec<Xf>,
    curves: Vec<usize>,
    surfaces: Vec<usize>,
    /// By the file's numbering: the last written is 1.
    shapes: Vec<Shape>,
    root: Child,
}

fn parse(text: &[u8]) -> Option<File<'_>> {
    let text = std::str::from_utf8(text).ok()?;
    let mut t = Toks {
        t: text.split_ascii_whitespace().collect(),
        at: 0,
    };
    t.find("Locations")?;
    let n = t.count()?;
    let mut locs = vec![IDENTITY];
    for _ in 0..n {
        let l = match t.int()? {
            1 => {
                let mut r = [[0.0; 3]; 3];
                let mut x = [0.0; 3];
                for i in 0..3 {
                    for rk in &mut r[i] {
                        *rk = t.num()?;
                    }
                    x[i] = t.num()?;
                }
                Xf::new(r, x)
            }
            2 => {
                let mut l = IDENTITY;
                loop {
                    let k = t.count()?;
                    if k == 0 {
                        break l;
                    }
                    let b = *locs.get(k)?;
                    let p = t.int()?;
                    if p.unsigned_abs() > 64 {
                        return None;
                    }
                    let b = if p < 0 { b.inverse() } else { b };
                    for _ in 0..p.unsigned_abs() {
                        l = b.then(&l);
                    }
                }
            }
            _ => return None,
        };
        locs.push(l);
    }
    t.find("Curves")?;
    let n = t.count()?;
    let mut curves = Vec::new();
    for _ in 0..n {
        curves.push(t.at);
        t.skip_curve(0)?;
    }
    t.find("Surfaces")?;
    let n = t.count()?;
    let mut surfaces = Vec::new();
    for _ in 0..n {
        surfaces.push(t.at);
        t.skip_surface(0)?;
    }
    t.find("TShapes")?;
    let n = t.count()?;
    let mut shapes = Vec::new();
    for _ in 0..n {
        let tag = t.next()?;
        let kind = match tag {
            "Ve" => {
                t.skip(1)?;
                let p = t.point()?;
                loop {
                    match t.int()? {
                        0 => {
                            t.skip(1)?;
                            break;
                        }
                        1 => t.skip(3)?,
                        2 | 3 => t.skip(4)?,
                        _ => return None,
                    }
                }
                Kind::Vertex(p)
            }
            "Ed" => {
                t.skip(3)?;
                let degenerate = t.int()? != 0;
                let mut curve = None;
                loop {
                    match t.int()? {
                        0 => break,
                        1 => {
                            let (c, l) = (t.count()?, t.count()?);
                            curve = Some((c, l, t.num()?, t.num()?));
                        }
                        2 => t.skip(5)?,
                        // two pcurves, the second's number run into its continuity ("2CN")
                        3 => t.skip(6)?,
                        4 => t.skip(5)?,
                        5 => t.skip(2)?,
                        6 => t.skip(3)?,
                        7 => t.skip(4)?,
                        _ => return None,
                    }
                }
                Kind::Edge { degenerate, curve }
            }
            "Fa" => {
                t.skip(2)?;
                let (surface, loc) = (t.count()?, t.count()?);
                // a triangulation's number, where one is kept
                if t.t.get(t.at) == Some(&"2") {
                    t.skip(2)?;
                }
                Kind::Face { surface, loc }
            }
            "Wi" => Kind::Group("Wi"),
            "Sh" => Kind::Group("Sh"),
            "So" => Kind::Group("So"),
            "CS" => Kind::Group("CS"),
            "Co" => Kind::Group("Co"),
            _ => return None,
        };
        // the flags, then the children
        t.skip(1)?;
        let mut children = Vec::new();
        loop {
            let c = t.next()?;
            if c == "*" {
                break;
            }
            children.push(child(c, t.count()?)?);
        }
        shapes.push(Shape { kind, children });
    }
    // the file lists children before parents and numbers from its end
    shapes.reverse();
    let c = t.next()?;
    let root = child(c, t.count()?)?;
    Some(File {
        toks: t,
        locs,
        curves,
        surfaces,
        shapes,
        root,
    })
}

fn child(c: &str, loc: usize) -> Option<Child> {
    let mut chars = c.chars();
    let ori = match chars.next()? {
        '+' => Ori::F,
        '-' => Ori::R,
        'i' => Ori::I,
        'e' => Ori::E,
        _ => return None,
    };
    Some(Child {
        ori,
        shape: chars.as_str().parse().ok()?,
        loc,
    })
}

/// The whole knot vector from values and multiplicities. A periodic one is unrolled past both ends:
/// `Some((knots, extra, turn))`, with `extra` poles more that wrap round and pole `i` of the
/// result the file's `(i + poles - turn) % poles`, so the domain starts at the first knot.
/// None when they do not add up.
fn flat_knots(
    deg: usize,
    knots: &[(f64, usize)],
    poles: usize,
    periodic: bool,
) -> Option<(Vec<f64>, usize, usize)> {
    let total: usize = knots.iter().try_fold(0usize, |a, k| a.checked_add(k.1))?;
    if total > nurbs::MAX_CTRL || poles > nurbs::MAX_CTRL || knots.len() < 2 {
        return None;
    }
    if !periodic {
        let mut out = Vec::with_capacity(total);
        for k in knots {
            out.extend(std::iter::repeat_n(k.0, k.1));
        }
        return Some((out, 0, 0));
    }
    let period = knots[knots.len() - 1].0 - knots[0].0;
    // one period's knots, the seam's counted once
    let mut one = Vec::with_capacity(poles);
    for k in &knots[..knots.len() - 1] {
        one.extend(std::iter::repeat_n(k.0, k.1));
    }
    if one.len() != poles || poles == 0 || period <= 0.0 {
        return None;
    }
    // the seam's last copy is knot `deg`, leaving `deg` before the domain; a span there starts at
    // the file's pole -m0 (its first knot's multiplicity), hence the turn
    let at = |i: isize| {
        let n = poles as isize;
        let (q, r) = (i.div_euclid(n), i.rem_euclid(n));
        one[r as usize] + q as f64 * period
    };
    let m0 = knots[0].1 as isize;
    let first = m0 - deg as isize - 1;
    let out = (0..poles + 2 * deg + 1).map(|i| at(first + i as isize)).collect();
    Some((out, deg, knots[0].1 % poles))
}

struct Read<'a> {
    f: File<'a>,
    p: Proto,
    verts: BTreeMap<(usize, [i64; 12]), usize>,
    edges: BTreeMap<(usize, [i64; 12]), Option<usize>>,
    stats: Stats,
    salt: Name,
    made: u64,
    faces: usize,
}

impl Read<'_> {
    fn name(&mut self) -> Name {
        self.made += 1;
        self.salt.with(self.made)
    }

    fn knots_of(&mut self, n: usize) -> Option<Vec<(f64, usize)>> {
        let t = &mut self.f.toks;
        if n > nurbs::MAX_CTRL {
            return None;
        }
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let (k, m) = (t.num()?, t.count()?);
            out.push((k, m));
        }
        Some(out)
    }

    /// Point, normal, x, y: a frame that is right handed, and whether the file's was not (its
    /// surface's natural normal is then the other way).
    fn frame(&mut self, xf: &Xf) -> Option<(Frame, bool)> {
        let t = &mut self.f.toks;
        let o = xf.point(t.point()?);
        let n = xf.vec(t.vec()?);
        let x = xf.vec(t.vec()?);
        let y = xf.vec(t.vec()?);
        let z = x.cross(y);
        Some((Frame::from_axes(o, x, y, z)?, z.dot(n) < 0.0))
    }

    fn curve(&mut self, entity: usize, xf: &Xf, depth: u32) -> R<Curve> {
        let k = "curve";
        let bad = || Refusal {
            entity: entity as u32,
            kind: k.into(),
            why: "the file breaks off or holds a bad number".into(),
        };
        if depth > MAX_WRAP {
            return refuse(entity, k, "curves wrapped too deep");
        }
        match self.f.toks.int().ok_or_else(bad)? {
            1 => {
                let t = &mut self.f.toks;
                let (o, d) = (t.point().ok_or_else(bad)?, t.vec().ok_or_else(bad)?);
                match xf.vec(d).normalize() {
                    Some(dir) => Ok(Curve::Line(Line {
                        origin: xf.point(o),
                        dir,
                    })),
                    None => refuse(entity, "line", "no direction"),
                }
            }
            2 => {
                let (frame, _) = self.frame(xf).ok_or_else(bad)?;
                let radius = self.f.toks.num().ok_or_else(bad)?;
                Ok(Curve::Circle(Circle { frame, radius }))
            }
            3 => {
                let (frame, _) = self.frame(xf).ok_or_else(bad)?;
                let t = &mut self.f.toks;
                let (major, minor) = (t.num().ok_or_else(bad)?, t.num().ok_or_else(bad)?);
                Ok(Curve::Ellipse(Ellipse { frame, major, minor }))
            }
            4 => {
                let (frame, _) = self.frame(xf).ok_or_else(bad)?;
                let focal = self.f.toks.num().ok_or_else(bad)?;
                Ok(Curve::Parabola(Parabola { frame, focal }))
            }
            5 => {
                let (frame, _) = self.frame(xf).ok_or_else(bad)?;
                let t = &mut self.f.toks;
                let (a, b) = (t.num().ok_or_else(bad)?, t.num().ok_or_else(bad)?);
                Ok(Curve::Hyperbola(Hyperbola { frame, a, b }))
            }
            6 => {
                let t = &mut self.f.toks;
                let (rat, deg) = (t.count().ok_or_else(bad)?, t.count().ok_or_else(bad)?);
                if !(1..=forge_geom::spline::MAX_DEGREE).contains(&deg) {
                    return refuse(entity, "bezier curve", format!("degree {deg}"));
                }
                let (pts, w) = self.poles(deg + 1, rat > 0, xf).ok_or_else(bad)?;
                let mut knots = vec![0.0; deg + 1];
                knots.extend(std::iter::repeat_n(1.0, deg + 1));
                spline_curve(entity, deg, &knots, &pts, w.as_deref())
            }
            7 => {
                let t = &mut self.f.toks;
                let (rat, periodic) = (t.count().ok_or_else(bad)?, t.count().ok_or_else(bad)?);
                let (deg, np, nk) = (
                    t.count().ok_or_else(bad)?,
                    t.count().ok_or_else(bad)?,
                    t.count().ok_or_else(bad)?,
                );
                if !(1..=forge_geom::spline::MAX_DEGREE).contains(&deg) || np > nurbs::MAX_CTRL {
                    return refuse(entity, "b-spline curve", format!("degree {deg}, {np} poles"));
                }
                let (mut pts, mut w) = self.poles(np, rat > 0, xf).ok_or_else(bad)?;
                let knots = self.knots_of(nk).ok_or_else(bad)?;
                let Some((flat, extra, turn)) = flat_knots(deg, &knots, np, periodic > 0) else {
                    return refuse(entity, "b-spline curve", "knots and poles do not add up");
                };
                let at = |i: usize| (i + np - turn) % np;
                if extra > 0 {
                    pts = (0..np + extra).map(|i| pts[at(i)]).collect();
                    if let Some(x) = &mut w {
                        *x = (0..np + extra).map(|i| x[at(i)]).collect();
                    }
                }
                spline_curve(entity, deg, &flat, &pts, w.as_deref())
            }
            // the edge's range bounds it; only the curve matters
            8 => {
                self.f.toks.skip(2).ok_or_else(bad)?;
                self.curve(entity, xf, depth + 1)
            }
            9 => refuse(entity, "offset curve", "not read yet"),
            n => refuse(entity, k, format!("kind {n}")),
        }
    }

    /// The net of a curve a surface moves. A trimmed conic gives only its arc, a little past each
    /// end: a whole circle could cross the axis it turns about, and the surface fold on itself.
    fn basis(&mut self, entity: usize, xf: &Xf, depth: u32) -> R<Option<Net>> {
        let t = &mut self.f.toks;
        let trim = if t.t.get(t.at) == Some(&"8") {
            t.skip(1);
            match (t.num(), t.num()) {
                (Some(a), Some(b)) if a < b => Some((a, b)),
                _ => return refuse(entity, "trimmed curve", "a bad range"),
            }
        } else {
            None
        };
        let c = self.curve(entity, xf, depth + 1)?;
        let arc = |o: Point3, x: Vec3, y: Vec3, (a, b): (f64, f64)| {
            let past = 0.02 * (b - a);
            let sweep = (b - a + 2.0 * past).min(dmath::TAU);
            Spline::ellipse(o, x, y, a - past, sweep).map(|s| net_of(&s))
        };
        Ok(match (&c, trim) {
            (Curve::Circle(k), Some(r)) => arc(
                k.frame.origin,
                Vec3::from(k.frame.x()) * k.radius,
                Vec3::from(k.frame.y()) * k.radius,
                r,
            ),
            (Curve::Ellipse(k), Some(r)) => arc(
                k.frame.origin,
                Vec3::from(k.frame.x()) * k.major,
                Vec3::from(k.frame.y()) * k.minor,
                r,
            ),
            _ => net_of_curve(&c),
        })
    }

    fn poles(&mut self, n: usize, rational: bool, xf: &Xf) -> Option<(Vec<Point3>, Option<Vec<f64>>)> {
        let t = &mut self.f.toks;
        let size = n.checked_mul(if rational { 4 } else { 3 })?;
        // the room is there before any is asked for
        if t.at.checked_add(size)? > t.t.len() {
            return None;
        }
        let mut pts = Vec::with_capacity(n);
        let mut w = rational.then(|| Vec::with_capacity(n));
        for _ in 0..n {
            pts.push(xf.point(t.point()?));
            if let Some(w) = &mut w {
                let x = t.num()?;
                if x <= 0.0 {
                    return None;
                }
                w.push(x);
            }
        }
        Some((pts, w))
    }

    /// The surface and whether its natural normal is against the kernel's for it.
    fn surface(&mut self, entity: usize, xf: &Xf, depth: u32) -> R<(Surface, bool)> {
        let k = "surface";
        let bad = || Refusal {
            entity: entity as u32,
            kind: k.into(),
            why: "the file breaks off or holds a bad number".into(),
        };
        if depth > MAX_WRAP {
            return refuse(entity, k, "surfaces wrapped too deep");
        }
        match self.f.toks.int().ok_or_else(bad)? {
            // a plane's normal is its x cross y, whichever way the file's own points
            1 => {
                let (frame, _) = self.frame(xf).ok_or_else(bad)?;
                Ok((Surface::Plane(Plane { frame }), false))
            }
            2 => {
                let (frame, flip) = self.frame(xf).ok_or_else(bad)?;
                let radius = self.f.toks.num().ok_or_else(bad)?;
                Ok((Surface::Cylinder(Cylinder { frame, radius }), flip))
            }
            3 => {
                let t = &mut self.f.toks;
                let o = xf.point(t.point().ok_or_else(bad)?);
                let n = xf.vec(t.vec().ok_or_else(bad)?);
                let x = xf.vec(t.vec().ok_or_else(bad)?);
                let y = xf.vec(t.vec().ok_or_else(bad)?);
                let (radius, half) = (t.num().ok_or_else(bad)?, t.num().ok_or_else(bad)?);
                // ours opens along its z: the file's axis, or against it for a negative angle.
                // Its natural normal is outward either way, unless its frame is left handed
                let z = if half < 0.0 { -n } else { n };
                let frame = Frame::from_axes(o, x, z.cross(x), z);
                let cone = frame.and_then(|f| Cone::new(f, radius, half.abs()));
                match cone {
                    Some(c) => Ok((Surface::Cone(c), x.cross(y).dot(n) < 0.0)),
                    None => refuse(entity, "cone", format!("radius {radius}, half angle {half}")),
                }
            }
            4 => {
                let (frame, flip) = self.frame(xf).ok_or_else(bad)?;
                let radius = self.f.toks.num().ok_or_else(bad)?;
                Ok((Surface::Sphere(Sphere { frame, radius }), flip))
            }
            5 => {
                let (frame, flip) = self.frame(xf).ok_or_else(bad)?;
                let t = &mut self.f.toks;
                let (major, minor) = (t.num().ok_or_else(bad)?, t.num().ok_or_else(bad)?);
                match Torus::new(frame, major, minor) {
                    Some(s) => Ok((Surface::Torus(s), flip)),
                    None => refuse(entity, "torus", format!("radii {major} and {minor}")),
                }
            }
            6 => {
                let dir = xf.vec(self.f.toks.vec().ok_or_else(bad)?);
                let Some(dir) = dir.normalize() else {
                    return refuse(entity, "extrusion", "no direction");
                };
                let Some(net) = self.basis(entity, xf, depth)? else {
                    return refuse(entity, "extrusion", "its curve cannot be moved exactly");
                };
                match extruded(net, dir.into()) {
                    Some(s) => Ok((s, false)),
                    None => refuse(entity, "extrusion", "no surface from its curve"),
                }
            }
            7 => {
                let t = &mut self.f.toks;
                let (o, d) = (t.point().ok_or_else(bad)?, t.vec().ok_or_else(bad)?);
                let (o, d) = (xf.point(o), xf.vec(d));
                let Some(d) = d.normalize() else {
                    return refuse(entity, "revolution", "no direction");
                };
                let Some(net) = self.basis(entity, xf, depth)? else {
                    return refuse(entity, "revolution", "its curve cannot be moved exactly");
                };
                match turned((o, d.into()), net, 1e-6) {
                    Some(s) => Ok((s, false)),
                    None => refuse(entity, "revolution", "no surface from its curve"),
                }
            }
            8 => {
                let t = &mut self.f.toks;
                let rat = t.count().ok_or_else(bad)? + t.count().ok_or_else(bad)? > 0;
                let (p, q) = (t.count().ok_or_else(bad)?, t.count().ok_or_else(bad)?);
                if !(1..=nurbs::MAX_DEGREE).contains(&p) || !(1..=nurbs::MAX_DEGREE).contains(&q) {
                    return refuse(entity, "bezier surface", format!("degrees {p}, {q}"));
                }
                let (pts, w) = self.poles((p + 1) * (q + 1), rat, xf).ok_or_else(bad)?;
                let clamped = |d: usize| {
                    let mut k = vec![0.0; d + 1];
                    k.extend(std::iter::repeat_n(1.0, d + 1));
                    k
                };
                match Nurbs::new(
                    (p, q),
                    (clamped(p), clamped(q)),
                    (p + 1, q + 1),
                    &pts,
                    w.as_deref(),
                ) {
                    Some(n) => Ok((n.surface(), false)),
                    None => refuse(entity, "bezier surface", "does not make a surface"),
                }
            }
            9 => self.bspline_surface(entity, xf).map(|s| (s, false)),
            10 => {
                self.f.toks.skip(4).ok_or_else(bad)?;
                self.surface(entity, xf, depth + 1)
            }
            11 => refuse(entity, "offset surface", "not read yet"),
            n => refuse(entity, k, format!("kind {n}")),
        }
    }

    fn bspline_surface(&mut self, entity: usize, xf: &Xf) -> R<Surface> {
        let k = "b-spline surface";
        let bad = || Refusal {
            entity: entity as u32,
            kind: k.into(),
            why: "the file breaks off or holds a bad number".into(),
        };
        let t = &mut self.f.toks;
        let mut c = || t.count().ok_or_else(bad);
        let rat = c()? + c()? > 0;
        let (per_u, per_v) = (c()? > 0, c()? > 0);
        let (p, q, nu, nv, ku, kv) = (c()?, c()?, c()?, c()?, c()?, c()?);
        if !(1..=nurbs::MAX_DEGREE).contains(&p) || !(1..=nurbs::MAX_DEGREE).contains(&q) {
            return refuse(entity, k, format!("degrees {p}, {q}"));
        }
        let Some(total) = nu.checked_mul(nv).filter(|&n| n > 0 && n <= nurbs::MAX_CTRL) else {
            return refuse(entity, k, "too many poles or none");
        };
        let (pts, w) = self.poles(total, rat, xf).ok_or_else(bad)?;
        let knots_u = self.knots_of(ku).ok_or_else(bad)?;
        let knots_v = self.knots_of(kv).ok_or_else(bad)?;
        let (Some((fu, xu, tu)), Some((fv, xv, tv))) = (
            flat_knots(p, &knots_u, nu, per_u),
            flat_knots(q, &knots_v, nv, per_v),
        ) else {
            return refuse(entity, k, "knots and poles do not add up");
        };
        // poles that wrap round, in both directions
        let (mu, mv) = (nu + xu, nv + xv);
        let Some(all) = mu.checked_mul(mv).filter(|&n| n <= nurbs::MAX_CTRL) else {
            return refuse(entity, k, "too many poles");
        };
        let mut net = Vec::with_capacity(all);
        let mut ws = w.as_ref().map(|_| Vec::with_capacity(all));
        for i in 0..mu {
            for j in 0..mv {
                let at = ((i + nu - tu) % nu) * nv + (j + nv - tv) % nv;
                net.push(pts[at]);
                if let (Some(ws), Some(w)) = (&mut ws, &w) {
                    ws.push(w[at]);
                }
            }
        }
        match Nurbs::new((p, q), (fu, fv), (mu, mv), &net, ws.as_deref()) {
            Some(n) => Ok(n.surface()),
            None => refuse(entity, k, "knots, poles and weights do not make a surface"),
        }
    }

    fn shape(&self, c: Child) -> R<(&Shape, Xf)> {
        let (Some(s), Some(l)) = (self.f.shapes.get(c.shape.wrapping_sub(1)), self.f.locs.get(c.loc)) else {
            return refuse(c.shape, "shape", "a reference to nothing");
        };
        Ok((s, *l))
    }

    fn vertex(&mut self, c: Child, xf: &Xf) -> R<usize> {
        let (s, l) = self.shape(c)?;
        let Kind::Vertex(p) = s.kind else {
            return refuse(c.shape, "edge", "a child that is no vertex");
        };
        let xf = xf.then(&l);
        let key = (c.shape, xf.key());
        if let Some(&v) = self.verts.get(&key) {
            return Ok(v);
        }
        let name = self.name();
        self.p.verts.push((xf.point(p), name));
        let v = self.p.verts.len() - 1;
        self.verts.insert(key, v);
        Ok(v)
    }

    /// The edge at its place, None for a degenerate one (a pole: seams reach it in healing).
    fn edge(&mut self, c: Child, xf: &Xf) -> R<Option<usize>> {
        let (s, l) = self.shape(c)?;
        let xf = xf.then(&l);
        let key = (c.shape, xf.key());
        if let Some(&e) = self.edges.get(&key) {
            return Ok(e);
        }
        let Kind::Edge { degenerate, curve } = s.kind else {
            return refuse(c.shape, "wire", "a child that is no edge");
        };
        if degenerate {
            self.edges.insert(key, None);
            return Ok(None);
        }
        let Some((ci, cl, first, last)) = curve else {
            return refuse(c.shape, "edge", "only curves on surfaces, no 3D curve");
        };
        let children = self.f.shapes[c.shape - 1].children.clone();
        let (mut start, mut end) = (None, None);
        for v in children {
            match v.ori {
                Ori::F => start = Some(self.vertex(v, &xf)?),
                Ori::R => end = Some(self.vertex(v, &xf)?),
                _ => {}
            }
        }
        let (Some(start), Some(end)) = (start, end) else {
            return refuse(c.shape, "edge", "without two ends");
        };
        let Some((&at, &loc)) = self.f.curves.get(ci.wrapping_sub(1)).zip(self.f.locs.get(cl)) else {
            return refuse(c.shape, "edge", "a curve that is not there");
        };
        self.f.toks.at = at;
        let curve = self.curve(c.shape, &xf.then(&loc), 0)?;
        let bad = match &curve {
            Curve::Circle(k) => k.radius <= 0.0,
            Curve::Ellipse(k) => k.major <= 0.0 || k.minor <= 0.0,
            _ => false,
        };
        if bad {
            return refuse(c.shape, "edge", "a curve of no size");
        }
        let (a, b) = (self.p.verts[start].0, self.p.verts[end].0);
        let (range, off) = range_of(&curve, first, last, a, b);
        self.stats.vertex_off_curve = self.stats.vertex_off_curve.max(off);
        if off > tol::LINEAR {
            self.stats.vertices_off += 1;
        }
        let name = self.name();
        self.p.edges.push(PEdge {
            curve,
            range,
            start,
            end,
            name,
        });
        self.stats.edges += 1;
        let e = self.p.edges.len() - 1;
        self.edges.insert(key, Some(e));
        Ok(Some(e))
    }

    /// A wire's edges as closed loops, walked by their vertices as far as each goes: a file need
    /// not keep them in order, and one wire may close more than once.
    fn wire_loops(&self, id: usize, uses: Vec<(usize, bool)>) -> R<Vec<Vec<(usize, bool)>>> {
        let n = uses.len();
        let mut used = vec![false; n];
        let mut out = Vec::new();
        for s in 0..n {
            if used[s] {
                continue;
            }
            used[s] = true;
            let mut lp = vec![uses[s]];
            let begin = self.p.start_of(uses[s]);
            let mut last = s;
            loop {
                let here = self.p.start_of((uses[last].0, !uses[last].1));
                let joins = |i: usize| !used[i] && self.p.start_of(uses[i]) == here;
                let around = (last + 1..n).chain(0..last);
                // an edge closed on itself here goes first: a rim, between a seam's two uses
                let rim = around
                    .clone()
                    .find(|&i| joins(i) && self.p.start_of((uses[i].0, !uses[i].1)) == here);
                let next = rim.or_else(|| around.clone().find(|&i| joins(i)));
                let Some(i) = next else {
                    if here != begin {
                        return refuse(id, "wire", "does not close");
                    }
                    break;
                };
                used[i] = true;
                lp.push(uses[i]);
                last = i;
            }
            out.push(lp);
        }
        Ok(out)
    }

    fn face(&mut self, c: Child, ori: Ori, xf: &Xf, shell: usize) -> R<()> {
        let (s, l) = self.shape(c)?;
        let Kind::Face { surface, loc } = s.kind else {
            return refuse(c.shape, "shell", "a child that is no face");
        };
        let wires = s.children.clone();
        self.faces += 1;
        if self.faces > MAX_FACES {
            return refuse(c.shape, "face", "too many faces");
        }
        let xf = xf.then(&l);
        let Some((&at, &sl)) = self
            .f
            .surfaces
            .get(surface.wrapping_sub(1))
            .zip(self.f.locs.get(loc))
        else {
            return refuse(c.shape, "face", "a surface that is not there");
        };
        self.f.toks.at = at;
        let (surf, flip) = self.surface(c.shape, &xf.then(&sl), 0)?;
        let name = self.name();
        self.stats.faces += 1;
        let mut loops = Vec::new();
        for w in wires {
            // what lies inside a face without bounding it is no bound
            if matches!(w.ori, Ori::I | Ori::E) {
                continue;
            }
            let (ws, wl) = self.shape(w)?;
            if !matches!(ws.kind, Kind::Group("Wi")) {
                return refuse(w.shape, "face", "a child that is no wire");
            }
            let wx = xf.then(&wl);
            let wo = ori.with(w.ori);
            let mut uses = Vec::new();
            for e in ws.children.clone() {
                let eo = wo.with(e.ori);
                if matches!(eo, Ori::I | Ori::E) {
                    continue;
                }
                if let Some(x) = self.edge(e, &wx)? {
                    uses.push((x, eo == Ori::F));
                }
            }
            loops.extend(self.wire_loops(w.shape, uses)?);
        }
        self.p.faces.push(PFace {
            surface: surf,
            same_sense: (ori == Ori::F) != flip,
            name,
            shell,
            loops,
        });
        Ok(())
    }

    fn solid(&mut self, c: Child, ori: Ori, xf: &Xf) -> R<()> {
        let (s, _) = self.shape(c)?;
        for sh in s.children.clone() {
            let (shape, l) = self.shape(sh)?;
            if !matches!(shape.kind, Kind::Group("Sh")) {
                return refuse(sh.shape, "solid", "a child that is no shell");
            }
            let so = ori.with(sh.ori);
            if matches!(so, Ori::I | Ori::E) {
                continue;
            }
            let faces = shape.children.clone();
            let sx = xf.then(&l);
            let n = self.p.shells;
            self.p.shells += 1;
            for f in faces {
                let fo = so.with(f.ori);
                if matches!(fo, Ori::I | Ori::E) {
                    continue;
                }
                self.face(f, fo, &sx, n)?;
            }
        }
        Ok(())
    }
}

fn spline_curve(entity: usize, deg: usize, knots: &[f64], pts: &[Point3], w: Option<&[f64]>) -> R<Curve> {
    let Some(s) = Spline::from_nurbs(deg, knots, pts, w) else {
        return refuse(
            entity,
            "b-spline curve",
            "knots, poles and degree do not make a curve",
        );
    };
    let dom = s.domain();
    Ok(SplineCurve::curve(std::sync::Arc::new(s), dom))
}

/// The edge's range on our curve, and how far its ends lie from it. A line's, a conic's and a
/// spline's parameter are the file's; a parabola's and a hyperbola's are not.
fn range_of(c: &Curve, first: f64, last: f64, a: Point3, b: Point3) -> (Interval, f64) {
    let off = |r: Interval| c.eval(r.lo).distance(a).max(c.eval(r.hi).distance(b));
    let d = c.domain();
    let direct = match c {
        Curve::Line(_) | Curve::Circle(_) | Curve::Ellipse(_) => true,
        Curve::Parabola(_) | Curve::Hyperbola(_) => false,
        // inside the spline's domain, as a periodic one's unrolled may not be
        _ => first >= d.lo - 1e-9 && last <= d.hi + 1e-9,
    };
    if direct && first < last {
        let r = Interval::new(first, last);
        return (r, off(r));
    }
    let far = 4.0 * tol::MODEL_HALF_SIZE;
    let whole = if d.lo.is_finite() && d.hi.is_finite() {
        Interval::new(d.lo, d.hi)
    } else {
        Interval::new(-far, far)
    };
    let (t0, t1) = (c.closest(a, whole), c.closest(b, whole));
    let r = if t0 < t1 {
        Interval::new(t0, t1)
    } else {
        // through a periodic curve's seam, or the whole of a closed one
        Interval::new(t0, t1 + d.period.unwrap_or(whole.width()))
    };
    (r, off(r))
}

/// Every solid below `c`; shapes that are not solids are refused by name, as they make no body.
fn collect(
    f: &File,
    c: Child,
    ori: Ori,
    xf: Xf,
    depth: u32,
    out: &mut Vec<(Child, Ori, Xf)>,
    no: &mut Vec<Refusal>,
) {
    let Some((s, l)) = f.shapes.get(c.shape.wrapping_sub(1)).zip(f.locs.get(c.loc)) else {
        no.push(Refusal {
            entity: c.shape as u32,
            kind: "shape".into(),
            why: "a reference to nothing".into(),
        });
        return;
    };
    let xf = xf.then(l);
    let ori = ori.with(c.ori);
    match s.kind {
        Kind::Group("So") => out.push((c, ori, xf)),
        Kind::Group("Co" | "CS") => {
            if depth >= MAX_WRAP || out.len() + no.len() > MAX_FACES {
                no.push(Refusal {
                    entity: c.shape as u32,
                    kind: "compound".into(),
                    why: "nested too deep or holding too much".into(),
                });
                return;
            }
            for k in &s.children {
                collect(f, *k, ori, xf, depth + 1, out, no);
            }
        }
        Kind::Group(k) => no.push(Refusal {
            entity: c.shape as u32,
            kind: if k == "Sh" { "shell" } else { "wire" }.into(),
            why: "not a solid".into(),
        }),
        Kind::Face { .. } => no.push(Refusal {
            entity: c.shape as u32,
            kind: "face".into(),
            why: "not a solid".into(),
        }),
        Kind::Edge { .. } | Kind::Vertex(_) => no.push(Refusal {
            entity: c.shape as u32,
            kind: "edge".into(),
            why: "not a solid".into(),
        }),
    }
}

/// Whether these bytes are a `.brep`.
pub fn is_brep(text: &[u8]) -> bool {
    text.windows(16).take(256).any(|w| w == b"CASCADE Topology")
}

pub fn import(text: &[u8], lim: &Limits) -> Result<Imported, ImportError> {
    let Some(f) = parse(text) else {
        return Err(ImportError::Brep);
    };
    // children come before parents in a file, so a reference only ever goes one way
    let bad = f
        .shapes
        .iter()
        .enumerate()
        .any(|(i, s)| s.children.iter().any(|c| c.shape <= i + 1));
    if bad {
        return Err(ImportError::Brep);
    }
    let (mut roots, mut refusals) = (Vec::new(), Vec::new());
    collect(&f, f.root, Ori::F, IDENTITY, 0, &mut roots, &mut refusals);
    let mut r = Read {
        f,
        p: Proto::default(),
        verts: BTreeMap::new(),
        edges: BTreeMap::new(),
        stats: Stats {
            units_mm: 1.0,
            uncertainty: 1e-7,
            ..Stats::default()
        },
        salt: Name::NONE,
        made: 0,
        faces: 0,
    };
    let mut solids = Vec::new();
    for (n, (c, ori, xf)) in roots.into_iter().enumerate() {
        if !xf.rigid {
            refusals.push(Refusal {
                entity: c.shape as u32,
                kind: "solid".into(),
                why: "placed with a scale or a mirror".into(),
            });
            continue;
        }
        let salt = Name::feature(n as u64 + 1).with_str("brep");
        r.salt = salt;
        r.p = Proto {
            salt,
            ..Proto::default()
        };
        r.verts.clear();
        r.edges.clear();
        // a solid's own location is in `xf`: its children start from there
        if let Err(x) = r.solid(c, ori, &xf) {
            refusals.push(x);
            continue;
        }
        let proto = std::mem::take(&mut r.p);
        match finish(proto, 1e-3, lim, &mut r.stats) {
            Ok(body) => solids.push(Solid {
                entity: c.shape as u32,
                name: String::new(),
                body,
            }),
            Err(why) => refusals.push(Refusal {
                entity: c.shape as u32,
                kind: "solid".into(),
                why,
            }),
        }
    }
    if solids.is_empty() && refusals.is_empty() {
        return Err(ImportError::NoSolids);
    }
    Ok(Imported {
        solids,
        refusals,
        schema: "OpenCascade brep".into(),
        system: String::new(),
        stats: r.stats,
    })
}

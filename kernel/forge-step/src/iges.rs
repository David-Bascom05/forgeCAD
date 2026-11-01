//! IGES 5.3, the boundary-representation entities: 186 and what it stands on. Written from the
//! specification. A file of trimmed surfaces (144, 143) holds no topology to read, so it is refused
//! by name: sewing them into a solid is another matter.

use crate::brep::{Xf, spline_curve};
use crate::heal::{PEdge, PFace, Proto};
use crate::nurbs::Nurbs;
use crate::part21::Limits;
use crate::read::{ImportError, Imported, Refusal, Solid, Stats, edge_range, finish};
use forge_geom::{Circle, Cone, Curve, Cylinder, Ellipse, Line, Plane, Sphere, Surface, Torus};
use forge_math::{Frame, Point3, Vec3, dmath};
use forge_topo::Name;
use std::collections::BTreeMap;

type R<T> = Result<T, Refusal>;

fn refuse<T>(id: usize, kind: &str, why: impl Into<String>) -> R<T> {
    Err(Refusal {
        entity: id as u32,
        kind: kind.to_string(),
        why: why.into(),
    })
}

/// One directory entry, by the number of its first line.
struct Entry {
    kind: u32,
    form: u32,
    matrix: usize,
    params: Vec<f64>,
}

/// A number of a Fortran file: D for E in the exponent.
fn number(t: &str) -> Option<f64> {
    let t = t.trim();
    if t.is_empty() {
        return Some(0.0);
    }
    if t.contains(['D', 'd']) {
        return t.replace(['D', 'd'], "E").parse().ok();
    }
    t.parse().ok().filter(|x: &f64| x.is_finite())
}

/// The parameters of a free-format run, strings as zero: `nHtext` runs to its count.
fn fields(s: &str, comma: u8, semi: u8) -> Vec<String> {
    let b = s.as_bytes();
    let (mut out, mut at, mut start) = (Vec::new(), 0, 0);
    while at < b.len() {
        let c = b[at];
        if c == comma || c == semi {
            out.push(s.get(start..at).unwrap_or("").to_string());
            at += 1;
            start = at;
            if c == semi {
                return out;
            }
        } else if c == b'H'
            && at > start
            && s.get(start..at)
                .is_some_and(|d| d.trim().parse::<usize>().is_ok())
        {
            let n: usize = s[start..at].trim().parse().unwrap_or(0);
            at = (at + 1).saturating_add(n);
        } else {
            at += 1;
        }
    }
    if start < b.len() {
        out.push(s.get(start..).unwrap_or("").to_string());
    }
    out
}

pub fn is_iges(text: &[u8]) -> bool {
    let line = text.split(|&c| c == b'\n').next().unwrap_or(&[]);
    line.len() >= 73
        && line[72] == b'S'
        && line
            .get(73..80)
            .is_some_and(|n| n.iter().all(|c| c.is_ascii_digit() || *c == b' '))
}

/// A curve's plane from where its x and y went: a matrix may mirror them (a writer's way to an arc
/// running clockwise), and its numbers hold only so many digits.
fn plane_frame(o: Point3, x: Vec3, y: Vec3) -> Option<Frame> {
    Frame::from_zx(o, x.cross(y).normalize()?, x)
}

struct File {
    entries: BTreeMap<usize, Entry>,
    /// Model units to mm.
    scale: f64,
    resolution: f64,
}

fn parse(text: &[u8]) -> Option<File> {
    let (mut global, mut dir, mut param): (String, Vec<&[u8]>, Vec<&[u8]>) =
        (String::new(), Vec::new(), Vec::new());
    for l in text.split(|&c| c == b'\n') {
        match l.get(72) {
            Some(b'G') => global.push_str(std::str::from_utf8(l.get(..72)?).ok()?),
            Some(b'D') => dir.push(l),
            Some(b'P') => param.push(l),
            _ => {}
        }
    }
    let (mut comma, mut semi) = (b',', b';');
    let g = global.as_bytes();
    if g.first() == Some(&b'1') && g.get(1) == Some(&b'H') {
        comma = *g.get(2)?;
    }
    if let Some(i) = global.find("1H")
        && i <= 6
        && g.get(i + 2) == Some(&b';')
    {
        semi = b';';
    }
    let gf = fields(&global, comma, semi);
    let at = |k: usize| gf.get(k - 1).map(String::as_str).unwrap_or("");
    let model = number(at(13)).filter(|&x| x > 0.0).unwrap_or(1.0);
    let unit = match number(at(14))? as i64 {
        1 => 25.4,
        2 => 1.0,
        4 => 304.8,
        5 => 1_609_344.0,
        6 => 1000.0,
        7 => 1e6,
        8 => 0.0254,
        9 => 1e-3,
        10 => 10.0,
        11 => 2.54e-5,
        _ => return None,
    };
    let scale = unit / model;
    let resolution = number(at(19)).unwrap_or(1e-6).abs() * scale;
    let word = |l: &[u8], k: usize| -> Option<usize> {
        let s = std::str::from_utf8(l.get(k * 8..(k + 1) * 8)?).ok()?.trim();
        if s.is_empty() { Some(0) } else { s.parse().ok() }
    };
    let mut entries = BTreeMap::new();
    for (i, pair) in dir.as_chunks::<2>().0.iter().enumerate() {
        let (kind, first, matrix, lines, form) = (
            word(pair[0], 0)?,
            word(pair[0], 1)?,
            word(pair[0], 6)?,
            word(pair[1], 3)?,
            word(pair[1], 4).unwrap_or(0),
        );
        if first == 0 || first > param.len() || lines > param.len() - first + 1 {
            continue;
        }
        let mut run = String::new();
        for l in &param[first - 1..first - 1 + lines] {
            run.push_str(std::str::from_utf8(l.get(..64)?).ok()?);
        }
        let values = fields(&run, comma, semi);
        let params: Option<Vec<f64>> = values.iter().skip(1).map(|x| number(x)).collect();
        // a string among the parameters reads as no number: an entity not read here has its way
        let params = params.unwrap_or_default();
        entries.insert(
            2 * i + 1,
            Entry {
                kind: kind as u32,
                form: form as u32,
                matrix,
                params,
            },
        );
    }
    Some(File {
        entries,
        scale,
        resolution,
    })
}

struct Read<'a> {
    f: &'a File,
    p: Proto,
    made: u64,
    salt: Name,
    verts: BTreeMap<(usize, usize), usize>,
    edges: BTreeMap<(usize, usize), (usize, bool)>,
    stats: Stats,
}

impl Read<'_> {
    fn name(&mut self) -> Name {
        self.made += 1;
        self.salt.with(self.made)
    }

    fn entry(&self, id: usize, kind: &str) -> R<&'_ Entry> {
        match self.f.entries.get(&id) {
            Some(e) => Ok(e),
            None => refuse(id, kind, "no such entity"),
        }
    }

    /// An entity's transformation, matrices chained, with the file's units on the translation.
    fn xf(&self, id: usize, depth: u32) -> R<Xf> {
        let e = self.entry(id, "matrix")?;
        if e.matrix == 0 {
            return Ok(Xf::IDENTITY);
        }
        if depth > 16 {
            return refuse(id, "matrix", "matrices wrapped too deep");
        }
        let m = self.entry(e.matrix, "matrix")?;
        if m.kind != 124 || m.params.len() < 12 {
            return refuse(e.matrix, "matrix", "not a matrix");
        }
        let p = &m.params;
        let s = self.f.scale;
        let x = Xf::new(
            [[p[0], p[1], p[2]], [p[4], p[5], p[6]], [p[8], p[9], p[10]]],
            [p[3] * s, p[7] * s, p[11] * s],
        );
        if !x.no_scale() {
            return refuse(e.matrix, "matrix", "a scale");
        }
        Ok(self.xf(e.matrix, depth + 1)?.then(&x))
    }

    fn point(&self, xf: &Xf, x: f64, y: f64, z: f64) -> Point3 {
        let s = self.f.scale;
        xf.point(Point3::new(x * s, y * s, z * s))
    }

    /// A point entity (116), its own matrix on it.
    fn point_entity(&self, id: usize, xf: &Xf) -> R<Point3> {
        let e = self.entry(id, "point")?;
        if e.kind != 116 || e.params.len() < 3 {
            return refuse(id, "point", "not a point");
        }
        let own = self.xf(id, 0)?;
        let p = &e.params;
        Ok(xf.point(own.point(Point3::new(
            p[0] * self.f.scale,
            p[1] * self.f.scale,
            p[2] * self.f.scale,
        ))))
    }

    /// A direction entity (123).
    fn direction(&self, id: usize, xf: &Xf) -> R<Vec3> {
        let e = self.entry(id, "direction")?;
        if e.kind != 123 || e.params.len() < 3 {
            return refuse(id, "direction", "not a direction");
        }
        let own = self.xf(id, 0)?;
        let p = &e.params;
        Ok(xf.vec(own.vec(Vec3::new(p[0], p[1], p[2]))))
    }

    fn curve(&mut self, id: usize) -> R<Curve> {
        let xf = self.xf(id, 0)?;
        let s = self.f.scale;
        let e = self.entry(id, "curve")?;
        let p = e.params.clone();
        let short = |n: usize| -> R<()> {
            if p.len() < n {
                refuse(id, "curve", "breaks off")
            } else {
                Ok(())
            }
        };
        match e.kind {
            110 => {
                short(6)?;
                let (a, b) = (
                    self.point(&xf, p[0], p[1], p[2]),
                    self.point(&xf, p[3], p[4], p[5]),
                );
                match (b - a).normalize() {
                    Some(dir) => Ok(Curve::Line(Line { origin: a, dir })),
                    None => refuse(id, "line", "no length"),
                }
            }
            100 => {
                short(7)?;
                let centre = self.point(&xf, p[1], p[2], p[0]);
                let (dx, dy) = (p[3] - p[1], p[4] - p[2]);
                let radius = s * (dx * dx + dy * dy).sqrt();
                match plane_frame(centre, xf.vec(Vec3::X), xf.vec(Vec3::Y)) {
                    Some(frame) if radius > 0.0 => Ok(Curve::Circle(Circle { frame, radius })),
                    _ => refuse(id, "circle", "no radius"),
                }
            }
            104 => {
                short(7)?;
                let [a, b, c, d, f_, g] = [p[0], p[1], p[2], p[3], p[4], p[5]];
                // a conic centred on its axes, the only way a writer has of an ellipse
                if e.form != 1 || b != 0.0 || d != 0.0 || f_ != 0.0 || a * g >= 0.0 || c * g >= 0.0 {
                    return refuse(id, "conic", "not an ellipse on its axes");
                }
                let (ra, rb) = ((-g / a).sqrt() * s, (-g / c).sqrt() * s);
                let centre = self.point(&xf, 0.0, 0.0, p[6]);
                let (x, y) = if ra >= rb {
                    (Vec3::X, Vec3::Y)
                } else {
                    (Vec3::Y, -Vec3::X)
                };
                match plane_frame(centre, xf.vec(x), xf.vec(y)) {
                    Some(frame) => Ok(Curve::Ellipse(Ellipse {
                        frame,
                        major: ra.max(rb),
                        minor: ra.min(rb),
                    })),
                    None => refuse(id, "conic", "no frame"),
                }
            }
            126 => {
                short(6)?;
                let (k, m) = (p[0] as usize, p[1] as usize);
                let nk = (k + m + 2).min(p.len());
                let at = 6;
                if p.len() < at + nk + (k + 1) * 4 {
                    return refuse(id, "b-spline curve", "breaks off");
                }
                let knots = &p[at..at + nk];
                let w = &p[at + nk..at + nk + k + 1];
                let c = &p[at + nk + k + 1..];
                let pts: Vec<Point3> = (0..=k)
                    .map(|i| self.point(&xf, c[3 * i], c[3 * i + 1], c[3 * i + 2]))
                    .collect();
                let rational = p[4] == 0.0;
                spline_curve(id, m, knots, &pts, rational.then_some(w))
            }
            k => refuse(id, "curve", format!("entity {k} not read")),
        }
    }

    fn vertex(&mut self, list: usize, k: usize) -> R<usize> {
        if let Some(&v) = self.verts.get(&(list, k)) {
            return Ok(v);
        }
        let xf = self.xf(list, 0)?;
        let e = self.entry(list, "vertex list")?;
        if e.kind != 502 || k == 0 || e.params.len() < 3 * k + 1 {
            return refuse(list, "vertex list", format!("no vertex {k}"));
        }
        let at = 3 * (k - 1) + 1;
        let (x, y, z) = (e.params[at], e.params[at + 1], e.params[at + 2]);
        let q = self.point(&xf, x, y, z);
        let name = self.name();
        self.p.verts.push((q, name));
        self.verts.insert((list, k), self.p.verts.len() - 1);
        Ok(self.p.verts.len() - 1)
    }

    /// Edge `k` of a list, as this reader holds it, and whether its curve runs against the list.
    fn edge(&mut self, list: usize, k: usize) -> R<(usize, bool)> {
        if let Some(&x) = self.edges.get(&(list, k)) {
            return Ok(x);
        }
        let e = self.entry(list, "edge list")?;
        if e.kind != 504 || k == 0 || e.params.len() < 5 * k + 1 {
            return refuse(list, "edge list", format!("no edge {k}"));
        }
        let at = 5 * (k - 1) + 1;
        let w: Vec<usize> = e.params[at..at + 5].iter().map(|&x| x as usize).collect();
        let curve = self.curve(w[0])?;
        let (a, b) = (self.vertex(w[1], w[2])?, self.vertex(w[3], w[4])?);
        let (range, off, against) = edge_range(&curve, self.p.verts[a].0, self.p.verts[b].0, a == b);
        self.stats.vertex_off_curve = self.stats.vertex_off_curve.max(off);
        if off > forge_math::tol::LINEAR {
            self.stats.vertices_off += 1;
        }
        let (s, t) = if against { (b, a) } else { (a, b) };
        let name = self.name();
        self.p.edges.push(PEdge {
            curve,
            range,
            start: s,
            end: t,
            name,
        });
        self.stats.edges += 1;
        let x = (self.p.edges.len() - 1, against);
        self.edges.insert((list, k), x);
        Ok(x)
    }

    fn surface(&mut self, id: usize) -> R<Surface> {
        let xf = self.xf(id, 0)?;
        if xf.mirrors() {
            return refuse(id, "surface", "a mirror");
        }
        let s = self.f.scale;
        let e = self.entry(id, "surface")?;
        let (kind, form) = (e.kind, e.form);
        let p = e.params.clone();
        let short = |n: usize| -> R<()> {
            if p.len() < n {
                refuse(id, "surface", "breaks off")
            } else {
                Ok(())
            }
        };
        // a frame of location and axis, its x from a reference direction if the form has one
        let frame = |r: &Self, loc: usize, axis: usize, refd: Option<usize>| -> R<Frame> {
            let o = r.point_entity(loc, &xf)?;
            let z = r.direction(axis, &xf)?;
            let Some(z) = z.normalize() else {
                return refuse(id, "surface", "no axis");
            };
            match refd {
                Some(d) => {
                    let x = r.direction(d, &xf)?;
                    match Frame::from_zx(o, z, x) {
                        Some(f) => Ok(f),
                        None => refuse(id, "surface", "reference along the axis"),
                    }
                }
                None => Ok(Frame::from_z(o, z)),
            }
        };
        let w = |k: usize| p[k] as usize;
        match kind {
            190 => {
                short(2)?;
                let refd = (form == 1 && p.len() > 2).then(|| w(2));
                // the location and the normal, in that order
                Ok(Surface::Plane(Plane {
                    frame: frame(self, w(0), w(1), refd)?,
                }))
            }
            192 => {
                short(3)?;
                let refd = (form == 1 && p.len() > 3).then(|| w(3));
                Ok(Surface::Cylinder(Cylinder {
                    frame: frame(self, w(0), w(1), refd)?,
                    radius: p[2] * s,
                }))
            }
            194 => {
                short(4)?;
                let refd = (form == 1 && p.len() > 4).then(|| w(4));
                let f = frame(self, w(0), w(1), refd)?;
                match Cone::new(f, p[2] * s, p[3] * dmath::PI / 180.0) {
                    Some(c) => Ok(Surface::Cone(c)),
                    None => refuse(id, "cone", format!("radius {}, angle {}", p[2], p[3])),
                }
            }
            196 => {
                short(2)?;
                let o = self.point_entity(w(0), &xf)?;
                let f = if form == 1 && p.len() > 3 {
                    frame(self, w(0), w(2), Some(w(3)))?
                } else {
                    Frame::from_z(o, forge_math::UnitVec3::Z)
                };
                Ok(Surface::Sphere(Sphere {
                    frame: f,
                    radius: p[1] * s,
                }))
            }
            198 => {
                short(4)?;
                let refd = (form == 1 && p.len() > 4).then(|| w(4));
                let f = frame(self, w(0), w(1), refd)?;
                match Torus::new(f, p[2] * s, p[3] * s) {
                    Some(t) => Ok(Surface::Torus(t)),
                    None => refuse(id, "torus", format!("radii {} and {}", p[2], p[3])),
                }
            }
            128 => {
                short(9)?;
                let (k1, k2, m1, m2) = (w(0), w(1), w(2), w(3));
                let (su, sv) = (k1 + m1 + 2, k2 + m2 + 2);
                let (nu, nv) = (k1 + 1, k2 + 1);
                let Some(n) = nu.checked_mul(nv) else {
                    return refuse(id, "b-spline surface", "too many poles");
                };
                if p.len()
                    < 9usize
                        .saturating_add(su)
                        .saturating_add(sv)
                        .saturating_add(n.saturating_mul(4))
                {
                    return refuse(id, "b-spline surface", "breaks off");
                }
                let u = p[9..9 + su].to_vec();
                let v = p[9 + su..9 + su + sv].to_vec();
                let weights = &p[9 + su + sv..9 + su + sv + n];
                let c = &p[9 + su + sv + n..];
                // the file runs u fastest; ours keeps rows of constant u
                let (mut net, mut ws) = (Vec::with_capacity(n), Vec::with_capacity(n));
                for i in 0..nu {
                    for j in 0..nv {
                        let k = j * nu + i;
                        net.push(self.point(&xf, c[3 * k], c[3 * k + 1], c[3 * k + 2]));
                        ws.push(weights[k]);
                    }
                }
                match Nurbs::new((m1, m2), (u, v), (nu, nv), &net, Some(&ws)) {
                    Some(n) => Ok(n.surface()),
                    None => refuse(id, "b-spline surface", "knots, poles and degrees do not make one"),
                }
            }
            k => refuse(id, "surface", format!("entity {k} not read")),
        }
    }

    fn lp(&mut self, id: usize) -> R<Vec<(usize, bool)>> {
        let e = self.entry(id, "loop")?;
        if e.kind != 508 || e.params.is_empty() {
            return refuse(id, "loop", "not a loop");
        }
        let p = e.params.clone();
        let n = p[0] as usize;
        let (mut at, mut out) = (1, Vec::new());
        for _ in 0..n {
            if p.len() < at + 5 {
                return refuse(id, "loop", "breaks off");
            }
            let (kind, list, k, along, curves) = (
                p[at] as i64,
                p[at + 1] as usize,
                p[at + 2] as usize,
                p[at + 3] != 0.0,
                p[at + 4] as usize,
            );
            at += 5 + 2 * curves;
            // a vertex alone in a loop is a pole or apex: the seams come from healing
            if kind == 1 {
                continue;
            }
            let (e, against) = self.edge(list, k)?;
            out.push((e, along != against));
        }
        Ok(out)
    }

    fn face(&mut self, id: usize, along: bool, shell: usize) -> R<()> {
        let e = self.entry(id, "face")?;
        if e.kind != 510 || e.params.len() < 3 {
            return refuse(id, "face", "not a face");
        }
        let p = e.params.clone();
        let surface = self.surface(p[0] as usize)?;
        let n = p[1] as usize;
        if p.len() < 3 + n {
            return refuse(id, "face", "breaks off");
        }
        let mut loops = Vec::new();
        for &l in &p[3..3 + n] {
            let mut lp = self.lp(l as usize)?;
            // a loop goes round on the surface's own side; a body's, on the face's
            if !along {
                lp.reverse();
                lp.iter_mut().for_each(|u| u.1 = !u.1);
            }
            if !lp.is_empty() {
                loops.push(lp);
            }
        }
        let name = self.name();
        self.p.faces.push(PFace {
            surface,
            same_sense: along,
            name,
            shell,
            loops,
        });
        self.stats.faces += 1;
        Ok(())
    }

    fn shell(&mut self, id: usize, along: bool) -> R<()> {
        let e = self.entry(id, "shell")?;
        if e.kind != 514 || e.params.is_empty() {
            return refuse(id, "shell", "not a shell");
        }
        let p = e.params.clone();
        let n = p[0] as usize;
        if p.len() < 1 + 2 * n {
            return refuse(id, "shell", "breaks off");
        }
        let at = self.p.shells;
        self.p.shells += 1;
        for k in 0..n {
            let (f, o) = (p[1 + 2 * k] as usize, p[2 + 2 * k] != 0.0);
            self.face(f, o == along, at)?;
        }
        Ok(())
    }
}

pub fn import(text: &[u8], lim: &Limits) -> Result<Imported, ImportError> {
    let f = parse(text).ok_or(ImportError::Iges)?;
    let (mut solids, mut refusals) = (Vec::new(), Vec::new());
    let mut stats = Stats::default();
    let ids: Vec<usize> = f
        .entries
        .iter()
        .filter(|x| x.1.kind == 186)
        .map(|x| *x.0)
        .collect();
    for id in &ids {
        let e = &f.entries[id];
        let salt = Name::feature(*id as u64).with_str("iges");
        let mut r = Read {
            f: &f,
            p: Proto {
                salt,
                ..Proto::default()
            },
            made: 0,
            salt,
            verts: BTreeMap::new(),
            edges: BTreeMap::new(),
            stats: Stats::default(),
        };
        let built = (|| {
            let p = &e.params;
            if p.len() < 3 {
                return refuse(*id, "solid", "breaks off");
            }
            r.shell(p[0] as usize, p[1] != 0.0)?;
            let n = p[2] as usize;
            if p.len() < 3 + 2 * n {
                return refuse(*id, "solid", "breaks off");
            }
            for k in 0..n {
                r.shell(p[3 + 2 * k] as usize, p[4 + 2 * k] != 0.0)?;
            }
            Ok(())
        })();
        let Read {
            p: proto, stats: s, ..
        } = r;
        stats.edges += s.edges;
        stats.faces += s.faces;
        stats.vertices_off += s.vertices_off;
        stats.vertex_off_curve = stats.vertex_off_curve.max(s.vertex_off_curve);
        match built {
            Ok(()) => match finish(proto, f.resolution, lim, &mut stats) {
                Ok(body) => solids.push(Solid {
                    entity: *id as u32,
                    name: String::new(),
                    body,
                }),
                Err(why) => refusals.push(Refusal {
                    entity: *id as u32,
                    kind: "solid".into(),
                    why,
                }),
            },
            Err(x) => refusals.push(x),
        }
    }
    if ids.is_empty() {
        let trimmed = f.entries.iter().find(|x| matches!(x.1.kind, 144 | 143));
        if let Some((id, e)) = trimmed {
            refusals.push(Refusal {
                entity: *id as u32,
                kind: format!("entity {}", e.kind),
                why: "trimmed surfaces with no topology: a solid would have to be sewn from them, which is not done".into(),
            });
        }
    }
    if solids.is_empty() && refusals.is_empty() {
        return Err(ImportError::NoSolids);
    }
    Ok(Imported {
        solids,
        refusals,
        schema: "IGES".into(),
        system: String::new(),
        stats,
    })
}

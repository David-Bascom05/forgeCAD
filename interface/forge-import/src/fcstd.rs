//! FreeCAD `.FCStd` PartDesign bodies (and the Part workbench solids they build on) as `.fc`
//! histories. Document.xml is read as XML and each object's stored shape as an OpenCascade
//! `.brep`: a fillet's `EdgeN` is sampled from its base's stored shape and found on our part by
//! position. Macros and scripts in the file are never touched.

use crate::arrangement::{Arrangement, Ent, Kind, Shape, circle_from};
use crate::py::{self, V2, V3, add, dot, neg, num, pmax, pmin, vfine, vtext};
use crate::state::{Edge, State};
use crate::zip::Zip;
use crate::{Body, Fail, bug};
use roxmltree::Node;
use std::collections::{BTreeMap, HashMap, HashSet};

/// A stored edge's samples lie this close to ours (mm).
const TOL: f64 = 2e-3;

const SOLID: &[&str] = &[
    "PartDesign::Pad",
    "PartDesign::Pocket",
    "PartDesign::Revolution",
    "PartDesign::Groove",
    "PartDesign::Fillet",
    "PartDesign::Chamfer",
    "PartDesign::Mirrored",
    "PartDesign::PolarPattern",
    "PartDesign::LinearPattern",
    "PartDesign::Hole",
    "PartDesign::Loft",
    "PartDesign::Pipe",
    "PartDesign::AdditiveLoft",
    "PartDesign::AdditivePipe",
    "PartDesign::SubtractiveLoft",
    "PartDesign::SubtractivePipe",
    "PartDesign::Draft",
    "PartDesign::Thickness",
    "PartDesign::MultiTransform",
    "PartDesign::Scaled",
    "PartDesign::AdditiveBox",
    "PartDesign::AdditiveCylinder",
    "PartDesign::Boolean",
    "PartDesign::Helix",
    "PartDesign::AdditiveHelix",
    "PartDesign::SubtractiveHelix",
    "PartDesign::FeatureBase",
    "PartDesign::SubtractiveBox",
    "PartDesign::SubtractiveCylinder",
    "PartDesign::AdditiveSphere",
    "PartDesign::SubtractiveSphere",
    "PartDesign::AdditiveCone",
    "PartDesign::SubtractiveCone",
    "PartDesign::AdditiveTorus",
    "PartDesign::SubtractiveTorus",
    "PartDesign::AdditivePrism",
    "PartDesign::SubtractivePrism",
    "PartDesign::AdditiveWedge",
    "PartDesign::SubtractiveWedge",
    "PartDesign::AdditiveEllipsoid",
    "PartDesign::SubtractiveEllipsoid",
    "Part::Revolution",
    "Part::Box",
    "Part::Cylinder",
    "Part::Cut",
    "Part::Fuse",
    "Part::Common",
    "Part::MultiFuse",
    "Part::MultiCommon",
    "Part::Sphere",
];
const TRANSFORMS: &[&str] = &[
    "PartDesign::PolarPattern",
    "PartDesign::LinearPattern",
    "PartDesign::Mirrored",
];
/// What a pad's profile can be; only a sketch has rows, the rest a stored shape.
const FLAT: &[&str] = &["Sketcher::SketchObject", "Part::Part2DObjectPython"];
const NEEDS: &[&str] = &[
    "PartDesign::FeatureBase",
    "Part::Cut",
    "Part::Fuse",
    "Part::Common",
    "Part::MultiFuse",
    "Part::MultiCommon",
    "Part::Sphere",
    "PartDesign::Hole",
    "PartDesign::MultiTransform",
    "PartDesign::AdditiveBox",
    "PartDesign::SubtractiveBox",
    "PartDesign::AdditiveCylinder",
    "PartDesign::SubtractiveCylinder",
    "Part::Revolution",
    "Part::Box",
    "Part::Cylinder",
    "PartDesign::Pad",
    "PartDesign::Pocket",
    "PartDesign::Revolution",
    "PartDesign::Groove",
    "PartDesign::Fillet",
    "PartDesign::Chamfer",
    "PartDesign::Mirrored",
    "PartDesign::PolarPattern",
    "PartDesign::LinearPattern",
];

fn is(set: &[&str], t: Option<&str>) -> bool {
    t.is_some_and(|t| set.contains(&t))
}

fn last_word(t: &str) -> &str {
    t.rsplit("::").next().unwrap_or(t)
}

fn kid<'a, 'i>(n: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == tag)
}

fn kids<'a, 'i>(n: Node<'a, 'i>, tag: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == tag)
}

fn float(s: Option<&str>) -> Result<f64, Fail> {
    match s.map(|s| s.trim().parse::<f64>()) {
        Some(Ok(x)) => Ok(x),
        _ => bug("a number that would not read"),
    }
}

fn int(s: Option<&str>) -> Result<i64, Fail> {
    match s.map(|s| s.trim().parse::<i64>()) {
        Some(Ok(x)) => Ok(x),
        _ => bug("an integer that would not read"),
    }
}

fn vector(e: Node) -> Result<V3, Fail> {
    Ok((
        float(e.attribute("valueX"))?,
        float(e.attribute("valueY"))?,
        float(e.attribute("valueZ"))?,
    ))
}

struct Doc<'a, 'i> {
    zip: &'a Zip<'a>,
    /// Object names in document order, and their types.
    names: Vec<String>,
    types: HashMap<String, String>,
    props: HashMap<String, HashMap<String, Node<'a, 'i>>>,
    /// Each stored shape's volume, by file, for files without bodies.
    volumes: Option<HashMap<String, Option<f64>>>,
    bodied: std::cell::OnceCell<HashSet<String>>,
}

type Placement = (V3, (f64, f64, f64, f64));

impl<'a, 'i> Doc<'a, 'i> {
    fn ty(&self, n: &str) -> Option<&str> {
        self.types.get(n).map(String::as_str)
    }

    fn prop(&self, obj: &str, name: &str) -> Result<Option<Node<'a, 'i>>, Fail> {
        match self.props.get(obj) {
            Some(p) => Ok(p.get(name).copied()),
            None => bug("an object with no data"),
        }
    }

    fn flt(&self, obj: &str, name: &str, d: f64) -> Result<f64, Fail> {
        match self.prop(obj, name)?.and_then(|p| kid(p, "Float")) {
            Some(e) => float(e.attribute("value")),
            None => Ok(d),
        }
    }

    fn boolean(&self, obj: &str, name: &str) -> Result<bool, Fail> {
        let p = self.prop(obj, name)?;
        Ok(p.and_then(|p| kid(p, "Bool"))
            .is_some_and(|b| b.attribute("value") == Some("true")))
    }

    fn integer(&self, obj: &str, name: &str, d: i64) -> Result<i64, Fail> {
        match self.prop(obj, name)?.and_then(|p| kid(p, "Integer")) {
            Some(e) => int(e.attribute("value")),
            None => Ok(d),
        }
    }

    fn link(&self, obj: &str, name: &str) -> Result<String, Fail> {
        let mut p = self.prop(obj, name)?;
        if p.is_none() && name == "Profile" {
            // older files
            p = self.prop(obj, "Sketch")?;
        }
        let Some(p) = p else { return Ok(String::new()) };
        let e = kid(p, "Link").or_else(|| kid(p, "LinkSub"));
        Ok(e.and_then(|e| e.attribute("value")).unwrap_or("").into())
    }

    fn subs(&self, obj: &str, name: &str) -> Result<Vec<String>, Fail> {
        let Some(e) = self.prop(obj, name)?.and_then(|p| kid(p, "LinkSub")) else {
            return Ok(vec![]);
        };
        kids(e, "Sub")
            .map(|s| {
                s.attribute("value")
                    .map(String::from)
                    .ok_or(Fail::Bug("a sub with no value".into()))
            })
            .collect()
    }

    fn links(&self, obj: &str, name: &str) -> Result<Vec<String>, Fail> {
        let Some(e) = self.prop(obj, name)?.and_then(|p| kid(p, "LinkList")) else {
            return Ok(vec![]);
        };
        Ok(kids(e, "Link")
            .map(|s| s.attribute("value").unwrap_or("").into())
            .collect())
    }

    fn placement(&self, obj: &str) -> Result<Placement, Fail> {
        let Some(e) = self
            .prop(obj, "Placement")?
            .and_then(|p| kid(p, "PropertyPlacement"))
        else {
            return Ok(((0.0, 0.0, 0.0), (0.0, 0.0, 0.0, 1.0)));
        };
        let f = |k| float(e.attribute(k));
        Ok((
            (f("Px")?, f("Py")?, f("Pz")?),
            (f("Q0")?, f("Q1")?, f("Q2")?, f("Q3")?),
        ))
    }

    fn shape_file(&self, obj: &str) -> Result<Option<String>, Fail> {
        let p = self.prop(obj, "Shape")?.and_then(|p| kid(p, "Part"));
        Ok(p.and_then(|e| e.attribute("file")).map(String::from))
    }

    fn vec_prop(&self, obj: &str, name: &str) -> Result<V3, Fail> {
        match self.prop(obj, name)?.and_then(|p| kid(p, "PropertyVector")) {
            Some(e) => vector(e),
            None => bug("a vector property missing"),
        }
    }

    fn role(&self, obj: &str) -> Result<Option<String>, Fail> {
        self.prop(obj, "Role")?
            .map(|r| {
                kid(r, "String")
                    .and_then(|s| s.attribute("value"))
                    .map(String::from)
            })
            .map(|s| s.ok_or(Fail::Bug("a role with no string".into())))
            .transpose()
    }
}

fn rotate(q: (f64, f64, f64, f64), v: V3) -> V3 {
    let (x, y, z, w) = q;
    let t = (
        2.0 * (y * v.2 - z * v.1),
        2.0 * (z * v.0 - x * v.2),
        2.0 * (x * v.1 - y * v.0),
    );
    (
        v.0 + w * t.0 + y * t.2 - z * t.1,
        v.1 + w * t.1 + z * t.0 - x * t.2,
        v.2 + w * t.2 + x * t.1 - y * t.0,
    )
}

#[derive(Clone, Copy)]
struct Frame {
    o: V3,
    x: V3,
    y: V3,
    n: V3,
}

impl Frame {
    fn new(pl: Placement) -> Frame {
        let (o, q) = pl;
        Frame {
            o,
            x: rotate(q, (1.0, 0.0, 0.0)),
            y: rotate(q, (0.0, 1.0, 0.0)),
            n: rotate(q, (0.0, 0.0, 1.0)),
        }
    }

    fn uvw(&self, p: V3) -> V3 {
        let d = add(p, self.o, -1.0);
        (dot(d, self.x), dot(d, self.y), dot(d, self.n))
    }

    fn world(&self, u: f64, v: f64) -> V3 {
        add(add(add(self.o, self.x, u), self.y, v), self.n, 0.0)
    }
}

fn construction(g: Node) -> bool {
    for k in ["Construction", "Construct"] {
        if kid(g, k).is_some_and(|c| c.attribute("value") == Some("1")) {
            return true;
        }
    }
    if let Some(ext) = kid(g, "GeoExtensions") {
        for x in ext.children().filter(|c| c.is_element()) {
            if let Some(f) = x.attribute("geometryModeFlags") {
                let b = f.as_bytes();
                if b.len() > 1 && b[b.len() - 2] == b'1' {
                    return true;
                }
            }
        }
    }
    false
}

fn sketch_ents(doc: &Doc, name: &str) -> Result<Vec<Ent>, Fail> {
    let Some(p) = doc.prop(name, "Geometry")? else {
        return Err(Fail::Unsupported("a sketch without geometry".into()));
    };
    let Some(list) = kid(p, "GeometryList") else {
        return bug("a sketch's geometry without its list");
    };
    let mut ents = vec![];
    for (k, g) in kids(list, "Geometry").enumerate() {
        let t = g.attribute("type");
        let con = construction(g);
        let el = |tag| kid(g, tag).ok_or(Fail::Bug("geometry without its element".into()));
        let id = format!("e{k}");
        let shape = match t {
            Some("Part::GeomLineSegment") => {
                let e = el("LineSegment")?;
                let f = |k| float(e.attribute(k));
                Shape::Line {
                    start: (f("StartX")?, f("StartY")?),
                    end: (f("EndX")?, f("EndY")?),
                }
            }
            Some("Part::GeomCircle") => {
                let e = el("Circle")?;
                let f = |k| float(e.attribute(k));
                Shape::Circle {
                    center: (f("CenterX")?, f("CenterY")?),
                    radius: f("Radius")?,
                }
            }
            Some("Part::GeomArcOfCircle") => {
                let e = el("ArcOfCircle")?;
                let f = |k| float(e.attribute(k));
                let (c, r) = ((f("CenterX")?, f("CenterY")?), f("Radius")?);
                let nz = float(Some(e.attribute("NormalZ").unwrap_or("1")))?;
                let xu = float(Some(e.attribute("AngleXU").unwrap_or("0")))?;
                let (a0, mut a1) = (f("StartAngle")?, f("EndAngle")?);
                while a1 < a0 {
                    a1 += py::TAU;
                }
                let pt = |a: f64| (c.0 + r * py::cos(xu + nz * a), c.1 + r * py::sin(xu + nz * a));
                Shape::Arc {
                    start: pt(a0),
                    mid: pt((a0 + a1) / 2.0),
                    end: pt(a1),
                }
            }
            Some("Part::GeomBSplineCurve") => {
                let e = el("BSplineCurve")?;
                if e.attribute("IsPeriodic") == Some("1") {
                    return Err(Fail::Unsupported("a periodic spline".into()));
                }
                Shape::Poly {
                    points: bspline_points(e)?,
                }
            }
            Some("Part::GeomPoint") => continue,
            _ if con => continue,
            Some(t) => return Err(Fail::Unsupported(format!("sketch curve {}", last_word(t)))),
            None => return bug("geometry of no type"),
        };
        ents.push(Ent {
            id,
            shape,
            construction: con,
        });
    }
    Ok(ents)
}

/// A fine polyline along a non-periodic NURBS: the part is then marked approx.
fn bspline_points(e: Node) -> Result<Vec<V2>, Fail> {
    let deg = int(e.attribute("Degree"))?;
    let deg = usize::try_from(deg).or(bug("a negative degree"))?;
    let mut cp = vec![];
    for p in kids(e, "Pole") {
        cp.push((
            float(p.attribute("X"))?,
            float(p.attribute("Y"))?,
            float(Some(p.attribute("Weight").unwrap_or("1")))?,
        ));
    }
    let mut k = vec![];
    for x in kids(e, "Knot") {
        let v = float(x.attribute("Value"))?;
        let m = int(x.attribute("Mult"))?;
        if m > 0 {
            k.extend(std::iter::repeat_n(v, m as usize));
        }
    }
    if cp.len() <= deg || k.len() < cp.len() + deg + 1 || cp.len() > 100_000 {
        return bug("a spline's knots and poles do not add up");
    }
    let at = |u: f64| -> Result<V2, Fail> {
        let mut s = deg;
        while s < cp.len() - 1 && k[s + 1] <= u {
            s += 1;
        }
        let mut d: Vec<V3> = (0..=deg)
            .map(|j| {
                let c = cp[s - deg + j];
                (c.0 * c.2, c.1 * c.2, c.2)
            })
            .collect();
        for r in 1..=deg {
            for j in (r..=deg).rev() {
                let i = s - deg + j;
                let den = k[i + deg - r + 1] - k[i];
                let a = if den != 0.0 { (u - k[i]) / den } else { 0.0 };
                let (p, q) = (d[j - 1], d[j]);
                d[j] = (
                    p.0 * (1.0 - a) + q.0 * a,
                    p.1 * (1.0 - a) + q.1 * a,
                    p.2 * (1.0 - a) + q.2 * a,
                );
            }
        }
        if d[deg].2 == 0.0 {
            return bug("a spline weight of zero");
        }
        Ok((d[deg].0 / d[deg].2, d[deg].1 / d[deg].2))
    };
    let (lo, hi) = (k[deg], k[cp.len()]);
    let mut spans: Vec<f64> = vec![];
    for &x in &k {
        if lo <= x && x <= hi && !spans.contains(&x) {
            spans.push(x);
        }
    }
    spans.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![];
    for w in spans.windows(2) {
        let (a, b) = (w[0], w[1]);
        for i in 0..16 {
            out.push(at(a + (b - a) * i as f64 / 16.0)?);
        }
    }
    out.push(at(hi)?);
    Ok(out)
}

fn ends(e: &Ent) -> Option<(V2, V2)> {
    match &e.shape {
        Shape::Line { start, end } | Shape::Arc { start, end, .. } => Some((*start, *end)),
        Shape::Poly { points } => Some((points[0], points[points.len() - 1])),
        Shape::Circle { .. } => None,
    }
}

/// FreeCAD's Sketch::toShape: from the first edge left, pull in the first edge in document order
/// that meets the wire's last or first vertex, until nothing meets or the wire closes. So at a
/// junction the order decides, and a wire can end open; vertices meet within
/// Precision::Confusion. The closed wires, as indices into `ents`.
fn closed_wires(ents: &[Ent]) -> Vec<Vec<usize>> {
    let same = |p: V2, q: V2| py::dist2(p, q) <= 1e-7;
    let mut left = std::collections::VecDeque::new();
    let mut keep = vec![];
    for (i, e) in ents.iter().enumerate() {
        if e.construction {
            continue;
        }
        match ends(e) {
            None => keep.push(vec![i]),
            Some((a, b)) => left.push_back((i, a, b)),
        }
    }
    while let Some((i, mut first, mut last)) = left.pop_front() {
        let mut wire = vec![i];
        while !left.is_empty() && !same(first, last) {
            let mut hit = None;
            for (k, &(j, a, b)) in left.iter().enumerate() {
                if same(last, a) {
                    last = b;
                } else if same(first, b) {
                    first = a;
                } else if same(last, b) {
                    last = a;
                } else if same(first, a) {
                    first = b;
                } else {
                    continue;
                }
                hit = Some((k, j));
                break;
            }
            let Some((k, j)) = hit else { break };
            wire.push(j);
            left.remove(k);
        }
        if same(first, last) {
            keep.push(wire);
        }
    }
    keep
}

/// Regions at even nesting are solid, odd ones holes: FreeCAD's bullseye where nothing touches.
fn even_regions(a: &Arrangement) -> Vec<usize> {
    let mut out = vec![];
    for (i, f) in a.faces.iter().enumerate() {
        let pt = a.point_within(&a.loops[f.outer]);
        let depth = a
            .faces
            .iter()
            .enumerate()
            .filter(|(j, g)| *j != i && g.area > f.area && a.inside(pt, &a.loops[g.outer]))
            .count();
        if depth % 2 == 0 {
            out.push(i);
        }
    }
    out
}

/// A point inside region i, beside its outer boundary, clear of its holes.
fn inner_point(a: &Arrangement, i: usize) -> V2 {
    let lp = &a.loops[a.faces[i].outer];
    for &(pc, fwd) in lp {
        let pts = a.pieces[pc].sample(8);
        let k = pts.len() / 2;
        let (u, v) = (pts[k - 1], pts[k]);
        let d = py::hypot(&[v.0 - u.0, v.1 - u.1]);
        if d < 1e-9 {
            continue;
        }
        let s = if fwd { 1.0 } else { -1.0 };
        let n = (-(v.1 - u.1) / d * s, (v.0 - u.0) / d * s);
        let m = ((u.0 + v.0) / 2.0, (u.1 + v.1) / 2.0);
        for w in [1e-3, 1e-4, 1e-5] {
            let q = (m.0 + n.0 * w, m.1 + n.1 * w);
            if a.holds(i, q) {
                return q;
            }
        }
    }
    a.point_within(lp)
}

/// FreeCAD's bullseye, which nests wires, not regions: a region inside an odd number of wires is
/// solid. Wires that touch cut the arrangement's regions apart without changing that.
fn wire_regions(a: &Arrangement, ents: &[Ent], wires: &[Vec<usize>]) -> Result<Vec<usize>, Fail> {
    let mut outlines = vec![];
    for w in wires {
        let sub: Vec<Ent> = w.iter().map(|&i| ents[i].clone()).collect();
        let g = Arrangement::new(&sub)?;
        let mut best: Option<usize> = None;
        for (k, f) in g.faces.iter().enumerate() {
            if best.is_none_or(|b| f.area > g.faces[b].area) {
                best = Some(k);
            }
        }
        if let Some(b) = best {
            let lp = g.loops[g.faces[b].outer].clone();
            outlines.push((g, lp));
        }
    }
    Ok((0..a.faces.len())
        .filter(|&i| {
            let q = inner_point(a, i);
            outlines.iter().filter(|(g, lp)| g.inside(q, lp)).count() % 2 == 1
        })
        .collect())
}

/// cadfs.loop_text: a loop of the arrangement as .fc loop text, y flipped if asked.
fn loop_text(a: &Arrangement, lp: &[(usize, bool)], flip: bool) -> String {
    let f = |p: V2| if flip { (p.0, -p.1) } else { p };
    if lp.len() == 1 && a.pieces[lp[0].0].kind == Kind::Circle {
        let p = &a.pieces[lp[0].0];
        let c = f(p.c);
        return format!("circle {} {} {}", num(c.0), num(c.1), num(p.r));
    }
    let mut parts = vec![];
    for &(i, fwd) in lp {
        let p = &a.pieces[i];
        let (mut s, mut e) = (a.verts[p.va], a.verts[p.vb]);
        if !fwd {
            (s, e) = (e, s);
        }
        let (s, e) = (f(s), f(e));
        match p.kind {
            Kind::Arc => {
                let (_, d) = p.sweep();
                let d = (if fwd { d } else { -d }) * if flip { -1.0 } else { 1.0 };
                let mut c = f(p.c);
                // refit through the shared ends
                let ch = py::dist2(s, e);
                let mut r = p.r;
                if ch > 1e-9 {
                    r = pmax(r, ch / 2.0);
                    let h = pmax(r * r - ch * ch / 4.0, 0.0).sqrt();
                    let m = ((s.0 + e.0) / 2.0, (s.1 + e.1) / 2.0);
                    let n = ((e.1 - s.1) / ch, (s.0 - e.0) / ch);
                    let (c1, c2) = ((m.0 + h * n.0, m.1 + h * n.1), (m.0 - h * n.0, m.1 - h * n.1));
                    c = if py::dist2(c1, c) <= py::dist2(c2, c) {
                        c1
                    } else {
                        c2
                    };
                }
                let a0 = py::degrees(py::atan2(s.1 - c.1, s.0 - c.0));
                let a1 = py::degrees(py::atan2(e.1 - c.1, e.0 - c.0));
                let mut dg = a1 - a0;
                while d > 0.0 && dg <= 0.0 {
                    dg += 360.0;
                }
                while d < 0.0 && dg >= 0.0 {
                    dg -= 360.0;
                }
                parts.push(format!(
                    "arc {} {} {} {} {}",
                    num(c.0),
                    num(c.1),
                    num(r),
                    py::f10(a0),
                    py::f10(dg)
                ));
            }
            Kind::Poly => {
                let mut ps: Vec<V2> = p.pts.iter().map(|&q| f(q)).collect();
                if !fwd {
                    ps.reverse();
                }
                let n = ps.len();
                ps[0] = s;
                ps[n - 1] = e;
                for w in ps.windows(2) {
                    parts.push(format!(
                        "line {} {} {} {}",
                        num(w[0].0),
                        num(w[0].1),
                        num(w[1].0),
                        num(w[1].1)
                    ));
                }
            }
            _ => parts.push(format!(
                "line {} {} {} {}",
                num(s.0),
                num(s.1),
                num(e.0),
                num(e.1)
            )),
        }
    }
    format!("loop {}", parts.join(" "))
}

/// A stored edge: its curve's kind and samples.
type Sampled = (&'static str, Vec<V3>);

/// What a stored shape gives the translation.
#[derive(Default)]
struct Stored {
    file: String,
    /// Samples of the edges each sub names: `Edge3`, `Face2` (its edges), `ALL`.
    subs: HashMap<String, Vec<Vec<V3>>>,
    /// A profile's wires: each edge's kind and 97 samples.
    wires: Option<Vec<Vec<Sampled>>>,
    readable: bool,
}

fn p3(p: forge_math::Point3) -> V3 {
    (p.x, p.y, p.z)
}

/// fcstd_truth.py's samples of the stored shapes, or None where it would have stopped.
fn stored(
    zip: &Zip,
    shapes: &[(String, Option<String>)],
    subs: &BTreeMap<String, Vec<String>>,
    wires: &[String],
) -> Option<HashMap<String, Stored>> {
    let mut out = HashMap::new();
    for (name, file) in shapes {
        let file = file.as_ref()?;
        let data = zip.read(file)?;
        let mut r = Stored {
            file: file.clone(),
            ..Stored::default()
        };
        let Some(t) = forge_step::BrepTopo::new(&data) else {
            out.insert(name.clone(), r);
            continue;
        };
        r.readable = true;
        if wires.contains(name) {
            let mut ws = vec![];
            for w in t.wires()? {
                let mut es = vec![];
                for e in w {
                    es.push((t.kind(e), t.sample(e, 96)?.into_iter().map(p3).collect()));
                }
                ws.push(es);
            }
            r.wires = Some(ws);
        }
        if let Some(ss) = subs.get(name).filter(|s| !s.is_empty()) {
            let em = t.edges()?;
            let fm = t.faces()?;
            let pick = |m: &[forge_step::Used], s: &str| -> Option<forge_step::Used> {
                let i: i64 = s.parse().ok()?;
                (i >= 1 && i as usize <= m.len()).then(|| m[i as usize - 1])
            };
            let samples = |es: &[forge_step::Used]| -> Option<Vec<Vec<V3>>> {
                es.iter()
                    .map(|&e| Some(t.sample(e, 256)?.into_iter().map(p3).collect()))
                    .collect()
            };
            for s in ss {
                if s == "ALL" {
                    r.subs.insert(s.clone(), samples(&em)?);
                } else if let Some(n) = s.strip_prefix("Edge") {
                    r.subs.insert(s.clone(), samples(&[pick(&em, n)?])?);
                } else if let Some(n) = s.strip_prefix("Face") {
                    let fe = t.edges_of(pick(&fm, n)?)?;
                    r.subs.insert(s.clone(), samples(&fe)?);
                }
            }
        }
        out.insert(name.clone(), r);
    }
    Some(out)
}

/// A stored shape's solids: their volume, summed, and box; None if it would not read.
fn solids_of(data: &[u8]) -> Option<(f64, Option<[f64; 6]>)> {
    let imp = match forge_step::import(data, &forge_step::Limits::default()) {
        Ok(i) => i,
        Err(forge_step::ImportError::NoSolids) => return Some((0.0, None)),
        Err(_) => return None,
    };
    let (mut vol, mut bb): (f64, Option<[f64; 6]>) = (0.0, None);
    for s in &imp.solids {
        vol += forge_topo::mass::mass(&s.body).ok()?.volume.abs();
        let b = forge_bool::Index::new(&s.body).bbox;
        let x = [b.x.lo, b.y.lo, b.z.lo, b.x.hi, b.y.hi, b.z.hi];
        bb = Some(match bb {
            None => x,
            Some(o) => [
                pmin(o[0], x[0]),
                pmin(o[1], x[1]),
                pmin(o[2], x[2]),
                pmax(o[3], x[3]),
                pmax(o[4], x[4]),
                pmax(o[5], x[5]),
            ],
        });
    }
    Some((vol, bb))
}

/// Do these samples lie on one line or one circle?
fn line_or_circle(poly: &[V3]) -> bool {
    let n = poly.len();
    let (a, m, b) = (poly[0], poly[n / 3], poly[2 * n / 3]);
    let (u, v) = (add(m, a, -1.0), add(b, a, -1.0));
    let cr = py::cross(u, v);
    let nn = dot(cr, cr);
    if nn < 1e-18 {
        return poly.iter().all(|&q| {
            let w = add(q, a, -1.0);
            let x = dot(w, v);
            pmax(dot(w, w) - x * x / pmax(dot(v, v), 1e-18), 0.0).sqrt() < 1e-6
        });
    }
    let s1 = py::cross(v, cr);
    let s2 = py::cross(cr, u);
    let (du, dv) = (dot(u, u), dot(v, v));
    let sum = add(
        (s1.0 * du, s1.1 * du, s1.2 * du),
        (s2.0 * dv, s2.1 * dv, s2.2 * dv),
        1.0,
    );
    let c = add(a, sum, 1.0 / (2.0 * nn));
    let r = py::dist3(a, c);
    poly.iter().all(|&q| (py::dist3(q, c) - r).abs() < 1e-6)
}

/// Grows the picked edges along edges tangent where they meet.
fn along_tangents(edges: &[Edge], picked: &mut BTreeMap<String, usize>) {
    let mut at: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
    for (i, e) in edges.iter().enumerate() {
        for (k, &v) in e.v.iter().enumerate() {
            at.entry(v).or_default().push((i, k));
        }
    }
    let mut todo: Vec<usize> = picked.values().copied().collect();
    while let Some(i) = todo.pop() {
        let e = &edges[i];
        for (k, v) in e.v.iter().enumerate() {
            let Some(t) = e.t[k] else { continue };
            for &(g, j) in &at[v] {
                let ge = &edges[g];
                if picked.contains_key(&ge.h) || ge.c == "smooth" {
                    continue;
                }
                let Some(b) = ge.t[j] else { continue };
                let cr = py::cross(t, b);
                if dot(cr, cr).sqrt() < 1e-4 {
                    picked.insert(ge.h.clone(), g);
                    todo.push(g);
                }
            }
        }
    }
}

/// How far a sampled curve's chords stray from it: half the bow of two steps.
fn sag(poly: &[V3]) -> f64 {
    let i = poly.len() / 2;
    if i == 0 || i + 1 >= poly.len() {
        return 0.0;
    }
    near(poly[i], &[poly[i - 1], poly[i + 1]], 0.0) / 2.0
}

fn near(p: V3, poly: &[V3], enough: f64) -> f64 {
    let mut best = f64::INFINITY;
    for w in poly.windows(2) {
        let (a, b) = (w[0], w[1]);
        let e = add(b, a, -1.0);
        let l2 = dot(e, e);
        let t = if l2 == 0.0 {
            0.0
        } else {
            pmax(0.0, pmin(1.0, dot(add(p, a, -1.0), e) / l2))
        };
        let q = add(a, e, t);
        best = pmin(best, py::dist3(p, q));
        if best <= enough {
            return best;
        }
    }
    best
}

fn doc_approx(d: &Doc, name: &str) -> Result<bool, Fail> {
    let sk = d.link(name, "Profile")?;
    let p = if d.props.contains_key(&sk) {
        d.prop(&sk, "Geometry")?
    } else {
        None
    };
    let Some(p) = p else { return Ok(false) };
    let Some(list) = kid(p, "GeometryList") else {
        return bug("a sketch's geometry without its list");
    };
    Ok(kids(list, "Geometry").any(|g| g.attribute("type") == Some("Part::GeomBSplineCurve")))
}

/// A feature of an older file whose stored result is just its own tool has no base.
fn alone(doc: &Doc, n: &str) -> Result<bool, Fail> {
    let mut p = doc.prop(n, "AddSubShape")?;
    if p.is_none() {
        p = doc.prop(n, "AddShape")?;
    }
    let a = p
        .and_then(|p| kid(p, "Part"))
        .and_then(|e| e.attribute("file"))
        .unwrap_or("");
    let b = doc.shape_file(n)?.unwrap_or_default();
    let Some(vols) = &doc.volumes else {
        return Ok(false);
    };
    if a.is_empty() || b.is_empty() {
        return Ok(false);
    }
    let (va, vb) = (vols.get(a).copied().flatten(), vols.get(&b).copied().flatten());
    Ok(matches!((va, vb), (Some(va), Some(vb)) if (va - vb).abs() <= 1e-6 * pmax(vb, 1e-12)))
}

/// What a sketch is attached to, across the file versions' three forms.
fn support_of(doc: &Doc, sk: &str) -> Result<String, Fail> {
    if let Some(e) = doc.prop(sk, "Support")?.and_then(|p| kid(p, "LinkSubList")) {
        return Ok(kids(e, "Link")
            .next()
            .and_then(|l| l.attribute("obj"))
            .unwrap_or("")
            .into());
    }
    doc.link(sk, "Support")
}

const NOT_A_BASE: &[&str] = &[
    "App::Plane",
    "App::Origin",
    "PartDesign::Plane",
    "App::Line",
    "PartDesign::Line",
    "App::Part",
    "PartDesign::Body",
    "PartDesign::CoordinateSystem",
];

/// The feature before n: older files without BaseFeature say it through a blend's Base, a
/// pattern's original, or the solid its sketch sits on.
fn base_of(doc: &Doc, n: &str) -> Result<String, Fail> {
    let b = doc.link(n, "BaseFeature")?;
    if !b.is_empty() {
        return Ok(b);
    }
    if doc.prop(n, "BaseFeature")?.is_some() {
        // empty inside a body means first. Outside one (0.18 to 0.21 files without bodies) it says
        // nothing: the sketch's support is the base
        if doc.bodied.get().is_none() {
            let mut s = HashSet::new();
            for k in &doc.names {
                if doc.ty(k) == Some("PartDesign::Body") {
                    s.extend(doc.links(k, "Group")?);
                }
            }
            let _ = doc.bodied.set(s);
        }
        let prof = doc.link(n, "Profile")?;
        let s = if doc.props.contains_key(&prof) {
            support_of(doc, &prof)?
        } else {
            String::new()
        };
        let bodied = doc.bodied.get().is_some_and(|b| b.contains(n));
        return Ok(if !bodied && is(SOLID, doc.ty(&s)) {
            s
        } else {
            String::new()
        });
    }
    let b = doc.link(n, "Base")?;
    if is(SOLID, doc.ty(&b)) {
        return Ok(b);
    }
    if let Some(o) = doc.links(n, "Originals")?.pop() {
        return Ok(o);
    }
    let sk = doc.link(n, "Profile")?;
    if doc.props.contains_key(&sk) {
        let s = support_of(doc, &sk)?;
        // a sketch on a solid the importer has no form for says so here, not as a cut with nothing to cut
        if let Some(t) = doc.ty(&s)
            && !NOT_A_BASE.contains(&t)
            && !t.starts_with("Sketcher")
        {
            return Ok(s);
        }
    }
    // no link at all: these files fused each feature onto the one before it in the document
    if alone(doc, n)? {
        return Ok(String::new());
    }
    let at = doc.names.iter().position(|x| x == n).unwrap_or(0);
    for b in doc.names[..at].iter().rev() {
        // a Part solid just before it says nothing: it was as likely a tool of some boolean
        let t = doc.ty(b).unwrap_or("");
        if SOLID.contains(&t) && !t.starts_with("Part::") {
            return Ok(b.clone());
        }
    }
    Ok(String::new())
}

/// A chain's features and, through any Part boolean under it, every chain those are made of.
fn tree_members(doc: &Doc, chain: &[String]) -> Result<Vec<String>, Fail> {
    let (mut out, mut todo): (Vec<String>, Vec<String>) = (vec![], chain.to_vec());
    while let Some(c) = todo.pop() {
        if out.contains(&c) {
            continue;
        }
        out.push(c.clone());
        let t = doc.ty(&c).unwrap_or("");
        let ks = match t {
            "Part::MultiFuse" | "Part::MultiCommon" => doc.links(&c, "Shapes")?,
            "Part::Cut" | "Part::Fuse" | "Part::Common" => vec![doc.link(&c, "Base")?, doc.link(&c, "Tool")?],
            _ => vec![],
        };
        for k in ks {
            if doc.ty(&k).unwrap_or("").starts_with("PartDesign") {
                match chain_of(doc, &k) {
                    Ok(c) => todo.extend(c),
                    Err(Fail::Unsupported(_)) => {}
                    Err(x) => return Err(x),
                }
            } else {
                todo.push(k);
            }
        }
    }
    Ok(out)
}

fn chain_of(doc: &Doc, body: &str) -> Result<Vec<String>, Fail> {
    let mut tip = if doc.ty(body) == Some("PartDesign::Body") {
        doc.link(body, "Tip")?
    } else {
        body.to_string()
    };
    let mut out = vec![];
    let mut steps = 0;
    while !tip.is_empty() {
        steps += 1;
        if steps > 4 * doc.names.len() + 4 {
            return bug("a chain that loops");
        }
        if doc.ty(&tip) == Some("PartDesign::Body") {
            // a body based on another body starts from that body's result
            tip = doc.link(&tip, "Tip")?;
            continue;
        }
        let t = doc.ty(&tip);
        if !is(SOLID, t) {
            return Err(Fail::Unsupported(format!(
                "a chain through {}",
                t.unwrap_or("None")
            )));
        }
        out.push(tip.clone());
        tip = if t.is_some_and(|t| t.starts_with("Part::")) {
            String::new()
        } else {
            base_of(doc, &tip)?
        };
    }
    out.reverse();
    Ok(out)
}

struct Tr<'d, 'a, 'i> {
    doc: &'d Doc<'a, 'i>,
    truth: HashMap<String, Stored>,
    lines: Vec<String>,
    labels: HashMap<String, String>,
    state: State,
    bodies: usize,
    approx: bool,
    tree: usize,
    chain_rest: Vec<String>,
}

fn unsupported<T>(s: impl Into<String>) -> Result<T, Fail> {
    Err(Fail::Unsupported(s.into()))
}

impl Tr<'_, '_, '_> {
    fn profile_ents(&mut self, sk: &str, fr: &Frame) -> Result<Vec<Ent>, Fail> {
        if self.doc.ty(sk) == Some("Sketcher::SketchObject") {
            return sketch_ents(self.doc, sk);
        }
        let Some(ws) = self.truth.get(sk).and_then(|s| s.wires.as_ref()) else {
            return Err(Fail::Unresolved(
                "a profile whose stored wires would not read".into(),
            ));
        };
        let mut ents = vec![];
        let mut approx = false;
        for w in ws {
            for (k, p) in w {
                let uv: Vec<V2> = p
                    .iter()
                    .map(|&q| {
                        let r = fr.uvw(q);
                        (r.0, r.1)
                    })
                    .collect();
                let id = format!("w{}", ents.len());
                let (first, last) = (uv[0], uv[uv.len() - 1]);
                let shape = if *k == "line" {
                    Shape::Line {
                        start: first,
                        end: last,
                    }
                } else if *k == "circle" && py::dist2(first, last) < 1e-7 {
                    let Some((c, r, _)) = circle_from(uv[0], uv[32], uv[64]) else {
                        return bug("a stored circle through three points in line");
                    };
                    Shape::Circle { center: c, radius: r }
                } else if *k == "circle" {
                    Shape::Arc {
                        start: first,
                        mid: uv[48],
                        end: last,
                    }
                } else {
                    approx = true;
                    Shape::Poly { points: uv }
                };
                ents.push(Ent {
                    id,
                    shape,
                    construction: false,
                });
            }
        }
        self.approx |= approx;
        Ok(ents)
    }

    #[allow(clippy::too_many_arguments)]
    fn profile(
        &mut self,
        name: &str,
        op: &str,
        head: &str,
        flip: bool,
        frame_z: V3,
        tail: &str,
        sk: Option<&str>,
    ) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        let sk = match sk {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => d.link(name, "Profile")?,
        };
        if !is(FLAT, d.ty(&sk)) {
            return unsupported("a profile not from a sketch");
        }
        let fr = Frame::new(d.placement(&sk)?);
        let ents = self.profile_ents(&sk, &fr)?;
        let wires = closed_wires(&ents);
        let wired: HashSet<usize> = wires.iter().flatten().copied().collect();
        let a = Arrangement::new(&ents)?;
        // older files wire a sketch some other way (a line drawn twice closes there, not here): if
        // the wires do not account for every curve, its regions are all there is to go on
        let all = ents
            .iter()
            .enumerate()
            .all(|(i, e)| e.construction || wired.contains(&i));
        let regs = if all {
            wire_regions(&a, &ents, &wires)?
        } else {
            even_regions(&a)
        };
        if regs.is_empty() {
            return Err(Fail::Unresolved("a sketch with no closed region".into()));
        }
        let frame = format!("z {} x {} at {}", vtext(frame_z), vtext(fr.x), vtext(fr.o));
        let mut out = vec![];
        for prof in a.merged(&regs)? {
            let loops: Vec<String> = prof.iter().map(|lp| loop_text(&a, lp, flip)).collect();
            let mut this = op;
            if self.bodies == 0 {
                if op == "cut" {
                    return unsupported("first feature cuts");
                }
                this = if self.tree > 0 { "new" } else { "" };
                self.bodies = 1;
            }
            let k = if tail.is_empty() { "extrude" } else { "revolve" };
            let this = if this.is_empty() {
                String::new()
            } else {
                format!("{this} ")
            };
            out.push(format!("{k} {this}{head} {frame} {tail}{}", loops.join(" ")));
        }
        Ok(out)
    }

    fn pad(&mut self, name: &str, cut: bool) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        for k in ["TaperAngle", "TaperAngle2", "Offset"] {
            if d.flt(name, k, 0.0)?.abs() > 1e-12 {
                return unsupported(format!("a pad with {k}"));
            }
        }
        let sk = d.link(name, "Profile")?;
        if !is(FLAT, d.ty(&sk)) {
            return unsupported("a profile not from a sketch");
        }
        let n = Frame::new(d.placement(&sk)?).n;
        let rev = d.boolean(name, "Reversed")?;
        let mut down = cut != rev; // a pocket runs against the sketch normal
        let mut z = if down { neg(n) } else { n };
        let stored = d.prop(name, "Direction")?.and_then(|p| kid(p, "PropertyVector"));
        if d.ty(&sk) != Some("Sketcher::SketchObject")
            && let Some(e) = stored
        {
            // a shape's plane may face either way; the feature records which way it went, or 1 1 1
            // if it never wrote it
            let v = vector(e)?;
            let l = dot(v, v).sqrt();
            if l > 0.0 && ((dot(v, n).abs() / l) - 1.0).abs() < 1e-9 {
                down = dot(v, n) < 0.0;
                z = if down { neg(n) } else { n };
            }
        }
        let t = d.integer(name, "Type", 0)?;
        let (mut l, mut l2) = (d.flt(name, "Length", 0.0)?, d.flt(name, "Length2", 0.0)?);
        let mid = d.boolean(name, "Midplane")?;
        let mut along = String::new();
        if d.boolean(name, "UseCustomVector")? {
            // the direction is stored as used, a pocket's too: only Reversed turns it. Length runs
            // along it unless AlongSketchNormal
            let Some(e) = stored else {
                return unsupported("a pad along a custom direction with none stored");
            };
            let Some(v) = py::unit(vector(e)?) else {
                return bug("a zero direction");
            };
            let c = dot(v, n);
            if c.abs() < 1e-9 || ![0, 1, 4].contains(&t) {
                return unsupported("a pad along a custom direction of this form");
            }
            down = (c < 0.0) != rev;
            z = if down { neg(n) } else { n };
            if !d.boolean(name, "AlongSketchNormal")? {
                (l, l2) = (l * c.abs(), l2 * c.abs());
            }
            if (c.abs() - 1.0).abs() > 1e-12 {
                along = format!(" along {}", vfine(v));
            }
        }
        let mut head = match t {
            0 if mid => format!("sym {}", num(l)),
            0 => num(l),
            4 => format!("two {} {}", num(l), num(l2)),
            1 if cut && mid => "thru both".into(),
            1 if cut => "thru".into(),
            2 => "next".into(),
            _ => return unsupported(format!("a pad or pocket of type {t}")),
        };
        head += &along;
        self.profile(name, if cut { "cut" } else { "add" }, &head, down, z, "", None)
    }

    fn axis_tail(&self, fr: &Frame, base: V3, ax: V3) -> Result<(f64, f64, f64, f64), Fail> {
        let (u, v, w) = fr.uvw(base);
        let (du, dv, dw) = (dot(ax, fr.x), dot(ax, fr.y), dot(ax, fr.n));
        if w.abs() > 1e-6 || dw.abs() > 1e-6 {
            return unsupported("a revolve axis out of the sketch's plane");
        }
        Ok((u, v, du, dv))
    }

    fn revolve(&mut self, name: &str, cut: bool) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        let (p, q) = (d.prop(name, "Base")?, d.prop(name, "Axis")?);
        let (Some(p), Some(q)) = (p, q) else {
            return unsupported("a revolve without its axis");
        };
        let (Some(b), Some(c)) = (kid(p, "PropertyVector"), kid(q, "PropertyVector")) else {
            return bug("a revolve's axis not a vector");
        };
        let (base, ax) = (vector(b)?, vector(c)?);
        let sk = d.link(name, "Profile")?;
        if !is(FLAT, d.ty(&sk)) {
            return unsupported("a profile not from a sketch");
        }
        let fr = Frame::new(d.placement(&sk)?);
        let (u, v, mut du, mut dv) = self.axis_tail(&fr, base, ax)?;
        if d.boolean(name, "Reversed")? {
            (du, dv) = (-du, -dv);
        }
        let ang = d.flt(name, "Angle", 360.0)?;
        let head = if (ang - 360.0).abs() < 1e-9 {
            String::new()
        } else if d.boolean(name, "Midplane")? {
            format!("sym {}", num(ang))
        } else {
            num(ang)
        };
        let tail = format!("axis {} {} {} {} ", num(u), num(v), num(du), num(dv));
        self.profile(
            name,
            if cut { "cut" } else { "add" },
            &head,
            false,
            fr.n,
            &tail,
            None,
        )
    }

    /// Part workbench: a sketch turned about Axis through Base.
    fn part_revolve(&mut self, name: &str) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        if !d.boolean(name, "Solid")? {
            return unsupported("a Part revolve that makes a shell");
        }
        let (base, ax) = (d.vec_prop(name, "Base")?, d.vec_prop(name, "Axis")?);
        let sk = d.link(name, "Source")?;
        if !is(FLAT, d.ty(&sk)) {
            return unsupported("a Part revolve of something not a sketch");
        }
        let fr = Frame::new(d.placement(&sk)?);
        let (u, v, du, dv) = self.axis_tail(&fr, base, ax)?;
        let ang = d.flt(name, "Angle", 360.0)?;
        let mut head = if (ang - 360.0).abs() < 1e-9 {
            String::new()
        } else {
            num(ang)
        };
        if d.boolean(name, "Symmetric")? && !head.is_empty() {
            head = format!("sym {head}");
        }
        let tail = format!("axis {} {} {} {} ", num(u), num(v), num(du), num(dv));
        self.profile(name, "add", &head, false, fr.n, &tail, Some(&sk))
    }

    /// Does a real feature follow this clone in the body being built? Those are in the body's own
    /// frame, not the moved one.
    fn after_moved(&self, name: &str) -> Result<bool, Fail> {
        if self.chain_rest.is_empty() {
            return Ok(false);
        }
        let Some(i) = self.chain_rest.iter().position(|c| c == name) else {
            return bug("a clone outside the chain");
        };
        Ok(self.chain_rest[i + 1..]
            .iter()
            .any(|c| self.doc.ty(c) != Some("PartDesign::FeatureBase")))
    }

    /// A Part workbench solid and what it is made of, as bodies and combines. Tools first, so the
    /// base is the last body made and the lines after it act on that. Returns the result's label.
    fn part_tree(&mut self, name: &str) -> Result<String, Fail> {
        let d = self.doc;
        let t = d.ty(name).unwrap_or("");
        if matches!(
            t,
            "Part::Cut" | "Part::Fuse" | "Part::Common" | "Part::MultiFuse" | "Part::MultiCommon"
        ) {
            let pl = d.placement(name)?;
            let o = pl.0;
            if [o.0, o.1, o.2].iter().any(|x| x.abs() > 1e-9) || (pl.1.3.abs() - 1.0).abs() > 1e-9 {
                return unsupported("a moved Part boolean");
            }
            let ks = if matches!(t, "Part::MultiFuse" | "Part::MultiCommon") {
                d.links(name, "Shapes")?
            } else {
                vec![d.link(name, "Base")?, d.link(name, "Tool")?]
            };
            if ks.len() < 2 || ks.iter().any(String::is_empty) {
                return unsupported("a Part boolean short of shapes");
            }
            let mut labs = vec![];
            for k in ks[1..].iter().chain(&ks[..1]) {
                labs.push(self.part_tree(k)?);
            }
            let word = match t {
                "Part::Cut" => "cut",
                "Part::Common" | "Part::MultiCommon" => "keep",
                _ => "add",
            };
            let base = labs.pop().expect("two or more");
            self.lines
                .push(format!("combine {word} {base} {}", labs.join(" ")));
            return Ok(base);
        }
        if t == "PartDesign::Body" || SOLID.contains(&t) && !t.starts_with("Part::") {
            // a PartDesign chain as one shape of the tree: a body of its own, what follows it acting on it
            let sub = chain_of(d, name)?;
            self.tree += 1;
            self.bodies = 0;
            let first = self.lines.len();
            for n in &sub {
                self.step(n)?;
            }
            let Some(head) = self.lines.get(first) else {
                return bug("a chain that made no line");
            };
            self.bodies = 1;
            let w0 = head.split(' ').next().unwrap_or("");
            if !w0.contains(':') {
                return unsupported("a PartDesign shape in a Part tree with no label");
            }
            return Ok(head.split(':').next().unwrap_or("").into());
        }
        let mut ls = match t {
            "Part::Revolution" => self.part_revolve(name)?,
            "Part::Box" | "Part::Cylinder" => self.part_solid(name, t)?,
            "Part::Sphere" => {
                if (d.flt(name, "Angle1", -90.0)? + 90.0).abs() > 1e-9
                    || (d.flt(name, "Angle2", 90.0)? - 90.0).abs() > 1e-9
                    || (d.flt(name, "Angle3", 360.0)? - 360.0).abs() > 1e-9
                {
                    return unsupported("a part of a sphere");
                }
                let fr = Frame::new(d.placement(name)?);
                vec![format!(
                    "ball {} z {} x {} at {}",
                    num(d.flt(name, "Radius", 0.0)?),
                    vtext(fr.n),
                    vtext(fr.x),
                    vtext(fr.o)
                )]
            }
            _ => {
                let t = d.ty(name).unwrap_or("None");
                return unsupported(format!("a Part tree through {t}"));
            }
        };
        if ls.len() != 1 {
            return unsupported("a Part solid of several regions");
        }
        self.tree += 1;
        let lab = format!("p{}", self.tree);
        if self.tree > 1 {
            // after the first, each its own body
            let (word, rest) = ls[0].split_once(' ').unwrap_or((&ls[0], ""));
            let rest = rest.strip_prefix("add ").unwrap_or(rest);
            ls = vec![format!("{word} new {rest}")];
        }
        self.lines.push(format!("{lab}: {}", ls[0]));
        Ok(lab)
    }

    /// Part workbench primitives, on their placement.
    fn part_solid(&mut self, name: &str, t: &str) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        let fr = Frame::new(d.placement(name)?);
        let frame = format!("z {} x {} at {}", vtext(fr.n), vtext(fr.x), vtext(fr.o));
        self.bodies = 1;
        let h = num(d.flt(name, "Height", 0.0)?);
        if t == "Part::Box" {
            return Ok(vec![format!(
                "extrude {h} {frame} rect 0 0 {} {}",
                num(d.flt(name, "Length", 0.0)?),
                num(d.flt(name, "Width", 0.0)?)
            )]);
        }
        if (d.flt(name, "Angle", 360.0)? - 360.0).abs() > 1e-9 {
            return unsupported("a part of a cylinder");
        }
        Ok(vec![format!(
            "extrude {h} {frame} circle 0 0 {}",
            num(d.flt(name, "Radius", 0.0)?)
        )])
    }

    fn blend(&mut self, name: &str, line: String, all_edges: bool) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        let base = d.link(name, "Base")?;
        let mut subs = if all_edges {
            vec!["ALL".to_string()]
        } else {
            d.subs(name, "Base")?
        };
        if subs.is_empty() {
            subs = vec!["ALL".into()];
        }
        let edges = self.state.edges(&self.lines)?;
        let mut picked: BTreeMap<String, usize> = BTreeMap::new();
        let mut bbox = None;
        for sub in &subs {
            let Some(data) = self.truth.get(&base).and_then(|s| s.subs.get(sub)) else {
                let s: String = sub.chars().take(4).collect();
                return Err(Fail::Unresolved(format!("a blend on {s} not sampled")));
            };
            for poly in data {
                if self.approx && !line_or_circle(poly) {
                    // the part's spline is a polyline here, so its edge is many short ones the blend
                    // cannot round as one
                    return unsupported("a blend along a spline");
                }
                // one of ours can span several of theirs (a seam, a split circle): its middle stretch decides.
                // Their samples are chords: on a big circle the curve between them lies further off than TOL
                let tol = TOL + sag(poly);
                let lo = |f: fn(&V3) -> f64| poly.iter().map(f).fold(f64::INFINITY, pmin) - tol;
                let hi = |f: fn(&V3) -> f64| poly.iter().map(f).fold(f64::NEG_INFINITY, pmax) + tol;
                let (l, h) = (
                    [lo(|q| q.0), lo(|q| q.1), lo(|q| q.2)],
                    [hi(|q| q.0), hi(|q| q.1), hi(|q| q.2)],
                );
                let inbox = |p: &V3| {
                    let p = [p.0, p.1, p.2];
                    (0..3).all(|k| l[k] <= p[k] && p[k] <= h[k])
                };
                let mid = |e: &Edge| [e.p[4], e.p[8], e.p[12]];
                let mut hits: Vec<usize> = (0..edges.len())
                    .filter(|&i| {
                        mid(&edges[i])
                            .iter()
                            .any(|p| inbox(p) && near(*p, poly, tol) <= tol)
                    })
                    .collect();
                if hits.is_empty() && self.approx {
                    // where a polyline stands in for a spline, what it meets moves a little: the
                    // nearest within the part's slack
                    let bx = *bbox.get_or_insert_with(|| self.base_box(&base));
                    let slack = 1e-3 * py::dist3((bx[0], bx[1], bx[2]), (bx[3], bx[4], bx[5]));
                    let mut scored: Vec<(f64, usize)> = (0..edges.len())
                        .map(|i| {
                            (
                                mid(&edges[i])
                                    .iter()
                                    .map(|&p| near(p, poly, 0.0))
                                    .fold(f64::NEG_INFINITY, pmax),
                                i,
                            )
                        })
                        .collect();
                    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                    if let Some(&(s0, _)) = scored.first()
                        && s0 <= slack
                    {
                        // only a clear winner: a second edge as close would be a guess
                        let close: HashSet<&str> = scored
                            .iter()
                            .filter(|x| x.0 <= s0 + 1e-9)
                            .map(|x| edges[x.1].h.as_str())
                            .collect();
                        hits = (0..edges.len())
                            .filter(|&i| close.contains(edges[i].h.as_str()))
                            .collect();
                    }
                }
                if hits.is_empty() && !all_edges {
                    return Err(Fail::Unresolved("no edge where the base's edge lies".into()));
                }
                for i in hits {
                    if edges[i].c != "smooth" {
                        picked.insert(edges[i].h.clone(), i);
                    }
                }
            }
        }
        if picked.is_empty() {
            return Err(Fail::Unresolved("no edge where the base's edge lies".into()));
        }
        // OpenCascade carries a blend along tangent edges
        along_tangents(&edges, &mut picked);
        let hs: Vec<&str> = picked.keys().map(String::as_str).collect();
        Ok(vec![format!("{line} {}", hs.join(" "))])
    }

    /// The base's stored solids' box, or zeros where it has none.
    fn base_box(&self, base: &str) -> [f64; 6] {
        let file = self
            .truth
            .get(base)
            .filter(|s| s.readable)
            .map(|s| s.file.clone());
        file.and_then(|f| self.doc.zip.read(&f))
            .and_then(|data| solids_of(&data))
            .and_then(|s| s.1)
            .unwrap_or([0.0; 6])
    }

    /// A pattern's direction or axis: a sketch's H, V or N axis, or the body's origin line.
    fn axis_of(&self, name: &str, prop: &str) -> Result<(V3, V3), Fail> {
        let d = self.doc;
        let (r, subs) = (d.link(name, prop)?, d.subs(name, prop)?);
        let s = subs.first().map_or("", String::as_str);
        let fail = || Err(Fail::Unsupported(format!("a pattern reference {prop}")));
        match d.ty(&r) {
            Some("Sketcher::SketchObject") => {
                let fr = Frame::new(d.placement(&r)?);
                let a = match s {
                    "H_Axis" => fr.x,
                    "V_Axis" => fr.y,
                    "N_Axis" => fr.n,
                    _ => return fail(),
                };
                Ok((fr.o, a))
            }
            Some("App::Line") => {
                let a = match d.role(&r)?.as_deref() {
                    Some("X_Axis") => (1.0, 0.0, 0.0),
                    Some("Y_Axis") => (0.0, 1.0, 0.0),
                    Some("Z_Axis") => (0.0, 0.0, 1.0),
                    _ => return fail(),
                };
                Ok(((0.0, 0.0, 0.0), a))
            }
            Some("PartDesign::Line") => {
                let fr = Frame::new(d.placement(&r)?);
                Ok((fr.o, fr.n))
            }
            t => unsupported(format!("a pattern axis on {}", t.unwrap_or("nothing"))),
        }
    }

    fn transform(&self, name: &str, t: &str, src: &str) -> Result<String, Fail> {
        let d = self.doc;
        if t == "PartDesign::PolarPattern" {
            let (o, mut a) = self.axis_of(name, "Axis")?;
            if d.boolean(name, "Reversed")? {
                a = neg(a);
            }
            return Ok(format!(
                "pattern {src} circ {} {} {} {}",
                vtext(o),
                vtext(a),
                d.integer(name, "Occurrences", 0)?,
                num(d.flt(name, "Angle", 360.0)?)
            ));
        }
        if t == "PartDesign::LinearPattern" {
            let (_, mut a) = self.axis_of(name, "Direction")?;
            if d.boolean(name, "Reversed")? {
                a = neg(a);
            }
            let n = d.integer(name, "Occurrences", 0)?;
            let step = d.flt(name, "Length", 0.0)? / (n - 1).max(1) as f64;
            return Ok(format!("pattern {src} lin {} {n} {}", vtext(a), num(step)));
        }
        let (r, subs) = (d.link(name, "MirrorPlane")?, d.subs(name, "MirrorPlane")?);
        let fail = || Err(Fail::Unsupported("a pattern reference mirror".into()));
        let (n, o) = match d.ty(&r) {
            Some("Sketcher::SketchObject") => {
                let fr = Frame::new(d.placement(&r)?);
                let n = match subs.first().map_or("", String::as_str) {
                    "H_Axis" => fr.y,
                    "V_Axis" => fr.x,
                    "" => fr.n,
                    _ => return fail(),
                };
                (n, fr.o)
            }
            Some("App::Plane") => {
                let Some(role) = d.role(&r)? else {
                    return bug("a plane with no role");
                };
                let n = match role.as_str() {
                    "XY_Plane" => (0.0, 0.0, 1.0),
                    "XZ_Plane" => (0.0, 1.0, 0.0),
                    "YZ_Plane" => (1.0, 0.0, 0.0),
                    _ => return fail(),
                };
                (n, (0.0, 0.0, 0.0))
            }
            Some("PartDesign::Plane") => {
                let fr = Frame::new(d.placement(&r)?);
                (fr.n, fr.o)
            }
            t => return unsupported(format!("a mirror in {}", t.unwrap_or("nothing"))),
        };
        Ok(format!("pattern {src} mirror z {} at {}", vtext(n), vtext(o)))
    }

    /// One hole at each of the sketch's circles, drilled against its normal; its seat from what the
    /// file stored.
    fn hole(&self, name: &str) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        if d.boolean(name, "Tapered")?
            || d.boolean(name, "ModelActualThread")?
            || d.boolean(name, "Midplane")?
        {
            return unsupported("a tapered, threaded or two-way hole");
        }
        if d.boolean(name, "Threaded")? {
            // measured: bodies with this set stored more than the plain bore
            return unsupported("a hole with its thread in the shape");
        }
        let sk = d.link(name, "Profile")?;
        let fr = Frame::new(d.placement(&sk)?);
        let ents: Vec<Ent> = sketch_ents(d, &sk)?
            .into_iter()
            .filter(|e| !e.construction)
            .collect();
        if ents.is_empty() || ents.iter().any(|e| !matches!(e.shape, Shape::Circle { .. })) {
            return unsupported("a hole on something other than circles");
        }
        let z = if d.boolean(name, "Reversed")? {
            fr.n
        } else {
            neg(fr.n)
        };
        let t = d.integer(name, "DepthType", 0)?;
        let mut ext = match t {
            1 => "thru".to_string(),
            0 => num(d.flt(name, "Depth", 0.0)?),
            _ => return unsupported("a hole up to a face"),
        };
        if t == 0 && d.integer(name, "DrillPoint", 0)? == 1 {
            ext += &format!(" pt {}", num(d.flt(name, "DrillPointAngle", 0.0)?));
        }
        let mut seat = String::new();
        if d.integer(name, "HoleCutType", 0)? != 0 {
            let dia = num(d.flt(name, "HoleCutDiameter", 0.0)?);
            seat = if d.flt(name, "HoleCutDepth", 0.0)? > 0.0 {
                format!(" cb {dia} {}", num(d.flt(name, "HoleCutDepth", 0.0)?))
            } else {
                format!(" cs {dia} {}", num(d.flt(name, "HoleCutCountersinkAngle", 0.0)?))
            };
        }
        let dia = num(d.flt(name, "Diameter", 0.0)?);
        Ok(ents
            .iter()
            .map(|e| {
                let Shape::Circle { center, .. } = e.shape else {
                    unreachable!("circles only")
                };
                format!(
                    "hole {dia} z {} x {} at {}{seat} {ext}",
                    vtext(z),
                    vtext(fr.x),
                    vtext(fr.world(center.0, center.1))
                )
            })
            .collect())
    }

    fn primitive(&mut self, name: &str, t: &str) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        let fr = Frame::new(d.placement(name)?);
        let frame = format!("z {} x {} at {}", vtext(fr.n), vtext(fr.x), vtext(fr.o));
        let cut = t.contains("Subtractive");
        if cut && self.bodies == 0 {
            return unsupported("first feature cuts");
        }
        let op = match (self.bodies > 0, cut) {
            (false, _) => "",
            (true, true) => "cut ",
            (true, false) => "add ",
        };
        self.bodies = 1;
        let h = num(d.flt(name, "Height", 0.0)?);
        if t.ends_with("Box") {
            return Ok(vec![format!(
                "extrude {op}{h} {frame} rect 0 0 {} {}",
                num(d.flt(name, "Length", 0.0)?),
                num(d.flt(name, "Width", 0.0)?)
            )]);
        }
        if (d.flt(name, "Angle", 360.0)? - 360.0).abs() > 1e-9
            || d.flt(name, "FirstAngle", 0.0)?.abs() > 1e-9
            || d.flt(name, "SecondAngle", 0.0)?.abs() > 1e-9
        {
            return unsupported("a part of a cylinder");
        }
        Ok(vec![format!(
            "extrude {op}{h} {frame} circle 0 0 {}",
            num(d.flt(name, "Radius", 0.0)?)
        )])
    }

    fn originals(&self, name: &str) -> Result<String, Fail> {
        let mut out = vec![];
        for o in self.doc.links(name, "Originals")? {
            match self.labels.get(&o) {
                Some(l) => out.push(l.clone()),
                None => {
                    return Err(Fail::Unresolved(
                        "a pattern of a feature outside the chain".into(),
                    ));
                }
            }
        }
        Ok(out.join(" "))
    }

    fn run(&mut self, chain: &[String]) -> Result<(), Fail> {
        for name in chain {
            let t = self.doc.ty(name).unwrap_or("");
            if !NEEDS.contains(&t) {
                return unsupported(format!("feature {}", last_word(t)));
            }
            if doc_approx(self.doc, name)? {
                self.approx = true;
            }
        }
        self.chain_rest = chain.to_vec();
        for name in chain {
            self.step(name)?;
        }
        Ok(())
    }

    fn step(&mut self, name: &str) -> Result<(), Fail> {
        let d = self.doc;
        let t = d.ty(name).unwrap_or("").to_string();
        if t == "PartDesign::FeatureBase" {
            // stands for its BaseFeature, which the chain has already built, moved by its own placement
            // (measured: a chain of clones composes them, and the base feature's own placement plays no part)
            let (by, q) = d.placement(name)?;
            let sn = (q.0 * q.0 + q.1 * q.1 + q.2 * q.2).sqrt();
            let moved = dot(by, by).sqrt() >= 1e-9;
            if sn < 1e-12 && !moved {
                return Ok(());
            }
            if self.after_moved(name)? {
                return unsupported("features after a moved clone");
            }
            let mut words = vec![];
            if sn >= 1e-12 {
                let ax = (q.0 / sn, q.1 / sn, q.2 / sn);
                words.push(format!(
                    "rot 0 0 0 {} {}",
                    vtext(ax),
                    num(py::degrees(2.0 * py::atan2(sn, q.3)))
                ));
            }
            if moved {
                words.push(format!("by {}", vtext(by)));
            }
            self.lines.push(format!("movebody {}", words.join(" ")));
            return Ok(());
        }
        let mut root = String::new();
        let mut ls = match t.as_str() {
            _ if t.starts_with("Part::") => {
                root = self.part_tree(name)?;
                vec![]
            }
            "PartDesign::Pad" | "PartDesign::Pocket" => self.pad(name, t.ends_with("Pocket"))?,
            "PartDesign::Revolution" | "PartDesign::Groove" => self.revolve(name, t.ends_with("Groove"))?,
            "PartDesign::Fillet" => {
                let line = format!("fillet {}", num(d.flt(name, "Radius", 0.0)?));
                self.blend(name, line, d.boolean(name, "UseAllEdges")?)?
            }
            "PartDesign::Chamfer" => {
                if d.integer(name, "ChamferType", 0)? != 0 {
                    return unsupported("an unequal chamfer");
                }
                let line = format!("chamfer {}", num(d.flt(name, "Size", 0.0)?));
                self.blend(name, line, d.boolean(name, "UseAllEdges")?)?
            }
            _ if TRANSFORMS.contains(&t.as_str()) => {
                let src = self.originals(name)?;
                vec![self.transform(name, &t, &src)?]
            }
            "PartDesign::MultiTransform" => {
                let mut ls = vec![];
                let mut src = self.originals(name)?;
                for (k, m) in d.links(name, "Transformations")?.iter().enumerate() {
                    let mt = d.ty(m);
                    if !is(TRANSFORMS, mt) {
                        let w = mt.map_or("None", last_word);
                        return unsupported(format!("a multi-transform with {w}"));
                    }
                    let lab = format!("m{}x{k}", self.labels.len());
                    ls.push(format!("{lab}: {}", self.transform(m, mt.unwrap_or(""), &src)?));
                    src += &format!(" {lab}");
                }
                ls
            }
            "PartDesign::Hole" => self.hole(name)?,
            "PartDesign::AdditiveBox"
            | "PartDesign::SubtractiveBox"
            | "PartDesign::AdditiveCylinder"
            | "PartDesign::SubtractiveCylinder" => self.primitive(name, &t)?,
            _ => return bug("a feature with no step"),
        };
        if t.starts_with("Part::") {
            self.labels.insert(name.into(), root);
            self.bodies = 1;
        } else if t == "PartDesign::MultiTransform" {
            // already labelled, each its own
            let Some(l) = ls.last() else {
                return bug("a multi-transform of nothing");
            };
            let l = l.split(':').next().unwrap_or("").to_string();
            self.labels.insert(name.into(), l);
        } else {
            // a feature of several lines (regions that do not touch): a pattern of it needs them all
            let base = format!("t{}", self.labels.len());
            let names: Vec<String> = (0..ls.len())
                .map(|k| {
                    if k == 0 {
                        base.clone()
                    } else {
                        format!("{base}x{k}")
                    }
                })
                .collect();
            self.labels.insert(name.into(), names.join(" "));
            ls = names.iter().zip(&ls).map(|(n, l)| format!("{n}: {l}")).collect();
        }
        self.lines.extend(ls);
        Ok(())
    }
}

/// Every body of the file, each translated or not, in the file's order: its PartDesign bodies, then
/// the ends of older files' chains of features with no body. Err when the file will not read.
pub fn translate(data: &[u8]) -> Result<Vec<Body>, String> {
    let zip = Zip::new(data).ok_or("not a zip file")?;
    if zip.size("Document.xml").ok_or("no Document.xml")? > 64 << 20 {
        return Err("a huge document".into());
    }
    let xml = zip.read("Document.xml").ok_or("Document.xml would not read")?;
    let text = String::from_utf8(xml).map_err(|_| "Document.xml not UTF-8")?;
    let opt = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let root = roxmltree::Document::parse_with_options(&text, opt).map_err(|e| e.to_string())?;
    let top = root.root_element();
    let (mut names, mut types) = (vec![], HashMap::new());
    for o in kids(kid(top, "Objects").ok_or("no Objects")?, "Object") {
        let n = o.attribute("name").unwrap_or("").to_string();
        if types
            .insert(n.clone(), o.attribute("type").unwrap_or("").to_string())
            .is_none()
        {
            names.push(n);
        }
    }
    let mut props = HashMap::new();
    for o in kids(kid(top, "ObjectData").ok_or("no ObjectData")?, "Object") {
        let ps = kid(o, "Properties").ok_or("an object without properties")?;
        let m: HashMap<String, Node> = kids(ps, "Property")
            .map(|p| (p.attribute("name").unwrap_or("").to_string(), p))
            .collect();
        props.insert(o.attribute("name").unwrap_or("").to_string(), m);
    }
    let mut doc = Doc {
        zip: &zip,
        names,
        types,
        props,
        volumes: None,
        bodied: Default::default(),
    };
    file_bodies(&mut doc).map_err(|f| f.why().to_string())
}

fn file_bodies(doc: &mut Doc) -> Result<Vec<Body>, Fail> {
    let solid = |doc: &Doc, n: &String| is(SOLID, doc.ty(n));
    let mut bodies: Vec<String> = doc
        .names
        .iter()
        .filter(|n| doc.ty(n) == Some("PartDesign::Body"))
        .cloned()
        .collect();
    let mut legacy = vec![];
    for n in doc.names.iter().filter(|n| solid(doc, n)) {
        if doc.prop(n, "BaseFeature")?.is_none() {
            legacy.push(n.clone());
        }
    }
    if !legacy.is_empty() {
        let mut files: Vec<String> = vec![];
        for n in &legacy {
            for k in ["Shape", "AddShape", "AddSubShape"] {
                if let Some(f) = doc
                    .prop(n, k)?
                    .and_then(|p| kid(p, "Part"))
                    .and_then(|e| e.attribute("file"))
                    && !f.is_empty()
                    && !files.iter().any(|x| x == f)
                {
                    files.push(f.into());
                }
            }
        }
        // the volumes are all there or none are, as one OpenCascade run read them
        if files.iter().all(|f| doc.zip.has(f)) {
            let mut vols = HashMap::new();
            for f in files {
                let v = doc.zip.read(&f).and_then(|d| solids_of(&d)).map(|s| s.0);
                vols.insert(f, v);
            }
            doc.volumes = Some(vols);
        }
    }
    let doc = &*doc;
    // older files: a chain of features with no body; each end of a chain is a part
    let mut inbody = HashSet::new();
    for b in &bodies {
        inbody.extend(doc.links(b, "Group")?);
    }
    let mut based = HashSet::new();
    for n in doc.names.iter().filter(|n| solid(doc, n)) {
        based.insert(base_of(doc, n)?);
    }
    // a Part solid alone is no part of this: only the PartDesign chains that start from one
    for n in &doc.names {
        if solid(doc, n)
            && !based.contains(n)
            && !inbody.contains(n)
            && !doc.ty(n).unwrap_or("").starts_with("Part::")
        {
            bodies.push(n.clone());
        }
    }
    Ok(bodies.iter().map(|b| one_body(doc, b)).collect())
}

fn one_body(doc: &Doc, b: &str) -> Body {
    let mut out = Body {
        name: b.into(),
        features: vec![],
        approx: false,
        fc: None,
        fail: None,
    };
    let mut tr = None;
    let r = (|| -> Result<(), Fail> {
        let chain = chain_of(doc, b)?;
        if chain.is_empty() {
            return unsupported("an empty body");
        }
        out.features = chain
            .iter()
            .map(|c| last_word(doc.ty(c).unwrap_or("")).to_string())
            .collect();
        let tip = &chain[chain.len() - 1];
        let mut shapes: Vec<(String, Option<String>)> = vec![(tip.clone(), doc.shape_file(tip)?)];
        let mut put = |n: &str, f: Option<String>| match shapes.iter_mut().find(|s| s.0 == n) {
            Some(s) => s.1 = f,
            None => shapes.push((n.into(), f)),
        };
        let mut subs: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut wires = vec![];
        let members = tree_members(doc, &chain)?;
        for c in &members {
            let sk = doc.link(c, "Profile")?;
            if doc.ty(&sk) == Some("Part::Part2DObjectPython") {
                put(&sk, doc.shape_file(&sk)?);
                wires.push(sk);
            }
        }
        for c in &members {
            let Some(t) = doc.ty(c) else {
                return bug("a member of no type");
            };
            if t == "PartDesign::Fillet" || t == "PartDesign::Chamfer" {
                let base = doc.link(c, "Base")?;
                put(&base, doc.shape_file(&base)?);
                let mut s = doc.subs(c, "Base")?;
                if s.is_empty() || doc.boolean(c, "UseAllEdges")? {
                    s = vec!["ALL".into()];
                }
                subs.entry(base).or_default().extend(s);
            }
        }
        let Some(truth) = stored(doc.zip, &shapes, &subs, &wires) else {
            return Err(Fail::Unresolved("the stored shapes would not read".into()));
        };
        let t = tr.insert(Tr {
            doc,
            truth,
            lines: vec![],
            labels: HashMap::new(),
            state: State::default(),
            bodies: 0,
            approx: false,
            tree: 0,
            chain_rest: vec![],
        });
        t.run(&chain)
    })();
    let r = r.err();
    if let Some(t) = tr {
        out.approx = t.approx;
        if !matches!(r, Some(Fail::Unsupported(_) | Fail::Unresolved(_) | Fail::Bug(_))) {
            out.fc = Some(t.lines.join("\n") + "\n");
        }
    }
    out.fail = r;
    out
}

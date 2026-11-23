//! FreeCAD `.FCStd` PartDesign bodies (and the Part workbench solids they build on) as Forge
//! (`.fg`) parts. Document.xml is read as XML and each object's stored shape as an OpenCascade
//! `.brep`: a fillet's `EdgeN` is sampled from its base's stored shape and found on our part by
//! position. Macros and scripts in the file are never touched.

use crate::arrangement::{Arrangement, Ent, Kind, Nurbs, Shape, circle_from};
use crate::py::{self, V2, V3, add, dot, neg, num, pmax, pmin};
use crate::state::{Edge, State};
use crate::zip::Zip;
use crate::{Body, Fail, bug};
use forge_math::{Point3, Vec3};
use forge_topo::Name;
use roxmltree::Node;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

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
    "Part::Extrusion",
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
    "PartDesign::Thickness",
    "PartDesign::Draft",
    "PartDesign::AdditivePipe",
    "PartDesign::SubtractivePipe",
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
    "Part::Extrusion",
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
                let points = bspline_points(e)?;
                match bspline_exact(e)? {
                    Some(n) => Shape::Nurbs {
                        points,
                        nurbs: Rc::new(n),
                    },
                    None => Shape::Poly { points },
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

/// A clamped, non-periodic NURBS as Forge writes one (`spline(.., degree:, knots:, weights:)`);
/// none where its ends are not its end poles.
fn bspline_exact(e: Node) -> Result<Option<Nurbs>, Fail> {
    let degree = usize::try_from(int(e.attribute("Degree"))?).or(bug("a negative degree"))?;
    let (mut poles, mut weights) = (vec![], vec![]);
    for p in kids(e, "Pole") {
        poles.push((float(p.attribute("X"))?, float(p.attribute("Y"))?));
        weights.push(float(Some(p.attribute("Weight").unwrap_or("1")))?);
    }
    let mut ks = vec![];
    for x in kids(e, "Knot") {
        ks.push((float(x.attribute("Value"))?, int(x.attribute("Mult"))?));
    }
    let clamped = |m: i64| usize::try_from(m).ok() == Some(degree + 1);
    let (Some(first), Some(last)) = (ks.first(), ks.last()) else {
        return Ok(None);
    };
    if ks.len() < 2 || !clamped(first.1) || !clamped(last.1) || weights.iter().any(|&w| w <= 0.0) {
        return Ok(None);
    }
    let mut knots = vec![];
    for &(v, m) in &ks {
        // bspline_points has already capped the counts
        knots.extend(std::iter::repeat_n(v, usize::try_from(m).unwrap_or(0)));
    }
    if knots.len() != poles.len() + degree + 1 || knots.windows(2).any(|w| w[1] < w[0]) {
        return Ok(None);
    }
    Ok(Some(Nurbs {
        degree,
        poles,
        knots,
        weights,
    }))
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
    // a file is untrusted: knots, poles and degree are capped before anything is made of them
    const MAX: usize = 100_000;
    let mut k = vec![];
    for x in kids(e, "Knot") {
        let v = float(x.attribute("Value"))?;
        let m = int(x.attribute("Mult"))?;
        if m > 0 {
            let m = usize::try_from(m).ok().filter(|&m| k.len() + m <= MAX);
            let Some(m) = m else {
                return bug("a spline of too many knots");
            };
            k.extend(std::iter::repeat_n(v, m));
        }
    }
    let need = cp.len().checked_add(deg).and_then(|n| n.checked_add(1));
    if cp.len() <= deg || cp.len() > MAX || need.is_none_or(|n| k.len() < n) {
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
        Shape::Poly { points } | Shape::Nurbs { points, .. } => Some((points[0], points[points.len() - 1])),
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

/// A stored edge: its curve's kind, samples, and samples without the root location.
type Sampled = (&'static str, Vec<V3>, Option<Vec<V3>>);

/// What a stored shape gives the translation.
#[derive(Default)]
struct Stored {
    /// Samples of the edges each sub names: `Edge3`, `Face2` (its edges), `ALL`.
    subs: HashMap<String, Vec<Vec<V3>>>,
    /// A profile's wires: each edge's kind and 97 samples, and the same in the shape's own
    /// coordinates where only its root location places it.
    wires: Option<Vec<Vec<Sampled>>>,
    /// The root location, rows of a 3x4 matrix.
    root: Option<[[f64; 4]; 3]>,
    /// Its vertices' box, where it holds a solid.
    vbox: Option<[f64; 6]>,
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
        let mut r = Stored::default();
        let Some(t) = forge_step::BrepTopo::new(&data) else {
            out.insert(name.clone(), r);
            continue;
        };
        r.vbox = t.vertex_box();
        if wires.contains(name) {
            let mut ws = vec![];
            for w in t.wires()? {
                let mut es = vec![];
                for e in w {
                    let local = t.local_sample(e, 96).map(|l| l.into_iter().map(p3).collect());
                    es.push((t.kind(e), t.sample(e, 96)?.into_iter().map(p3).collect(), local));
                }
                ws.push(es);
            }
            r.wires = Some(ws);
            r.root = t.root_matrix();
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
fn volume_of(data: &[u8]) -> Option<f64> {
    let imp = match forge_step::import(data, &forge_step::Limits::default()) {
        Ok(i) => i,
        Err(forge_step::ImportError::NoSolids) => return Some(0.0),
        Err(_) => return None,
    };
    let mut vol = 0.0;
    for s in &imp.solids {
        vol += forge_topo::mass::mass(&s.body).ok()?.volume.abs();
    }
    Some(vol)
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
fn along_tangents(edges: &[Edge], picked: &mut BTreeMap<Name, usize>) {
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
                    picked.insert(ge.h, g);
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

/// The pitch, height and hand of the helix a spine's stored shape holds (a shape binder keeps the
/// helix as it was when bound, which an edit of the helix since does not move): from its samples,
/// the rise along the axis through `o` along `ax` and the turn about it. The sweep starts where the
/// profile is and rises that far.
fn helix_turns(ws: Option<&Vec<Vec<Sampled>>>, o: V3, ax: V3) -> Result<(f64, f64, bool), Fail> {
    let mut pts: Vec<V3> = ws
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|e| e.1.iter().copied())
        .collect();
    if pts.len() < 8 {
        return Err(Fail::Unresolved(
            "a helix whose stored shape would not read".into(),
        ));
    }
    // one edge after another, each run its own way: the samples in order of their rise
    let rise = |p: &V3| dot(add(*p, o, -1.0), ax);
    pts.sort_by(|a, b| rise(a).partial_cmp(&rise(b)).unwrap_or(std::cmp::Ordering::Equal));
    let k = if ax.0.abs() < 0.9 {
        (1.0, 0.0, 0.0)
    } else {
        (0.0, 1.0, 0.0)
    };
    let Some(e1) = py::unit(add(k, ax, -dot(k, ax))) else {
        return bug("a helix of no axis");
    };
    let e2 = py::cross(ax, e1);
    let (mut hs, mut th): (Vec<f64>, Vec<f64>) = (vec![], vec![]);
    for p in &pts {
        let r = add(*p, o, -1.0);
        let mut a = py::atan2(dot(r, e2), dot(r, e1));
        if let Some(&last) = th.last() {
            while a - last > py::PI {
                a -= py::TAU;
            }
            while a - last < -py::PI {
                a += py::TAU;
            }
        }
        hs.push(dot(r, ax));
        th.push(a);
    }
    let (height, turn) = (hs[hs.len() - 1] - hs[0], th[th.len() - 1] - th[0]);
    if height <= 1e-9 || turn.abs() < 1e-6 {
        return unsupported("a pipe along a helix of no rise or turn");
    }
    let pitch = height * py::TAU / turn.abs();
    // the same pitch all the way: a taper or a varying pitch is no coil
    if hs
        .iter()
        .zip(&th)
        .any(|(h, t)| ((h - hs[0]) - (t - th[0]).abs() * pitch / py::TAU).abs() > 1e-4 * height.max(1.0))
    {
        return unsupported("a pipe along a helix of varying pitch");
    }
    let r9 = |x: f64| (x * 1e9).round() / 1e9;
    Ok((r9(pitch), r9(height), turn < 0.0))
}

/// The plane through points that are flat (within 1e-7 of it): its unit normal and a point.
fn plane_of(pts: &[V3]) -> Option<(V3, V3)> {
    if pts.len() < 3 {
        return None;
    }
    let k = pts.len() as f64;
    let c = pts.iter().fold((0.0, 0.0, 0.0), |s, p| {
        (s.0 + p.0 / k, s.1 + p.1 / k, s.2 + p.2 / k)
    });
    let far = |from: V3| {
        pts.iter()
            .copied()
            .fold((from, -1.0), |best, p| {
                let d = py::dist3(p, from);
                if d > best.1 { (p, d) } else { best }
            })
            .0
    };
    let a = far(c);
    let b = far(a);
    let ab = py::unit(add(b, a, -1.0))?;
    let off = |p: V3| py::dist3(add(a, ab, dot(add(p, a, -1.0), ab)), p);
    let e = pts
        .iter()
        .copied()
        .fold(
            (a, -1.0),
            |best, p| if off(p) > best.1 { (p, off(p)) } else { best },
        )
        .0;
    let n = py::unit(py::cross(add(b, a, -1.0), add(e, a, -1.0)))?;
    let span = py::dist3(a, b).max(1.0);
    if pts.iter().any(|&p| dot(add(p, a, -1.0), n).abs() > 1e-7 * span) {
        return None;
    }
    Some((n, a))
}

/// Does our edge (its samples `ours`) hold every sample of their edge `poly`? Ours are coarse:
/// their distance from our chords is allowed our chords' own bow.
fn contains(ours: &[V3], poly: &[V3]) -> bool {
    let tol = TOL + sag(ours);
    let lo = |f: fn(&V3) -> f64| ours.iter().map(f).fold(f64::INFINITY, pmin) - tol;
    let hi = |f: fn(&V3) -> f64| ours.iter().map(f).fold(f64::NEG_INFINITY, pmax) + tol;
    let (l, h) = (
        [lo(|q| q.0), lo(|q| q.1), lo(|q| q.2)],
        [hi(|q| q.0), hi(|q| q.1), hi(|q| q.2)],
    );
    let Some(&last) = poly.last() else { return false };
    poly.iter().step_by(4).chain([&last]).all(|p| {
        let q = [p.0, p.1, p.2];
        (0..3).all(|k| l[k] <= q[k] && q[k] <= h[k]) && near(*p, ours, tol) <= tol
    })
}

/// Is every sample of our edge on one of their edges?
fn covered(ours: &[V3], polys: &[&Vec<V3>]) -> bool {
    ours.iter().all(|&p| {
        polys.iter().any(|poly| {
            let tol = TOL + sag(poly);
            near(p, poly, tol) <= tol
        })
    })
}

/// Our edges where their edge (samples `poly`) lies: ours on theirs, or theirs a piece of ours.
fn edge_hits(edges: &[Edge], poly: &[V3]) -> Vec<usize> {
    let tol = TOL + sag(poly);
    let lo = |f: fn(&V3) -> f64| poly.iter().map(f).fold(f64::INFINITY, pmin) - tol;
    let hi = |f: fn(&V3) -> f64| poly.iter().map(f).fold(f64::NEG_INFINITY, pmax) + tol;
    let (l, h) = (
        [lo(|q| q.0), lo(|q| q.1), lo(|q| q.2)],
        [hi(|q| q.0), hi(|q| q.1), hi(|q| q.2)],
    );
    let inbox = |p: &V3| (0..3).all(|k| l[k] <= [p.0, p.1, p.2][k] && [p.0, p.1, p.2][k] <= h[k]);
    let on: Vec<usize> = (0..edges.len())
        .filter(|&i| {
            [4, 8, 12]
                .iter()
                .any(|&k| inbox(&edges[i].p[k]) && near(edges[i].p[k], poly, tol) <= tol)
        })
        .collect();
    if !on.is_empty() {
        return on;
    }
    (0..edges.len())
        .filter(|&i| contains(&edges[i].p, poly))
        .collect()
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
        let mut s = if doc.props.contains_key(&prof) {
            support_of(doc, &prof)?
        } else {
            String::new()
        };
        let bodied = doc.bodied.get().is_some_and(|b| b.contains(n));
        if s.is_empty() && !bodied && matches!(doc.ty(n), Some("PartDesign::Fillet" | "PartDesign::Chamfer"))
        {
            // a blend outside any body acts on its Base
            s = doc.link(n, "Base")?;
        }
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

/// The program's first statement: the world plane every later placement is made from (a sketch
/// with no rows: its plane, and its origin).
const PRELUDE: &str = "sketch ref(on: xy) {\n}\n";

/// A number as the program writes it, and as Forge reads it back.
fn back(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

fn v3(p: V3) -> Vec3 {
    Vec3::new(p.0, p.1, p.2)
}

/// A plane in the program: its text (reading the sketch's param `d` where it is moved off the
/// origin), the param's value, and the frame Forge makes of it.
struct Placed {
    on: String,
    d: Option<String>,
    frame: forge_math::Frame,
}

impl Placed {
    /// A world point in the plane's own coordinates.
    fn uv(&self, p: V3) -> V2 {
        let d = Point3::new(p.0, p.1, p.2) - self.frame.origin;
        (d.dot(self.frame.x().get()), d.dot(self.frame.y().get()))
    }
}

/// A sketch being written: its points and radii as its own params, read by its rows.
#[derive(Default)]
struct Sk {
    pts: Vec<String>,
    radii: Vec<String>,
    rows: Vec<String>,
    /// Further params: a spline's degree, knots and weights.
    more: Vec<(String, String)>,
}

impl Sk {
    fn p(&mut self, q: V2) -> String {
        let t = format!("({}, {})", num(q.0), num(q.1));
        let i = match self.pts.iter().position(|x| *x == t) {
            Some(i) => i,
            None => {
                self.pts.push(t);
                self.pts.len() - 1
            }
        };
        format!("p[{i}]")
    }

    /// A point with every digit (`p` writes ten decimals).
    fn p_exact(&mut self, q: V2) -> String {
        let z = |x: f64| if x == 0.0 { 0.0 } else { x };
        self.pts.push(format!("({:?}, {:?})", z(q.0), z(q.1)));
        format!("p[{}]", self.pts.len() - 1)
    }

    fn r(&mut self, r: f64) -> String {
        let t = num(r);
        let i = match self.radii.iter().position(|x| *x == t) {
            Some(i) => i,
            None => {
                self.radii.push(t);
                self.radii.len() - 1
            }
        };
        format!("r[{i}]")
    }

    /// A row, named by its kind and place: `l3`, `a4`, `c0`.
    fn row(&mut self, kind: &str, text: String) -> String {
        let name = format!("{kind}{}", self.rows.len());
        self.rows.push(format!("  {name} = {text}"));
        name
    }

    /// The sketch's statement and its params.
    fn block(&self, id: &str, at: &Placed) -> String {
        let mut out = String::new();
        if let Some(d) = &at.d {
            out += &format!("param {id}.d = {d}\n");
        }
        if !self.pts.is_empty() {
            out += &format!("param {id}.p = [{}]\n", self.pts.join(", "));
        }
        if !self.radii.is_empty() {
            out += &format!("param {id}.r = [{}]\n", self.radii.join(", "));
        }
        for (k, v) in &self.more {
            out += &format!("param {id}.{k} = {v}\n");
        }
        out += &format!("sketch {id}(on: {}) {{\n", at.on);
        for r in &self.rows {
            out += r;
            out.push('\n');
        }
        out + "}\n"
    }

    /// A loop of the arrangement, its points mapped into the plane by `to`; `turn` is -1 where
    /// the mapping mirrors (the plane faces against the sketch's normal).
    fn lp(&mut self, a: &Arrangement, lp: &[(usize, bool)], to: &dyn Fn(V2) -> V2, turn: f64) {
        if lp.len() == 1 && a.pieces[lp[0].0].kind == Kind::Circle {
            let p = &a.pieces[lp[0].0];
            let (c, r) = (self.p(to(p.c)), self.r(p.r));
            self.row("c", format!("circle({c}, {r})"));
            return;
        }
        for &(i, fwd) in lp {
            let p = &a.pieces[i];
            let (mut s, mut e) = (a.verts[p.va], a.verts[p.vb]);
            if !fwd {
                (s, e) = (e, s);
            }
            match p.kind {
                Kind::Arc => {
                    let (_, d) = p.sweep();
                    let d = if fwd { d } else { -d };
                    let mut c = p.c;
                    // refit through the shared ends
                    let ch = py::dist2(s, e);
                    if ch > 1e-9 {
                        let r = pmax(p.r, ch / 2.0);
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
                    let (cq, f, t) = (to(c), self.p(to(s)), self.p(to(e)));
                    // the ends as written: a centre rounded as they are can sit off their
                    // bisector by more than the solver allows; then it is the exact centre
                    // between the written ends, every digit written
                    let r =
                        |q: V2| -> V2 { (num(q.0).parse().unwrap_or(q.0), num(q.1).parse().unwrap_or(q.1)) };
                    let (rs, re) = (r(to(s)), r(to(e)));
                    let c = if (py::dist2(r(cq), rs) - py::dist2(r(cq), re)).abs() <= 1e-12 {
                        self.p(cq)
                    } else {
                        let ch = py::dist2(rs, re);
                        let rr = pmax(py::dist2(cq, rs), ch / 2.0);
                        let h = pmax(rr * rr - ch * ch / 4.0, 0.0).sqrt();
                        let m = ((rs.0 + re.0) / 2.0, (rs.1 + re.1) / 2.0);
                        let n = ((re.1 - rs.1) / ch, (rs.0 - re.0) / ch);
                        let (c1, c2) = ((m.0 + h * n.0, m.1 + h * n.1), (m.0 - h * n.0, m.1 - h * n.1));
                        let x = if py::dist2(c1, cq) <= py::dist2(c2, cq) {
                            c1
                        } else {
                            c2
                        };
                        self.p_exact(x)
                    };
                    let cw = if d * turn < 0.0 { ", cw: true" } else { "" };
                    self.row("a", format!("arc(center: {c}, from: {f}, to: {t}{cw})"));
                }
                Kind::Poly if p.src.is_some() => {
                    // the whole of a clamped NURBS: its poles, knots and weights, run the loop's way
                    let Some(nb) = &p.src else { continue };
                    let (mut poles, mut ws, mut ks) =
                        (nb.poles.clone(), nb.weights.clone(), nb.knots.clone());
                    if !fwd {
                        poles.reverse();
                        ws.reverse();
                        let (k0, k1) = (ks[0], ks[ks.len() - 1]);
                        ks = ks.iter().rev().map(|k| k0 + k1 - k).collect();
                    }
                    let n = poles.len();
                    poles[0] = s;
                    poles[n - 1] = e;
                    let ps: Vec<String> = poles.iter().map(|&q| self.p(to(q))).collect();
                    let k = self.rows.len();
                    let ratios = |xs: &[f64]| {
                        let r: Vec<String> = xs.iter().map(|&x| format!("ratio({})", num(x))).collect();
                        format!("[{}]", r.join(", "))
                    };
                    self.more
                        .push((format!("sd{k}"), format!("count({})", nb.degree)));
                    self.more.push((format!("sk{k}"), ratios(&ks)));
                    let mut t = format!("spline([{}], degree: sd{k}, knots: sk{k}", ps.join(", "));
                    if ws.iter().any(|&w| w != 1.0) {
                        self.more.push((format!("sw{k}"), ratios(&ws)));
                        t += &format!(", weights: sw{k}");
                    }
                    self.row("s", t + ")");
                }
                Kind::Poly => {
                    let mut ps: Vec<V2> = p.pts.clone();
                    if !fwd {
                        ps.reverse();
                    }
                    let n = ps.len();
                    ps[0] = s;
                    ps[n - 1] = e;
                    for w in ps.windows(2) {
                        let (f, t) = (self.p(to(w[0])), self.p(to(w[1])));
                        if f != t {
                            self.row("l", format!("line({f}, {t})"));
                        }
                    }
                }
                _ => {
                    let (f, t) = (self.p(to(s)), self.p(to(e)));
                    self.row("l", format!("line({f}, {t})"));
                }
            }
        }
    }
}

/// How far an extrude runs.
enum Ext {
    D(f64),
    Sym(f64),
    Two(f64, f64),
    Thru,
    ThruAll,
    Next,
    /// Walls slanted along `w` (scaled so a step of 1 along it is 1 along the travel): the
    /// profile from `from` to `to` along it, a ruled loft between its two ends.
    Along {
        w: V3,
        from: f64,
        to: f64,
    },
    /// Through the part with walls slanted along `w` (both ways with `both`): `thru` or
    /// `thru_all` along it.
    AlongThru {
        w: V3,
        both: bool,
    },
}

/// What a profile makes: an extrude (or cut) running along `travel`, or a revolve about an axis
/// in the sketch's coordinates (a point and a direction), by an angle (none: a whole turn),
/// symmetric about the sketch's plane or not.
enum Make {
    Extrude {
        cut: bool,
        ext: Ext,
        travel: V3,
    },
    Revolve {
        cut: bool,
        at: V2,
        dir: V2,
        angle: Option<f64>,
        sym: bool,
    },
    /// Wound about an axis in the sketch's coordinates, rising along it.
    Coil {
        cut: bool,
        at: V2,
        dir: V2,
        pitch: f64,
        height: f64,
        left: bool,
    },
}

/// A turn of `p` about the axis through `o` along unit `k` by `a` radians (right-handed).
fn turned(p: V3, o: V3, k: V3, a: f64) -> V3 {
    let v = add(p, o, -1.0);
    let (s, c) = (py::sin(a), py::cos(a));
    let kv = py::cross(k, v);
    let kd = dot(k, v);
    let r = (
        v.0 * c + kv.0 * s + k.0 * kd * (1.0 - c),
        v.1 * c + kv.1 * s + k.1 * kd * (1.0 - c),
        v.2 * c + kv.2 * s + k.2 * kd * (1.0 - c),
    );
    add(o, r, 1.0)
}

struct Tr<'d, 'a, 'i> {
    doc: &'d Doc<'a, 'i>,
    truth: HashMap<String, Stored>,
    /// The program's statements after the prelude, a feature (with its sketch and params) each.
    blocks: Vec<String>,
    /// Each feature's label: the ids its statements made, space apart.
    labels: HashMap<String, String>,
    /// World points written as helper sketches' points, by their text.
    points: HashMap<String, String>,
    state: State,
    bodies: usize,
    /// The body the chain's features act on: the feature that made it.
    body: Option<String>,
    approx: bool,
    tree: usize,
    /// Features made, for fresh ids.
    made: usize,
    chain_rest: Vec<String>,
    /// The feature ids written so far (`t3`, `m4x0`; not their `x1`, `_sk` kin), for `fresh`.
    taken: HashSet<String>,
}

fn unsupported<T>(s: impl Into<String>) -> Result<T, Fail> {
    Err(Fail::Unsupported(s.into()))
}

/// Several ids as one value: a name, or a list of them.
fn ids(labels: &str) -> String {
    let v: Vec<&str> = labels.split(' ').filter(|s| !s.is_empty()).collect();
    match v[..] {
        [one] => one.into(),
        _ => format!("[{}]", v.join(", ")),
    }
}

impl Tr<'_, '_, '_> {
    /// `id`, or with a suffix where a statement already has it: a Forge id is bound once, and a
    /// feature stepped again (a chain two bodies of a Part tree share) makes its statements anew.
    fn fresh(&self, id: String) -> String {
        let (mut out, mut k) = (id.clone(), 0);
        while self.taken.contains(&out) {
            k += 1;
            out = format!("{id}r{k}");
        }
        out
    }

    /// The program so far.
    fn program(&self) -> String {
        let mut s = String::from(PRELUDE);
        for b in &self.blocks {
            s += b;
        }
        s
    }

    /// A world point, as a helper sketch's point (`q3.p`), or the origin.
    fn point(&mut self, p: V3) -> String {
        let (x, y, z) = (num(p.0), num(p.1), num(p.2));
        if [&x, &y, &z].iter().all(|t| *t == "0") {
            return "ref.origin".into();
        }
        let key = format!("{x} {y} {z}");
        if let Some(n) = self.points.get(&key) {
            return format!("{n}.p");
        }
        let n = format!("q{}", self.points.len());
        let mut b = format!("param {n}.q = ({x}, {y})\n");
        let on = if z == "0" {
            "ref".to_string()
        } else {
            b += &format!("param {n}.h = {z}\n");
            "offset(ref, h)".to_string()
        };
        b += &format!("sketch {n}(on: {on}) {{\n  p = pt(q[0], q[1])\n}}\n");
        self.blocks.push(b);
        self.points.insert(key, n.clone());
        format!("{n}.p")
    }

    /// A world direction: the axis from the origin through its tip.
    fn dir(&mut self, v: V3) -> Result<String, Fail> {
        let Some(u) = py::unit(v) else {
            return bug("a zero direction");
        };
        // a long direction keeps its digits when its tip is written to ten decimals
        Ok(format!("axis(ref.origin, {})", self.point(u)))
    }

    /// The plane through `o` facing `n`, as Forge places it: the world plane moved, or a plane
    /// through the origin facing the direction, moved along it.
    fn plane(&mut self, n: V3, o: V3) -> Result<Placed, Fail> {
        let Some(u) = py::unit(n) else {
            return bug("a zero normal");
        };
        let tip = (back(&num(u.0)), back(&num(u.1)), back(&num(u.2)));
        let Some(z) = v3(tip).normalize() else {
            return bug("a zero normal");
        };
        let dt = num(dot(o, (z.get().x, z.get().y, z.get().z)));
        let dv = back(&dt);
        let moved = dv != 0.0;
        let (base, frame) = if tip.0 == 0.0 && tip.1 == 0.0 && tip.2 > 0.0 {
            ("ref".to_string(), forge_math::Frame::WORLD)
        } else {
            let q = self.point(tip);
            (
                format!("plane(at: ref.origin, normal: axis(ref.origin, {q}))"),
                forge_math::Frame::from_z(Point3::ORIGIN, z),
            )
        };
        let frame = frame.with_origin(Point3::ORIGIN + z.get() * dv);
        Ok(Placed {
            on: if moved { format!("offset({base}, d)") } else { base },
            d: moved.then_some(dt),
            frame,
        })
    }

    /// The id of a statement this feature makes: `t3`, then `t3x1`, `t3x2` for its others.
    fn id(lab: &str, k: usize) -> String {
        if k == 0 { lab.into() } else { format!("{lab}x{k}") }
    }

    fn profile_ents(&mut self, sk: &str, fr: &Frame) -> Result<Vec<Ent>, Fail> {
        if self.doc.ty(sk) == Some("Sketcher::SketchObject") {
            return sketch_ents(self.doc, sk);
        }
        let Some(ws) = self.truth.get(sk).and_then(|s| s.wires.as_ref()) else {
            return Err(Fail::Unresolved(
                "a profile whose stored wires would not read".into(),
            ));
        };
        // a stored shape placed only by its own location, which is the profile's placement: its
        // own coordinates are the sketch's
        let local = self.truth[sk].root.is_some_and(|m| {
            let cols = [fr.x, fr.y, fr.n, fr.o];
            (0..3).all(|i| {
                (0..4).all(|j| {
                    let c = [cols[j].0, cols[j].1, cols[j].2][i];
                    let tol = if j == 3 { pmax(1.0, c.abs()) } else { 1.0 };
                    (m[i][j] - c).abs() <= 1e-7 * tol
                })
            })
        }) && ws.iter().flatten().all(|e| e.2.is_some());
        let mut ents = vec![];
        let mut approx = false;
        for w in ws {
            for (k, p, l) in w {
                let uv: Vec<V2> = match l {
                    Some(l) if local => l.iter().map(|q| (q.0, q.1)).collect(),
                    _ => p
                        .iter()
                        .map(|&q| {
                            let r = fr.uvw(q);
                            (r.0, r.1)
                        })
                        .collect(),
                };
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

    /// A sketch's closed regions, each a sketch and the feature it makes, labelled `lab` and on;
    /// the ids made.
    fn profile(&mut self, name: &str, lab: &str, make: Make, sk: Option<&str>) -> Result<Vec<String>, Fail> {
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
        // the sketch's points in the world, and where a revolve's axis lies
        let (mut world_of, mut normal): (Box<dyn Fn(V2) -> V3>, V3) =
            (Box::new(move |q: V2| fr.world(q.0, q.1)), fr.n);
        let mut axis = None;
        if let Make::Revolve {
            at, dir, angle, sym, ..
        } = &make
        {
            let (o, k) = (fr.world(at.0, at.1), add(fr.world(dir.0, dir.1), fr.o, -1.0));
            let Some(k) = py::unit(k) else {
                return bug("a revolve about a zero axis");
            };
            if *sym {
                // symmetric about the sketch's plane: the sketch turned back half the angle first
                let half = -angle.unwrap_or(360.0) * py::PI / 360.0;
                world_of = Box::new(move |q: V2| turned(fr.world(q.0, q.1), o, k, half));
                normal = add(
                    turned(add(fr.o, fr.n, 1.0), o, k, half),
                    turned(fr.o, o, k, half),
                    -1.0,
                );
            }
            axis = Some((o, k));
        }
        if let Make::Coil { at, dir, .. } = &make {
            let (o, k) = (fr.world(at.0, at.1), add(fr.world(dir.0, dir.1), fr.o, -1.0));
            let Some(k) = py::unit(k) else {
                return bug("a coil about a zero axis");
            };
            axis = Some((o, k));
        }
        let mut make = make;
        if let Make::Extrude {
            cut: true,
            ext: Ext::Two(f, b),
            travel,
        } = make
        {
            // Forge's two-sided extent runs a cut's way only as lengths along the normal, which the
            // kernel refuses: the same prism, cut from where its back end lies
            world_of = Box::new(move |q: V2| add(fr.world(q.0, q.1), travel, -b));
            make = Make::Extrude {
                cut: true,
                ext: Ext::D(f + b),
                travel,
            };
        }
        let (cut, travel) = match &make {
            Make::Extrude { cut, travel, .. } => (*cut, *travel),
            Make::Revolve { cut, .. } | Make::Coil { cut, .. } => (*cut, normal),
        };
        // an extrude runs along its sketch's normal, a cut against it
        let n = if matches!(make, Make::Extrude { .. }) && cut {
            neg(travel)
        } else {
            travel
        };
        let mut slant = None;
        if let Make::Extrude {
            ext: Ext::Along { w, from, to },
            ..
        } = make
        {
            let w0 = (w.0 * from, w.1 * from, w.2 * from);
            world_of = Box::new(move |q: V2| add(fr.world(q.0, q.1), w0, 1.0));
            slant = Some((w.0 * (to - from), w.1 * (to - from), w.2 * (to - from)));
        }
        let at = self.plane(n, world_of((0.0, 0.0)))?;
        let along = match &make {
            Make::Extrude {
                ext: Ext::AlongThru { w, .. },
                ..
            } => Some(self.dir(*w)?),
            _ => None,
        };
        let at2 = match slant {
            Some(t) => Some(self.plane(n, add(world_of((0.0, 0.0)), t, 1.0))?),
            None => None,
        };
        let turn = if dot(n, normal) < 0.0 { -1.0 } else { 1.0 };
        let mut out = vec![];
        for (k, prof) in a.merged(&regs)?.into_iter().enumerate() {
            let id = Self::id(lab, k);
            let to = |q: V2| at.uv(world_of(q));
            let mut s = Sk::default();
            for lp in &prof {
                s.lp(&a, lp, &to, turn);
                // a spline the arrangement cut, or one not clamped, is written as a polyline
                if lp
                    .iter()
                    .any(|&(i, _)| a.pieces[i].kind == Kind::Poly && a.pieces[i].src.is_none())
                {
                    self.approx = true;
                }
            }
            if let Some((o, k)) = axis {
                let (p, q) = (s.p(at.uv(o)), s.p(at.uv(add(o, k, 1.0))));
                s.rows.push(format!("  ax = line({p}, {q}, construction: true)"));
            }
            let mut new = false;
            if self.bodies == 0 {
                if cut {
                    return unsupported("first feature cuts");
                }
                new = self.tree > 0;
                self.bodies = 1;
                self.body = Some(id.clone());
            }
            let skid = format!("{id}_sk");
            let mut b = s.block(&skid, &at);
            let mut params = vec![];
            let mut opts = String::new();
            let call = match &make {
                Make::Extrude { ext, .. } => {
                    let e = match ext {
                        Ext::D(l) => {
                            params.push(("d", num(*l)));
                            "d".to_string()
                        }
                        Ext::Sym(l) => {
                            params.push(("d", num(*l)));
                            "sym(d)".into()
                        }
                        Ext::Two(f, r) => {
                            params.extend([("d", num(*f)), ("back", num(*r))]);
                            "two(d, back)".into()
                        }
                        Ext::Thru => "thru".into(),
                        Ext::ThruAll => "thru_all".into(),
                        Ext::Next => "next".into(),
                        Ext::Along { .. } => String::new(),
                        Ext::AlongThru { both, .. } => {
                            if let Some(d) = &along {
                                opts += &format!(", along: {d}");
                            }
                            if *both { "thru_all" } else { "thru" }.into()
                        }
                    };
                    match (&at2, slant) {
                        (Some(at2), Some(t)) => {
                            // the same loops at the far end, a ruled loft between
                            let to2 = |q: V2| at2.uv(add(world_of(q), t, 1.0));
                            let mut s2 = Sk::default();
                            for lp in &prof {
                                s2.lp(&a, lp, &to2, turn);
                            }
                            b += &s2.block(&format!("{id}_end"), at2);
                            if cut {
                                opts += ", cut: true";
                            }
                            format!("loft([{skid}, {id}_end]")
                        }
                        _ => format!("{}({skid}, {e}", if cut { "cut" } else { "extrude" }),
                    }
                }
                Make::Revolve { angle, .. } => {
                    if let Some(t) = angle {
                        params.push(("a", format!("{}deg", num(*t))));
                        opts += ", angle: a";
                    }
                    if cut {
                        opts += ", cut: true";
                    }
                    format!("revolve({skid}, axis: {skid}.ax")
                }
                Make::Coil {
                    pitch, height, left, ..
                } => {
                    params.extend([("pitch", num(*pitch)), ("h", num(*height))]);
                    if *left {
                        opts += ", left: true";
                    }
                    if cut {
                        opts += ", cut: true";
                    }
                    format!("coil({skid}, axis: {skid}.ax, pitch: pitch, height: h")
                }
            };
            if new {
                opts += ", new: true";
            }
            for (k, v) in params {
                b += &format!("param {id}.{k} = {v}\n");
            }
            b += &format!("{id} = {call}{opts})\n");
            self.blocks.push(b);
            out.push(id);
        }
        Ok(out)
    }

    fn pad(&mut self, name: &str, lab: &str, cut: bool) -> Result<Vec<String>, Fail> {
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
                // Forge's extrude runs square to its sketch: walls along a slant are a ruled loft
                // between the profile and its copy moved along it, each end the length along the
                // normal it would have had
                let s = if dot(v, z) < 0.0 { -1.0 } else { 1.0 } / c.abs();
                let w = (v.0 * s, v.1 * s, v.2 * s);
                let (from, to) = match t {
                    0 if mid => (-l / 2.0, l / 2.0),
                    0 => (0.0, l),
                    4 => (-l2, l),
                    // a pocket through: literally through, its walls along the slant
                    _ if cut => {
                        let ext = Ext::AlongThru { w, both: mid };
                        return self.profile(name, lab, Make::Extrude { cut, ext, travel: z }, None);
                    }
                    _ => {
                        // through: past every edge of the part so far
                        let src = self.program();
                        let reach = self.state.reach(&src)? / c.abs() + 1.0;
                        if mid { (-reach, reach) } else { (0.0, reach) }
                    }
                };
                let ext = Ext::Along { w, from, to };
                return self.profile(name, lab, Make::Extrude { cut, ext, travel: z }, None);
            }
        }
        let ext = match t {
            0 if mid => Ext::Sym(l),
            0 => Ext::D(l),
            4 => Ext::Two(l, l2),
            1 if cut && mid => Ext::ThruAll,
            1 if cut => Ext::Thru,
            2 => Ext::Next,
            _ => return unsupported(format!("a pad or pocket of type {t}")),
        };
        self.profile(name, lab, Make::Extrude { cut, ext, travel: z }, None)
    }

    fn axis_tail(&self, fr: &Frame, base: V3, ax: V3) -> Result<(V2, V2), Fail> {
        let (u, v, w) = fr.uvw(base);
        let (du, dv, dw) = (dot(ax, fr.x), dot(ax, fr.y), dot(ax, fr.n));
        if w.abs() > 1e-6 || dw.abs() > 1e-6 {
            return unsupported("a revolve axis out of the sketch's plane");
        }
        Ok(((u, v), (du, dv)))
    }

    fn revolve(&mut self, name: &str, lab: &str, cut: bool) -> Result<Vec<String>, Fail> {
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
        let (at, mut dir) = self.axis_tail(&fr, base, ax)?;
        if d.boolean(name, "Reversed")? {
            dir = (-dir.0, -dir.1);
        }
        let ang = d.flt(name, "Angle", 360.0)?;
        let whole = (ang - 360.0).abs() < 1e-9;
        let make = Make::Revolve {
            cut,
            at,
            dir,
            angle: (!whole).then_some(ang),
            sym: !whole && d.boolean(name, "Midplane")?,
        };
        self.profile(name, lab, make, None)
    }

    /// Part workbench: a sketch turned about Axis through Base.
    /// A pipe along a helix in Frenet mode, as a coil: the Frenet frame of a helix turns with the
    /// screw motion, so the profile is wound rigidly about the helix's axis.
    fn pipe(&mut self, name: &str, lab: &str, cut: bool) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        if d.prop(name, "Sections")?
            .is_some_and(|p| p.descendants().any(|n| n.has_tag_name("Link")))
        {
            return unsupported("a pipe through sections");
        }
        for k in ["AuxillerySpine", "AuxiliarySpine"] {
            if !d.link(name, k)?.is_empty() {
                return unsupported("a pipe with an auxiliary spine");
            }
        }
        let sp = d.link(name, "Spine")?;
        let helix = match d.ty(&sp) {
            Some("Part::Helix") => sp.clone(),
            Some("PartDesign::ShapeBinder") => {
                let got: Vec<String> = d
                    .prop(&sp, "Support")?
                    .map(|p| {
                        p.descendants()
                            .filter(|n| n.has_tag_name("Link"))
                            .filter_map(|n| n.attribute("obj").or(n.attribute("value")).map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                match &got[..] {
                    [h] if d.ty(h) == Some("Part::Helix") && d.placement(&sp)? == d.placement(h)? => {
                        h.clone()
                    }
                    _ => {
                        let w: Vec<&str> = got.iter().map(|g| d.ty(g).map_or("None", last_word)).collect();
                        return unsupported(format!("a pipe along a shape binder of {}", w.join(", ")));
                    }
                }
            }
            t => return unsupported(format!("a pipe along {}", t.map_or("None", last_word))),
        };
        if d.integer(name, "Mode", 0)? != 2 {
            return unsupported("a pipe along a helix, not Frenet");
        }
        if d.flt(&helix, "Angle", 0.0)?.abs() > 1e-12 || d.integer(&helix, "Style", 1)? != 1 {
            return unsupported("a pipe along a tapered or old-style helix");
        }
        let sk = d.link(name, "Profile")?;
        if !is(FLAT, d.ty(&sk)) {
            return unsupported("a profile not from a sketch");
        }
        let fr = Frame::new(d.placement(&sk)?);
        let (o, q) = d.placement(&helix)?;
        let ax = rotate(q, (0.0, 0.0, 1.0));
        let (u, v, w) = fr.uvw(o);
        let (du, dv, dw) = (dot(ax, fr.x), dot(ax, fr.y), dot(ax, fr.n));
        if w.abs() > 1e-6 || dw.abs() > 1e-9 {
            return unsupported("a helix axis out of the profile's plane");
        }
        let ws = self.truth.get(&sp).and_then(|s| s.wires.as_ref());
        let (pitch, height, left) = helix_turns(ws, o, ax)?;
        let make = Make::Coil {
            cut,
            at: (u, v),
            dir: (du, dv),
            pitch,
            height,
            left,
        };
        self.profile(name, lab, make, Some(&sk))
    }

    fn part_revolve(&mut self, name: &str, lab: &str) -> Result<Vec<String>, Fail> {
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
        let (at, dir) = self.axis_tail(&fr, base, ax)?;
        let ang = d.flt(name, "Angle", 360.0)?;
        let whole = (ang - 360.0).abs() < 1e-9;
        let make = Make::Revolve {
            cut: false,
            at,
            dir,
            angle: (!whole).then_some(ang),
            sym: !whole && d.boolean(name, "Symmetric")?,
        };
        self.profile(name, lab, make, Some(&sk))
    }

    /// Part workbench: a sketch's faces moved along Dir. Older files have no LengthFwd: Dir's own
    /// length is the distance. Newer ones run LengthFwd along it (and LengthRev back), Dir's
    /// length when both are zero.
    fn part_extrude(&mut self, name: &str, lab: &str) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        if !d.boolean(name, "Solid")? {
            return unsupported("a Part extrusion that makes a shell");
        }
        if d.flt(name, "TaperAngle", 0.0)?.abs() > 1e-12 || d.flt(name, "TaperAngleRev", 0.0)?.abs() > 1e-12 {
            return unsupported("a tapered Part extrusion");
        }
        let (o, q) = d.placement(name)?;
        if [o.0, o.1, o.2].iter().any(|x| x.abs() > 1e-9) || (q.3.abs() - 1.0).abs() > 1e-9 {
            return unsupported("a moved Part extrusion");
        }
        let sk = d.link(name, "Base")?;
        if !is(FLAT, d.ty(&sk)) {
            return unsupported("a Part extrusion of something not a sketch");
        }
        let n = Frame::new(d.placement(&sk)?).n;
        let mut v = d.vec_prop(name, "Dir")?;
        let mut lv = dot(v, v).sqrt();
        let (mut fwd, mut rev, mut sym) = (lv, 0.0, false);
        if d.prop(name, "LengthFwd")?.is_some() {
            match d.integer(name, "DirMode", 0)? {
                0 => {}
                2 => (v, lv) = (n, 1.0),
                _ => return unsupported("a Part extrusion along an edge"),
            }
            (fwd, rev, sym) = (
                d.flt(name, "LengthFwd", 0.0)?,
                d.flt(name, "LengthRev", 0.0)?,
                d.boolean(name, "Symmetric")?,
            );
            if fwd.abs() < 1e-12 && rev.abs() < 1e-12 {
                fwd = lv;
            }
            if d.boolean(name, "Reversed")? {
                v = neg(v);
            }
        }
        let Some(u) = py::unit(v).filter(|_| lv >= 1e-12) else {
            return unsupported("a Part extrusion of no direction");
        };
        let c = dot(u, n);
        if c.abs() < 1e-9 {
            return unsupported("a Part extrusion along its sketch's plane");
        }
        let z = if c < 0.0 { neg(n) } else { n };
        // the distances run along Dir; Forge measures them along the normal
        let (fwd, rev) = (fwd * c.abs(), rev * c.abs());
        let ext = if (c.abs() - 1.0).abs() > 1e-12 {
            // walls along a slant: a ruled loft, as a slanted pad's
            let w = (u.0 / c.abs(), u.1 / c.abs(), u.2 / c.abs());
            let (from, to) = if sym { (-fwd / 2.0, fwd / 2.0) } else { (-rev, fwd) };
            Ext::Along { w, from, to }
        } else if sym {
            Ext::Sym(fwd)
        } else if rev > 1e-12 {
            Ext::Two(fwd, rev)
        } else {
            Ext::D(fwd)
        };
        self.profile(
            name,
            lab,
            Make::Extrude {
                cut: false,
                ext,
                travel: z,
            },
            Some(&sk),
        )
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
    /// base is the last body made and the features after it act on that. Returns the result's
    /// label.
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
            let id = format!("join{}", self.made);
            self.made += 1;
            self.blocks.push(format!(
                "{id} = combine({base}, tools: {}, op: \"{word}\")\n",
                ids(&labs.join(" "))
            ));
            self.body = Some(base.clone());
            return Ok(base);
        }
        if t == "PartDesign::Body" || SOLID.contains(&t) && !t.starts_with("Part::") {
            // a PartDesign chain as one shape of the tree: a body of its own, what follows it acting on it
            let sub = chain_of(d, name)?;
            self.tree += 1;
            self.bodies = 0;
            self.body = None;
            for n in &sub {
                self.step(n)?;
            }
            let Some(head) = self.body.clone() else {
                return bug("a chain that made no body");
            };
            self.bodies = 1;
            return Ok(head);
        }
        self.tree += 1;
        let lab = format!("p{}", self.tree);
        let new = self.tree > 1;
        let ls = match t {
            "Part::Revolution" => {
                // after the first, each its own body
                if new {
                    self.bodies = 0;
                }
                self.part_revolve(name, &lab)?
            }
            "Part::Extrusion" => {
                if new {
                    self.bodies = 0;
                }
                self.part_extrude(name, &lab)?
            }
            "Part::Box" | "Part::Cylinder" => self.part_solid(name, t, &lab, new)?,
            "Part::Sphere" => {
                if (d.flt(name, "Angle1", -90.0)? + 90.0).abs() > 1e-9
                    || (d.flt(name, "Angle2", 90.0)? - 90.0).abs() > 1e-9
                    || (d.flt(name, "Angle3", 360.0)? - 360.0).abs() > 1e-9
                {
                    return unsupported("a part of a sphere");
                }
                let fr = Frame::new(d.placement(name)?);
                let c = self.point(fr.o);
                let opt = if new { ", new: true" } else { "" };
                self.blocks.push(format!(
                    "param {lab}.r = {}\n{lab} = ball({c}, r{opt})\n",
                    num(d.flt(name, "Radius", 0.0)?)
                ));
                self.bodies = 1;
                vec![lab.clone()]
            }
            _ => {
                let t = d.ty(name).unwrap_or("None");
                return unsupported(format!("a Part tree through {t}"));
            }
        };
        if ls.len() != 1 {
            return unsupported("a Part solid of several regions");
        }
        self.body = Some(lab.clone());
        Ok(lab)
    }

    /// A rectangle (`Length` by `Width`) or circle (`Radius`) on a placement, extruded `Height`
    /// along its z: Part and PartDesign primitives.
    fn box_or_cylinder(&mut self, name: &str, lab: &str, cut: bool, new: bool) -> Result<String, Fail> {
        let d = self.doc;
        let t = d.ty(name).unwrap_or("");
        let fr = Frame::new(d.placement(name)?);
        let at = self.plane(if cut { neg(fr.n) } else { fr.n }, fr.o)?;
        let mut s = Sk::default();
        if t.ends_with("Box") {
            let (l, w) = (d.flt(name, "Length", 0.0)?, d.flt(name, "Width", 0.0)?);
            let c: Vec<String> = [(0.0, 0.0), (l, 0.0), (l, w), (0.0, w)]
                .iter()
                .map(|q| s.p(at.uv(fr.world(q.0, q.1))))
                .collect();
            for k in 0..4 {
                s.row("l", format!("line({}, {})", c[k], c[(k + 1) % 4]));
            }
        } else {
            let (c, r) = (s.p(at.uv(fr.o)), s.r(d.flt(name, "Radius", 0.0)?));
            s.row("c", format!("circle({c}, {r})"));
        }
        let skid = format!("{lab}_sk");
        let mut b = s.block(&skid, &at);
        b += &format!("param {lab}.d = {}\n", num(d.flt(name, "Height", 0.0)?));
        let opt = if new { ", new: true" } else { "" };
        b += &format!(
            "{lab} = {}({skid}, d{opt})\n",
            if cut { "cut" } else { "extrude" }
        );
        self.blocks.push(b);
        Ok(lab.into())
    }

    /// Part workbench primitives, on their placement.
    fn part_solid(&mut self, name: &str, t: &str, lab: &str, new: bool) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        if t == "Part::Cylinder" && (d.flt(name, "Angle", 360.0)? - 360.0).abs() > 1e-9 {
            return unsupported("a part of a cylinder");
        }
        self.bodies = 1;
        Ok(vec![self.box_or_cylinder(name, lab, false, new)?])
    }

    fn blend(
        &mut self,
        name: &str,
        lab: &str,
        chamfer: bool,
        size: f64,
        all_edges: bool,
    ) -> Result<Vec<String>, Fail> {
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
        let edges = self.state.edges(&self.program())?;
        let mut picked: BTreeMap<Name, usize> = BTreeMap::new();
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
                        let close: HashSet<Name> = scored
                            .iter()
                            .filter(|x| x.0 <= s0 + 1e-9)
                            .map(|x| edges[x.1].h)
                            .collect();
                        hits = (0..edges.len())
                            .filter(|&i| close.contains(&edges[i].h))
                            .collect();
                    }
                }
                if hits.is_empty() {
                    // theirs a piece of one of ours (a loop they split at a vertex we do not have):
                    // every sample of theirs on our edge, and our edge covered by the edges they
                    // pick, so the blend rounds no more than theirs
                    let every: Vec<&Vec<V3>> = subs
                        .iter()
                        .filter_map(|x| self.truth.get(&base).and_then(|s| s.subs.get(x)))
                        .flatten()
                        .collect();
                    hits = (0..edges.len())
                        .filter(|&i| contains(&edges[i].p, poly) && covered(&edges[i].p, &every))
                        .collect();
                }
                if hits.is_empty() && !all_edges {
                    return Err(Fail::Unresolved("no edge where the base's edge lies".into()));
                }
                for i in hits {
                    if edges[i].c != "smooth" {
                        picked.insert(edges[i].h, i);
                    }
                }
            }
        }
        if picked.is_empty() {
            return Err(Fail::Unresolved("no edge where the base's edge lies".into()));
        }
        // OpenCascade carries a blend along tangent edges
        along_tangents(&edges, &mut picked);
        let want: BTreeSet<Name> = picked.keys().copied().collect();
        // the features so far, any of which may have made an edge's face
        let mut feats: Vec<String> = self
            .labels
            .values()
            .flat_map(|l| l.split(' ').map(String::from))
            .collect();
        feats.sort();
        feats.dedup();
        let (sel, params) = self.state.select(&want, &feats)?;
        let mut b = format!(
            "param {lab}.{} = {}\n",
            if chamfer { "d" } else { "r" },
            num(size)
        );
        for (k, v) in params {
            b += &format!("param {lab}.{k} = {}\n", num(v));
        }
        let (word, arg) = if chamfer {
            ("chamfer", "d")
        } else {
            ("fillet", "r")
        };
        b += &format!("{lab} = {word}({sel}, {arg})\n");
        self.blocks.push(b);
        Ok(vec![lab.into()])
    }

    /// Our face bounded by their face's edges (samples `polys`): the one face every one of those
    /// edges borders, or where another shares them all (a disc on a cylinder's end), the one
    /// bounded by nothing else.
    fn face_of(edges: &[Edge], polys: &[Vec<V3>]) -> Result<Name, Fail> {
        let mut common: Option<BTreeSet<Name>> = None;
        let mut matched = HashSet::new();
        for poly in polys {
            let mut fs = BTreeSet::new();
            for i in edge_hits(edges, poly) {
                fs.extend(edges[i].f.iter().copied());
                matched.insert(edges[i].h);
            }
            let c = match common {
                Some(c) => c.intersection(&fs).copied().collect(),
                None => fs,
            };
            let empty = c.is_empty();
            common = Some(c);
            if empty {
                break;
            }
        }
        let mut common = common.unwrap_or_default();
        if common.is_empty() {
            return Err(Fail::Unresolved("no face where the base's face lies".into()));
        }
        if common.len() > 1 {
            common.retain(|f| {
                edges
                    .iter()
                    .filter(|e| e.f.contains(f))
                    .all(|e| matched.contains(&e.h))
            });
        }
        match common.into_iter().collect::<Vec<_>>()[..] {
            [f] => Ok(f),
            _ => Err(Fail::Unresolved("a face its edges do not settle".into())),
        }
    }

    /// A hollow: the faces of Base opened, walls Value thick, inward when Reversed (FreeCAD's
    /// "inwards"), as `shell(t, open: FACES, out:)`. Only the skin mode.
    fn thickness(&mut self, name: &str, lab: &str) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        let mode = d.integer(name, "Mode", 0)?;
        if mode != 0 {
            return unsupported(format!("a thickness of mode {mode}"));
        }
        let base = d.link(name, "Base")?;
        let edges = self.state.edges(&self.program())?;
        let mut want = BTreeSet::new();
        for sub in d.subs(name, "Base")?.into_iter().filter(|x| !x.is_empty()) {
            let polys = self
                .truth
                .get(&base)
                .and_then(|s| s.subs.get(&sub))
                .filter(|_| sub.starts_with("Face"));
            let Some(polys) = polys.cloned() else {
                let s: String = sub.chars().take(4).collect();
                return Err(Fail::Unresolved(format!("a thickness on {s} not sampled")));
            };
            want.insert(Self::face_of(&edges, &polys)?);
        }
        if want.is_empty() {
            return unsupported("a thickness with no open face");
        }
        let mut feats: Vec<String> = self
            .labels
            .values()
            .flat_map(|l| l.split(' ').map(String::from))
            .collect();
        feats.sort();
        feats.dedup();
        let sel = self.state.select_faces(&want, &feats)?;
        let out = if d.boolean(name, "Reversed")? {
            ""
        } else {
            ", out: true"
        };
        self.blocks.push(format!(
            "param {lab}.t = {}\n{lab} = shell(t, open: {sel}{out})\n",
            num(d.flt(name, "Value", 0.0)?)
        ));
        Ok(vec![lab.into()])
    }

    /// Faces tilted about where they cross a neutral plane, the pull along the plane's outward
    /// normal (or a line edge's way), turned by Reversed: FreeCAD's BRepOffsetAPI_DraftAngle.
    fn draft(&mut self, name: &str, lab: &str) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        let base = d.link(name, "Base")?;
        let sampled = |o: &str, sub: &str| self.truth.get(o).and_then(|s| s.subs.get(sub)).cloned();
        let nb = d.link(name, "NeutralPlane")?;
        let ns: Vec<String> = d
            .subs(name, "NeutralPlane")?
            .into_iter()
            .filter(|x| !x.is_empty())
            .collect();
        if nb != base || ns.len() != 1 || !ns[0].starts_with("Face") {
            return unsupported("a draft's neutral plane not a face of its base");
        }
        let edges = self.state.edges(&self.program())?;
        let mut want = BTreeSet::new();
        for sub in d.subs(name, "Base")?.into_iter().filter(|x| !x.is_empty()) {
            let polys = sampled(&base, &sub).filter(|_| sub.starts_with("Face"));
            let Some(polys) = polys else {
                let s: String = sub.chars().take(4).collect();
                return Err(Fail::Unresolved(format!("a draft on {s} not sampled")));
            };
            want.insert(Self::face_of(&edges, &polys)?);
        }
        if want.is_empty() {
            return Err(Fail::Unresolved("a draft of no faces".into()));
        }
        let Some(np) = sampled(&base, &ns[0]) else {
            return Err(Fail::Unresolved("a draft's neutral face not sampled".into()));
        };
        let Some(pl) = self.state.plane(Self::face_of(&edges, &np)?) else {
            return unsupported("a draft's neutral face not a plane");
        };
        let mut n = pl.n;
        let pb = d.link(name, "PullDirection")?;
        if !pb.is_empty() {
            // its own pull: a line edge, run from its start, square to the neutral plane (Forge
            // pulls along the plane's normal)
            let ps: Vec<String> = d
                .subs(name, "PullDirection")?
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect();
            let poly = match &ps[..] {
                [e] if e.starts_with("Edge") => sampled(&pb, e).filter(|p| p.len() == 1),
                _ => None,
            };
            let Some(poly) = poly else {
                return unsupported("a draft's pull direction not an edge");
            };
            let Some(v) = py::unit(add(poly[0][poly[0].len() - 1], poly[0][0], -1.0)) else {
                return unsupported("a draft's pull direction not an edge");
            };
            if (dot(v, n).abs() - 1.0).abs() > 1e-9 {
                return unsupported("a draft pulled off its neutral plane's normal");
            }
            n = v;
        }
        if d.boolean(name, "Reversed")? {
            n = neg(n);
        }
        let mut feats: Vec<String> = self
            .labels
            .values()
            .flat_map(|l| l.split(' ').map(String::from))
            .collect();
        feats.sort();
        feats.dedup();
        let sel = self.state.select_faces(&want, &feats)?;
        let at = self.plane(n, pl.at)?;
        let mut b = format!("param {lab}.a = {}deg\n", num(d.flt(name, "Angle", 0.0)?));
        if let Some(dd) = &at.d {
            b += &format!("param {lab}.d = {dd}\n");
        }
        b += &format!("{lab} = draft({sel}, angle: a, neutral: {})\n", at.on);
        self.blocks.push(b);
        Ok(vec![lab.into()])
    }

    /// The base's stored solids' box, or zeros where it has none.
    fn base_box(&self, base: &str) -> [f64; 6] {
        self.truth.get(base).and_then(|s| s.vbox).unwrap_or([0.0; 6])
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
            t if is(SOLID, t) && s.starts_with("Edge") => {
                // a straight edge of a feature's stored shape: from its first sample to its last
                let polys = self.truth.get(&r).and_then(|x| x.subs.get(s));
                let Some([poly]) = polys.map(|p| &p[..]) else {
                    return unsupported("a pattern axis on an edge not sampled");
                };
                let (a, b) = (poly[0], poly[poly.len() - 1]);
                let Some(u) = py::unit(add(b, a, -1.0)) else {
                    return unsupported("a pattern axis on an edge of no length");
                };
                let off = |q: V3| py::dist3(add(a, u, dot(add(q, a, -1.0), u)), q);
                if poly.iter().any(|&q| off(q) > 1e-6) {
                    return unsupported("a pattern axis on a curved edge");
                }
                Ok((a, u))
            }
            t => unsupported(format!("a pattern axis on {}", t.unwrap_or("nothing"))),
        }
    }

    /// A pattern of `src` (ids, space apart), labelled `lab`.
    fn transform(&mut self, name: &str, t: &str, src: &str, lab: &str) -> Result<String, Fail> {
        let d = self.doc;
        let of = ids(src);
        if t == "PartDesign::PolarPattern" {
            let (o, mut a) = self.axis_of(name, "Axis")?;
            if d.boolean(name, "Reversed")? {
                a = neg(a);
            }
            let (p, q) = (self.point(o), self.point(add(o, a, 1.0)));
            return Ok(format!(
                "param {lab}.n = count({})\nparam {lab}.a = {}deg\n{lab} = pattern(of: {of}, layout: circ(about: axis({p}, {q}), n: n, over: a))\n",
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
            let along = self.dir(a)?;
            return Ok(format!(
                "param {lab}.n = count({n})\nparam {lab}.step = {}\n{lab} = pattern(of: {of}, layout: lin(along: {along}, n: n, step: step))\n",
                num(step)
            ));
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
            t if is(SOLID, t) && subs.first().is_some_and(|x| x.starts_with("Face")) => {
                // a planar face of a feature's stored shape: the plane through its edges' samples
                let pts: Vec<V3> = self
                    .truth
                    .get(&r)
                    .and_then(|x| x.subs.get(&subs[0]))
                    .map(|ps| ps.iter().flatten().copied().collect())
                    .unwrap_or_default();
                let Some(pl) = plane_of(&pts) else {
                    return unsupported("a mirror in a face not sampled or not flat");
                };
                pl
            }
            t => return unsupported(format!("a mirror in {}", t.unwrap_or("nothing"))),
        };
        let (p, dn) = (self.point(o), self.dir(n)?);
        Ok(format!(
            "{lab} = pattern(of: {of}, layout: mirror(on: plane(at: {p}, normal: {dn})))\n"
        ))
    }

    /// One hole at each of the sketch's circles, drilled against its normal; its seat from what the
    /// file stored.
    fn hole(&mut self, name: &str, lab: &str) -> Result<Vec<String>, Fail> {
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
        // drilled along z: against the normal of the plane its points sit on
        let z = if d.boolean(name, "Reversed")? {
            fr.n
        } else {
            neg(fr.n)
        };
        let t = d.integer(name, "DepthType", 0)?;
        let mut params = vec![("d".to_string(), num(d.flt(name, "Diameter", 0.0)?))];
        let mut args = vec!["d: d".to_string()];
        match t {
            1 => args.push("depth: thru".into()),
            0 => {
                params.push(("depth".into(), num(d.flt(name, "Depth", 0.0)?)));
                args.push("depth: depth".into());
            }
            _ => return unsupported("a hole up to a face"),
        }
        if d.integer(name, "HoleCutType", 0)? != 0 {
            params.push(("seat_d".into(), num(d.flt(name, "HoleCutDiameter", 0.0)?)));
            if d.flt(name, "HoleCutDepth", 0.0)? > 0.0 {
                params.push(("seat_depth".into(), num(d.flt(name, "HoleCutDepth", 0.0)?)));
                args.push("cbore: (seat_d, seat_depth)".into());
            } else {
                let a = d.flt(name, "HoleCutCountersinkAngle", 0.0)?;
                params.push(("seat_a".into(), format!("{}deg", num(a))));
                args.push("csink: (seat_d, seat_a)".into());
            }
        }
        if t == 0 && d.integer(name, "DrillPoint", 0)? == 1 {
            let a = d.flt(name, "DrillPointAngle", 0.0)?;
            params.push(("point".into(), format!("{}deg", num(a))));
            args.push("point: point".into());
        }
        let at = self.plane(neg(z), fr.o)?;
        let mut s = Sk::default();
        let mut pts = vec![];
        for e in &ents {
            let Shape::Circle { center, .. } = e.shape else {
                unreachable!("circles only")
            };
            let q = s.p(at.uv(fr.world(center.0, center.1)));
            pts.push(format!("{lab}_sk.{}", s.row("h", format!("pt({q}[0], {q}[1])"))));
        }
        let mut b = s.block(&format!("{lab}_sk"), &at);
        for (k, v) in params {
            b += &format!("param {lab}.{k} = {v}\n");
        }
        let at = if pts.len() == 1 {
            pts[0].clone()
        } else {
            format!("[{}]", pts.join(", "))
        };
        b += &format!("{lab} = hole(at: {at}, {})\n", args.join(", "));
        self.blocks.push(b);
        Ok(vec![lab.into()])
    }

    fn primitive(&mut self, name: &str, lab: &str, t: &str) -> Result<Vec<String>, Fail> {
        let d = self.doc;
        let cut = t.contains("Subtractive");
        if cut && self.bodies == 0 {
            return unsupported("first feature cuts");
        }
        if !t.ends_with("Box")
            && ((d.flt(name, "Angle", 360.0)? - 360.0).abs() > 1e-9
                || d.flt(name, "FirstAngle", 0.0)?.abs() > 1e-9
                || d.flt(name, "SecondAngle", 0.0)?.abs() > 1e-9)
        {
            return unsupported("a part of a cylinder");
        }
        if self.bodies == 0 {
            self.body = Some(lab.into());
        }
        self.bodies = 1;
        Ok(vec![self.box_or_cylinder(name, lab, cut, false)?])
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
        }
        self.chain_rest = chain.to_vec();
        for name in chain {
            self.step(name)?;
        }
        // the whole program, built: a body that stops says so
        let src = self.program();
        self.state.check(&src)
    }

    fn step(&mut self, name: &str) -> Result<(), Fail> {
        let d = self.doc;
        let t = d.ty(name).unwrap_or("").to_string();
        let lab = self.fresh(format!("t{}", self.labels.len()));
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
            let Some(body) = self.body.clone() else {
                return bug("a clone of no body");
            };
            let id = format!("m{}", self.made);
            self.made += 1;
            let (mut ps, mut moves) = (String::new(), vec![]);
            if sn >= 1e-12 {
                let ax = (q.0 / sn, q.1 / sn, q.2 / sn);
                let tip = self.point(ax);
                ps += &format!(
                    "param {id}.a = {}deg\n",
                    num(py::degrees(2.0 * py::atan2(sn, q.3)))
                );
                moves.push(format!("spin(about: axis(ref.origin, {tip}), by: a)"));
            }
            if moved {
                let along = self.dir(by)?;
                ps += &format!("param {id}.d = {}\n", num(dot(by, by).sqrt()));
                moves.push(format!("slide(along: {along}, by: d)"));
            }
            self.blocks.push(format!(
                "{ps}{id} = movebody(of: {body}, moves: [{}])\n",
                moves.join(", ")
            ));
            return Ok(());
        }
        let mut root = String::new();
        let ls = match t.as_str() {
            _ if t.starts_with("Part::") => {
                root = self.part_tree(name)?;
                vec![]
            }
            "PartDesign::Pad" | "PartDesign::Pocket" => self.pad(name, &lab, t.ends_with("Pocket"))?,
            "PartDesign::Revolution" | "PartDesign::Groove" => {
                self.revolve(name, &lab, t.ends_with("Groove"))?
            }
            "PartDesign::Fillet" => {
                let r = d.flt(name, "Radius", 0.0)?;
                self.blend(name, &lab, false, r, d.boolean(name, "UseAllEdges")?)?
            }
            "PartDesign::Chamfer" => {
                if d.integer(name, "ChamferType", 0)? != 0 {
                    return unsupported("an unequal chamfer");
                }
                let s = d.flt(name, "Size", 0.0)?;
                self.blend(name, &lab, true, s, d.boolean(name, "UseAllEdges")?)?
            }
            _ if TRANSFORMS.contains(&t.as_str()) => {
                let src = self.originals(name)?;
                let b = self.transform(name, &t, &src, &lab)?;
                self.blocks.push(b);
                vec![lab.clone()]
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
                    let id = self.fresh(format!("m{}x{k}", self.labels.len()));
                    self.taken.insert(id.clone());
                    let b = self.transform(m, mt.unwrap_or(""), &src, &id)?;
                    self.blocks.push(b);
                    src += &format!(" {id}");
                    ls.push(id);
                }
                ls
            }
            "PartDesign::Hole" => self.hole(name, &lab)?,
            "PartDesign::Draft" => self.draft(name, &lab)?,
            "PartDesign::Thickness" => self.thickness(name, &lab)?,
            "PartDesign::AdditivePipe" | "PartDesign::SubtractivePipe" => {
                self.pipe(name, &lab, t == "PartDesign::SubtractivePipe")?
            }
            "PartDesign::AdditiveBox"
            | "PartDesign::SubtractiveBox"
            | "PartDesign::AdditiveCylinder"
            | "PartDesign::SubtractiveCylinder" => self.primitive(name, &lab, &t)?,
            _ => return bug("a feature with no step"),
        };
        if t.starts_with("Part::") {
            self.labels.insert(name.into(), root);
            self.bodies = 1;
        } else if t == "PartDesign::MultiTransform" {
            // each its own: the last is the whole
            let Some(l) = ls.last() else {
                return bug("a multi-transform of nothing");
            };
            self.labels.insert(name.into(), l.clone());
        } else {
            // a feature of several statements (regions that do not touch): a pattern of it needs them all
            self.labels.insert(name.into(), ls.join(" "));
            self.taken.insert(lab);
        }
        Ok(())
    }
}

/// File names for each body's part file, `<stem>.<body>.fg`, safe for any names a file holds: only
/// `[A-Za-z0-9_-]`, the rest `_`, and kept apart by a `-N` where two would meet.
pub fn part_files(stem: &str, bodies: &[Body]) -> Vec<String> {
    let clean = |s: &str| -> String {
        let t: String = s
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if t.is_empty() { "_".into() } else { t }
    };
    let stem = clean(stem);
    let mut seen = HashSet::new();
    bodies
        .iter()
        .map(|b| {
            let base = format!("{stem}.{}", clean(&b.name));
            let mut name = format!("{base}.fg");
            let mut n = 1;
            while !seen.insert(name.clone()) {
                n += 1;
                name = format!("{base}-{n}.fg");
            }
            name
        })
        .collect()
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
                let v = doc.zip.read(&f).and_then(|d| volume_of(&d));
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

/// A body's outcome; a panic in the kernel or here is that body's bug, not the file's end.
fn one_body(doc: &Doc, b: &str) -> Body {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body_of(doc, b))).unwrap_or_else(|_| Body {
        name: b.into(),
        features: vec![],
        approx: false,
        fg: None,
        fail: Some(Fail::Bug("a panic".into())),
    })
}

fn body_of(doc: &Doc, b: &str) -> Body {
    let mut out = Body {
        name: b.into(),
        features: vec![],
        approx: false,
        fg: None,
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
            let sp = doc.link(c, "Spine")?;
            let pipe = matches!(
                doc.ty(c),
                Some("PartDesign::AdditivePipe" | "PartDesign::SubtractivePipe")
            );
            if pipe && matches!(doc.ty(&sp), Some("PartDesign::ShapeBinder" | "Part::Helix")) {
                put(&sp, doc.shape_file(&sp)?);
                wires.push(sp);
            }
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
            if is(TRANSFORMS, Some(t)) || t == "PartDesign::MultiTransform" {
                let mut ms = vec![c.clone()];
                ms.extend(doc.links(c, "Transformations")?);
                for m in ms {
                    for k in ["MirrorPlane", "Axis", "Direction"] {
                        let o = if doc.props.contains_key(&m) {
                            doc.link(&m, k)?
                        } else {
                            String::new()
                        };
                        let s: Vec<String> = doc.subs(&m, k)?.into_iter().filter(|x| !x.is_empty()).collect();
                        if is(SOLID, doc.ty(&o)) && !s.is_empty() {
                            put(&o, doc.shape_file(&o)?);
                            subs.entry(o).or_default().extend(s);
                        }
                    }
                }
            }
            if t == "PartDesign::Thickness" {
                let o = doc.link(c, "Base")?;
                if doc.props.contains_key(&o) {
                    put(&o, doc.shape_file(&o)?);
                    let s = doc.subs(c, "Base")?.into_iter().filter(|x| !x.is_empty());
                    subs.entry(o).or_default().extend(s);
                }
            }
            if t == "PartDesign::Draft" {
                for k in ["Base", "NeutralPlane", "PullDirection"] {
                    let o = doc.link(c, k)?;
                    if doc.props.contains_key(&o) {
                        put(&o, doc.shape_file(&o)?);
                        let s = doc.subs(c, k)?.into_iter().filter(|x| !x.is_empty());
                        subs.entry(o).or_default().extend(s);
                    }
                }
            }
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
            blocks: vec![],
            labels: HashMap::new(),
            points: HashMap::new(),
            state: State::default(),
            bodies: 0,
            body: None,
            approx: false,
            tree: 0,
            made: 0,
            chain_rest: vec![],
            taken: HashSet::new(),
        });
        t.run(&chain)
    })();
    let r = r.err();
    if let Some(t) = tr {
        out.approx = t.approx;
        if !matches!(r, Some(Fail::Unsupported(_) | Fail::Unresolved(_) | Fail::Bug(_))) {
            let mut head = format!("# FreeCAD body {}: {}\n", out.name, out.features.join(", "));
            if t.approx {
                head += "# its splines read as fine polylines\n";
            }
            if let Some(Fail::Stop(why)) = &r {
                head += &format!("# stops: {why}\n");
            }
            out.fg = Some(head + &t.program());
        }
    }
    out.fail = r;
    out
}

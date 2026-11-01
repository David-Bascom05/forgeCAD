//! Relations between results that must hold whatever the part: the same history moved, mirrored
//! or scaled; boolean identities; independent features in either order; every regen path; and
//! the B-rep against its CSG tree point by point. Each property takes a part and a seed (which
//! picks its transform or tool) and returns its first failure, so a failure shrinks against the
//! same transform.

use crate::cert::certify;
use crate::map::{self, Map};
use forge_bool::{Index, Member, Op, Shape, Where};
use forge_geom::Surface;
use forge_math::rng::Rng;
use forge_math::{Box3, Point3};
use forge_mesh::{Quality, tessellate};
use forge_model::{Backend, Combine, Extent, Feature, Part, Turn, suite};
use forge_ops::{Profile, ProfileLoop, Segment};
use forge_topo::Body;

/// Chord of the meshes volumes and areas are measured on.
pub const CHORD: f64 = 0.02;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sig {
    pub verts: usize,
    pub edges: usize,
    pub faces: usize,
    pub shells: usize,
    /// Vertices with two edges between the same two faces: nothing is lost merging those edges.
    pub redundant: usize,
    /// Faces on planes, cylinders, cones, spheres, tori and procedural surfaces.
    pub kinds: [usize; 6],
}

pub fn sig(b: &Body) -> Sig {
    let b = b.compact();
    let mut kinds = [0; 6];
    for f in b.face_ids() {
        kinds[match b.face(f).surface {
            Surface::Plane(_) => 0,
            Surface::Cylinder(_) => 1,
            Surface::Cone(_) => 2,
            Surface::Sphere(_) => 3,
            Surface::Torus(_) => 4,
            Surface::Procedural(_) => 5,
        }] += 1;
    }
    let mut edges_at: Vec<Vec<forge_topo::EdgeId>> = vec![Vec::new(); b.vertices.len()];
    for e in b.edge_ids() {
        let ed = b.edge(e);
        edges_at[ed.start.index()].push(e);
        if ed.end != ed.start {
            edges_at[ed.end.index()].push(e);
        }
    }
    let faces_of = |e: forge_topo::EdgeId| {
        let mut f: Vec<u32> = b.edge(e).coedges.iter().map(|&c| b.coedge_face(c).0).collect();
        f.sort_unstable();
        f
    };
    let redundant = edges_at
        .iter()
        .filter(|es| es.len() == 2 && es[0] != es[1] && faces_of(es[0]) == faces_of(es[1]))
        .count();
    Sig {
        redundant,
        verts: b.vertices.len(),
        edges: b.edges.len(),
        faces: b.faces.len(),
        shells: b.shells.len(),
        kinds,
    }
}

impl Sig {
    /// The counts with every redundant vertex merged away.
    fn merged(&self) -> Sig {
        Sig {
            verts: self.verts - self.redundant,
            edges: self.edges - self.redundant,
            redundant: 0,
            ..self.clone()
        }
    }
}

impl std::fmt::Display for Sig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let k = self.kinds;
        write!(
            f,
            "V{} E{} F{} S{} R{} [pl{} cy{} co{} sp{} to{} pr{}]",
            self.verts,
            self.edges,
            self.faces,
            self.shells,
            self.redundant,
            k[0],
            k[1],
            k[2],
            k[3],
            k[4],
            k[5]
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub prop: &'static str,
    /// What went wrong, without numbers: failures of one kind are one bug.
    pub kind: String,
    pub detail: String,
}

fn found(prop: &'static str, kind: impl Into<String>, detail: impl Into<String>) -> Option<Finding> {
    Some(Finding {
        prop,
        kind: kind.into(),
        detail: detail.into(),
    })
}

/// The part regenerated feature by feature, a regen error or panic as its short kind.
pub fn build(part: &Part) -> Result<Body, String> {
    guarded(|| part.regenerate().map_err(|e| short(&format!("{e:?}"))))
}

fn guarded(f: impl FnOnce() -> Result<Body, String>) -> Result<Body, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|p| {
        let m = p
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        Err(format!("panic {}", m.chars().take(60).collect::<String>()))
    })
}

/// An error's variant names down to the first value: `Boolean { feature: 3, error:
/// Internal("x") }` is `Boolean Internal("x")`.
fn short(e: &str) -> String {
    let number = |w: &str| w.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '#');
    let mut out = Vec::new();
    for w in e.split([' ', '{', '}', ',']).filter(|w| !w.is_empty()) {
        if w.ends_with(':') || number(w) {
            continue;
        }
        let (head, inner) = w.split_once('(').unwrap_or((w, ""));
        out.push(head.to_string());
        if w.contains('(') {
            let inner = inner.trim_end_matches(')');
            if !inner.is_empty() && !number(inner) {
                out.push(inner.to_string());
            }
            break;
        }
    }
    out.join(" ")
}

/// Volume and area of a mesh of the body, and the most either can be off by (the chord times
/// the area).
pub fn measure(b: &Body, chord: f64) -> Option<(f64, f64, f64)> {
    let m = tessellate(
        b,
        &Quality {
            chord,
            max_angle: 0.5,
        },
    )
    .ok()?;
    let a = m.area();
    Some((m.volume(), a, chord * a))
}

/// A body that builds and certifies, with its signature and measures.
struct Good {
    body: Body,
    sig: Sig,
    vol: f64,
    area: f64,
    slack: f64,
}

fn good(part: &Part) -> Result<Good, String> {
    good_at(part, CHORD)
}

/// `good`, measured on a mesh of this chord (a scaled part's chord scaled with it meshes alike).
fn good_at(part: &Part, chord: f64) -> Result<Good, String> {
    good_from(build(part), chord)
}

fn good_from(body: Result<Body, String>, chord: f64) -> Result<Good, String> {
    let body = body?;
    if let Some(p) = forge_topo::check(&body).first() {
        return Err(format!("invalid {}", short(&format!("{p:?}"))));
    }
    let (vol, area, slack) = measure(&body, chord).ok_or("does not mesh")?;
    Ok(Good {
        sig: sig(&body),
        body,
        vol,
        area,
        slack,
    })
}

/// `b` is `a` mapped: the same topology, volume times `k3`, area times `k2`.
fn same(prop: &'static str, a: &Good, b: &Result<Good, String>, k3: f64, k2: f64) -> Option<Finding> {
    let b = match b {
        Ok(b) => b,
        Err(e) => return found(prop, format!("fails: {e}"), "the original builds valid"),
    };
    if a.sig.merged() != b.sig.merged() {
        return found(prop, "topology differs", format!("{} vs {}", a.sig, b.sig));
    }
    let dv = (b.vol - a.vol * k3).abs();
    if dv > a.slack * k3 + b.slack {
        return found(
            prop,
            "volume differs",
            format!("{} vs {} (x{k3}), slack {}", a.vol, b.vol, a.slack * k3 + b.slack),
        );
    }
    let da = (b.area - a.area * k2).abs();
    if da > 1e-3 * a.area * k2 {
        return found(prop, "area differs", format!("{} vs {} (x{k2})", a.area, b.area));
    }
    if a.sig != b.sig {
        return found(
            prop,
            "redundant vertices differ",
            format!("{} vs {}", a.sig, b.sig),
        );
    }
    None
}

fn mapped(prop: &'static str, part: &Part, m: &Map, k: f64) -> Option<Finding> {
    let a = good(part).ok()?;
    let moved = m.part(part)?;
    same(prop, &a, &good_at(&moved, CHORD * k), k * k * k, k * k)
}

/// The history moved by a rigid motion: a quarter turn (exact) or a Pythagorean rotation.
pub fn rigid(part: &Part, seed: u64) -> Option<Finding> {
    mapped("rigid", part, &rigid_map(seed), 1.0)
}

pub fn rigid_map(seed: u64) -> Map {
    let mut r = Rng::new(seed ^ 0x51);
    if seed.is_multiple_of(2) {
        map::quarter(&mut r)
    } else {
        map::rigid(&mut r)
    }
}

/// The history reflected in a coordinate plane (exact) or a tilted one.
pub fn mirror(part: &Part, seed: u64) -> Option<Finding> {
    mapped("mirror", part, &mirror_map(seed), 1.0)
}

pub fn mirror_map(seed: u64) -> Map {
    map::mirror(&mut Rng::new(seed ^ 0x52), seed % 2 == 1)
}

/// The history scaled by a power of two: every coordinate scales exactly, so only absolute
/// tolerances can tell the copies apart.
pub fn scale(part: &Part, seed: u64) -> Option<Finding> {
    let k = scale_factor(seed);
    mapped("scale", part, &Map::Scale(k), k)
}

pub fn scale_factor(seed: u64) -> f64 {
    [2.0, 0.5, 4.0, 0.25][(seed % 4) as usize]
}

/// How a feature combines, if it is a plain tool (an extrude, revolve or ball).
fn tool_op(f: &Feature) -> Option<Combine> {
    match f {
        Feature::Extrude { op, .. } | Feature::Revolve { op, .. } | Feature::Ball { op, .. } => Some(*op),
        _ => None,
    }
}

fn with_op(f: &Feature, to: Combine) -> Feature {
    let mut f = f.clone();
    if let Feature::Extrude { op, .. } | Feature::Revolve { op, .. } | Feature::Ball { op, .. } = &mut f {
        *op = to;
    }
    f
}

/// Whether every feature is a plain tool, the first making the part and none another body.
pub fn plain(part: &Part) -> bool {
    part.features.iter().enumerate().all(|(i, f)| match tool_op(f) {
        Some(Combine::New) => i == 0,
        Some(_) => i > 0,
        None => false,
    })
}

/// Two neighbouring tools that both add or both cut, swapped: the same solid.
pub fn order(part: &Part, seed: u64) -> Option<Finding> {
    let (i, _) = order_pair(part, seed)?;
    let a = good(part).ok()?;
    let mut swapped = part.clone();
    swapped.features.swap(i, i + 1);
    same("order", &a, &good(&swapped), 1.0, 1.0).map(|mut f| {
        f.detail = format!("features {i} and {} swapped: {}", i + 1, f.detail);
        f
    })
}

/// Which neighbouring pair `order` swaps, and how many there were to pick from.
fn order_pair(part: &Part, seed: u64) -> Option<(usize, usize)> {
    if !plain(part) {
        return None;
    }
    let pairs: Vec<usize> = (1..part.features.len().saturating_sub(1))
        .filter(|&i| tool_op(&part.features[i]) == tool_op(&part.features[i + 1]))
        .collect();
    (!pairs.is_empty()).then(|| (pairs[(seed as usize) % pairs.len()], pairs.len()))
}

/// Each stage of the history (the body after features `..k`, valid) moved by a few ulps (even
/// seeds) or up to 1e-12 (odd), then feature `k` applied: a valid body with the same topology as
/// from the stage unmoved. A kernel with tolerance 1e-5 must not see 1e-14.
pub fn ulp(part: &Part, seed: u64) -> Option<Finding> {
    if forge_model::bodies::multi(&part.features) {
        return None;
    }
    let h = forge_model::History::new(&part.features, None);
    let mut body: Option<Body> = None;
    for k in 0..part.features.len() {
        let next = guarded(|| {
            forge_model::step(h, k, body.clone())
                .map_err(|e| short(&format!("{e:?}")))?
                .ok_or_else(|| "no body".to_string())
        });
        let Ok(next) = next else { return None };
        if let Some(b) = &body
            && forge_topo::check(&next).is_empty()
        {
            let moved = nudged(b, seed.wrapping_add(2000 * k as u64));
            let r = guarded(|| {
                forge_model::step(h, k, Some(moved.clone()))
                    .map_err(|e| short(&format!("{e:?}")))?
                    .ok_or_else(|| "no body".to_string())
            });
            let hit = match &r {
                Err(e) => Some((format!("fails: {e}"), String::new())),
                Ok(r) => match forge_topo::check(r).first() {
                    Some(p) => Some((format!("invalid {}", short(&format!("{p:?}"))), String::new())),
                    None => {
                        // (a vertex between two edges of the same two faces is nothing lost, as in `same`)
                        let (a, b) = (sig(&next), sig(r));
                        (a.merged() != b.merged())
                            .then(|| ("topology differs".to_string(), format!("{a} vs {b}")))
                    }
                },
            };
            // a nudge that broke the stage itself says nothing of the feature
            if let Some((kind, detail)) = hit
                && forge_topo::check(&moved).is_empty()
            {
                return found("ulp", kind, format!("feature {k}, seed {seed}: {detail}"));
            }
        }
        body = Some(next);
    }
    None
}

/// `b` with every vertex, edge range, line origin and circle centre and radius moved: by up to
/// four ulps for even seeds, by up to 1e-12 for odd ones.
pub fn nudged(b: &Body, seed: u64) -> Body {
    let mut r = Rng::new(seed ^ 0x0715);
    let ulps = seed.is_multiple_of(2);
    let mut n = |x: f64| {
        if ulps {
            let k = r.below(9) as i64 - 4;
            if x == 0.0 {
                x
            } else {
                f64::from_bits((x.to_bits() as i64 + k) as u64)
            }
        } else {
            x + r.range(-1e-12, 1e-12)
        }
    };
    let p3 = |p: Point3, n: &mut dyn FnMut(f64) -> f64| Point3::new(n(p.x), n(p.y), n(p.z));
    let mut out = b.clone();
    out.hulls = Default::default();
    for v in &mut out.vertices {
        v.point = p3(v.point, &mut n);
    }
    for e in &mut out.edges {
        e.range = forge_math::Interval::new(n(e.range.lo), n(e.range.hi));
        match &mut e.curve {
            forge_geom::Curve::Line(l) => l.origin = p3(l.origin, &mut n),
            forge_geom::Curve::Circle(c) => {
                c.frame.origin = p3(c.frame.origin, &mut n);
                c.radius = n(c.radius);
            }
            _ => {}
        }
        e.bbox.clear();
    }
    for f in &mut out.faces {
        f.bbox.clear();
        f.query.clear();
    }
    out
}

/// A random tool for boolean identities, from the fuzz generator.
pub fn tool(seed: u64) -> Feature {
    let p = suite::fuzz(seed ^ 0xB0, 4);
    let n = p.features.len() - 1;
    with_op(&p.features[1 + (seed as usize) % n], Combine::Add)
}

/// With B a random tool: A - B and A & B partition A point by point, their volumes add to A's,
/// vol(A | B) + vol(A & B) = vol(A) + vol(B), and (A - B) | (A & B) is A again.
pub fn identities(part: &Part, seed: u64) -> Option<Finding> {
    identities_with(part, &tool(seed), seed)
}

pub fn identities_with(part: &Part, b: &Feature, seed: u64) -> Option<Finding> {
    const P: &str = "boolean";
    let a = good(part).ok()?;
    let with = |op| {
        let mut p = part.clone();
        p.features.push(with_op(b, op));
        good(&p)
    };
    let alone = good(&Part {
        features: vec![with_op(b, Combine::New)],
    })
    .ok()?;
    let (u, i, d) = (with(Combine::Add), with(Combine::Keep), with(Combine::Cut));
    // an empty intersection or difference is an error, not a body: leave those cases
    let (u, i, d) = match (u, i, d) {
        (Ok(u), Ok(i), Ok(d)) => (u, i, d),
        (u, i, d) => {
            for (name, r) in [("A | B", &u), ("A & B", &i), ("A - B", &d)] {
                if let Err(e) = r
                    && !e.contains("Empty")
                    && !e.contains("ZeroThickness")
                {
                    return found(P, format!("{name} fails: {e}"), "A and B build valid alone");
                }
            }
            return None;
        }
    };
    let slack = u.slack + i.slack + a.slack + alone.slack;
    let lhs = u.vol + i.vol;
    let rhs = a.vol + alone.vol;
    if (lhs - rhs).abs() > slack {
        return found(
            P,
            "vol(A|B) + vol(A&B) != vol(A) + vol(B)",
            format!("{lhs} vs {rhs}, slack {slack}"),
        );
    }
    if (d.vol + i.vol - a.vol).abs() > d.slack + i.slack + a.slack {
        return found(
            P,
            "vol(A-B) + vol(A&B) != vol(A)",
            format!("{} + {} vs {}", d.vol, i.vol, a.vol),
        );
    }
    // point by point: inside A is inside exactly one of A - B and A & B; outside A, in neither
    let (ia, id, ii) = (Index::new(&a.body), Index::new(&d.body), Index::new(&i.body));
    let mut r = Rng::new(seed ^ 0xB1);
    let bb = bbox(&a.body).grow(1.0);
    for _ in 0..400 {
        let p = sample(&mut r, &bb);
        let (wa, wd, wi) = (ia.classify(p), id.classify(p), ii.classify(p));
        if [wa, wd, wi].contains(&Where::On) {
            continue;
        }
        let ins = [wd, wi].iter().filter(|w| **w == Where::Inside).count();
        if (wa == Where::Inside && ins != 1) || (wa == Where::Outside && ins != 0) {
            return found(
                P,
                "A-B and A&B do not partition A",
                format!("at {p:?}: A {wa:?}, A-B {wd:?}, A&B {wi:?}"),
            );
        }
    }
    let joined = guarded(|| {
        forge_bool::boolean(&d.body, &i.body, Op::Union, 999).map_err(|e| short(&format!("{e:?}")))
    });
    match joined {
        Err(e) => found(P, format!("(A-B) | (A&B) fails: {e}"), ""),
        Ok(j) => {
            // both pieces carry names from the same features: only the shape is asked of the join
            let shape = |p: &&forge_topo::Problem| !matches!(p, forge_topo::Problem::DuplicateName { .. });
            if let Some(p) = forge_topo::check(&j).iter().find(shape) {
                return found(P, "(A-B) | (A&B) invalid", format!("{p:?}"));
            }
            let (v, _, s) = measure(&j, CHORD)?;
            if (v - a.vol).abs() > s + a.slack {
                return found(P, "(A-B) | (A&B) != A", format!("volume {v} vs {}", a.vol));
            }
            None
        }
    }
}

/// Feature by feature against n-ary on one thread; that against n-ary on two threads, placing
/// no points by the tools' shapes, and with the region batch on the GPU (the CPU without one).
/// Each n-ary path is held to the first, so a difference names the path that makes it.
pub fn backends(part: &Part, _seed: u64) -> Option<Finding> {
    let seq = good(part).ok()?;
    let nary = |name: &str| {
        let body = guarded(|| {
            err(match name {
                "nary 1" => part.regenerate_nary_on(1, Backend::Cpu),
                "nary 2" => part.regenerate_nary_on(2, Backend::Cpu),
                "nary no shapes" => part.regenerate_nary_with(1, false),
                _ => part.regenerate_nary_on(1, Backend::Gpu),
            })
        });
        good_from(body, CHORD)
    };
    let one = nary("nary 1");
    let named = |name: &str, f: Finding| Finding {
        kind: format!("{name}: {}", f.kind),
        ..f
    };
    // the n-ary paths first: they should agree even where feature by feature does not
    if let Ok(one) = &one {
        let other = ["nary 2", "nary no shapes", "nary gpu"]
            .into_iter()
            .find_map(|name| same("backends", one, &nary(name), 1.0, 1.0).map(|f| named(name, f)));
        if other.is_some() {
            return other;
        }
    }
    same("backends", &seq, &one, 1.0, 1.0).map(|f| named("nary 1", f))
}

fn err(r: Result<Body, forge_model::RegenError>) -> Result<Body, String> {
    r.map_err(|e| short(&format!("{e:?}")))
}

/// The certificates of `cert::certify` on the part as built.
pub fn certificate(part: &Part, _seed: u64) -> Option<Finding> {
    let body = build(part).ok()?;
    if !forge_topo::check(&body).is_empty() {
        return None;
    }
    let flaws = certify(&body, CHORD * 2.5);
    let first = flaws.first()?;
    found(
        "certify",
        first.kind(),
        format!("{first:?} ({} flaws)", flaws.len()),
    )
}

/// Random points classified against the B-rep and against the tools' solids combined in
/// order: wherever the tools say clearly in or out, the B-rep must agree.
pub fn membership(part: &Part, seed: u64) -> Option<Finding> {
    if !plain(part) {
        return None;
    }
    membership_of(part, &build(part).ok()?, seed)
}

/// `membership` of `body` against `part`'s tools.
pub fn membership_of(part: &Part, body: &Body, seed: u64) -> Option<Finding> {
    let (shapes, ops) = part.nary_solids().ok()?;
    let index = Index::new(body);
    let mut r = Rng::new(seed ^ 0x3E);
    let bb = bbox(body).grow(2.0);
    // half the points anywhere, half in a random tool's box, so small tools are hit too
    let tools: Vec<Box3> = part
        .features
        .iter()
        .filter_map(|f| {
            let alone = Part {
                features: vec![with_op(f, Combine::New)],
            };
            Some(bbox(&alone.regenerate().ok()?).grow(0.5))
        })
        .collect();
    for k in 0..2000 {
        let p = if k % 2 == 0 || tools.is_empty() {
            sample(&mut r, &bb)
        } else {
            let t = r.below(tools.len());
            sample(&mut r, &tools[t])
        };
        let tree = csg(&shapes, &ops, p, 1e-3);
        let brep = index.classify(p);
        if matches!(
            (tree, brep),
            (Member::In, Where::Outside) | (Member::Out, Where::Inside)
        ) {
            return found(
                "membership",
                format!("CSG {tree:?}, B-rep {brep:?}"),
                format!("at {p:?}"),
            );
        }
    }
    None
}

/// Where `p` is in the tools combined in turn, clear of every boundary by `m`, or Near.
fn csg(shapes: &[Shape], ops: &[Op], p: Point3, m: f64) -> Member {
    let mut v = shapes[0].member(p, m);
    for (s, op) in shapes[1..].iter().zip(ops) {
        let w = s.member(p, m);
        v = match (op, v, w) {
            (_, Member::Near, _) | (_, _, Member::Near) => Member::Near,
            (Op::Union, a, b) => {
                if a == Member::In || b == Member::In {
                    Member::In
                } else {
                    Member::Out
                }
            }
            (Op::Subtract, a, b) => {
                if a == Member::In && b == Member::Out {
                    Member::In
                } else {
                    Member::Out
                }
            }
            (Op::Intersect, a, b) => {
                if a == Member::In && b == Member::In {
                    Member::In
                } else {
                    Member::Out
                }
            }
        };
    }
    v
}

fn bbox(b: &Body) -> Box3 {
    Box3::from_points(b.vertices.iter().map(|v| v.point)).hull(
        &b.face_ids()
            .fold(Box3::from_points([]), |acc, f| acc.hull(&b.face_box(f))),
    )
}

fn sample(r: &mut Rng, b: &Box3) -> Point3 {
    Point3::new(
        r.range(b.x.lo, b.x.hi),
        r.range(b.y.lo, b.y.hi),
        r.range(b.z.lo, b.z.hi),
    )
}

/// What regen entry points to hold a feature to: each gives the bodies its prefix ends with.
type Entry = (
    &'static str,
    fn(&Part) -> Result<Vec<Body>, forge_model::RegenError>,
);

fn solids(
    r: Result<Vec<forge_model::Solid>, forge_model::RegenError>,
) -> Result<Vec<Body>, forge_model::RegenError> {
    r.map(|s| s.into_iter().map(|s| s.body).collect())
}

const ENTRIES: [Entry; 5] = [
    ("regenerate_solids", |p| solids(p.regenerate_solids())),
    ("regenerate_nary_solids", |p| solids(p.regenerate_nary_solids(1))),
    ("regenerate", |p| p.regenerate().map(|b| vec![b])),
    ("regenerate_nary", |p| p.regenerate_nary(1).map(|b| vec![b])),
    ("regenerate_nary_with", |p| {
        p.regenerate_nary_with(1, false).map(|b| vec![b])
    }),
];

/// A body's faces' and edges' names and its vertices.
type Fingerprint = (Vec<u64>, Vec<u64>, Vec<[u64; 3]>);

/// A result as what could show a feature took effect: each body's faces, edges and vertices.
fn fingerprint(bodies: &[Body]) -> Vec<Fingerprint> {
    let mut out: Vec<_> = bodies
        .iter()
        .map(|b| {
            let b = b.compact();
            let mut faces: Vec<u64> = b.face_ids().map(|f| b.face(f).name.0).collect();
            let mut edges: Vec<u64> = b.edge_ids().map(|e| b.edge(e).name.0).collect();
            let mut verts: Vec<[u64; 3]> = b
                .vertices
                .iter()
                .map(|v| [v.point.x.to_bits(), v.point.y.to_bits(), v.point.z.to_bits()])
                .collect();
            faces.sort_unstable();
            edges.sort_unstable();
            verts.sort_unstable();
            (faces, edges, verts)
        })
        .collect();
    out.sort();
    out
}

/// Whether a feature only defines something for later ones.
fn makes_nothing(f: &Feature) -> bool {
    matches!(
        f,
        Feature::Param(_) | Feature::Sketch(_) | Feature::Plane(_) | Feature::Work(_)
    )
}

fn feature_name(f: &Feature) -> String {
    let d = format!("{f:?}");
    d.split([' ', '(', '{']).next().unwrap_or("").to_string()
}

/// Points in `s`, from `reach` narrowed to the cells the solid may be in.
fn points_in(s: &Shape, reach: Box3, r: &mut Rng, want: usize) -> Vec<Point3> {
    let mut cells = vec![reach];
    for _ in 0..5 {
        let mut next = Vec::with_capacity(cells.len() * 2);
        for c in &cells {
            let (m, w) = (c.center(), [c.x.width(), c.y.width(), c.z.width()]);
            let axis = (0..3).max_by(|&a, &b| w[a].total_cmp(&w[b])).unwrap_or(0);
            let (mut a, mut b) = (*c, *c);
            match axis {
                0 => {
                    a.x.hi = m.x;
                    b.x.lo = m.x;
                }
                1 => {
                    a.y.hi = m.y;
                    b.y.lo = m.y;
                }
                _ => {
                    a.z.hi = m.z;
                    b.z.lo = m.z;
                }
            }
            next.extend([a, b].into_iter().filter(|x| !s.box_out(x, 0.0)));
        }
        cells = next;
    }
    let mut out = Vec::new();
    if cells.is_empty() {
        return out;
    }
    for _ in 0..want * 8 {
        let c = r.below(cells.len());
        let p = sample(r, &cells[c]);
        if s.member(p, 1e-3) == Member::In {
            out.push(p);
            if out.len() == want {
                break;
            }
        }
    }
    out
}

/// Where tools of feature `k` disagree with the body it left: a tool added must be there, a tool
/// cut gone, and nothing outside a tool kept: each instance on its own, so a pattern's skipped copy
/// shows. And, given `unchanged`, why nothing changed is that the tool could change nothing.
pub fn tools_held(part: &Part, k: usize, body: &Body, unchanged: bool, r: &mut Rng) -> Option<String> {
    let h = forge_model::History::new(&part.features, None);
    let tools = forge_model::tool_shapes(h, k).ok()?;
    if tools.is_empty() {
        return unchanged.then(|| "made no tools and changed nothing".to_string());
    }
    let index = Index::new(body);
    let reach = bbox(body).grow(5.0);
    let last = tools.len() - 1;
    let alike = tools.iter().all(|t| t.1 == tools[0].1);
    for (n, (shape, op)) in tools.iter().enumerate() {
        // what a later tool of the feature may undo is judged only with all of one kind
        if !alike && n != last {
            continue;
        }
        let pts = points_in(shape, reach, r, 200);
        for p in pts {
            match (op, index.classify(p)) {
                (Combine::Add | Combine::New, Where::Outside) => {
                    return Some(format!("tool {n} of {} added is not there", tools.len()));
                }
                (Combine::Cut, Where::Inside) => {
                    return Some(format!("tool {n} of {} cut is still there", tools.len()));
                }
                _ => {}
            }
        }
        if *op == Combine::Keep {
            for _ in 0..400 {
                let p = sample(r, &bbox(body));
                if shape.member(p, 1e-3) == Member::Out && index.classify(p) == Where::Inside {
                    return Some("material kept outside the tool".to_string());
                }
            }
        }
    }
    None
}

/// How many bodies a feature on bodies leaves, against how many there were. A cut may take a whole
/// body of several: David's decision, reported by `Model::removed_bodies` (all of them is `Empty`).
pub fn bodies_held(f: &Feature, before: &[Body], after: &[Body]) -> Option<&'static str> {
    use forge_model::{BodyFeature, Keep};
    let Feature::Body(b) = f else { return None };
    let cuts = |g: &Feature| match g {
        Feature::Extrude { op, .. } | Feature::Revolve { op, .. } | Feature::Ball { op, .. } => {
            *op == Combine::Cut
        }
        Feature::Hole(_) => true,
        g => forge_model::swept::op(g) == Some(Combine::Cut),
    };
    let (want, may_lose) = match &**b {
        BodyFeature::Combine { tools, keep, op, .. } => (
            before.len() - if *keep { 0 } else { tools.len() },
            usize::from(*op == Combine::Cut),
        ),
        BodyFeature::Move { copy: true, .. } => (before.len() + 1, 0),
        BodyFeature::Split { keep: Keep::Both, .. } => (before.len() + 1, 0),
        BodyFeature::In { feature, .. } if cuts(feature) => (before.len(), before.len().saturating_sub(1)),
        _ => (before.len(), 0),
    };
    (after.len() > want || after.len() + may_lose < want).then_some("left the wrong number of bodies")
}

/// Each named edge a round or a bevel was asked for is gone from the body it left.
fn edges_taken(f: &Feature, bodies: &[Body]) -> Option<String> {
    let names: Vec<forge_topo::Name> = match f {
        Feature::Fillet { edges, .. } => edges.iter().map(|e| e.0).collect(),
        Feature::Chamfer { edges, .. } => edges.clone(),
        _ => return None,
    };
    names
        .iter()
        .position(|&n| bodies.iter().any(|b| b.edge_by_name(n).is_some()))
        .map(|i| format!("edge {i} of {} still on the part", names.len()))
}

/// After each feature, on every regen entry point, the regen errs or the feature visibly took
/// effect: the body changed, or nothing could (a tool that misses), and every instance and every
/// named edge is done. A feature that returns Ok with no change and no reason fails.
pub fn effect(part: &Part, seed: u64) -> Option<Finding> {
    let n = part.features.len();
    let mut r = Rng::new(seed ^ 0xEFFEC7);
    // the n-ary paths rebuild every prefix: on a long history its last few
    let from_nary = n.saturating_sub(16);
    for (name, entry) in ENTRIES {
        let single = name == "regenerate" || name == "regenerate_nary" || name == "regenerate_nary_with";
        let stride_all = name == "regenerate_solids" || n <= 40;
        let start = if stride_all { 0 } else { from_nary };
        let mut before: Option<Vec<Body>> = None;
        if start > 0 {
            let p = Part {
                features: part.features[..start].to_vec(),
            };
            before = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| entry(&p)))
                .ok()
                .and_then(Result::ok);
        }
        for k in start..n {
            let features = part.features[..=k].to_vec();
            if single && forge_model::bodies::multi(&features) {
                break;
            }
            let p = Part { features };
            let got = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| entry(&p)));
            // a refusal is an answer; and from there on the history is the refused one's
            let Ok(Ok(got)) = got else {
                // (no solid yet: parameters and sketches above the first)
                if before.is_none() {
                    continue;
                }
                break;
            };
            let f = &part.features[k];
            let kind = feature_name(f);
            if makes_nothing(f) {
                before = Some(got);
                continue;
            }
            let same = before
                .as_ref()
                .is_some_and(|b| fingerprint(b) == fingerprint(&got));
            if let Some(w) = edges_taken(f, &got) {
                return found(
                    "effect",
                    format!("{name}: {kind} partial"),
                    format!("feature {k}: {w}"),
                );
            }
            if let Some(w) = before.as_ref().and_then(|b| bodies_held(f, b, &got)) {
                return found("effect", format!("{name}: {kind} {w}"), format!("feature {k}"));
            }
            let multi = forge_model::bodies::multi(&p.features);
            let held = if !multi && got.len() == 1 {
                let b = got[0].compact();
                tools_of(f).then(|| tools_held(&p, k, &b, same, &mut r)).flatten()
            } else {
                None
            };
            if let Some(w) = held {
                return found(
                    "effect",
                    format!(
                        "{name}: {kind} {}",
                        w.split(' ').skip(2).collect::<Vec<_>>().join(" ")
                    ),
                    format!("feature {k}: {w}"),
                );
            }
            // a slab thickened outward and cut lies outside what it cuts: nothing to change
            let slab_outside = matches!(
                f,
                Feature::Edit(forge_model::Edit::Thicken {
                    side: forge_model::Side::Pos,
                    op: Combine::Cut,
                    ..
                })
            );
            if same && !tools_of(f) && !slab_outside {
                return found(
                    "effect",
                    format!("{name}: {kind} changed nothing"),
                    format!("feature {k}"),
                );
            }
            before = Some(got);
        }
    }
    None
}

/// Whether a feature's effect is that of tools combined with the part.
fn tools_of(f: &Feature) -> bool {
    matches!(
        f,
        Feature::Extrude { .. }
            | Feature::Revolve { .. }
            | Feature::Ball { .. }
            | Feature::Hole(_)
            | Feature::Sweep(_)
            | Feature::Coil(_)
            | Feature::Loft(_)
            | Feature::Thread(_)
            | Feature::Text(_)
    ) || matches!(
        f,
        Feature::Pattern {
            of: forge_model::Source::Features(_),
            ..
        }
    )
}

pub type Property = fn(&Part, u64) -> Option<Finding>;

/// `prop` on `part`, a panic anywhere in it (a mesh, a classifier) a finding of its own.
pub fn run(name: &'static str, prop: Property, part: &Part, seed: u64) -> Option<Finding> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| prop(part, seed))).unwrap_or_else(|p| {
        let m = p
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        found(
            name,
            format!("panic {}", m.chars().take(60).collect::<String>()),
            "",
        )
    })
}

/// The other history a failure compares against, if it has one: the part mapped or reordered,
/// or with the boolean's tool cut from it.
pub fn witness(prop: &str, part: &Part, seed: u64) -> Option<Part> {
    match prop {
        "rigid" => rigid_map(seed).part(part),
        "mirror" => mirror_map(seed).part(part),
        "scale" => Map::Scale(scale_factor(seed)).part(part),
        "boolean" => {
            let mut p = part.clone();
            p.features.push(with_op(&tool(seed), Combine::Cut));
            Some(p)
        }
        "order" => {
            let (f, _) = order_pair(part, seed)?;
            let mut p = part.clone();
            p.features.swap(f, f + 1);
            Some(p)
        }
        _ => None,
    }
}

/// Every property, by name.
pub fn all() -> Vec<(&'static str, Property)> {
    vec![
        ("rigid", rigid),
        ("mirror", mirror),
        ("scale", scale),
        ("order", order),
        ("boolean", identities),
        ("backends", backends),
        ("certify", certificate),
        ("membership", membership),
        ("effect", effect),
        ("volume", tool_volume),
        ("mass", mass_mesh),
        ("ulp", ulp),
        ("blend", blend),
    ]
}

/// Every fillet of a part against the part before it (`blends`): each ball surface at the asked
/// radius from two supports, seams meant to be smooth smooth, the volume changed what points say,
/// no patch where two blends meet beside a sharp edge, every point the round changed answered for
/// by a ball surface.
pub fn blend(part: &Part, seed: u64) -> Option<Finding> {
    for (k, f) in part.features.iter().enumerate().take(8) {
        let Feature::Fillet { edges, setbacks, .. } = f else {
            continue;
        };
        let upto = |n: usize| {
            Part {
                features: part.features[..n].to_vec(),
            }
            .regenerate()
            .ok()
        };
        let (Some(before), Some(after)) = (upto(k), upto(k + 1)) else {
            continue;
        };
        let radii = crate::blends::Radii::of(edges.iter().map(|e| &e.1));
        let a = crate::blends::audit(&before, &after, &radii, seed);
        if a.unsupported > 0 {
            return found(
                "blend",
                "a ball surface off its supports",
                format!("feature {k}, {:e} at {:?}", a.worst_gap, a.worst_at),
            );
        }
        if a.worst_kink > 1e-6 {
            return found(
                "blend",
                "a seam meant to be smooth is not",
                format!("feature {k}, sine {:e} at {:?}", a.worst_kink, a.kink_at),
            );
        }
        if let Some(at) = crate::blends::ridges(&before, &after).first() {
            return found(
                "blend",
                "a ball rests on a rim the face beyond runs into",
                format!("feature {k}, at {at:?}"),
            );
        }
        let pts = crate::blends::points(&before, &after, 200, seed);
        if pts.wrong > 0 {
            return found(
                "blend",
                "a point no ball surface answers for",
                format!("feature {k}, {} of {} at {:?}", pts.wrong, pts.tried, pts.worst),
            );
        }
        // the volume it changed, by those points against the masses, within five sigma
        if pts.sampled > 50
            && let (Ok(mb), Ok(ma)) = (forge_topo::mass::mass(&before), forge_topo::mass::mass(&after))
        {
            let f = pts.changed as f64 / pts.sampled as f64;
            let sigma = (f.max(1.0 / pts.sampled as f64) * (1.0 - f) / pts.sampled as f64).sqrt();
            let (estimate, moved) = (f * pts.reach_volume, (mb.volume - ma.volume).abs());
            if (estimate - moved).abs() > 5.0 * sigma * pts.reach_volume + 1e-6 * mb.volume {
                return found(
                    "blend",
                    "the volume a fillet changed is not what points say",
                    format!(
                        "feature {k}: mass {moved}, points {estimate} of box {}",
                        pts.reach_volume
                    ),
                );
            }
        }
        let (off, kink) = crate::blends::patch_seams(&before, &after);
        if off > forge_math::tol::LINEAR || kink > 1e-4 {
            return found(
                "blend",
                "a corner patch off its neighbours",
                format!("feature {k}, {off:e} away, sine {kink:e}"),
            );
        }
        if setbacks.is_empty() && !crate::blends::needless_patches(&before, &after).is_empty() {
            return found(
                "blend",
                "a patch where two blends should mitre",
                format!("feature {k}"),
            );
        }
    }
    None
}

/// The exact mass integrals and meshes of the same body are two independent roads to one volume
/// and one area per face. A mesh's error goes as its sag, so a mesh at a quarter of it tells both
/// the limit and how far off that may be. A mass that cannot be had of a body that meshes is a
/// finding too.
pub fn mass_mesh(part: &Part, _seed: u64) -> Option<Finding> {
    let body = build(part).ok()?;
    if !forge_topo::check(&body).is_empty() {
        return None;
    }
    let body = body.compact();
    let mesh = |chord: f64, max_angle: f64| guarded_mesh(&body, chord, max_angle);
    // (a small part's mesh is as fine for its size)
    let diagonal = body
        .face_ids()
        .fold(forge_math::Box3::EMPTY, |b, f| b.hull(&body.face_box(f)))
        .diagonal();
    let chord = CHORD.min(1e-3 * diagonal);
    let (coarse, fine) = (mesh(chord, 0.5)?, mesh(chord / 4.0, 0.25)?);
    let m = match forge_topo::mass::mass(&body) {
        Ok(m) => m,
        Err(e) => return found("mass", "no mass", format!("{e}; the mesh says {}", fine.volume())),
    };
    let limit = |c: f64, f: f64| (f + (f - c) / 3.0, 0.5 * (f - c).abs());
    let (v, dv) = limit(coarse.volume(), fine.volume());
    // (a mesh of few triangles can stay put from one chord to the next: its sag still counts)
    let sag = chord / 4.0;
    if (m.volume - v).abs() > 2.0 * dv + m.err[0] + 0.25 * sag * m.area {
        return found(
            "mass",
            "volume differs from the mesh",
            format!("mass {} +- {}, mesh {v} +- {dv}", m.volume, m.err[0]),
        );
    }
    let faces = |t: &forge_mesh::TriMesh| {
        let mut a = vec![0.0; body.faces.len()];
        for (tri, &f) in t.triangles.iter().zip(&t.face_of) {
            let [p, q, r] = tri.map(|i| t.positions[i as usize]);
            a[f as usize] += (q - p).cross(r - p).length() * 0.5;
        }
        a
    };
    let (ca, fa) = (faces(&coarse), faces(&fine));
    for &(f, area) in &m.faces {
        let k = f.0 as usize;
        let (want, slack) = limit(ca[k], fa[k]);
        if (area - want).abs() > slack + m.err[1] + 4.0 * sag * body.face_box(f).diagonal() {
            // (a sliver on a thread's flank can mesh badly at these chords: asked once more, far finer)
            if let Some(finer) = mesh(sag / 16.0, 0.06)
                && (area - faces(&finer)[k]).abs() <= m.err[1] + sag * body.face_box(f).diagonal()
            {
                continue;
            }
            return found(
                "mass",
                "a face's area differs from the mesh",
                format!("face {}: mass {area}, mesh {want} +- {slack}", f.0),
            );
        }
    }
    None
}

fn guarded_mesh(b: &Body, chord: f64, max_angle: f64) -> Option<forge_mesh::TriMesh> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tessellate(b, &Quality { chord, max_angle }).ok()
    }))
    .ok()
    .flatten()
}

/// How many points each checked feature samples, and how many features of a part are checked.
const VOLUME_SAMPLES: usize = 1200;
const VOLUME_FEATURES: usize = 6;

/// A cut or add changes the volume by what its tool has in common with the part, or what it has
/// beyond it: estimated here by points tested against the part before and the tool's own solids,
/// no boolean involved. A boolean that loses part of its cut is valid and changes something, so
/// only this sees it. Beyond 5 sigma of the sampling (and a bound for what the part's mass
/// integrates to) is a finding.
pub fn tool_volume(part: &Part, seed: u64) -> Option<Finding> {
    if forge_model::bodies::multi(&part.features) {
        return None;
    }
    let mut r = Rng::new(seed ^ 0x0701_07E5);
    let h = forge_model::History::new(&part.features, None);
    let is_tool = |f: &forge_model::Feature| {
        !makes_nothing(f)
            && matches!(
                f,
                forge_model::Feature::Extrude { .. }
                    | forge_model::Feature::Revolve { .. }
                    | forge_model::Feature::Ball { .. }
                    | forge_model::Feature::Hole(_)
                    | forge_model::Feature::Sweep(_)
                    | forge_model::Feature::Coil(_)
                    | forge_model::Feature::Loft(_)
                    | forge_model::Feature::Thread(_)
                    | forge_model::Feature::Pattern { .. }
            )
    };
    let n = part.features.len();
    let candidates = (1..n).filter(|&k| is_tool(&part.features[k])).count();
    let stride = (candidates / VOLUME_FEATURES).max(1);
    let mut seen = 0;
    let mut body: Option<Body> = None;
    for k in 0..n {
        let next = guarded(|| {
            forge_model::step(h, k, body.clone())
                .map_err(|e| short(&format!("{e:?}")))?
                .ok_or_else(|| "no body".to_string())
        });
        let Ok(next) = next else { return None };
        let check = k > 0 && is_tool(&part.features[k]) && {
            seen += 1;
            seen % stride == 0
        };
        if check && let Some(before) = &body {
            let tools =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| forge_model::feature_tools(h, k)));
            if let Ok(Ok(tools)) = tools
                && let Some((want, sigma, got)) = volume_change(before, &next, &tools, &mut r)
                && (got - want).abs() > 5.0 * sigma + 1e-4 * want.abs().max(got.abs())
            {
                return found(
                    "volume",
                    format!(
                        "{} changed the volume by other than its tool",
                        feature_name(&part.features[k])
                    ),
                    format!("feature {k}: changed {got:.4}, sampled {want:.4} +- {sigma:.4}"),
                );
            }
        }
        body = Some(next);
    }
    None
}

/// The signed volume change expected of the tools (from points), its sampling sigma, and the
/// change the bodies' masses show. None where a mass or a sample cannot be had.
fn volume_change(
    before: &Body,
    after: &Body,
    tools: &[(Body, forge_model::Combine)],
    r: &mut Rng,
) -> Option<(f64, f64, f64)> {
    use forge_bool::{Index, Where};
    use forge_model::Combine;
    let mass = |b: &Body| forge_topo::mass::mass(&b.compact()).ok().map(|m| m.volume);
    let (v0, v1) = (mass(before)?, mass(after)?);
    let op = tools.first()?.1;
    if tools.iter().any(|t| t.1 != op) || !matches!(op, Combine::Add | Combine::Cut) {
        return None;
    }
    let solids: Vec<Index<'_>> = tools.iter().map(|t| Index::new(&t.0)).collect();
    let mut bb = solids.first()?.bbox;
    for s in &solids {
        bb = bb.hull(&s.bbox);
    }
    let ix = Index::new(before);
    if op == Combine::Cut {
        let Some(b) = overlap_box(&bb, &ix.bbox) else {
            return Some((0.0, 0.0, v0 - v1));
        };
        bb = b;
    }
    let volume = (bb.x.hi - bb.x.lo) * (bb.y.hi - bb.y.lo) * (bb.z.hi - bb.z.lo);
    if volume <= 0.0 || volume.is_nan() {
        return None;
    }
    let mut hits = 0.0;
    // a point on a boundary counts half (left out, a tool a few microns thick loses its share)
    let share = |w: Where| match w {
        Where::Inside => 1.0,
        Where::On => 0.5,
        _ => 0.0,
    };
    for _ in 0..VOLUME_SAMPLES {
        let q = Point3::new(
            r.range(bb.x.lo, bb.x.hi),
            r.range(bb.y.lo, bb.y.hi),
            r.range(bb.z.lo, bb.z.hi),
        );
        let in_tool = solids.iter().map(|s| share(s.classify(q))).fold(0.0, f64::max);
        let in_body = share(ix.classify(q));
        hits += match op {
            Combine::Cut => in_tool * in_body,
            _ => in_tool * (1.0 - in_body),
        };
    }
    let asked = VOLUME_SAMPLES as f64;
    // (a share of none or all still has an error: one point's worth)
    let p = (hits + 1.0) / (asked + 2.0);
    // (points within tolerance of a boundary are only known that far: a slab microns thick)
    let (dx, dy, dz) = (bb.x.hi - bb.x.lo, bb.y.hi - bb.y.lo, bb.z.hi - bb.z.lo);
    let band = 2.0 * (dx * dy + dy * dz + dz * dx) * forge_math::tol::LINEAR;
    let sigma = volume * (p * (1.0 - p) / asked).sqrt() + band;
    let want = volume * hits / asked;
    let got = if op == Combine::Cut { v0 - v1 } else { v1 - v0 };
    Some((want, sigma, got))
}

fn overlap_box(a: &forge_math::Box3, b: &forge_math::Box3) -> Option<forge_math::Box3> {
    a.overlaps(b).then(|| {
        forge_math::Box3::from_points([
            Point3::new(a.x.lo.max(b.x.lo), a.y.lo.max(b.y.lo), a.z.lo.max(b.z.lo)),
            Point3::new(a.x.hi.min(b.x.hi), a.y.hi.min(b.y.hi), a.z.hi.min(b.z.hi)),
        ])
    })
}

/// The smallest history found that still fails `prop` with the same kind at `seed`: features
/// dropped one at a time, then numbers rounded.
pub fn shrink(name: &'static str, part: &Part, prop: Property, seed: u64) -> Part {
    let Some(f0) = run(name, prop, part, seed) else {
        return part.clone();
    };
    let fails = |p: &Part| run(name, prop, p, seed).is_some_and(|f| f.kind == f0.kind);
    let mut cur = part.clone();
    // runs of features dropped, halving the run each pass down to one at a time
    let mut run = (cur.features.len() / 2).max(1);
    loop {
        let mut end = cur.features.len();
        while end > 1 {
            let start = end.saturating_sub(run).max(1);
            let t = without(&cur, start..end);
            if fails(&t) {
                cur = t;
            }
            end = start;
        }
        if run == 1 {
            break;
        }
        run /= 2;
    }
    for digits in [0, 1, 2, 3] {
        let t = Part {
            features: cur.features.iter().map(|f| rounded(f, digits)).collect(),
        };
        if fails(&t) {
            cur = t;
            break;
        }
    }
    for i in 0..cur.features.len() {
        for digits in [0, 1, 2] {
            let mut t = cur.clone();
            t.features[i] = rounded(&cur.features[i], digits);
            if t != cur && fails(&t) {
                cur = t;
                break;
            }
        }
    }
    cur
}

/// The part without features `gone`, patterns' sources renumbered to match.
fn without(part: &Part, gone: std::ops::Range<usize>) -> Part {
    let mut t = part.clone();
    t.features.drain(gone.clone());
    for f in &mut t.features {
        if let Feature::Pattern {
            of: forge_model::Source::Features(src),
            ..
        } = f
        {
            src.retain(|i| !gone.contains(i));
            for i in src.iter_mut().filter(|i| **i >= gone.end) {
                *i -= gone.len();
            }
        }
    }
    t
}

/// Lengths and coordinates to `digits` decimals, directions to two more.
fn rounded(f: &Feature, digits: i32) -> Feature {
    let k = 10f64.powi(digits);
    let rd = |v: f64| (v * k).round() / k;
    let rdir = |v: f64| (v * k * 100.0).round() / (k * 100.0);
    let p2 = |p: forge_math::Point2| forge_math::Point2::new(rd(p.x), rd(p.y));
    let rf = |f: &forge_math::Frame| {
        let (o, z, x) = (f.origin, f.z().get(), f.x().get());
        forge_model::adversarial::fr(
            [rd(o.x), rd(o.y), rd(o.z)],
            [rdir(z.x), rdir(z.y), rdir(z.z)],
            [rdir(x.x), rdir(x.y), rdir(x.z)],
        )
    };
    let prof =
        |p: &Profile| Profile {
            plane: rf(&p.plane),
            loops: p
                .loops
                .iter()
                .map(|l| {
                    ProfileLoop::new(
                        l.segments
                            .iter()
                            .map(|(s, id)| {
                                let id = *id;
                                let s =
                                    match *s {
                                        Segment::Spline(ref c) => Segment::Spline(std::sync::Arc::new(
                                            c.remade(&|q| p2(q), &|r| rd(r), &|a| a, false),
                                        )),
                                        Segment::Line { a, b } => Segment::Line { a: p2(a), b: p2(b) },
                                        Segment::Arc {
                                            center,
                                            radius,
                                            start,
                                            sweep,
                                        } => Segment::Arc {
                                            center: p2(center),
                                            radius: rd(radius),
                                            start,
                                            sweep,
                                        },
                                        Segment::Circle { center, radius, ccw } => Segment::Circle {
                                            center: p2(center),
                                            radius: rd(radius),
                                            ccw,
                                        },
                                    };
                                (s, id)
                            })
                            .collect(),
                    )
                })
                .collect(),
        };
    match f {
        Feature::Extrude {
            profile,
            extent: Extent::Distance(d),
            op,
        } => Feature::Extrude {
            profile: prof(profile),
            extent: Extent::Distance(rd(*d)),
            op: *op,
        },
        Feature::Revolve {
            profile,
            axis,
            extent: Turn::Angle(a),
            op,
        } => Feature::Revolve {
            profile: prof(profile),
            axis: *axis,
            extent: Turn::Angle(*a),
            op: *op,
        },
        Feature::Ball { frame, radius, op } => Feature::Ball {
            frame: rf(frame),
            radius: rd(*radius),
            op: *op,
        },
        other => other.clone(),
    }
}

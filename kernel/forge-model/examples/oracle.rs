//! Exports part histories and our results as JSON for the OpenCascade oracle (tools/occt/oracle.py).
//! cargo run --release -p forge-model --example oracle -- [sets] > parts.json
//! env: THREADS, REPS, CHORD (mesh chord for volume), ONLY=name, PREFIXES=k, KEEP=i,j,.., LITERAL (print each part as Rust to stderr)
//! sets: comma separated from suite, torture, finishing, fillets, edits, models, fuzz:N, fc:FILE, dir:DIR
//! (every .fc file in it), adv:N (seeds per family), degen; default suite,torture,fuzz:20
//!
//! Every feature kind has an arm in `export` (the match is exhaustive, so a new kind does not
//! build until it has one). OCCT replays each with its own operation: tools from the feature's
//! own definition (extents, hole profiles, sweeps, lofts, text outlines), finishing ops and
//! face edits on the edges and faces through points picked off our part as it stood. A kind OCCT
//! builds its own way marks the part loose; one it has no operation for cuts the part short
//! there (both kernels then build the prefix). Each such reason is in `export`.
use forge_bool::{Seg2, Shape};
use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_math::{Frame, Point3};
use forge_mesh::{Quality, TriMesh, tessellate};
use forge_model::adversarial::{self, canonical};
use forge_model::{
    BodyFeature, Combine, Cutter, Edit, End, Extent, Feature, Head, History, Keep, Part, PathEnds, Pick,
    RegenError, Side, Source, Target, TextOn, Toward, Turn, body_motion, placements, suite, torture,
};
use forge_ops::{ChamferSize, GlyphSeg, LoftGuides, LoftKind, PathSeg, Profile, Radius, Segment};
use forge_topo::{Body, EdgeId, FaceId, Name};
use std::fmt::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

fn num(v: f64) -> String {
    if v.is_finite() {
        format!("{v:?}")
    } else {
        "null".into()
    }
}

fn v3(v: [f64; 3]) -> String {
    format!("[{},{},{}]", num(v[0]), num(v[1]), num(v[2]))
}

fn p3(p: Point3) -> String {
    v3([p.x, p.y, p.z])
}

fn frame(f: &Frame) -> String {
    let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
    format!(
        "{{\"o\":{},\"x\":{},\"y\":{},\"z\":{}}}",
        p3(f.origin),
        v3([x.x, x.y, x.z]),
        v3([y.x, y.y, y.z]),
        v3([z.x, z.y, z.z])
    )
}

fn segment(s: &Segment) -> String {
    match *s {
        Segment::Line { a, b } => format!(
            "{{\"line\":[{},{},{},{}]}}",
            num(a.x),
            num(a.y),
            num(b.x),
            num(b.y)
        ),
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => format!(
            "{{\"arc\":[{},{},{},{},{}]}}",
            num(center.x),
            num(center.y),
            num(radius),
            num(start),
            num(sweep)
        ),
        Segment::Circle { center, radius, ccw } => format!(
            "{{\"circle\":[{},{},{},{}]}}",
            num(center.x),
            num(center.y),
            num(radius),
            ccw
        ),
    }
}

fn profile(p: &Profile) -> String {
    let loops: Vec<String> = p
        .loops
        .iter()
        .map(|l| {
            let segs: Vec<String> = l.segments.iter().map(|(s, _)| segment(s)).collect();
            format!("[{}]", segs.join(","))
        })
        .collect();
    format!("\"plane\":{},\"loops\":[{}]", frame(&p.plane), loops.join(","))
}

fn op(c: Combine) -> &'static str {
    match c {
        Combine::New => "new",
        Combine::Add => "add",
        Combine::Cut => "cut",
        Combine::Keep => "keep",
    }
}

fn seg2(s: &Seg2) -> String {
    match *s {
        Seg2::Line { a, b } => format!(
            "{{\"line\":[{},{},{},{}]}}",
            num(a.x),
            num(a.y),
            num(b.x),
            num(b.y)
        ),
        Seg2::Arc { c, r, start, sweep } => format!(
            "{{\"arc\":[{},{},{},{},{}]}}",
            num(c.x),
            num(c.y),
            num(r),
            num(start),
            num(sweep)
        ),
    }
}

fn segs2(s: &[Seg2]) -> String {
    format!("[{}]", s.iter().map(seg2).collect::<Vec<_>>().join(","))
}

/// A tool's analytic solid (coils and threads are replayed from these: the winding and the
/// thread form are ours, the helical sweeps OCCT's).
fn shape(s: &Shape) -> Result<String, &'static str> {
    Ok(match s {
        Shape::Prism {
            frame: f,
            segs,
            lo,
            hi,
        } => format!(
            "{{\"prism\":{},\"segs\":{},\"lo\":{},\"hi\":{}}}",
            frame(f),
            segs2(segs),
            num(*lo),
            num(*hi)
        ),
        Shape::Revolved {
            frame: f,
            origin,
            dir,
            perp,
            segs,
            angle,
        } => format!(
            "{{\"revolved\":{},\"origin\":[{},{}],\"dir\":[{},{}],\"perp\":[{},{}],\"segs\":{},\"angle\":{}}}",
            frame(f),
            num(origin.x),
            num(origin.y),
            num(dir.x),
            num(dir.y),
            num(perp.x),
            num(perp.y),
            segs2(segs),
            angle.map_or("null".into(), num)
        ),
        Shape::Ball { center, radius } => format!("{{\"ball\":{},\"r\":{}}}", p3(*center), num(*radius)),
        Shape::Csg { first, rest } => {
            let rest: Result<Vec<String>, _> = rest
                .iter()
                .map(|(o, s)| {
                    let o = match o {
                        forge_bool::Op::Union => "add",
                        forge_bool::Op::Subtract => "cut",
                        forge_bool::Op::Intersect => "keep",
                    };
                    Ok(format!("{{\"op\":\"{o}\",\"shape\":{}}}", shape(s)?))
                })
                .collect();
            format!("{{\"csg\":{},\"rest\":[{}]}}", shape(first)?, rest?.join(","))
        }
        Shape::Coil {
            frame: f,
            segs,
            a,
            b,
            angle,
            ..
        } => format!(
            "{{\"coil\":{},\"segs\":{},\"a\":{},\"b\":{},\"angle\":{}}}",
            frame(f),
            segs2(segs),
            num(*a),
            num(*b),
            num(*angle)
        ),
        Shape::Brep(_) => return Err("a tool with no analytic solid"),
    })
}

/// A surface, whole: what an extent or a thread ends at.
fn surface(s: &Surface) -> String {
    match s {
        Surface::Plane(p) => format!("{{\"s\":\"plane\",\"f\":{}}}", frame(&p.frame)),
        Surface::Cylinder(c) => format!(
            "{{\"s\":\"cylinder\",\"f\":{},\"r\":{}}}",
            frame(&c.frame),
            num(c.radius)
        ),
        Surface::Cone(c) => format!(
            "{{\"s\":\"cone\",\"f\":{},\"r\":{},\"t\":{}}}",
            frame(&c.frame),
            num(c.radius),
            num(c.tan_a())
        ),
        Surface::Sphere(c) => format!(
            "{{\"s\":\"sphere\",\"f\":{},\"r\":{}}}",
            frame(&c.frame),
            num(c.radius)
        ),
        Surface::Torus(c) => format!(
            "{{\"s\":\"torus\",\"f\":{},\"R\":{},\"r\":{}}}",
            frame(&c.frame),
            num(c.major),
            num(c.minor)
        ),
        Surface::Procedural(_) => "null".into(),
    }
}

fn target(features: History<'_>, i: usize, t: &Target) -> String {
    forge_model::extent::surface(features, i, t).map_or("null".into(), |s| surface(&s))
}

/// The surface a body split cuts along (as bodies.rs reads its cutter).
fn cutter(features: History<'_>, i: usize, by: &Cutter) -> String {
    match *by {
        Cutter::Target(t) => target(features, i, &t),
        Cutter::Plane(f) => format!("{{\"s\":\"plane\",\"f\":{}}}", frame(&f)),
        Cutter::Curve(f, Segment::Line { a, b }) => {
            let (a, b) = (f.plane_point(a), f.plane_point(b));
            let Some(along) = (b - a).normalize() else {
                return "null".into();
            };
            let Some(z) = f.z().cross(along.get()).normalize() else {
                return "null".into();
            };
            Frame::from_zx(a, z, along.get()).map_or("null".into(), |p| {
                format!("{{\"s\":\"plane\",\"f\":{}}}", frame(&p))
            })
        }
        Cutter::Curve(f, Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. }) => {
            format!(
                "{{\"s\":\"cylinder\",\"f\":{},\"r\":{}}}",
                frame(&f.with_origin(f.plane_point(center))),
                num(radius)
            )
        }
    }
}

fn path(path: &[PathSeg]) -> String {
    let segs: Vec<String> = path
        .iter()
        .map(|seg| match seg {
            PathSeg::Line { a, b } => format!("{{\"line\":[{},{}]}}", p3(*a), p3(*b)),
            PathSeg::Arc { circle, start, sweep } => format!(
                "{{\"arc\":{},\"r\":{},\"start\":{},\"sweep\":{}}}",
                frame(&circle.frame),
                num(circle.radius),
                num(*start),
                num(*sweep)
            ),
        })
        .collect();
    format!("[{}]", segs.join(","))
}

/// The part as features `..i` left it, compacted, with a mesh to pick points off its faces.
struct Before {
    body: Body,
    mesh: Option<TriMesh>,
}

impl Before {
    /// Every body as one (names are unique across a part's bodies).
    fn new(solids: &[forge_model::Solid]) -> Before {
        let mut body = Body::new();
        for s in solids {
            body.append(&s.body.compact());
        }
        let q = Quality {
            chord: 0.01,
            max_angle: 0.3,
        };
        let mesh = catch_unwind(AssertUnwindSafe(|| tessellate(&body, &q).ok()))
            .ok()
            .flatten();
        Before { body, mesh }
    }

    fn edge(&self, n: Name) -> Result<EdgeId, &'static str> {
        self.body.edge_by_name(n).ok_or("an edge we do not have")
    }

    fn face(&self, n: Name) -> Result<FaceId, &'static str> {
        self.body.face_by_name(n).ok_or("a face we do not have")
    }

    /// Points along an edge, inside its ends (OCCT may split it; each piece gets one).
    fn edge_pts(&self, e: EdgeId) -> String {
        let pts = self.body.edge_points(e, 8);
        format!(
            "[{}]",
            pts[1..8].iter().map(|&p| p3(p)).collect::<Vec<_>>().join(",")
        )
    }

    /// Points inside a face, on its surface: the middles of mesh triangles spread over it (OCCT
    /// may split the face; each piece likely gets one).
    fn face_pts(&self, n: Name) -> Result<String, &'static str> {
        let f = self.face(n)?;
        let mesh = self.mesh.as_ref().ok_or("our part does not mesh")?;
        let mid = |t: &[u32; 3]| {
            let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
            let area = (b - a).cross(c - a).length();
            (
                Point3::new(
                    (a.x + b.x + c.x) / 3.0,
                    (a.y + b.y + c.y) / 3.0,
                    (a.z + b.z + c.z) / 3.0,
                ),
                area,
            )
        };
        let mut tris: Vec<(Point3, f64)> = mesh
            .triangles
            .iter()
            .zip(&mesh.face_of)
            .filter(|x| *x.1 as usize == f.index())
            .map(|x| mid(x.0))
            .collect();
        tris.sort_by(|a, b| b.1.total_cmp(&a.1));
        tris.truncate(200);
        let mut pick: Vec<Point3> = tris.first().map(|t| t.0).into_iter().collect();
        // farthest point sampling: the rest each as far as can be from those picked
        while pick.len() < 6 && pick.len() < tris.len() {
            let far = tris
                .iter()
                .map(|t| {
                    (
                        t.0,
                        pick.iter().map(|p| p.distance(t.0)).fold(f64::INFINITY, f64::min),
                    )
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .expect("triangles");
            if far.1 <= 1e-9 {
                break;
            }
            pick.push(far.0);
        }
        if pick.is_empty() {
            return Err("a face with no triangles");
        }
        let s = &self.body.face(f).surface;
        Ok(format!(
            "[{}]",
            pick.iter()
                .map(|&p| p3(s.eval(s.project(p))))
                .collect::<Vec<_>>()
                .join(",")
        ))
    }

    fn faces(&self, ns: &[Name]) -> Result<String, &'static str> {
        let fs: Result<Vec<String>, _> = ns.iter().map(|&n| self.face_pts(n)).collect();
        Ok(format!("[{}]", fs?.join(",")))
    }

    /// Whether a named edge runs on smoothly into an edge not named (both kernels round the
    /// whole chain; OCCT bevels it too, we cap a chamfer where the names end).
    fn runs_on(&self, ids: &[EdgeId]) -> bool {
        let b = &self.body;
        let ends = |e: EdgeId| {
            let ed = b.edge(e);
            [
                (ed.start, ed.curve.deriv(ed.range.lo).d1),
                (ed.end, ed.curve.deriv(ed.range.hi).d1),
            ]
        };
        ids.iter().any(|&e| {
            ends(e).iter().any(|&(v, t)| {
                b.edge_ids().filter(|f| !ids.contains(f)).any(|f| {
                    ends(f)
                        .iter()
                        .any(|&(w, u)| w == v && t.cross(u).length() <= 1e-6 * t.length() * u.length())
                })
            })
        })
    }

    /// The faces either side of an edge.
    fn sides(&self, e: EdgeId) -> Vec<FaceId> {
        self.body
            .edge(e)
            .coedges
            .iter()
            .map(|&c| self.body.coedge_face(c))
            .collect()
    }
}

/// A fillet's edges: points to find each by, and its law as (point, radius) along it; whether
/// a law runs through points; and whether blends meet at a corner with different radii, or
/// two meet beside an edge left sharp (each kernel patches such corners its own way).
fn rounds(
    b: &Before,
    edges: &[(Name, Radius)],
    starts: &[(Name, Name)],
) -> Result<(String, bool, bool), &'static str> {
    let mut out = Vec::new();
    let mut smooth = false;
    let mut ends: Vec<(forge_topo::VertexId, f64)> = Vec::new();
    for (n, r) in edges {
        let e = b.edge(*n)?;
        let ed = b.body.edge(e);
        // the law runs from the edge's first vertex, or the one `starts` names
        let flip = starts
            .iter()
            .any(|&(se, v)| se == *n && b.body.vertex(ed.end).name == v && b.body.vertex(ed.start).name != v);
        let pts = b.body.edge_points(e, 16);
        let at = |f: f64| {
            let f = if flip { 1.0 - f } else { f };
            match r {
                Radius::Constant(x) => *x,
                Radius::Linear(x, y) => x + (y - x) * f,
                Radius::Points(p) => {
                    let k = p
                        .iter()
                        .position(|q| q.0 >= f)
                        .unwrap_or(p.len() - 1)
                        .max(1)
                        .min(p.len() - 1);
                    let (a, c) = if p.len() == 1 {
                        (p[0], p[0])
                    } else {
                        (p[k - 1], p[k])
                    };
                    if c.0 > a.0 {
                        a.1 + (c.1 - a.1) * ((f - a.0) / (c.0 - a.0)).clamp(0.0, 1.0)
                    } else {
                        a.1
                    }
                }
            }
        };
        smooth |= matches!(r, Radius::Points(p) if p.len() > 2);
        ends.extend([(ed.start, at(0.0)), (ed.end, at(1.0))]);
        let law: Vec<String> = pts
            .iter()
            .enumerate()
            .map(|(k, &p)| format!("[{},{}]", p3(p), num(at(k as f64 / 16.0))))
            .collect();
        out.push(format!(
            "{{\"pts\":{},\"law\":[{}]}}",
            b.edge_pts(e),
            law.join(",")
        ));
    }
    let odd = ends.iter().any(|&(v, r)| {
        let here: Vec<f64> = ends.iter().filter(|e| e.0 == v).map(|e| e.1).collect();
        let degree = b
            .body
            .edge_ids()
            .filter(|&e| [b.body.edge(e).start, b.body.edge(e).end].contains(&v))
            .count();
        here.len() >= 2 && (here.iter().any(|&x| x != r) || degree == 3 && here.len() == 2)
    });
    Ok((format!("[{}]", out.join(",")), smooth, odd))
}

/// One feature as OCCT replays it.
enum Step {
    /// Its JSON, with why OCCT builds it its own way if it does.
    Json(String, Option<&'static str>),
    /// It makes nothing (a work plane, value or sketch: the part holds them evaluated).
    Nothing,
}

/// Feature `i` for the oracle, or why OCCT has no operation for it. `before` is the part
/// features `..i` left (None before the first solid).
fn export(features: History<'_>, i: usize, before: Option<&Before>) -> Result<Step, &'static str> {
    let need = || before.ok_or("nothing to change yet");
    let json = |s: String| Ok(Step::Json(s, None));
    match &features[i] {
        Feature::Extrude {
            profile: p,
            extent,
            op: c,
        } => {
            let e = match *extent {
                Extent::Distance(d) => format!("{{\"d\":{}}}", num(d)),
                Extent::Symmetric(d) => format!("{{\"sym\":{}}}", num(d)),
                Extent::TwoSided { forward, back } => format!("{{\"two\":[{},{}]}}", num(forward), num(back)),
                Extent::Through(t) => format!(
                    "{{\"through\":\"{}\"}}",
                    match t {
                        Toward::Forward => "forward",
                        Toward::Back => "back",
                        Toward::Both => "both",
                    }
                ),
                Extent::To { target: t, offset } => {
                    format!(
                        "{{\"to\":{},\"offset\":{}}}",
                        target(features, i, &t),
                        num(offset)
                    )
                }
                Extent::ToNext { reverse } => format!("{{\"next\":{reverse}}}"),
            };
            json(format!(
                "{{\"kind\":\"extrude\",\"op\":\"{}\",{},\"extent\":{e}}}",
                op(*c),
                profile(p)
            ))
        }
        Feature::Revolve {
            profile: p,
            axis,
            extent,
            op: c,
        } => {
            let t = match *extent {
                Turn::Angle(a) => format!("{{\"a\":{}}}", num(a)),
                Turn::Symmetric(a) => format!("{{\"sym\":{}}}", num(a)),
                Turn::TwoSided { forward, back } => format!("{{\"two\":[{},{}]}}", num(forward), num(back)),
                Turn::To { target: t, offset } => {
                    format!(
                        "{{\"to\":{},\"offset\":{}}}",
                        target(features, i, &t),
                        num(offset)
                    )
                }
            };
            json(format!(
                "{{\"kind\":\"revolve\",\"op\":\"{}\",{},\"axis\":[{},{},{},{}],\"turn\":{t}}}",
                op(*c),
                profile(p),
                num(axis.origin.x),
                num(axis.origin.y),
                num(axis.dir.x),
                num(axis.dir.y)
            ))
        }
        Feature::Ball {
            frame: f,
            radius,
            op: c,
        } => json(format!(
            "{{\"kind\":\"ball\",\"op\":\"{}\",\"center\":{},\"radius\":{}}}",
            op(*c),
            p3(f.origin),
            num(*radius)
        )),
        Feature::Hole(h) => {
            let head = match h.head {
                Head::Plain => "null".into(),
                Head::Counterbore { diameter, depth } => {
                    format!("{{\"cbore\":[{},{}]}}", num(diameter), num(depth))
                }
                Head::Countersink { diameter, angle } => {
                    format!("{{\"csink\":[{},{}]}}", num(diameter), num(angle))
                }
            };
            let end = match h.end {
                End::Through => "{\"through\":true}".into(),
                End::Depth(d) => format!("{{\"depth\":{}}}", num(d)),
                End::To { target: t, offset } => {
                    format!(
                        "{{\"to\":{},\"offset\":{}}}",
                        target(features, i, &t),
                        num(offset)
                    )
                }
                End::ToNext => "{\"next\":false}".into(),
            };
            json(format!(
                "{{\"kind\":\"hole\",\"op\":\"cut\",\"at\":{},\"d\":{},\"head\":{head},\"end\":{end},\"point\":{}}}",
                frame(&h.at),
                num(h.diameter),
                h.point.map_or("null".into(), num)
            ))
        }
        Feature::Sweep(s) => {
            let ends = match &s.ends {
                PathEnds::Path => "null".into(),
                PathEnds::Extended { start, end } => format!("{{\"ext\":[{},{}]}}", num(*start), num(*end)),
                PathEnds::To { target: t, offset } => {
                    format!(
                        "{{\"to\":{},\"offset\":{}}}",
                        target(features, i, t),
                        num(*offset)
                    )
                }
            };
            json(format!(
                "{{\"kind\":\"sweep\",\"op\":\"{}\",{},\"path\":{},\"ends\":{ends}}}",
                op(s.op),
                profile(&s.profile),
                path(&s.path)
            ))
        }
        Feature::Coil(_) | Feature::Thread(_) => {
            // a coil winds its profile, a thread cuts its form's groove (and run-out and
            // chamfer, a pattern's copies of a hole included) one tool after another
            let ts = forge_model::tool_shapes(features, i).map_err(|_| "a coil or thread that fails here")?;
            let ts: Result<Vec<String>, _> = ts
                .iter()
                .map(|(s, c)| Ok(format!("{{\"op\":\"{}\",\"shape\":{}}}", op(*c), shape(s)?)))
                .collect();
            json(format!("{{\"kind\":\"shapes\",\"tools\":[{}]}}", ts?.join(",")))
        }
        Feature::Loft(l) => {
            let ps: Vec<String> = l.profiles.iter().map(|p| format!("{{{}}}", profile(p))).collect();
            let (ruled, loose) = match l.kind {
                LoftKind::Ruled => (true, None),
                LoftKind::Smooth { start, end } => (
                    false,
                    (l.profiles.len() > 2 || start.is_some() || end.is_some()).then_some(
                        "smooth loft: OCCT's ThruSections approximates its own spline through the profiles",
                    ),
                ),
            };
            let (centre, loose) = match &l.guides {
                LoftGuides::None => ("null".into(), loose),
                LoftGuides::CentreLine(c) => (
                    path(c),
                    Some(
                        "loft along a centre line: OCCT's pipe shell runs its own blend between the sections",
                    ),
                ),
                LoftGuides::Rails(_) => return Err("loft on rails: OCCT has no loft that follows rails"),
            };
            Ok(Step::Json(
                format!(
                    "{{\"kind\":\"loft\",\"op\":\"{}\",\"ruled\":{ruled},\"centre\":{centre},\"profiles\":[{}]}}",
                    op(l.op),
                    ps.join(",")
                ),
                loose,
            ))
        }
        Feature::Pattern { of, layout } => {
            let m: Vec<String> = placements(features, i, layout)
                .map_err(|_| "a pattern that fails here")?
                .iter()
                .map(|m| format!("[{}]", m.map(p3).join(",")))
                .collect();
            json(match of {
                Source::Body => format!("{{\"kind\":\"pattern_body\",\"m\":[{}]}}", m.join(",")),
                Source::Features(src) => format!(
                    "{{\"kind\":\"pattern\",\"src\":{:?},\"m\":[{}]}}",
                    src,
                    m.join(",")
                ),
            })
        }
        // work planes, axes, points and frames resolve to frames later features use, evaluated
        Feature::Plane(_) | Feature::Param(_) | Feature::Sketch(_) | Feature::Work(_) => Ok(Step::Nothing),
        Feature::Body(b) => match &**b {
            BodyFeature::In { .. } => unreachable!("unwrapped by export_part"),
            BodyFeature::Combine {
                op: c,
                target: t,
                tools,
                keep,
            } => json(format!(
                "{{\"kind\":\"combine\",\"op\":\"{}\",\"target\":{t},\"tools\":{tools:?},\"keep\":{keep}}}",
                op(*c)
            )),
            BodyFeature::Split { by, keep } => json(format!(
                "{{\"kind\":\"split\",\"by\":{},\"keep\":\"{}\"}}",
                cutter(features, i, by),
                match keep {
                    Keep::Plus => "plus",
                    Keep::Minus => "minus",
                    Keep::Both => "both",
                }
            )),
            // the solid stays the same
            BodyFeature::SplitFace { .. } => Ok(Step::Nothing),
            BodyFeature::Move { steps, copy } => json(format!(
                "{{\"kind\":\"move\",\"m\":[{}],\"copy\":{copy}}}",
                body_motion(steps).map(p3).join(",")
            )),
        },
        Feature::Fillet {
            edges,
            starts,
            setbacks,
        } => {
            let b = need()?;
            let ids: Result<Vec<EdgeId>, _> = edges.iter().map(|e| b.edge(e.0)).collect();
            let (es, smooth, odd) = rounds(b, edges, starts)?;
            let varies = edges.iter().any(|e| e.1.constant().is_none());
            let loose = if !setbacks.is_empty() {
                Some("fillet setbacks: OCCT blends corners its own way")
            } else if varies && b.runs_on(&ids?) {
                Some(
                    "varying fillet run on round a tangent chain: OCCT keeps the edges it is not given constant",
                )
            } else if smooth {
                Some("fillet radius through points: OCCT interpolates its own law")
            } else if odd {
                Some(
                    "fillet corner of unequal radii, or of two blends and a sharp edge: each kernel patches it its own way",
                )
            } else {
                None
            };
            Ok(Step::Json(
                format!("{{\"kind\":\"fillet\",\"edges\":{es}}}"),
                loose,
            ))
        }
        Feature::FaceFillet {
            first,
            second,
            radius,
        } => {
            let b = need()?;
            let fs = |ns: &[Name]| ns.iter().map(|&n| b.face(n)).collect::<Result<Vec<FaceId>, _>>();
            let (f1, f2) = (fs(first)?, fs(second)?);
            // where the two sets meet at edges OCCT rounds those; a strip face between them it
            // cannot take away
            let meet: Vec<EdgeId> = b
                .body
                .edge_ids()
                .filter(|&e| {
                    let s = b.sides(e);
                    s.len() == 2
                        && ((f1.contains(&s[0]) && f2.contains(&s[1]))
                            || (f2.contains(&s[0]) && f1.contains(&s[1])))
                })
                .collect();
            if meet.is_empty() {
                return Err("face fillet across a strip face: OCCT has no fillet that takes a face away");
            }
            let es: Vec<String> = meet
                .iter()
                .map(|&e| {
                    let law: Vec<String> = b
                        .body
                        .edge_points(e, 2)
                        .iter()
                        .map(|&p| format!("[{},{}]", p3(p), num(*radius)))
                        .collect();
                    format!("{{\"pts\":{},\"law\":[{}]}}", b.edge_pts(e), law.join(","))
                })
                .collect();
            json(format!("{{\"kind\":\"fillet\",\"edges\":[{}]}}", es.join(",")))
        }
        Feature::FullRound { .. } => Err("full round: OCCT has no fillet that takes a face away"),
        Feature::Chamfer { edges, size } => {
            let b = need()?;
            let ids: Result<Vec<EdgeId>, _> = edges.iter().map(|&n| b.edge(n)).collect();
            let ids = ids?;
            if b.runs_on(&ids) {
                return Err("chamfer: OCCT bevels a whole chain of tangent edges, we only the edges named");
            }
            let es: Vec<String> = ids.iter().map(|&e| b.edge_pts(e)).collect();
            let size = match *size {
                ChamferSize::Equal(d) => format!("\"d\":[{}]", num(d)),
                ChamferSize::Distances { face, d1, d2 } => {
                    format!("\"d\":[{},{}],\"face\":{}", num(d1), num(d2), b.face_pts(face)?)
                }
                ChamferSize::DistanceAngle { face, d, angle } => format!(
                    "\"d\":[{}],\"angle\":{},\"face\":{}",
                    num(d),
                    num(angle),
                    b.face_pts(face)?
                ),
            };
            json(format!(
                "{{\"kind\":\"chamfer\",\"edges\":[{}],{size}}}",
                es.join(",")
            ))
        }
        Feature::Shell {
            thickness,
            outward,
            open,
        } => {
            let b = need()?;
            json(format!(
                "{{\"kind\":\"shell\",\"t\":{},\"out\":{outward},\"open\":{}}}",
                num(*thickness),
                b.faces(open)?
            ))
        }
        Feature::Draft {
            faces,
            neutral,
            angle,
        } => {
            let b = need()?;
            json(format!(
                "{{\"kind\":\"draft\",\"neutral\":{},\"angle\":{},\"faces\":{}}}",
                frame(neutral),
                num(*angle),
                b.faces(faces)?
            ))
        }
        Feature::Edit(e) => match e {
            Edit::Rib(r) => {
                let segs: Vec<String> = r.path.iter().map(segment).collect();
                json(format!(
                    "{{\"kind\":\"rib\",\"plane\":{},\"path\":[{}],\"t\":{},\"side\":\"{}\",\"web\":{},\"flip\":{},\"draft\":{}}}",
                    frame(&r.plane),
                    segs.join(","),
                    num(r.thickness),
                    side(r.side),
                    r.web,
                    r.flip,
                    num(r.draft)
                ))
            }
            Edit::Offset { faces, distance } => json(format!(
                "{{\"kind\":\"offset\",\"d\":{},\"faces\":{}}}",
                num(*distance),
                need()?.faces(faces)?
            )),
            Edit::Thicken {
                faces,
                thickness,
                side: s,
                op: c,
            } => json(format!(
                "{{\"kind\":\"thicken\",\"op\":\"{}\",\"t\":{},\"side\":\"{}\",\"faces\":{}}}",
                op(*c),
                num(*thickness),
                side(*s),
                need()?.faces(faces)?
            )),
            Edit::Delete { faces } => json(format!(
                "{{\"kind\":\"delete\",\"faces\":{}}}",
                need()?.faces(faces)?
            )),
            Edit::Replace { .. } => {
                Err("replace face: OCCT has no operation that puts a face on another surface")
            }
            Edit::Move { .. } => Err("move faces: OCCT has no operation that moves faces"),
        },
        Feature::Text(t) => {
            // the outlines as our layout sets them (OCCT's own text builder needs a font file
            // and lays text out its own way); the solids are OCCT's
            let font = t.font.font.as_ref().ok_or("text with no font")?;
            let regions =
                forge_model::text::layout(font, &t.text, t.size).map_err(|_| "text that fails here")?;
            let pt = |p: forge_math::Point2| format!("[{},{}]", num(p.x), num(p.y));
            let rs: Vec<String> = regions
                .iter()
                .map(|r| {
                    let ls: Vec<String> = r
                        .iter()
                        .map(|l| {
                            let ss: Vec<String> = l
                                .iter()
                                .map(|s| match *s {
                                    GlyphSeg::Line(a, b) => format!("[{},{}]", pt(a), pt(b)),
                                    GlyphSeg::Quad(a, m, b) => format!("[{},{},{}]", pt(a), pt(m), pt(b)),
                                })
                                .collect();
                            format!("[{}]", ss.join(","))
                        })
                        .collect();
                    format!("[{}]", ls.join(","))
                })
                .collect();
            let d = t.depth;
            let on = match t.on {
                TextOn::Plane(f) => format!(
                    "\"plane\":{},\"lo\":{},\"hi\":{}",
                    frame(&f),
                    num(if t.op == Combine::Add { 0.0 } else { -d }),
                    num(d)
                ),
                TextOn::Cylinder { axis, radius } => format!(
                    "\"axis\":{},\"r\":{},\"lo\":{},\"hi\":{}",
                    frame(&axis),
                    num(radius),
                    num(if t.op == Combine::Add { radius } else { radius - d }),
                    num(radius + d)
                ),
            };
            json(format!(
                "{{\"kind\":\"text\",\"op\":\"{}\",{on},\"regions\":[{}]}}",
                op(t.op),
                rs.join(",")
            ))
        }
    }
}

fn side(s: Side) -> &'static str {
    match s {
        Side::Both => "both",
        Side::Pos => "pos",
        Side::Neg => "neg",
    }
}

/// A part as the oracle replays it: each feature's JSON (with its place in the history), where
/// it was cut short and why, and why it is compared loosely. Stepping stops once our regen
/// fails (that feature is kept: OCCT is asked to build it).
struct Exported {
    features: Vec<String>,
    cut: Option<(usize, &'static str)>,
    loose: Vec<&'static str>,
}

fn export_part(part: &Part) -> Exported {
    let mut out = Exported {
        features: Vec::new(),
        cut: None,
        loose: Vec::new(),
    };
    // a feature on chosen bodies: the feature, with the bodies it is on
    let mut on: Vec<Option<String>> = Vec::new();
    let plain: Vec<Feature> = part
        .features
        .iter()
        .map(|f| match f {
            Feature::Body(b) => match &**b {
                BodyFeature::In { bodies, feature } => {
                    on.push(Some(match bodies {
                        Pick::All => "\"all\"".into(),
                        Pick::Some(ids) => format!("{ids:?}"),
                    }));
                    feature.clone()
                }
                _ => {
                    on.push(None);
                    f.clone()
                }
            },
            _ => {
                on.push(None);
                f.clone()
            }
        })
        .collect();
    let mut solids: Vec<forge_model::Solid> = Vec::new();
    for i in 0..plain.len() {
        // only finishing ops and face edits pick points off the part
        let picks = matches!(
            plain[i],
            Feature::Fillet { .. }
                | Feature::FaceFillet { .. }
                | Feature::Chamfer { .. }
                | Feature::Shell { .. }
                | Feature::Draft { .. }
                | Feature::Edit(_)
        );
        let before = (picks && !solids.is_empty()).then(|| Before::new(&solids));
        match export(History::new(&plain, None), i, before.as_ref()) {
            Ok(Step::Json(j, loose)) => {
                let on = on[i].as_ref().map_or(String::new(), |b| format!("\"in\":{b},"));
                out.features.push(format!("{{\"i\":{i},{on}{}", &j[1..]));
                if let Some(l) = loose.filter(|l| !out.loose.contains(l)) {
                    out.loose.push(l);
                }
            }
            Ok(Step::Nothing) => {}
            Err(why) => {
                out.cut = Some((i, why));
                return out;
            }
        }
        match catch_unwind(AssertUnwindSafe(|| {
            forge_model::step_solids(History::new(&part.features, None), i, std::mem::take(&mut solids))
        })) {
            Ok(Ok(b)) => solids = b,
            _ => {
                if i + 1 < part.features.len() {
                    out.cut = Some((i + 1, "our regen fails before here"));
                }
                return out;
            }
        }
    }
    out
}

/// A face's surface as the oracle keys it: kind and defining numbers (frames in world space).
fn surface_key(f: &Surface) -> String {
    let d = |u: forge_math::UnitVec3| {
        let u = u.get();
        v3([u.x, u.y, u.z])
    };
    match f {
        Surface::Plane(s) => format!(
            "\"s\":\"plane\",\"p\":{},\"n\":{}",
            p3(s.frame.origin),
            d(s.frame.z())
        ),
        Surface::Cylinder(s) => format!(
            "\"s\":\"cylinder\",\"p\":{},\"n\":{},\"r\":{}",
            p3(s.frame.origin),
            d(s.frame.z()),
            num(s.radius)
        ),
        Surface::Cone(s) => format!(
            "\"s\":\"cone\",\"p\":{},\"n\":{},\"t\":{}",
            p3(s.apex()),
            d(s.frame.z()),
            num(s.tan_a())
        ),
        Surface::Sphere(s) => format!(
            "\"s\":\"sphere\",\"p\":{},\"r\":{}",
            p3(s.frame.origin),
            num(s.radius)
        ),
        Surface::Torus(s) => format!(
            "\"s\":\"torus\",\"p\":{},\"n\":{},\"R\":{},\"r\":{}",
            p3(s.frame.origin),
            d(s.frame.z()),
            num(s.major),
            num(s.minor)
        ),
        Surface::Procedural(_) => "\"s\":\"procedural\"".into(),
    }
}

/// Volume and area of the body's mesh, and each face's surface, meshed area and meshed boundary
/// length (the length bounds how far a chord-`c` mesh's area can be off).
fn measure(b: &Body, chord: f64) -> Option<String> {
    let q = Quality {
        chord,
        max_angle: 1.0,
    };
    let m = catch_unwind(AssertUnwindSafe(|| tessellate(b, &q).ok()))
        .ok()
        .flatten()?;
    let n = b.faces.len();
    let mut area = vec![0.0; n];
    let mut sides: Vec<std::collections::HashMap<(u32, u32), i32>> = vec![Default::default(); n];
    for (t, &f) in m.triangles.iter().zip(&m.face_of) {
        let [a, bb, c] = t.map(|i| m.positions[i as usize]);
        area[f as usize] += (bb - a).cross(c - a).length() * 0.5;
        for (i, j) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *sides[f as usize].entry((i.min(j), i.max(j))).or_default() += 1;
        }
    }
    let faces: Vec<String> = (0..n)
        .filter(|&i| b.faces[i].alive)
        .map(|i| {
            let perim: f64 = sides[i]
                .iter()
                .filter(|e| *e.1 == 1)
                .map(|(e, _)| (m.positions[e.0 as usize] - m.positions[e.1 as usize]).length())
                .sum();
            format!(
                "{{{},\"a\":{},\"l\":{}}}",
                surface_key(&b.faces[i].surface),
                num(area[i]),
                num(perim)
            )
        })
        .collect();
    Some(format!(
        ",\"volume\":{},\"area\":{},\"chord\":{},\"face_areas\":[{}]",
        num(m.volume()),
        num(m.area()),
        num(chord),
        faces.join(",")
    ))
}

/// One regen's result: time (best of `reps`), outcome, sizes, and the mesh's measures (every
/// body's, together).
fn run(reps: usize, chord: f64, f: impl Fn() -> Result<Vec<forge_model::Solid>, RegenError>) -> String {
    let mut best = f64::INFINITY;
    let mut out = None;
    for _ in 0..reps {
        let t = Instant::now();
        let r = catch_unwind(AssertUnwindSafe(&f));
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
        out = Some(r);
    }
    match out.expect("ran") {
        Err(_) => format!("{{\"ms\":{},\"status\":\"panic\"}}", num(best)),
        Ok(Err(e)) => format!(
            "{{\"ms\":{},\"status\":\"error\",\"error\":{:?}}}",
            num(best),
            format!("{e:?}").chars().take(200).collect::<String>()
        ),
        Ok(Ok(solids)) => {
            let problems: Vec<_> = solids.iter().flat_map(|s| forge_topo::check(&s.body)).collect();
            let mut b = Body::new();
            for s in &solids {
                b.append(&s.body);
            }
            let mut s = format!(
                "{{\"ms\":{},\"status\":\"{}\",\"faces\":{},\"edges\":{},\"vertices\":{},\"shells\":{}",
                num(best),
                if problems.is_empty() { "ok" } else { "invalid" },
                b.face_ids().count(),
                b.edge_ids().count(),
                b.vertex_ids().count(),
                b.shells.len()
            );
            if let Some(p) = problems.first() {
                let _ = write!(
                    s,
                    ",\"problem\":{:?}",
                    format!("{p:?}").chars().take(160).collect::<String>()
                );
            }
            if let Some(m) = measure(&b, chord) {
                s.push_str(&m);
            }
            // from the surfaces: the cross-check of OCCT's own integration
            if let Ok(m) = forge_topo::mass::mass(&b) {
                let _ = write!(s, ",\"exact\":{},\"exact_err\":{}", num(m.volume), num(m.err[0]));
            }
            s.push('}');
            s
        }
    }
}

/// The one body, or every body where there are several.
fn one_or_all(
    one: Result<Body, RegenError>,
    all: impl Fn() -> Result<Vec<forge_model::Solid>, RegenError>,
) -> Result<Vec<forge_model::Solid>, RegenError> {
    match one {
        Err(RegenError::Bodies { .. }) => all(),
        r => r.map(|body| vec![forge_model::Solid { id: 0, body }]),
    }
}

/// A part file as the agent's model reads it (selectors resolved, parameters evaluated).
fn fc(path: &std::path::Path) -> (String, Part) {
    let src = std::fs::read_to_string(path).expect("a part file");
    let m = forge_agent::model::Model::new(&src);
    let name = path
        .file_stem()
        .map_or("fc".into(), |s| s.to_string_lossy().into_owned());
    (name, Part { features: m.features })
}

/// Part files for what only they write: parameters and sketches, work features, standard
/// holes, patterns along paths and at points, and parts of several bodies.
const MODELS: [(&str, &str); 3] = [
    (
        "model-sketch",
        "w = 20\ns: sketch xy\n  rect 0 0 w 10\nend\nb: extrude 5 s\n",
    ),
    (
        "model-work",
        "base: extrude 10 rect 0 0 60 40
boss: extrude 8 at 50 30 10 circle 0 0 4
top: plane off base (faces of base normal +z) 5
ax: axis boss (faces cyl)
c: point meet ax top
mid: point mid base (edges of base along +x front top)
ucs pts base (verts near 0 0 10) base (verts near 60 0 10) base (verts near 0 40 10)
plane ang base (faces of base normal +z) base (edges of base along +x front top) 30
extrude cut 2 on top at c circle 0 0 2
h: hole M6 tap -xy at 10 10 10 thru
hole M5 ansi close cbore -xy at 30 30 10 12
pattern h pts 10 10 10 10 20 10 10 30 10
pattern h path base (edges of base along +x front top) 3 15
pattern hole2 path base (edges of base line top) 6 fit fixed
",
    ),
    (
        "model-bodies",
        "base: extrude 10 rect 0 0 10 10
lid: extrude new 4 at 0 0 8 rect 0 0 10 10
extrude cut 12 at 5 5 -1 circle 0 0 2 in base
fillet 1 (edges body lid top along +x)
split plane at 0 0 10 in lid
splitface (faces body base normal -y) by plane z 1 0 0 at 5 0 0
spare: copybody by 30 0 0 in base
movebody rot 0 0 0 0 0 1 90 in spare
combine cut base lid keep
",
    ),
];

fn sets(sets: &str) -> Vec<(String, Part)> {
    let mut parts: Vec<(String, Part)> = Vec::new();
    let named = |v: Vec<(&'static str, Part, Option<f64>)>| v.into_iter().map(|(n, p, _)| (n.to_string(), p));
    for set in sets.split(',') {
        let (name, arg) = set.split_once(':').unwrap_or((set, ""));
        let n: u64 = arg.parse().unwrap_or(20);
        match name {
            "suite" => parts.extend(suite::parts().into_iter().map(|(n, p)| (n.to_string(), p))),
            "torture" => parts.push(("torture".into(), torture::part(221))),
            "finishing" => parts.extend(named(forge_model::finishing::parts())),
            "fillets" => {
                // (variable and variant cases included)
                parts.extend(named(forge_model::fillets::cases()))
            }
            "edits" => parts.extend(named(forge_model::edits::parts())),
            "models" => parts.extend(MODELS.iter().map(|(n, src)| {
                let m = forge_agent::model::Model::new(src);
                assert!(m.error.is_none(), "{n}: {:?}", m.error);
                (n.to_string(), Part { features: m.features })
            })),
            "fuzz" => parts.extend((1..=n).map(|s| (format!("fuzz{s}"), suite::fuzz(s, 80)))),
            "fc" => parts.push(fc(std::path::Path::new(arg))),
            "dir" => {
                let mut files: Vec<_> = std::fs::read_dir(arg)
                    .expect("a directory of part files")
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "fc"))
                    .collect();
                files.sort();
                parts.extend(files.iter().map(|p| fc(p)));
            }
            "degen" => parts.extend(adversarial::degenerate_parts()),
            "adv" => {
                for (fam, g) in adversarial::families()
                    .into_iter()
                    .filter(|f| !f.0.starts_with("degen_"))
                {
                    parts.extend((0..n).map(|s| (format!("{fam}/{s}"), canonical(&g(s)))));
                }
            }
            _ => panic!("unknown set {name}"),
        }
    }
    parts
}

fn main() {
    let env = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let threads = env("THREADS", 4.0) as usize;
    let reps = env("REPS", 3.0) as usize;
    let chord = env("CHORD", 0.002);
    let only = std::env::var("ONLY").ok();
    std::panic::set_hook(Box::new(|_| {}));
    let which = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "suite,torture,fuzz:20".into());
    let mut parts = sets(&which);
    // the fillet cases name their edges as the sequential regen does; the n-ary regen may split
    // and name edges otherwise, so it gets the same edges picked again (as fillets.rs checks it)
    let repick: Vec<&str> = if which.split(',').any(|s| s == "fillets") {
        forge_model::fillets::cases().iter().map(|c| c.0).collect()
    } else {
        vec![]
    };
    parts.retain(|(n, _)| only.as_deref().is_none_or(|o| o == n));
    // PREFIXES=k: each part also as its first k, 2k, ... features (to bisect a disagreement)
    let step = env("PREFIXES", 0.0) as usize;
    if step > 0 {
        parts = parts
            .into_iter()
            .flat_map(|(n, p)| {
                let len = p.features.len();
                (1..=len.div_ceil(step)).map(move |i| {
                    let k = (i * step).min(len);
                    let features = p.features[..k].to_vec();
                    (format!("{n}#{k}"), Part { features })
                })
            })
            .collect();
    }
    // KEEP=0,5,9: only those features of each part
    if let Ok(keep) = std::env::var("KEEP") {
        let keep: Vec<usize> = keep.split(',').filter_map(|k| k.parse().ok()).collect();
        for (_, p) in &mut parts {
            p.features = keep.iter().filter_map(|&k| p.features.get(k).cloned()).collect();
        }
    }
    println!("{{\"threads\":{threads},\"parts\":[");
    let mut first = true;
    for (name, mut part) in parts {
        let ex = export_part(&part);
        if let Some((k, why)) = ex.cut {
            eprintln!("{name}: cut at feature {k} of {}: {why}", part.features.len());
            part.features.truncate(k);
        }
        if ex.features.is_empty() {
            eprintln!("{name}: skipped, nothing for OCCT to build");
            continue;
        }
        eprintln!("{name}");
        if std::env::var_os("LITERAL").is_some() {
            eprintln!("{}", adversarial::literal(&canonical(&part)));
        }
        // one body: the single-body regens (what the app runs); several: every body
        let seq = run(reps, chord, || {
            one_or_all(part.regenerate(), || part.regenerate_solids())
        });
        let npart = match part.features.last() {
            Some(Feature::Fillet { .. }) if repick.contains(&name.as_str()) => {
                let mut base = part.clone();
                base.features.pop();
                catch_unwind(AssertUnwindSafe(|| {
                    let nb = base.regenerate_nary(2).ok()?;
                    Some(forge_model::fillets::with_fillet_from(
                        &base,
                        &nb,
                        &forge_model::fillets::picks(&part),
                    ))
                }))
                .ok()
                .flatten()
                .unwrap_or_else(|| part.clone())
            }
            _ => part.clone(),
        };
        let nary1 = run(1, chord, || {
            one_or_all(npart.regenerate_nary(1), || npart.regenerate_nary_solids(1))
        });
        let nary = run(reps, chord, || {
            one_or_all(npart.regenerate_nary(threads), || {
                npart.regenerate_nary_solids(threads)
            })
        });
        let cut = ex.cut.map_or("null".into(), |(k, why)| {
            format!("{{\"at\":{k},\"why\":{why:?}}}")
        });
        let loose: Vec<String> = ex.loose.iter().map(|l| format!("{l:?}")).collect();
        println!(
            "{}{{\"name\":{name:?},\"cut\":{cut},\"loose\":[{}],\"features\":[\n{}\n],\"forge\":{{\"seq\":{seq},\"nary1\":{nary1},\"nary\":{nary}}}}}",
            if first { "" } else { "," },
            loose.join(","),
            ex.features.join(",\n")
        );
        first = false;
    }
    println!("]}}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name of a feature's kind, one per variant (an edit by its own kind).
    fn kind(f: &Feature) -> &'static str {
        match f {
            Feature::Extrude { .. } => "extrude",
            Feature::Revolve { .. } => "revolve",
            Feature::Ball { .. } => "ball",
            Feature::Fillet { .. } => "fillet",
            Feature::FaceFillet { .. } => "face-fillet",
            Feature::FullRound { .. } => "full-round",
            Feature::Chamfer { .. } => "chamfer",
            Feature::Shell { .. } => "shell",
            Feature::Draft { .. } => "draft",
            Feature::Hole(_) => "hole",
            Feature::Pattern { .. } => "pattern",
            Feature::Plane(_) => "plane",
            Feature::Sweep(_) => "sweep",
            Feature::Coil(_) => "coil",
            Feature::Loft(_) => "loft",
            Feature::Edit(e) => match e {
                Edit::Rib(_) => "rib",
                Edit::Offset { .. } => "offset",
                Edit::Thicken { .. } => "thicken",
                Edit::Delete { .. } => "delete",
                Edit::Replace { .. } => "replace",
                Edit::Move { .. } => "move",
            },
            Feature::Param(_) => "param",
            Feature::Sketch(_) => "sketch",
            Feature::Thread(_) => "thread",
            Feature::Text(_) => "text",
            Feature::Work(_) => "work",
            Feature::Body(b) => match &**b {
                BodyFeature::In { feature, .. } => kind(feature),
                BodyFeature::Combine { .. } => "combine",
                BodyFeature::Split { .. } => "split",
                BodyFeature::SplitFace { .. } => "splitface",
                BodyFeature::Move { .. } => "movebody",
            },
        }
    }

    /// Kinds OCCT has no operation for: parts are cut short at them.
    const NO_OCCT: [&str; 3] = ["full-round", "replace", "move"];

    /// Every feature kind turns up in the oracle's parts and exports there (save the kinds OCCT
    /// has no operation for, which cut a part short): a new kind needs an arm in `export` to
    /// build, and a part in these sets to pass.
    #[test]
    fn every_kind_reaches_the_oracle() {
        let parts = sets("suite,finishing,fillets,edits,models");
        let mut seen: Vec<&str> = Vec::new();
        for (name, p) in &parts {
            for (i, f) in p.features.iter().enumerate() {
                let k = kind(f);
                if seen.contains(&k) {
                    continue;
                }
                // the shortest history that has it
                let ex = export_part(&Part {
                    features: p.features[..=i].to_vec(),
                });
                match ex.cut {
                    // a case of the kind OCCT has no operation for (a face fillet across a strip
                    // face): another part must export it
                    Some((at, why)) if at == i && !NO_OCCT.contains(&k) && why.contains("OCCT has no") => {
                        continue;
                    }
                    Some((at, why)) if at == i => {
                        assert!(
                            NO_OCCT.contains(&k),
                            "{name} feature {i} ({k}) does not export: {why}"
                        );
                    }
                    Some((_, why)) => panic!("{name}: our regen fails before feature {i} ({k}): {why}"),
                    None => assert!(!NO_OCCT.contains(&k), "{k} exports now: take it off NO_OCCT"),
                }
                seen.push(k);
            }
        }
        // every name `kind` gives
        let all = [
            "extrude",
            "revolve",
            "ball",
            "fillet",
            "face-fillet",
            "full-round",
            "chamfer",
            "shell",
            "draft",
            "hole",
            "pattern",
            "plane",
            "sweep",
            "coil",
            "loft",
            "rib",
            "offset",
            "thicken",
            "delete",
            "replace",
            "move",
            "param",
            "sketch",
            "thread",
            "text",
            "work",
            "combine",
            "split",
            "splitface",
            "movebody",
        ];
        let missing: Vec<&&str> = all.iter().filter(|k| !seen.contains(k)).collect();
        assert!(missing.is_empty(), "kinds in no oracle part: {missing:?}");
    }
}

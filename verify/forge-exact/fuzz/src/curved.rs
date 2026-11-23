//! The kernel's booleans of solids bounded by planes, cylinders, cones and spheres against the
//! exact curved reference (ForgeExact/QRef in verify/forge-exact).
//!
//! A part goes to the reference as its tools, each the intersection of its faces' inner sides
//! (`dump_curved`): the harness sends only tools that are, checked against the kernel's own
//! classifier at sample points. The reference answers a verdict, a certified volume enclosure, a
//! volume estimate, and the exact membership of probe points: a point whose box of half-width
//! `PROBE_H` lies in one cell of the arrangement (`Quad.member_box`), so the regularised
//! boolean holds at every point of the box or at none. The kernel is judged on its verdict, its
//! volume against the enclosure (and, more loosely, the estimate), and its classifier's answer
//! at every certified probe.

use crate::{Class, Kernel, Reference, exact, guarded, judge_body, short};
use forge_bool::Where;
use forge_geom::{Surface, SurfaceGeom};
use forge_math::{Point2, Point3, Vec3, tol::LINEAR};
use forge_model::{Combine, Feature, History, Part};
use forge_topo::Body;

/// Probes certify a box of this half-width (in LINEARs): no surface of the reference's within
/// it, so the kernel, whose surfaces may sit up to 2·LINEAR from the reference's after the snap,
/// must answer the probe's side.
pub const PROBE_H: f64 = 4.0;
/// Probe offsets off the carriers, in LINEARs.
pub const PROBE_OFF: [f64; 2] = [6.0, 60.0];

fn p3(p: Point3) -> String {
    format!("[{},{},{}]", exact(p.x), exact(p.y), exact(p.z))
}
fn v3(v: Vec3) -> String {
    format!("[{},{},{}]", exact(v.x), exact(v.y), exact(v.z))
}

/// One face as the reference reads it, and the f64 quadric the harness checks it with.
struct Face {
    json: String,
    /// inside: negative
    q: Box<dyn Fn(Point3) -> f64>,
}

fn face_of(body: &Body, f: forge_topo::FaceId) -> Result<Face, String> {
    let face = body.face(f);
    let bb = body.face_box(f);
    let lohi = format!(
        "\"lo\":{},\"hi\":{}",
        p3(Point3::new(bb.x.lo, bb.y.lo, bb.z.lo)),
        p3(Point3::new(bb.x.hi, bb.y.hi, bb.z.hi))
    );
    match &face.surface {
        Surface::Plane(pl) => {
            let o = pl.frame.origin;
            let z = pl.frame.z().get();
            let n = if face.same_sense { z } else { -z };
            Ok(Face {
                json: format!("{{\"kind\":\"plane\",\"o\":{},\"z\":{},{lohi}}}", p3(o), v3(n)),
                q: Box::new(move |p: Point3| (p - o).dot(n)),
            })
        }
        Surface::Cylinder(c) => {
            if !face.same_sense {
                return Err("a cylinder face with the solid outside".into());
            }
            let (o, z, r) = (c.frame.origin, c.frame.z().get(), c.radius);
            Ok(Face {
                json: format!(
                    "{{\"kind\":\"cyl\",\"o\":{},\"z\":{},\"r\":{},{lohi}}}",
                    p3(o),
                    v3(z),
                    exact(r)
                ),
                q: Box::new(move |p: Point3| {
                    let d = p - o;
                    d.dot(d) - d.dot(z).powi(2) - r * r
                }),
            })
        }
        Surface::Sphere(s) => {
            if !face.same_sense {
                return Err("a sphere face with the solid outside".into());
            }
            let (o, r) = (s.frame.origin, s.radius);
            Ok(Face {
                json: format!(
                    "{{\"kind\":\"sphere\",\"o\":{},\"r\":{},{lohi}}}",
                    p3(o),
                    exact(r)
                ),
                q: Box::new(move |p: Point3| (p - o).dot(p - o) - r * r),
            })
        }
        Surface::Cone(k) => {
            if !face.same_sense {
                return Err("a cone face with the solid outside".into());
            }
            let (o, z, r, s, c) = (k.frame.origin, k.frame.z().get(), k.radius, k.sin_a(), k.cos_a());
            Ok(Face {
                json: format!(
                    "{{\"kind\":\"cone\",\"o\":{},\"z\":{},\"r\":{},\"s\":{},\"c\":{},{lohi}}}",
                    p3(o),
                    v3(z),
                    exact(r),
                    exact(s),
                    exact(c)
                ),
                q: Box::new(move |p: Point3| {
                    let d = p - o;
                    let v = d.dot(z);
                    let rho2 = d.dot(d) - v * v;
                    let w = r * c + s * v;
                    if w <= 0.0 { 1.0 } else { c * c * rho2 - w * w }
                }),
            })
        }
        Surface::Torus(t) => {
            if !face.same_sense {
                return Err("a torus face with the solid outside".into());
            }
            let (o, z, big, r) = (t.frame.origin, t.frame.z().get(), t.major, t.minor);
            if !(0.0 < r && r < big) {
                return Err("a torus that is not a ring".into());
            }
            Ok(Face {
                json: format!(
                    "{{\"kind\":\"torus\",\"o\":{},\"z\":{},\"R\":{},\"r\":{},{lohi}}}",
                    p3(o),
                    v3(z),
                    exact(big),
                    exact(r)
                ),
                q: Box::new(move |p: Point3| {
                    let d = p - o;
                    let s2 = d.dot(d);
                    let w = d.dot(z);
                    let a = s2 + big * big - r * r;
                    a * a - 4.0 * big * big * (s2 - w * w / z.dot(z))
                }),
            })
        }
        Surface::Procedural(_) => Err("a face is procedural".into()),
    }
}

thread_local! {
    /// The feature ids of the part being judged (a forge_lang program's; `None`: positions).
    /// Names in a part refer to faces by these ids, so its regens and tools must use them.
    static IDS: std::cell::RefCell<Option<Vec<u64>>> = const { std::cell::RefCell::new(None) };
}

/// Runs `f` with the part's feature ids set (see `IDS`).
pub fn with_ids<T>(ids: Option<Vec<u64>>, f: impl FnOnce() -> T) -> T {
    let old = IDS.with(|c| c.replace(ids));
    let out = f();
    IDS.with(|c| *c.borrow_mut() = old);
    out
}

/// The ids for the first `n` features.
fn ids_for(n: usize) -> Option<Vec<u64>> {
    IDS.with(|c| c.borrow().as_ref().map(|v| v[..n.min(v.len())].to_vec()))
}

/// Deterministic sample points.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// A tool as the reference reads it, if it is the intersection of its faces' inner sides:
/// checked against the kernel's classifier at 400 points of its box grown by a fifth.
pub fn dump_curved(body: &Body, op: Combine) -> Result<String, String> {
    let b = body.compact();
    let mut faces = Vec::new();
    for f in b.face_ids() {
        faces.push(face_of(&b, f)?);
    }
    if faces.is_empty() {
        return Err("a tool with no faces".into());
    }
    let idx = forge_bool::Index::new(&b);
    let bb = b.faces_box();
    let g = 0.2 * (bb.x.hi - bb.x.lo).max(bb.y.hi - bb.y.lo).max(bb.z.hi - bb.z.lo);
    let mut rng = Lcg(0x5eed ^ faces.len() as u64);
    for _ in 0..400 {
        let p = Point3::new(
            bb.x.lo - g + (bb.x.hi - bb.x.lo + 2.0 * g) * rng.next(),
            bb.y.lo - g + (bb.y.hi - bb.y.lo + 2.0 * g) * rng.next(),
            bb.z.lo - g + (bb.z.hi - bb.z.lo + 2.0 * g) * rng.next(),
        );
        let vals: Vec<f64> = faces.iter().map(|f| (f.q)(p)).collect();
        // skip points near a carrier: the check is for the shape, not the tolerance
        if vals.iter().any(|v| v.abs() < 1e-6 * (1.0 + g * g)) {
            continue;
        }
        let formula = vals.iter().all(|v| *v < 0.0);
        match idx.classify(p) {
            Where::On => {}
            Where::Inside if !formula => return Err("not the intersection of its faces' inner sides".into()),
            Where::Outside if formula => return Err("not the intersection of its faces' inner sides".into()),
            _ => {}
        }
    }
    let op = match op {
        Combine::New => "new",
        Combine::Add => "add",
        Combine::Cut => "cut",
        Combine::Keep => "keep",
    };
    Ok(format!(
        "{{\"op\":\"{op}\",\"faces\":[{}]}}",
        faces
            .iter()
            .map(|f| f.json.as_str())
            .collect::<Vec<_>>()
            .join(",")
    ))
}

fn inert(f: &Feature) -> bool {
    matches!(
        f,
        Feature::Param(_) | Feature::Sketch(_) | Feature::Plane(_) | Feature::Work(_)
    )
}

/// The tools of a part's history, and the probe points: on every tool face, points pushed off
/// the carrier by `PROBE_OFF` both ways, and points spread over the whole box.
pub fn curved_tools(part: &Part) -> Result<(Vec<String>, Vec<Point3>, forge_math::Box3), String> {
    if forge_model::bodies::multi(&part.features) {
        return Err("several bodies".into());
    }
    let ids = ids_for(part.features.len());
    let h = History::new(&part.features, ids.as_deref());
    let mut solids = Vec::new();
    let mut probes = Vec::new();
    let mut bbox: Option<forge_math::Box3> = None;
    for (i, f) in part.features.iter().enumerate() {
        if inert(f) {
            continue;
        }
        let tools = guarded(|| forge_model::feature_tools(h, i).map_err(|e| format!("{e:?}")))
            .map_err(|e| format!("feature {i}'s tool: {}", short(&e)))?;
        for (body, op) in tools {
            if !solids.is_empty() && op == Combine::New {
                return Err("several bodies".into());
            }
            solids.push(dump_curved(&body, op).map_err(|e| format!("feature {i}: {e}"))?);
            let b = body.compact();
            let bb = b.faces_box();
            bbox = Some(match bbox {
                None => bb,
                Some(x) => x.hull(&bb),
            });
            let mut rng = Lcg(0xbeef ^ (i as u64) << 8 ^ solids.len() as u64);
            for f in b.face_ids() {
                let s = &b.face(f).surface;
                let fb = b.face_box(f);
                for _ in 0..6 {
                    let p = Point3::new(
                        fb.x.lo + (fb.x.hi - fb.x.lo) * rng.next(),
                        fb.y.lo + (fb.y.hi - fb.y.lo) * rng.next(),
                        fb.z.lo + (fb.z.hi - fb.z.lo) * rng.next(),
                    );
                    let uv: Point2 = s.project(p);
                    let on = s.eval(uv);
                    let Some(n) = s.normal(uv) else { continue };
                    for k in PROBE_OFF {
                        probes.push(on + n.get() * (k * LINEAR));
                        probes.push(on - n.get() * (k * LINEAR));
                    }
                }
            }
        }
    }
    let Some(bb) = bbox else {
        return Err("no solids".into());
    };
    let mut rng = Lcg(0xfeed);
    for _ in 0..96 {
        probes.push(Point3::new(
            bb.x.lo + (bb.x.hi - bb.x.lo) * rng.next(),
            bb.y.lo + (bb.y.hi - bb.y.lo) * rng.next(),
            bb.z.lo + (bb.z.hi - bb.z.lo) * rng.next(),
        ));
    }
    Ok((solids, probes, bb))
}

/// The reference's question for a part under a rule, with its probes.
pub fn curved_input(part: &Part, rule: &str) -> Result<(String, Vec<Point3>), String> {
    curved_input_hints(part, rule, &[])
}

/// The question with points for the reference to examine (`hints`: where the kernel refused).
pub fn curved_input_hints(
    part: &Part,
    rule: &str,
    hints: &[Point3],
) -> Result<(String, Vec<Point3>), String> {
    let (solids, probes, _) = curved_tools(part)?;
    Ok((
        format!(
            "{{\"curved\":true,\"linear\":{},\"rule\":\"{rule}\",\"probe_h\":{},\"probes\":[{}],\"hints\":[{}],\"solids\":[{}]}}",
            exact(LINEAR),
            exact(PROBE_H * LINEAR),
            probes.iter().map(|p| p3(*p)).collect::<Vec<_>>().join(","),
            hints.iter().map(|p| p3(*p)).collect::<Vec<_>>().join(","),
            solids.join(",")
        ),
        probes,
    ))
}

/// The location a ZeroThickness refusal names, if any.
pub fn refusal_at(e: &str) -> Option<Point3> {
    let (_, rest) = e.split_once("at: Point3 {")?;
    let num = |k: &str| -> Option<f64> {
        let (_, r) = rest.split_once(&format!("{k}: "))?;
        r.split([',', ' ', '}']).next()?.parse().ok()
    };
    Some(Point3::new(num("x")?, num("y")?, num("z")?))
}

/// The curved reference's answer.
#[derive(Clone, Debug, Default)]
pub struct CAnswer {
    pub verdict: String,
    pub lo: f64,
    pub hi: f64,
    pub estimate: f64,
    /// the estimate's own error: its two resolutions' difference
    pub estimate_err: f64,
    pub probes: Vec<i64>,
    pub notes: Vec<String>,
    pub error: Option<String>,
}

fn field<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let k = format!("\"{key}\":");
    let at = s.find(&k)? + k.len();
    Some(&s[at..])
}

pub fn parse_canswer(s: &str) -> CAnswer {
    // the shared reader takes arrays of strings: the numeric probes are cut out for it
    let rest = match s.find("\"probes\":[") {
        Some(at) => {
            let end = s[at..].find(']').map(|e| at + e + 1).unwrap_or(s.len());
            format!("{}{}", &s[..at], s[end..].trim_start_matches(','))
        }
        None => s.to_string(),
    };
    let a = crate::parse_answer(&rest);
    let num = |k: &str| {
        field(s, k)
            .and_then(|r| r.split([',', '}']).next())
            .and_then(|v| v.trim().parse::<f64>().ok())
            .unwrap_or(f64::NAN)
    };
    let probes = field(s, "probes")
        .and_then(|r| r.strip_prefix('['))
        .and_then(|r| r.split(']').next())
        .map(|r| r.split(',').filter_map(|v| v.trim().parse().ok()).collect())
        .unwrap_or_default();
    CAnswer {
        verdict: a.verdict,
        lo: num("volume_lo_f"),
        hi: num("volume_hi_f"),
        estimate: num("volume_f"),
        estimate_err: num("volume_err"),
        probes,
        notes: a.notes,
        error: a.error,
    }
}

impl Reference {
    pub fn ask_curved(&mut self, line: &str) -> CAnswer {
        let a = self.ask_raw(line);
        match a {
            Ok(s) => parse_canswer(&s),
            Err(e) => CAnswer {
                error: Some(e),
                ..CAnswer::default()
            },
        }
    }
}

/// Volumes agree with the estimate to this relative slack (the estimate is a midpoint rule over
/// exact splits of vertical lines, Gauss–Legendre between breakpoints: good to about 1e-9 on
/// these parts).
pub const ESTIMATE_SLACK: f64 = 1e-5;

/// The kernel's answer at each certified probe against the reference's: the first that
/// disagrees.
fn probe_check(body: &Body, probes: &[Point3], ans: &[i64]) -> Option<String> {
    let idx = forge_bool::Index::new(body);
    for (p, &a) in probes.iter().zip(ans) {
        if a == 0 {
            continue;
        }
        let w = idx.classify(*p);
        let ok = matches!((a, w), (1, Where::Inside) | (-1, Where::Outside));
        if !ok {
            return Some(format!(
                "probe ({:.6}, {:.6}, {:.6}) is {} where the kernel's classifier says {:?}",
                p.x,
                p.y,
                p.z,
                if a == 1 { "in" } else { "out" },
                w
            ));
        }
    }
    None
}

/// A kernel answer against the curved reference.
pub fn classify_curved(r: &CAnswer, k: &Kernel, body: Option<&Body>, probes: &[Point3]) -> Class {
    if let Some(e) = &r.error {
        return Class::Scope(format!("reference: {e}"));
    }
    if let Kernel::Refused(e) = k
        && e.starts_with("NothingToCombine")
    {
        return Class::Scope("the history combines before any part exists".into());
    }
    let volume_ok = |volume: f64, area: f64| -> Option<String> {
        let s = crate::SNAP_CAP * LINEAR * area + crate::VOLUME_SLACK * volume.abs().max(1.0);
        if volume < r.lo - s || volume > r.hi + s {
            Some(format!(
                "volume {volume} outside the certified enclosure [{}, {}]",
                r.lo, r.hi
            ))
        } else if (volume - r.estimate).abs()
            > ESTIMATE_SLACK * r.estimate.abs().max(1.0) + 10.0 * r.estimate_err.max(0.0) + s
        {
            Some(format!(
                "volume {volume} where the reference estimates {}",
                r.estimate
            ))
        } else {
            None
        }
    };
    match r.verdict.as_str() {
        "ambiguous" => Class::Ambiguous,
        "solid" | "sliver" => match k {
            Kernel::Refused(e) if r.verdict == "sliver" && e.contains("ZeroThickness") => Class::Agree,
            Kernel::Refused(e) => {
                Class::Refused(format!("refused {e} where the reference gives a {}", r.verdict))
            }
            Kernel::Built { invalid: Some(p), .. } => Class::Wrong(format!("invalid body {p}")),
            Kernel::Built { volume, area, .. } => {
                if let Some(w) = volume_ok(*volume, *area) {
                    return Class::Wrong(w);
                }
                if r.verdict == "solid"
                    && let Some(b) = body
                    && let Some(w) = probe_check(b, probes, &r.probes)
                {
                    return Class::Wrong(w);
                }
                Class::Agree
            }
        },
        "empty" => match k {
            Kernel::Refused(e) if e.starts_with("Empty") => Class::Agree,
            Kernel::Refused(e) => Class::Refused(format!("refused {e} where the result is empty")),
            Kernel::Built { volume, .. } => {
                Class::Wrong(format!("built volume {volume} where the result is empty"))
            }
        },
        "nonmanifold" => match k {
            Kernel::Refused(e) if e.contains("ZeroThickness") => Class::Agree,
            Kernel::Refused(e) => Class::Refused(format!(
                "refused {e} where the result is non-manifold (ZeroThickness expected)"
            )),
            Kernel::Built { volume, .. } => Class::Wrong(format!(
                "built volume {volume} where the result is non-manifold ({})",
                r.notes.first().cloned().unwrap_or_default()
            )),
        },
        v => Class::Scope(format!("reference verdict {v}")),
    }
}

/// The kernel's body for a path, and how it is judged.
pub fn kernel_path(part: &Part, path: &str) -> (Kernel, Option<Body>) {
    let ids = ids_for(part.features.len());
    let r = guarded(|| match (path, &ids) {
        ("nary", None) => part.regenerate_nary(1).map_err(|e| format!("{e:?}")),
        // with ids: the regen forge_lang runs, so the names are its names
        ("nary", Some(ids)) => part
            .regenerate_nary_solids_ids(1, Some(ids))
            .map_err(|e| format!("{e:?}"))
            .and_then(|mut s| match s.len() {
                1 => Ok(s.remove(0).body),
                n => Err(format!("{n} bodies")),
            }),
        (_, ids) => part.regenerate_ids(ids.as_deref()).map_err(|e| format!("{e:?}")),
    });
    let body = r.as_ref().ok().cloned();
    (judge_body(r), body)
}

/// A refusal at feature `n` against the reference's answer for the history up to `n`.
pub fn stage_class_curved(a: &CAnswer, e: &str, n: usize) -> Class {
    if let Some(err) = &a.error {
        return Class::Scope(format!("reference: {err}"));
    }
    match a.verdict.as_str() {
        "empty" | "nonmanifold" => Class::Agree,
        "sliver" if e.contains("ZeroThickness") => Class::Agree,
        "ambiguous" => Class::Ambiguous,
        v => Class::Refused(format!(
            "refused {e} where the reference gives {v} at feature {n}"
        )),
    }
}

/// Both kernel paths of a part, judged against one answer of the reference; refusals at the
/// stage they name.
pub fn judge_curved(
    reference: &mut Reference,
    part: &Part,
    rule: &str,
) -> Vec<(&'static str, Class, Option<CAnswer>)> {
    let (input, probes) = match curved_input(part, rule) {
        Ok(x) => x,
        Err(e) => {
            return vec![
                ("sequential", Class::Scope(e.clone()), None),
                ("nary", Class::Scope(e), None),
            ];
        }
    };
    let ans = reference.ask_curved(&input);
    let mut out = Vec::new();
    for path in ["sequential", "nary"] {
        let (k, body) = kernel_path(part, path);
        let class = if let Kernel::Refused(e) = &k
            && !e.starts_with("NothingToCombine")
            && let Some(n) = crate::refused_at(e)
        {
            if n + 1 < part.features.len() {
                let mut prefix = part.clone();
                prefix.features.truncate(n + 1);
                match curved_input(&prefix, rule) {
                    Err(e) => Class::Scope(e),
                    Ok((input, _)) => {
                        let c = stage_class_curved(&reference.ask_curved(&input), e, n);
                        match (&c, refusal_at(e)) {
                            (Class::Refused(_), Some(at)) => match curved_input_hints(&prefix, rule, &[at]) {
                                Ok((input, _)) => stage_class_curved(&reference.ask_curved(&input), e, n),
                                Err(_) => c,
                            },
                            _ => c,
                        }
                    }
                }
            } else {
                let c = stage_class_curved(&ans, e, n);
                // a refusal where the reference gives a solid: the reference examines the
                // refusal's location itself (a near touch there makes it a sliver)
                match (&c, refusal_at(e)) {
                    (Class::Refused(_), Some(at)) => match curved_input_hints(part, rule, &[at]) {
                        Ok((input, _)) => stage_class_curved(&reference.ask_curved(&input), e, n),
                        Err(_) => c,
                    },
                    _ => c,
                }
            }
        } else {
            classify_curved(&ans, &k, body.as_ref(), &probes)
        };
        out.push((path, class, Some(ans.clone())));
    }
    out
}

/// One path of a part, judged.
pub fn judge_curved_path(
    reference: &mut Reference,
    part: &Part,
    rule: &str,
    path: &str,
) -> (Class, Option<CAnswer>) {
    let all = judge_curved(reference, part, rule);
    let (_, c, a) = all.into_iter().find(|(p, _, _)| *p == path).expect("a path");
    (c, a)
}

/// The part with every feature whose tools are not quadric solids the reference takes dropped
/// (its convex-quadric sub-history); features that make nothing are kept.
pub fn curved_part(part: &Part) -> Part {
    curved_part_ids(part, None).0
}

/// `curved_part`, with the features' ids (a forge_lang program's) dropped alongside.
pub fn curved_part_ids(part: &Part, ids: Option<&[u64]>) -> (Part, Option<Vec<u64>>) {
    let mut p = part.clone();
    let mut ids = ids.map(|v| v.to_vec());
    let mut i = 1;
    while i < p.features.len() {
        let f = &p.features[i];
        let ok = inert(f)
            || guarded(|| {
                forge_model::feature_tools(History::new(&p.features, ids.as_deref()), i)
                    .map_err(|e| format!("{e:?}"))
            })
            .is_ok_and(|ts| ts.iter().all(|(b, op)| dump_curved(b, *op).is_ok()));
        if ok {
            i += 1;
        } else {
            p.features.remove(i);
            if let Some(v) = ids.as_mut() {
                v.remove(i);
            }
        }
    }
    (p, ids)
}

/// The tool (the last feature) moved by `d`.
pub fn tool_moved(part: &Part, d: Vec3) -> Option<Part> {
    let mut p = part.clone();
    let last = p.features.len() - 1;
    p.features[last] = forge_model::names::moved(&p.features[last], d)?;
    Some(p)
}

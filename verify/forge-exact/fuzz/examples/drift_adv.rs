//! FLAT-SITES δ-accumulation: adversarial drift search (research/booleans/drift.md).
//! `drift_adv FAMILY N ITERS [check]`: N sequential booleans on one plate (FAMILY edge, top or
//! angle), their parameters hill-climbed for ITERS rounds (seed 11) to maximise the largest vertex
//! distance from its planes; per op of the best sequence: that distance, the distance from the
//! exact meet of its best triple, the largest vertex tol, validity, refusal, and (with `check`) the
//! exact reference's verdict on the op's own inputs. Then: the most a vertex is past its tol from
//! a plane (`worst-under`), the ops breaking `check::vertices_off_planes` (`off-composed`).
//! Environment:
//! - `OBJ=under`: maximise (distance from a plane - the vertex's tol) instead;
//! - `QS_OUT=F` / `QS_IN=F`: save the best parameters / start from saved ones (ITERS 0: replay);
//! - `SKIP=i,j,..`, `UPTO=n`: leave out ops by their index, stop after op n (to shrink a case);
//! - `DETAIL=1`: each op's worst vertex, its edges, their tolerances and faces;
//! - `RESNAP=merge|cond|all`: the re-snap probe after each boolean (`resnap`, drift.md §7).
use forge_bool::{Op, boolean};
use forge_exact_fuzz::pairs::{kernel_pair, pair_input};
use forge_exact_fuzz::{Class, Reference, classify};
use forge_geom::CurveGeom as _;
use forge_geom::Surface;
use forge_math::exact::{DyPlane, DyV3, meet3};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_ops::{Profile, ProfileLoop, Segment, extrude};
use forge_topo::{Body, FaceId};

const L: f64 = 1e-5;

fn prism(f: Frame, pts: &[(f64, f64)], h: f64, id: u64) -> Option<Body> {
    let n = pts.len();
    let segs = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (
                Segment::Line {
                    a: Point2::new(a.0, a.1),
                    b: Point2::new(b.0, b.1),
                },
                id * 10 + i as u64,
            )
        })
        .collect();
    extrude(
        &Profile {
            plane: f,
            loops: vec![ProfileLoop::new(segs)],
        },
        h,
        id,
    )
    .ok()
}

/// Largest vertex distance to its faces' exact planes, to the exact meet of its best triple,
/// and the largest vertex tol.
fn drift(r: &Body) -> (f64, f64, f64) {
    let mut faces: Vec<Vec<FaceId>> = vec![Vec::new(); r.vertices.len()];
    for e in r.edge_ids() {
        let ed = r.edge(e);
        for &c in &ed.coedges {
            let f = r.coedge_face(c);
            for v in [ed.start, ed.end] {
                if !faces[v.index()].contains(&f) {
                    faces[v.index()].push(f);
                }
            }
        }
    }
    let (mut pm, mut mm, mut tm) = (0.0f64, 0.0f64, 0.0f64);
    let mut um = f64::NEG_INFINITY;
    for v in r.vertex_ids() {
        let p = r.vertex(v).point;
        tm = tm.max(r.vertex(v).tol);
        let pl: Vec<(DyPlane, Vec3)> = faces[v.index()]
            .iter()
            .filter_map(|&f| match &r.face(f).surface {
                Surface::Plane(q) => {
                    let (o, z) = (q.frame.origin, q.frame.z().get());
                    Some((DyPlane::from_frame([o.x, o.y, o.z], [z.x, z.y, z.z]), z))
                }
                _ => None,
            })
            .collect();
        for (d, z) in &pl {
            let dd = d.eval(&DyV3::from_f64(p.x, p.y, p.z)).to_f64().abs() / z.length();
            pm = pm.max(dd);
            um = um.max(dd - r.vertex(v).tol);
        }
        let mut best: Option<(f64, usize, usize, usize)> = None;
        for i in 0..pl.len() {
            for j in i + 1..pl.len() {
                for k in j + 1..pl.len() {
                    let det = pl[i].1.dot(pl[j].1.cross(pl[k].1)).abs();
                    if best.is_none_or(|b| det > b.0) {
                        best = Some((det, i, j, k));
                    }
                }
            }
        }
        if let Some((det, i, j, k)) = best
            && det > 1e-3
            && let Some(m) = meet3(&pl[i].0, &pl[j].0, &pl[k].0)
        {
            let a = m.to_f64();
            mm = mm.max(Point3::new(a[0], a[1], a[2]).distance(p));
        }
    }
    UNDER.with(|u| u.set(u.get().max(um)));
    (pm, mm, tm)
}

thread_local! {
    static BOOL_SECS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    /// Re-snapped vertices, booleans, seconds spent re-snapping.
    static RESNAP_STAT: std::cell::Cell<(usize, usize, f64)> = const { std::cell::Cell::new((0, 0, 0.0)) };
    /// Ops whose result has a vertex off a plane past its tolerances (`vertices_off_planes`).
    static OFF: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// The largest (vertex distance from one of its faces' planes) - (its tol) seen.
    static UNDER: std::cell::Cell<f64> = const { std::cell::Cell::new(f64::NEG_INFINITY) };
}

/// The tool of op i in a family, from its parameters (each in [-1, 1]).
fn tool(fam: &str, i: usize, q: &[f64]) -> Option<(Body, Op)> {
    let id = 100 + i as u64;
    let op = if i.is_multiple_of(2) {
        Op::Subtract
    } else {
        Op::Union
    };
    match fam {
        // a wall near the plate's y = 0 wall, its ends within ±2 LINEAR of it, over a stretch
        "edge" => {
            let (x0, x1) = (50.0 + 45.0 * q[0], 50.0 + 45.0 * q[0] + 5.0 + 40.0 * (q[1] + 1.0));
            let (e0, e1) = (2.0 * L * q[2], 2.0 * L * q[3]);
            let z0 = -1.0 + 6.0 * (q[4] + 1.0);
            let f = Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z0));
            let pts = if op == Op::Subtract {
                vec![(x0, -10.0), (x1, -10.0), (x1, 5.0 + e1), (x0, 5.0 + e0)]
            } else {
                vec![(x0, -3.0), (x1, -3.0), (x1, e1), (x0, e0)]
            };
            // subtract: a notch from y = 5 down past the wall: its walls cross y = 0 near the edge
            let pts = if op == Op::Subtract {
                vec![(x0, e0 - 3.0), (x1, e1 - 3.0), (x1, e1), (x0, e0)]
            } else {
                pts
            };
            prism(f, &pts, 4.0 + 2.0 * (q[5] + 1.0), id).map(|b| (b, op))
        }
        // near-coplanar stacks on the top face z = 10: slabs whose bottom or top is within 2 LINEAR
        "top" => {
            let (x0, y0) = (50.0 + 40.0 * q[0], 50.0 + 40.0 * q[1]);
            let (w, h) = (5.0 + 20.0 * (q[2] + 1.0), 5.0 + 20.0 * (q[3] + 1.0));
            let z = 10.0 + 2.0 * L * q[4];
            let tilt = 1e-7 * q[5];
            let n = Vec3::new(tilt, -tilt, 1.0).normalize()?;
            let f = Frame::from_zx(
                Point3::new(x0, y0, if op == Op::Union { z } else { z - 3.0 }),
                n,
                Vec3::new(1.0, 0.0, 0.0),
            )?;
            prism(f, &[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)], 3.0, id).map(|b| (b, op))
        }
        // tiny-angle walls: prisms through the plate tilted 1e-9 … 1e-6 rad, overlapping
        _ => {
            let a = forge_math::dmath::powf(10.0, -9.0 + 1.5 * (q[0] + 1.0));
            let n = Vec3::new(a * q[1].signum(), a * q[2], 1.0).normalize()?;
            let o = Point3::new(50.0 + 30.0 * q[3], 50.0 + 30.0 * q[4], -2.0);
            let f = Frame::from_zx(o, n, Vec3::new(1.0, 1e-3 * q[5], 0.0))?;
            prism(
                f,
                &[(0.0, 0.0), (20.0, 0.0), (20.0, 15.0), (0.0, 15.0)],
                if op == Op::Union { 5.0 } else { 20.0 },
                id,
            )
            .map(|b| (b, op))
        }
    }
}

/// The vertex farthest from one of its faces' planes: its tol, each face's distance, each edge's
/// tol and the vertex's distance from the edge's curve (and the edge's from the face).
fn detail(r: &Body) {
    let mut worst = (f64::NEG_INFINITY, None);
    for e in r.edge_ids() {
        let ed = r.edge(e);
        for &c in &ed.coedges {
            let f = r.coedge_face(c);
            if let Surface::Plane(q) = &r.face(f).surface {
                for v in [ed.start, ed.end] {
                    let d = q.signed_distance(r.vertex(v).point).abs() - r.vertex(v).tol;
                    if d > worst.0 {
                        worst = (d, Some(v));
                    }
                }
            }
        }
    }
    let Some(v) = worst.1 else { return };
    let x = r.vertex(v);
    eprintln!(
        "  worst vertex {v:?} at {:?} tol {:e}, beyond tol off a plane {:e}",
        x.point, x.tol, worst.0
    );
    for e in r.edge_ids() {
        let ed = r.edge(e);
        if ed.start != v && ed.end != v {
            continue;
        }
        let t = if ed.start == v { ed.range.lo } else { ed.range.hi };
        let dc = ed.curve.eval(t).distance(x.point);
        let fs: Vec<String> = ed
            .coedges
            .iter()
            .map(|&c| {
                let f = r.coedge_face(c);
                let (dv, nz) = match &r.face(f).surface {
                    Surface::Plane(q) => (
                        q.signed_distance(x.point).abs(),
                        format!(
                            "n {:?} o {:?} name {:?}",
                            q.frame.z().get(),
                            q.frame.origin,
                            r.face(f).name
                        ),
                    ),
                    _ => (f64::NAN, String::new()),
                };
                let off = forge_topo::check::edge_off_face(r, e, f)
                    .map(|p| format!("{p:?}"))
                    .unwrap_or_default();
                format!("face {f:?} vertex-off {dv:.3e} {off} [{nz}]")
            })
            .collect();
        eprintln!(
            "    edge {e:?} tol {:.3e} len {:.3e} vertex-off-curve {dc:.3e}: {}",
            ed.tol,
            ed.range.width(),
            fs.join("; ")
        );
    }
}

/// PROBE (re-snap at merge, research/booleans/drift.md (d)): each vertex chosen (`all`: every
/// one off a plane it bounds by more than 1e-12; else the merged ones, tol past LINEAR) moved to
/// the exact meet of its best-conditioned triple of planes (all its faces planar), each line edge
/// at it between two planes rebuilt as their meet through it, and the tolerances reset to what is
/// left: the vertex's to LINEAR or the most a curve ending at it now misses it by; the far end's
/// grown to its miss of the rebuilt line. Returns how many it moved.
fn resnap(r: &mut Body, all: bool, cond: bool) -> usize {
    use forge_geom::{Curve, Line};
    let mut faces: Vec<Vec<FaceId>> = vec![Vec::new(); r.vertices.len()];
    let mut edges: Vec<Vec<forge_topo::EdgeId>> = vec![Vec::new(); r.vertices.len()];
    for e in r.edge_ids() {
        let ed = r.edge(e);
        for v in [ed.start, ed.end] {
            edges[v.index()].push(e);
            for &c in &ed.coedges {
                let f = r.coedge_face(c);
                if !faces[v.index()].contains(&f) {
                    faces[v.index()].push(f);
                }
            }
        }
    }
    let plane = |r: &Body, f: FaceId| match &r.face(f).surface {
        Surface::Plane(q) => Some(*q),
        _ => None,
    };
    let mut moved = 0;
    for v in r.vertex_ids().collect::<Vec<_>>() {
        let fs = &faces[v.index()];
        let Some(pl): Option<Vec<forge_geom::Plane>> = fs.iter().map(|&f| plane(r, f)).collect() else {
            continue;
        };
        let p = r.vertex(v).point;
        let off = pl.iter().map(|q| q.signed_distance(p).abs()).fold(0.0, f64::max);
        let chosen = if all { off > 1e-12 } else { r.vertex(v).tol > L };
        if !chosen || pl.len() < 3 {
            continue;
        }
        // conditioned: no two of its planes within 1e-3 rad of one another (unless the same)
        if cond
            && (0..pl.len()).any(|i| {
                (i + 1..pl.len()).any(|j| {
                    let s = pl[i].frame.z().get().cross(pl[j].frame.z().get()).length();
                    s > 0.0 && s < 1e-3
                })
            })
        {
            continue;
        }
        let mut best: Option<(f64, usize, usize, usize)> = None;
        for i in 0..pl.len() {
            for j in i + 1..pl.len() {
                for k in j + 1..pl.len() {
                    let (a, b, c) = (
                        pl[i].frame.z().get(),
                        pl[j].frame.z().get(),
                        pl[k].frame.z().get(),
                    );
                    let det = a.dot(b.cross(c)).abs();
                    if best.is_none_or(|x| det > x.0) {
                        best = Some((det, i, j, k));
                    }
                }
            }
        }
        let Some((det, i, j, k)) = best else { continue };
        if det <= 1e-3 {
            continue;
        }
        let dy = |q: &forge_geom::Plane| {
            let (o, z) = (q.frame.origin, q.frame.z().get());
            DyPlane::from_frame([o.x, o.y, o.z], [z.x, z.y, z.z])
        };
        let Some(m) = meet3(&dy(&pl[i]), &dy(&pl[j]), &dy(&pl[k])) else {
            continue;
        };
        let a = m.to_f64();
        let np = Point3::new(a[0], a[1], a[2]);
        r.vertices[v.index()].point = np;
        moved += 1;
        let mut vt = L;
        for &e in &edges[v.index()] {
            let ed = r.edge(e).clone();
            let ef: Vec<FaceId> = ed.coedges.iter().map(|&c| r.coedge_face(c)).collect();
            if let (Curve::Line(old), [f1, f2]) = (&ed.curve, ef.as_slice())
                && let (Some(q1), Some(q2)) = (plane(r, *f1), plane(r, *f2))
                && let Some(d) = q1.frame.z().get().cross(q2.frame.z().get()).normalize()
            {
                let d = if d.get().dot(old.dir.get()) < 0.0 {
                    -d.get()
                } else {
                    d.get()
                };
                let Some(dir) = d.normalize() else { continue };
                let line = Line::new(np, dir);
                let (ps, pe) = (r.vertex(ed.start).point, r.vertex(ed.end).point);
                let (lo, hi) = ((ps - np).dot(dir.get()), (pe - np).dot(dir.get()));
                if lo >= hi {
                    continue;
                }
                let x = &mut r.edges[e.index()];
                x.curve = Curve::Line(line);
                x.range = forge_math::Interval::new(lo, hi);
                x.bbox = Default::default();
                let w = if ed.start == v { ed.end } else { ed.start };
                let tw = if ed.start == v { hi } else { lo };
                let far = (line.origin + dir.get() * tw).distance(r.vertex(w).point);
                if far > r.vertex(w).tol {
                    r.grow_vertex_tol(w, far);
                }
            }
            let ed = r.edge(e);
            let t = if ed.start == v { ed.range.lo } else { ed.range.hi };
            vt = vt.max(ed.curve.eval(t).distance(np));
        }
        r.vertices[v.index()].tol = vt;
        for &f in fs {
            let face = &mut r.faces[f.index()];
            face.bbox = Default::default();
            face.query = Default::default();
        }
    }
    moved
}

struct Run {
    per_op: Vec<(f64, f64, f64, bool, Option<String>)>,
    worst: f64,
}

fn run(fam: &str, qs: &[Vec<f64>], keep: bool) -> (Run, Vec<(Body, Body, Op)>) {
    let mut body = prism(
        Frame::WORLD,
        &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
        10.0,
        1,
    )
    .unwrap();
    let (mut per_op, mut worst, mut kept) = (Vec::new(), 0.0f64, Vec::new());
    UNDER.with(|u| u.set(f64::NEG_INFINITY));
    OFF.with(|o| o.set(0));
    let skip: Vec<usize> = std::env::var("SKIP")
        .map(|v| v.split(',').filter_map(|x| x.parse().ok()).collect())
        .unwrap_or_default();
    let upto: usize = std::env::var("UPTO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);
    let mode = std::env::var("RESNAP").unwrap_or_default();
    for (i, q) in qs.iter().enumerate() {
        if skip.contains(&i) || i > upto {
            continue;
        }
        let Some((t, op)) = tool(fam, i, q) else { continue };
        let tb = std::time::Instant::now();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            boolean(&body, &t, op, 100 + i as u64)
        }));
        BOOL_SECS.with(|c| c.set(c.get() + tb.elapsed().as_secs_f64()));
        match r {
            Ok(Ok(mut next)) if next.face_ids().next().is_some() => {
                if !mode.is_empty() {
                    let t0 = std::time::Instant::now();
                    let n = resnap(&mut next, mode != "merge", mode == "cond");
                    RESNAP_STAT.with(|c| {
                        let (a, b, d) = c.get();
                        c.set((a + n, b + 1, d + t0.elapsed().as_secs_f64()))
                    });
                }
                if keep {
                    kept.push((body.clone(), t, op));
                }
                let (pm, mm, tm) = drift(&next);
                let probs = forge_topo::check(&next);
                let valid = probs.is_empty();
                let off = forge_topo::check::vertices_off_planes(&next);
                if !off.is_empty() {
                    OFF.with(|o| o.set(o.get() + 1));
                    if keep {
                        eprintln!("OFF op {i} {op:?}: {:?}", &off[..off.len().min(4)]);
                    }
                }
                if !valid && keep {
                    eprintln!("INVALID op {i} {op:?}: {:?}", &probs[..probs.len().min(6)]);
                }
                if keep && std::env::var("DETAIL").is_ok() {
                    eprintln!("op {i} {op:?} plane {pm:.3e}");
                    detail(&next);
                }
                worst = worst.max(mm).max(pm);
                per_op.push((pm, mm, tm, valid, None));
                body = next;
            }
            Ok(Ok(_)) => per_op.push((0.0, 0.0, 0.0, true, Some("empty".into()))),
            Ok(Err(e)) => per_op.push((0.0, 0.0, 0.0, true, Some(format!("{e:?}")))),
            Err(_) => per_op.push((0.0, 0.0, 0.0, false, Some("panic".into()))),
        }
    }
    if std::env::var("OBJ").is_ok_and(|o| o == "under") {
        worst = UNDER.with(|u| u.get());
    }
    (Run { per_op, worst }, kept)
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (fam, n, iters) = (
        a[0].as_str(),
        a[1].parse::<usize>().unwrap(),
        a[2].parse::<usize>().unwrap(),
    );
    let check = a.get(3).is_some_and(|s| s == "check");
    let mut rng = Rng::new(11);
    let mut qs: Vec<Vec<f64>> = (0..n)
        .map(|_| (0..6).map(|_| rng.range(-1.0, 1.0)).collect())
        .collect();
    if let Ok(path) = std::env::var("QS_IN") {
        let t = std::fs::read_to_string(path).unwrap();
        qs = t
            .trim()
            .trim_start_matches("[[")
            .trim_end_matches("]]")
            .split("], [")
            .map(|r| r.split(", ").map(|x| x.parse().unwrap()).collect())
            .collect();
    }
    let (mut best, _) = run(fam, &qs, false);
    for it in 0..iters {
        let mut cand = qs.clone();
        for _ in 0..3 {
            let (i, j) = (
                (rng.range(0.0, n as f64) as usize).min(n - 1),
                (rng.range(0.0, 6.0) as usize).min(5),
            );
            cand[i][j] = (cand[i][j] + rng.range(-0.5, 0.5)).clamp(-1.0, 1.0);
        }
        let (r, _) = run(fam, &cand, false);
        if r.worst >= best.worst {
            if r.worst > best.worst {
                eprintln!("iter {it}: worst {:e}", r.worst);
            }
            best = r;
            qs = cand;
        }
    }
    if let Ok(path) = std::env::var("QS_OUT") {
        std::fs::write(path, format!("{qs:?}")).unwrap();
    }
    let (r, kept) = run(fam, &qs, check || std::env::var("DETAIL").is_ok());
    println!("family {fam} ops {n} iters {iters}: worst drift {:e}", r.worst);
    let (mut refused, mut invalid) = (0, 0);
    for (i, (pm, mm, tm, valid, err)) in r.per_op.iter().enumerate() {
        if err.is_some() {
            refused += 1;
        }
        if !valid {
            invalid += 1;
        }
        println!(
            "op {i:3} plane {pm:.3e} meet {mm:.3e} tol {tm:.3e} valid {valid} {}",
            err.as_deref().unwrap_or("")
        );
    }
    println!(
        "refused {refused} invalid {invalid} worst-under {:e} off-composed {} resnap {:?} boolean-secs {:.3}",
        UNDER.with(|u| u.get()),
        OFF.with(|o| o.get()),
        RESNAP_STAT.with(|c| c.get()),
        BOOL_SECS.with(|c| c.get())
    );
    if check {
        let mut reference = Reference::start();
        let mut tally = std::collections::BTreeMap::new();
        for (i, (a, b, op)) in kept.iter().enumerate() {
            let Ok(input) = pair_input(a, b, *op) else {
                *tally.entry("not flat".to_string()).or_insert(0) += 1;
                continue;
            };
            let ans = reference.ask(&input);
            let c = classify(&ans, &kernel_pair(a, b, *op));
            if !matches!(c, Class::Agree | Class::Ambiguous) {
                println!("op {i} reference: {} {}", c.key(), c.detail());
            }
            *tally.entry(c.key().to_string()).or_insert(0) += 1;
        }
        println!("reference per op: {tally:?}");
    }
}

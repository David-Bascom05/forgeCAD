//! The kernel side of the fillet spec's differential test.
//!
//! `fillet-spec parts F.fg ..`: every edge each fillet of each part picks, rolled alone at its radius
//! on the body before that fillet (`forge_ops::fillet::edge_blend`, the kernel's own `roll`).
//! `fillet-spec random N SEED`: every edge of N random parts (prisms with bosses, holes and cones), at
//! random radii. One JSON line per edge on stdout, every f64 exact as `[m, e]` (m · 2^e), the format
//! of verify/forge-exact/ForgeExact/Ref/Input.lean. The Lean driver `fillet-check` judges each line.

use forge_geom::blend::{Blend, spine_closest};
use forge_geom::{Curve, CurveGeom, Surface};
use forge_math::{Frame, Point3, Vec3};
use forge_topo::{Body, EdgeId};
use std::fmt::Write;

fn d(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return "[0,0]".into();
    }
    let bits = x.to_bits();
    let neg = bits >> 63 == 1;
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let frac = bits & ((1u64 << 52) - 1);
    let (mut m, mut e) = if exp == 0 { (frac, -1074) } else { (frac | (1u64 << 52), exp - 1075) };
    while m & 1 == 0 {
        m >>= 1;
        e += 1;
    }
    format!("[{}{},{}]", if neg { "-" } else { "" }, m, e)
}

fn p(q: Point3) -> String {
    format!("[{},{},{}]", d(q.x), d(q.y), d(q.z))
}

fn v(q: Vec3) -> String {
    format!("[{},{},{}]", d(q.x), d(q.y), d(q.z))
}

fn frame(f: &Frame) -> String {
    format!("\"o\":{},\"z\":{},\"x\":{}", p(f.origin), v(f.z().get()), v(f.x().get()))
}

fn surf(s: &Surface) -> String {
    match s {
        Surface::Plane(q) => format!("{{\"t\":\"plane\",{}}}", frame(&q.frame)),
        Surface::Cylinder(c) => format!("{{\"t\":\"cylinder\",{},\"r\":{}}}", frame(&c.frame), d(c.radius)),
        Surface::Cone(c) => format!(
            "{{\"t\":\"cone\",{},\"r\":{},\"sin\":{},\"cos\":{}}}",
            frame(&c.frame),
            d(c.radius),
            d(c.sin_a()),
            d(c.cos_a())
        ),
        Surface::Sphere(c) => format!("{{\"t\":\"sphere\",{},\"r\":{}}}", frame(&c.frame), d(c.radius)),
        Surface::Torus(t) => format!(
            "{{\"t\":\"torus\",{},\"major\":{},\"minor\":{}}}",
            frame(&t.frame),
            d(t.major),
            d(t.minor)
        ),
        Surface::Procedural(_) => "{\"t\":\"pipe\"}".into(),
    }
}

fn spine(c: &Curve) -> String {
    match c {
        Curve::Line(l) => format!("{{\"t\":\"line\",\"o\":{},\"d\":{}}}", p(l.origin), v(l.dir.get())),
        Curve::Circle(k) => format!("{{\"t\":\"circle\",{},\"r\":{}}}", frame(&k.frame), d(k.radius)),
        _ => "{\"t\":\"other\"}".into(),
    }
}

/// One edge rolled alone at radius r, as a JSON line.
fn line(src: &str, body: &Body, e: EdgeId, r: f64) -> String {
    let mut out = format!("{{\"src\":\"{}\",\"edge\":{},\"r\":{}", src.replace('"', "'"), e.index(), d(r));
    match forge_ops::fillet::edge_blend(body, e, r) {
        Err(err) => {
            let _ = write!(out, ",\"refused\":\"{err:?}\"}}");
        }
        Ok((b, springs, convex)) => {
            let _ = write!(out, ",\"convex\":{convex},{}", blend(body, e, &b, &springs));
            out.push('}');
        }
    }
    out
}

/// The faces sharing an edge with either of edge `e`'s two faces (not those two), each with whether
/// its material lies on its surface's natural-normal side's back (`sense`, the face's same_sense).
fn neighbours(body: &Body, e: EdgeId) -> String {
    let x = body.edge(e);
    let sup: Vec<_> = x.coedges.iter().map(|&c| body.coedge_face(c)).collect();
    let mut out: Vec<forge_topo::FaceId> = Vec::new();
    for &f in &sup {
        for &lp in &body.face(f).loops {
            for c in body.lp(lp).coedges() {
                for &o in &body.edge(body.coedge(c).edge).coedges {
                    let g = body.coedge_face(o);
                    if !sup.contains(&g) && !out.contains(&g) {
                        out.push(g);
                    }
                }
            }
        }
    }
    out.iter()
        .map(|&g| {
            let s = surf(&body.face(g).surface);
            format!("{{\"s\":{s},\"sense\":{}}}", body.face(g).same_sense)
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn blend(body: &Body, e: EdgeId, b: &Blend, springs: &[Curve; 2]) -> String {
    let x = body.edge(e);
    let ts: Vec<f64> = [0.1, 0.5, 0.9]
        .iter()
        .map(|&s| spine_closest(&b.spine, x.curve.eval(x.range.lo + x.range.width() * s)))
        .collect();
    let pts = |c: &Curve| ts.iter().map(|&t| p(c.eval(t))).collect::<Vec<_>>().join(",");
    format!(
        "\"a\":{},\"b\":{},\"da\":{},\"db\":{},\"radius\":{},\"surf\":{},\"spine\":{},\"centres\":[{}],\"springs\":[[{}],[{}]],\"nbrs\":[{}]",
        surf(&b.a),
        surf(&b.b),
        d(b.da),
        d(b.db),
        d(b.radius),
        surf(&b.surface),
        spine(&b.spine),
        pts(&b.spine),
        pts(&springs[0]),
        pts(&springs[1]),
        neighbours(body, e)
    )
}

/// The fuzz harness's options without its font directory (which needs a temp dir, which WASI has
/// not): these parts use no text, and the same source builds the same on native and wasm.
fn opts() -> forge_lang::Options {
    forge_lang::Options {
        dir: None,
        checks: false,
        overlap: false,
        expects: false,
        meshes: false,
        ..forge_lang::Options::default()
    }
}

/// The spec's predicates along one picked edge, sampled (a diagnosis, not a proof): P7, the faces'
/// normal curvature across the edge against r (focal where the ball side's radius is below r), and
/// P2, whether the ball at the edge's crease point fits (every point of a sphere just inside it
/// classified on the ball's side of the body).
fn probe_edge(body: &Body, idx: &forge_bool::classify::Index, e: EdgeId, r: f64) -> String {
    use forge_bool::classify::Where;
    use forge_geom::SurfaceGeom;
    let x = body.edge(e);
    let convex = forge_ops::fillet::fillet_kind(body, e);
    let (c0, c1) = (x.coedges[0], x.coedges[1]);
    let faces = [body.coedge_face(c0), body.coedge_face(c1)];
    let mut focal_min = f64::INFINITY;
    let (mut blocked, mut samples) = (0, 0);
    let mut worst_out = 0usize;
    let rolled = forge_ops::fillet::edge_blend(body, e, r);
    for i in 0..9 {
        let t = x.range.lo + x.range.width() * (i as f64 + 0.5) / 9.0;
        let d = x.curve.deriv(t);
        let Some(tan) = d.d1.normalize() else { continue };
        let mut ns = Vec::new();
        for &f in &faces {
            let s = &body.face(f).surface;
            let uv = s.project(d.p);
            let Some(n) = body.face_normal(f, uv) else { continue };
            let n = n.get();
            ns.push(n);
            // normal curvature across the edge, in the face, against its outward normal
            let sd = s.deriv(uv);
            let w = n.cross(tan.get());
            let (e_, f_, g_) = (sd.du.dot(sd.du), sd.du.dot(sd.dv), sd.dv.dot(sd.dv));
            let det = e_ * g_ - f_ * f_;
            if det.abs() < 1e-18 {
                continue;
            }
            let (bu, bv) = (sd.du.dot(w), sd.dv.dot(w));
            let a = (g_ * bu - f_ * bv) / det;
            let b = (e_ * bv - f_ * bu) / det;
            let ii = (sd.duu * (a * a) + sd.duv * (2.0 * a * b) + sd.dvv * (b * b)).dot(n);
            let i1 = (sd.du * a + sd.dv * b).length_sq();
            let k = ii / i1;
            // the ball's side: inside for a convex edge (-n), outside for a concave one (+n)
            let toward = if convex == Some(true) { -k } else { k };
            if toward > 1e-12 {
                focal_min = focal_min.min(1.0 / toward);
            }
        }
        if ns.len() != 2 {
            continue;
        }
        let cdot = ns[0].dot(ns[1]);
        let side = if convex == Some(true) { -1.0 } else { 1.0 };
        let c = match &rolled {
            // the kernel's crease point, which the differential test holds to the spec's
            Ok((b, _, _)) => b.spine.eval(spine_closest(&b.spine, d.p)),
            Err(_) => d.p + (ns[0] + ns[1]) * (side * r / (1.0 + cdot)),
        };
        samples += 1;
        let mut out = 0;
        for j in 0..240 {
            // a spiral of points on the sphere of radius 0.99 r
            let z = 1.0 - 2.0 * (j as f64 + 0.5) / 240.0;
            let a = j as f64 * 2.399_963_229_728_653;
            let q = (1.0 - z * z).max(0.0).sqrt();
            let p = c + Vec3::new(q * a.cos(), q * a.sin(), z) * (0.99 * r);
            let wrong = match idx.classify(p) {
                Where::Inside => convex != Some(true),
                Where::Outside => convex == Some(true),
                Where::On => true,
            };
            if wrong {
                out += 1;
            }
        }
        if out > 0 {
            blocked += 1;
        }
        worst_out = worst_out.max(out);
    }
    format!(
        "{{\"edge\":{},\"r\":{r},\"convex\":{:?},\"focal_min\":{},\"ball_blocked\":{blocked},\"of\":{samples},\"worst_out_of_240\":{worst_out},\"rolled\":\"{}\"}}",
        e.index(),
        convex,
        if focal_min.is_finite() { format!("{focal_min:.4}") } else { "null".into() },
        match &rolled {
            Ok(_) => "ok".to_string(),
            Err(err) => format!("{err:?}"),
        }
    )
}

fn probe(files: &[String]) {
    for f in files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let m = forge_lang::fuzz::build_with(&src, opts());
        for k in 0..m.features.len() {
            let forge_model::Feature::Fillet { edges, .. } = &m.features[k] else { continue };
            let Some(body) = m.body_after(k) else { continue };
            let body = if body.dead > 0 { body.compact() } else { body };
            let idx = forge_bool::classify::Index::new(&body);
            for (name, radius) in edges {
                let forge_ops::Radius::Constant(r) = radius else { continue };
                let Some(e) = body.edge_by_name(*name) else { continue };
                println!("{}#{k} {}", f, probe_edge(&body, &idx, e, *r));
            }
        }
    }
}

/// Each fillet's picked edges' midpoints and radius: `mids F.fg` (for an OCCT refillet of the same
/// edges of the body before).
fn mids(files: &[String]) {
    for f in files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let m = forge_lang::fuzz::build_with(&src, opts());
        for k in 0..m.features.len() {
            let forge_model::Feature::Fillet { edges, .. } = &m.features[k] else { continue };
            let Some(body) = m.body_after(k) else { continue };
            for (name, radius) in edges {
                let forge_ops::Radius::Constant(r) = radius else { continue };
                let Some(e) = body.edge_by_name(*name) else { continue };
                let x = body.edge(e);
                let q = x.curve.eval(x.range.mid());
                println!("{k} {r} {} {} {}", q.x, q.y, q.z);
            }
        }
    }
}

/// `near F.fg PTS`: for each point (x y z per line) and each picked edge's rolled blend, the distance
/// from the nearest spine point, against r (below r: the point is in a free ball, so air by the
/// closing), and whether the blend surface's own projection of the point lies at distance r from it.
fn near(f: &str, pts: &str) {
    let Ok(src) = std::fs::read_to_string(f) else { return };
    let m = forge_lang::fuzz::build_with(&src, opts());
    let mut blends = Vec::new();
    for k in 0..m.features.len() {
        let forge_model::Feature::Fillet { edges, .. } = &m.features[k] else { continue };
        let Some(body) = m.body_after(k) else { continue };
        let body = if body.dead > 0 { body.compact() } else { body };
        for (name, radius) in edges {
            let forge_ops::Radius::Constant(r) = radius else { continue };
            let Some(e) = body.edge_by_name(*name) else { continue };
            if let Ok((b, _, _)) = forge_ops::fillet::edge_blend(&body, e, *r) {
                blends.push((b, *r));
            }
        }
    }
    use forge_geom::SurfaceGeom;
    for l in std::fs::read_to_string(pts).unwrap_or_default().lines() {
        let v: Vec<f64> = l.split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect();
        if v.len() != 3 {
            continue;
        }
        let p = Point3::new(v[0], v[1], v[2]);
        for (b, r) in &blends {
            let t = spine_closest(&b.spine, p);
            let c = b.spine.eval(t);
            let q = b.surface.eval(b.surface.project(p));
            // inside the cap: the direction to p lies on the short arc between the feet
            let (fa, fb) = b.feet(t);
            let ang = |u: Vec3, w: Vec3| {
                let (Some(u), Some(w)) = (u.normalize(), w.normalize()) else { return f64::NAN };
                u.dot(w).clamp(-1.0, 1.0).acos()
            };
            let (a1, a2, a12) = (ang(p - c, fa - c), ang(p - c, fb - c), ang(fa - c, fb - c));
            println!(
                "{:.4} {:.4} {:.4}  |p-c| - r = {:+.3e}  surf(q): |q-c(q)| - r = {:+.3e}  cap excess {:+.3e} rad (to a {:.4}, to b {:.4}, arc {:.4})  centre off a {:+.3e} off b {:+.3e}",
                p.x, p.y, p.z, p.distance(c) - r, q.distance(b.spine.eval(spine_closest(&b.spine, q))) - r,
                a1 + a2 - a12, a1, a2, a12, b.a.distance(c) - r, b.b.distance(c) - r
            );
        }
    }
}

/// `onboth F.fg ..`: each part built whole, and every edge of each body before a fillet rolled at its
/// radius; how many closed-form spines the sampled check `on_both` would have refused (it is gone
/// from the kernel: lean-spec-design.md §14). One line per part with any, and the total.
fn onboth(files: &[String]) {
    use std::sync::atomic::Ordering;
    let mut total = 0;
    for f in files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let before = forge_geom::blend::ON_BOTH_FAILS.load(Ordering::Relaxed);
        let m = forge_lang::fuzz::build_with(&src, opts());
        let in_build = forge_geom::blend::ON_BOTH_FAILS.load(Ordering::Relaxed) - before;
        if in_build > 0 {
            println!("{f}: {in_build} in the part's own build");
        }
        for k in 0..m.features.len() {
            let forge_model::Feature::Fillet { edges, .. } = &m.features[k] else { continue };
            let Some(body) = m.body_after(k) else { continue };
            let body = if body.dead > 0 { body.compact() } else { body };
            let r = edges.iter().find_map(|(_, x)| match x {
                forge_ops::Radius::Constant(r) => Some(*r),
                _ => None,
            });
            if let Some(r) = r {
                for e in body.edge_ids().collect::<Vec<_>>() {
                    let b0 = forge_geom::blend::ON_BOTH_FAILS.load(Ordering::Relaxed);
                    let got = forge_ops::fillet::edge_blend(&body, e, r);
                    if forge_geom::blend::ON_BOTH_FAILS.load(Ordering::Relaxed) > b0 {
                        let x = body.edge(e);
                        let faces: Vec<String> = x
                            .coedges
                            .iter()
                            .map(|&c| surf(&body.face(body.coedge_face(c)).surface))
                            .collect();
                        println!(
                            "  edge {} at {:?} r {r}: {} | blend {}",
                            e.index(),
                            x.curve.eval(x.range.mid()),
                            faces.join(" / "),
                            match &got {
                                Ok((b, _, _)) => format!("{} {}", surf(&b.surface), spine(&b.spine)),
                                Err(err) => format!("{err:?}"),
                            }
                        );
                    }
                }
            }
        }
        let n = forge_geom::blend::ON_BOTH_FAILS.load(Ordering::Relaxed) - before;
        if n > 0 {
            println!("{f}: {n}");
        }
        total += n;
    }
    println!(
        "total {total} refused of {} closed-form spines, {} parts",
        forge_geom::blend::CLOSED_FORMS.load(Ordering::Relaxed),
        files.len()
    );
}

/// `score F.fg`: the points where the obstacle rules disagree, for the last fillet of a part (whose
/// truth is the part's STEP). One JSON line per point: where it is, the body before's side (Index,
/// which has no new faces to misjudge), the nearest picked edge's kind, and the rule-deciding fact:
/// for a concave pick, whether a free ball of the body before covers the point (then rules (b) and
/// (d) say air); for a convex pick, whether the ball at the edge's crease pokes out of the body
/// before (then rule (b) leaves the edge sharp). The OCCT script classifies them against the kernel's
/// result and the truth.
fn score(f: &str) {
    use forge_bool::classify::{Index, Where};
    let Ok(src) = std::fs::read_to_string(f) else { return };
    let m = forge_lang::fuzz::build_with(&src, opts());
    let Some(k) = (0..m.features.len()).rev().find(|&k| matches!(m.features[k], forge_model::Feature::Fillet { .. })) else {
        return;
    };
    let forge_model::Feature::Fillet { edges, .. } = &m.features[k] else { return };
    let Some(before) = m.body_after(k) else { return };
    let before = if before.dead > 0 { before.compact() } else { before };
    let idx = Index::new(&before);
    let outside = |p: Point3| matches!(idx.classify(p), Where::Outside);
    let inside = |p: Point3| matches!(idx.classify(p), Where::Inside);
    // picked edges: (edge, r, convex, pokes)
    let mut picks = Vec::new();
    for (name, radius) in edges {
        let forge_ops::Radius::Constant(r) = radius else { continue };
        let Some(e) = before.edge_by_name(*name) else { continue };
        let convex = forge_ops::fillet::fillet_kind(&before, e) == Some(true);
        let mut pokes = false;
        if convex && let Ok((b, _, _)) = forge_ops::fillet::edge_blend(&before, e, *r) {
            let x = before.edge(e);
            let c = b.spine.eval(spine_closest(&b.spine, x.curve.eval(x.range.mid())));
            let strict = std::env::var_os("SCORE_STRICT").is_some();
            let (n, k, need) = if strict { (400, 0.95, 20) } else { (120, 0.99, 1) };
            pokes = (0..n)
                .filter(|&j| {
                    let z = 1.0 - 2.0 * (j as f64 + 0.5) / n as f64;
                    let a = j as f64 * 2.399_963_229_728_653;
                    let q = (1.0 - z * z).max(0.0).sqrt();
                    outside(c + Vec3::new(q * a.cos(), q * a.sin(), z) * (k * r))
                })
                .count()
                >= need;
        }
        picks.push((e, *r, convex, pokes));
    }
    let mut g = Rng(0x5eed ^ f.len() as u64);
    let unit = |g: &mut Rng| loop {
        let v = Vec3::new(
            g.next() as f64 / u64::MAX as f64 * 2.0 - 1.0,
            g.next() as f64 / u64::MAX as f64 * 2.0 - 1.0,
            g.next() as f64 / u64::MAX as f64 * 2.0 - 1.0,
        );
        let l = v.length();
        if l > 0.05 && l <= 1.0 {
            return v * (1.0 / l);
        }
    };
    // a ball of radius r about c misses the body before (sampled: its centre and 60 sphere points)
    // SCORE_STRICT: 400 points on the sphere and on shells at 0.5 r and 0.8 r
    let n: usize = if std::env::var_os("SCORE_STRICT").is_some() { 400 } else { 60 };
    let shells: &[f64] = if n > 60 { &[1.0, 0.8, 0.5] } else { &[1.0] };
    let free = |c: Point3, r: f64| {
        outside(c)
            && shells.iter().all(|&k| {
                (0..n).all(|j| {
                    let z = 1.0 - 2.0 * (j as f64 + 0.5) / n as f64;
                    let a = j as f64 * 2.399_963_229_728_653;
                    let q = (1.0 - z * z).max(0.0).sqrt();
                    outside(c + Vec3::new(q * a.cos(), q * a.sin(), z) * (r * k))
                })
            })
    };
    for &(e, r, convex, pokes) in &picks {
        let x = before.edge(e);
        for _ in 0..60 {
            let t = x.range.lo + x.range.width() * (g.next() as f64 / u64::MAX as f64);
            let p = x.curve.eval(t) + unit(&mut g) * (r * (g.next() as f64 / u64::MAX as f64));
            let (bin, bout) = (inside(p), outside(p));
            let covered = if !convex && bout {
                (0..300).any(|_| {
                    let c = p + unit(&mut g) * (r * (0.3 + 0.69 * (g.next() as f64 / u64::MAX as f64)));
                    free(c, r)
                })
            } else {
                false
            };
            println!(
                "{{\"x\":{},\"y\":{},\"z\":{},\"before\":\"{}\",\"convex\":{convex},\"pokes\":{pokes},\"covered\":{covered},\"r\":{r}}}",
                p.x, p.y, p.z,
                if bin { "in" } else if bout { "out" } else { "on" }
            );
        }
    }
}

/// `supports F.fg K`: fillet feature K's picked edges on the body before it: the edge's middle and
/// direction, its radius and kind, and its two faces' surfaces (for comparing a truth's blend
/// cylinder against the carried supports). One JSON line per edge.
fn supports(f: &str, k: usize) {
    let Ok(src) = std::fs::read_to_string(f) else { return };
    let m = forge_lang::fuzz::build_with(&src, opts());
    let Some(forge_model::Feature::Fillet { edges, .. }) = m.features.get(k) else {
        eprintln!("feature {k} is no fillet ({} features)", m.features.len());
        return;
    };
    let Some(body) = m.body_after(k) else { return };
    let body = if body.dead > 0 { body.compact() } else { body };
    for (name, radius) in edges {
        let forge_ops::Radius::Constant(r) = radius else { continue };
        let Some(e) = body.edge_by_name(*name) else { continue };
        let x = body.edge(e);
        let d = x.curve.deriv(x.range.mid());
        let faces: Vec<String> = x
            .coedges
            .iter()
            .map(|&c| {
                let fid = body.coedge_face(c);
                format!("{{\"s\":{},\"sense\":{}}}", surf(&body.face(fid).surface), body.face(fid).same_sense)
            })
            .collect();
        // the ball tangent to both carried planes at the edge's middle: are its feet on the body?
        let feet_on = {
            use forge_bool::classify::{Index, Where};
            let planes: Vec<(Point3, Vec3)> = x
                .coedges
                .iter()
                .filter_map(|&c| {
                    let fid = body.coedge_face(c);
                    let face = body.face(fid);
                    match &face.surface {
                        Surface::Plane(q) => {
                            let n = q.frame.z().get() * if face.same_sense { 1.0 } else { -1.0 };
                            Some((q.frame.origin, n))
                        }
                        _ => None,
                    }
                })
                .collect();
            if planes.len() == 2 {
                let convex = forge_ops::fillet::fillet_kind(&body, e) == Some(true);
                let side = if convex { -1.0 } else { 1.0 };
                let (n0, n1) = (planes[0].1, planes[1].1);
                let c = d.p + (n0 + n1) * (side * r / (1.0 + n0.dot(n1)));
                let idx = Index::new(&body);
                let on = |q: Point3| matches!(idx.classify(q), Where::On);
                let f0 = c - n0 * (side * r);
                let f1 = c - n1 * (side * r);
                format!("[{},{}]", on(f0), on(f1))
            } else {
                "null".to_string()
            }
        };
        println!(
            "{{\"mid\":{},\"dir\":{},\"r\":{r},\"feet_on_body\":{feet_on},\"convex\":{},\"faces\":[{}]}}",
            p(d.p),
            v(d.d1),
            match forge_ops::fillet::fillet_kind(&body, e) {
                Some(true) => "true",
                Some(false) => "false",
                None => "null",
            },
            faces.join(",")
        );
    }
}

fn parts(files: &[String]) {
    for f in files {
        let Ok(src) = std::fs::read_to_string(f) else {
            eprintln!("{f}: unreadable");
            continue;
        };
        let m = forge_lang::fuzz::build_with(&src, opts());
        for k in 0..m.features.len().min(m.built()) {
            let forge_model::Feature::Fillet { edges, .. } = &m.features[k] else {
                continue;
            };
            let Some(body) = m.body_after(k) else { continue };
            let body = if body.dead > 0 { body.compact() } else { body };
            for (name, radius) in edges {
                let forge_ops::Radius::Constant(r) = radius else { continue };
                let Some(e) = body.edge_by_name(*name) else { continue };
                println!("{}", line(&format!("{f}#{k}"), &body, e, *r));
            }
        }
    }
}

/// A small deterministic generator (SplitMix64).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// A value in [lo, hi), on a grid of 1/64 so the parts read back exactly.
    fn grid(&mut self, lo: f64, hi: f64) -> f64 {
        let steps = ((hi - lo) * 64.0) as u64;
        lo + (self.next() % steps.max(1)) as f64 / 64.0
    }
}

/// A random part: a convex prism, with a boss, a blind hole and a cone revolved on its top.
fn random_part(g: &mut Rng) -> String {
    let n = 3 + (g.next() % 4) as usize;
    let rad = g.grid(30.0, 50.0);
    // a convex polygon: angles in order round the circle, radii varying a little
    let mut angles: Vec<f64> = (0..n).map(|i| (i as f64 + g.grid(0.0, 0.6)) * 360.0 / n as f64).collect();
    angles.sort_by(f64::total_cmp);
    let mut s = String::new();
    let _ = writeln!(s, "shared t = {}", g.grid(8.0, 20.0));
    let _ = writeln!(s, "param boss.h = {}", g.grid(4.0, 12.0));
    let _ = writeln!(s, "param hd.d = {}", 2.0 * g.grid(2.0, 6.0));
    let _ = writeln!(s, "param hd.depth = {}", g.grid(2.0, 6.0));
    let _ = writeln!(s, "shared bc = {}", g.grid(4.0, 8.0));
    for (i, a) in angles.iter().enumerate() {
        let _ = writeln!(s, "shared pa{i} = {a}deg");
    }
    let _ = writeln!(s, "shared pr = {rad}");
    let _ = writeln!(s, "sketch plan(on: xy) {{");
    let pts: Vec<String> = (0..n).map(|i| format!("(pr * cos(pa{i}), pr * sin(pa{i}))")).collect();
    let _ = writeln!(s, "  outline = poly([{}], closed: true)", pts.join(", "));
    let _ = writeln!(s, "}}");
    let _ = writeln!(s, "base = extrude(plan, t)");
    let _ = writeln!(s, "sketch top(on: base.end) {{");
    let _ = writeln!(s, "  c = circle((pr / 4, 0), bc)");
    let _ = writeln!(s, "  h = pt(-pr / 4, 0)");
    let _ = writeln!(s, "}}");
    let _ = writeln!(s, "boss = extrude(top.c, h)");
    let _ = writeln!(s, "hd = hole(at: top.h, d: d, depth: depth)");
    s
}

fn random(n: usize, seed: u64) {
    let mut g = Rng(seed);
    for i in 0..n {
        let src = random_part(&mut g);
        let m = forge_lang::fuzz::build_with(&src, opts());
        let Some(body) = m.body() else {
            eprintln!("random {seed}/{i}: did not build: {:?}", m.error());
            continue;
        };
        let body = if body.dead > 0 { body.compact() } else { body };
        let edges: Vec<EdgeId> = body.edge_ids().collect();
        for e in edges {
            let r = g.grid(0.25, 6.0);
            println!("{}", line(&format!("random {seed}/{i}"), &body, e, r));
        }
    }
    eprintln!(
        "on_both would have refused: {}",
        forge_geom::blend::ON_BOTH_FAILS.load(std::sync::atomic::Ordering::Relaxed)
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("parts") => parts(&args[1..]),
        Some("probe") => probe(&args[1..]),
        Some("mids") => mids(&args[1..]),
        Some("onboth") => onboth(&args[1..]),
        Some("score") => score(&args[1]),
        Some("supports") => supports(&args[1], args.get(2).and_then(|x| x.parse().ok()).unwrap_or(0)),
        Some("near") => near(&args[1], &args[2]),
        Some("random") => {
            let n = args.get(1).and_then(|x| x.parse().ok()).unwrap_or(20);
            let seed = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(1);
            random(n, seed);
        }
        _ => eprintln!("usage: fillet-spec parts F.fg .. | fillet-spec random N SEED"),
    }
}

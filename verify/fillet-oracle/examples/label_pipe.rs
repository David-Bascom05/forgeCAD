//! The pipe set's fillets labelled against the oracle: label_pipe FC_DIR FG_DIR OUT.jsonl [IDS..]
//!
//! Each part (research/pipe-speed/fuzz/gen.py) is a body (box or round bar) with an oblique round
//! boss, and the concave edges between them filleted. Its .fc twin gives the two solids exactly, as
//! the oracle's obstacle. Every concave edge of body ∪ boss is picked, so the spec is the whole
//! closing, except near vertices where a picked edge meets an unpicked one: the vertex table's
//! second row (`Zone`), from the kernel's body before the fillet.
//!
//! The kernel's result (the .fg built up to the fillet) is judged three ways:
//! - points just either side of each new face, at δ: inside should be filled (or body), outside
//!   air;
//! - random points in the box round the picked edges (grown by 2r), each held against the kernel's
//!   own classification;
//! - the fill's volume by those points, against the kernel's added volume.
//!
//! A disagreement counts only when the six points at δ round it get the same answers from both,
//! so that tolerance-sized differences at the boundary are not counted. A part is `wrong` when any
//! disagreement remains, `right` when none does, and `refused` or `invalid` as the kernel left it.
use forge_bool::{Index, Where as KWhere};
use forge_fillet_oracle::{Field, Search, Spec, Where, Zone};
use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_math::{Point2, Point3, Vec3, rng::Rng};
use forge_model::Feature;
use forge_topo::{Body, EdgeId, FaceId, VertexId};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

fn unit(v: Vec3) -> Vec3 {
    v * (1.0 / v.length())
}

/// One extrusion line of a .fc pipe part: a box or a capped bar.
fn solid(line: &str) -> Field {
    let t: Vec<&str> = line.split_whitespace().collect();
    let num = |k: usize| t[k].parse::<f64>().expect("a number");
    let at = |w: &str| t.iter().position(|&x| x == w).expect(w);
    let i = at("extrude");
    let j = if t[i + 1] == "add" { i + 2 } else { i + 1 };
    let h = num(j);
    let (zi, xi, ai) = (at("z"), at("x"), at("at"));
    let z = unit(Vec3::new(num(zi + 1), num(zi + 2), num(zi + 3)));
    let x = unit(Vec3::new(num(xi + 1), num(xi + 2), num(xi + 3)));
    let o = Point3::new(num(ai + 1), num(ai + 2), num(ai + 3));
    let y = z.cross(x);
    if let Some(ci) = t.iter().position(|&w| w == "circle") {
        let c = o + x * num(ci + 1) + y * num(ci + 2);
        Field::Rod {
            o: c,
            d: z,
            h,
            radius: num(ci + 3),
        }
    } else {
        let ri = at("rect");
        let c = o + x * num(ri + 1) + y * num(ri + 2);
        Field::Block {
            o: c,
            x,
            y,
            z,
            w: num(ri + 3),
            l: num(ri + 4),
            h,
        }
    }
}

/// A face's surface carried on, as the solid on its material side (planes and cylinders).
fn half_solid(b: &Body, f: FaceId) -> Option<Field> {
    let face = b.face(f);
    let s = if face.same_sense { 1.0 } else { -1.0 };
    match &face.surface {
        Surface::Plane(p) => Some(Field::Half {
            o: p.frame.origin,
            n: p.frame.z().get() * s,
        }),
        Surface::Cylinder(c) => Some(Field::Bar {
            o: c.frame.origin,
            d: c.frame.z().get(),
            radius: c.radius,
            solid: face.same_sense,
        }),
        _ => None,
    }
}

fn faces_of(b: &Body, e: EdgeId) -> Vec<FaceId> {
    b.edge(e).coedges.iter().map(|&c| b.coedge_face(c)).collect()
}

/// A seam: an edge with one face, or one surface, on both sides.
fn seam(b: &Body, e: EdgeId) -> bool {
    let fs = faces_of(b, e);
    fs.len() == 2 && (fs[0] == fs[1] || b.face(fs[0]).surface == b.face(fs[1]).surface)
}

fn edges_at(b: &Body, v: VertexId) -> Vec<EdgeId> {
    b.edge_ids()
        .filter(|&e| b.edge(e).start == v || b.edge(e).end == v)
        .collect()
}

/// Distance from p to edge e's curve within its range.
fn to_edge(b: &Body, e: EdgeId, p: Point3) -> f64 {
    let x = b.edge(e);
    x.curve.eval(x.curve.closest(p, x.range)).distance(p)
}

/// The zones of the vertex table's second row: each vertex where picked (concave) edges meet an
/// unpicked one that is no seam. ρ is just under the distance to the nearest generator not at it.
fn zones(s: &Body, picked: &BTreeSet<EdgeId>) -> Option<Vec<Zone>> {
    let ix = Index::new(s);
    let mut out = Vec::new();
    for v in s.vertex_ids() {
        let at = edges_at(s, v);
        let picks: Vec<EdgeId> = at.iter().copied().filter(|e| picked.contains(e)).collect();
        let others: Vec<EdgeId> = at
            .iter()
            .copied()
            .filter(|e| !picked.contains(e) && !seam(s, *e))
            .collect();
        if picks.is_empty() || others.is_empty() {
            continue;
        }
        let p = s.vertex(v).point;
        let mut wedges = Vec::new();
        for &e in &picks {
            let fs = faces_of(s, e);
            wedges.push(Field::Union(vec![half_solid(s, fs[0])?, half_solid(s, fs[1])?]));
        }
        // faces across an unpicked convex edge from a pick's support that support no pick
        let support: BTreeSet<FaceId> = picks.iter().flat_map(|&e| faces_of(s, e)).collect();
        let mut cap = Vec::new();
        for &e in &others {
            if forge_ops::fillet::fillet_kind(s, e) != Some(true) {
                continue;
            }
            let fs = faces_of(s, e);
            for (a, b) in [(fs[0], fs[1]), (fs[1], fs[0])] {
                if support.contains(&a) && !support.contains(&b) {
                    cap.push(half_solid(s, b)?);
                }
            }
        }
        // the nearest generator not at v: faces (by surface), edges and vertices
        let here: BTreeSet<FaceId> = at.iter().flat_map(|&e| faces_of(s, e)).collect();
        let same_surface = |f: FaceId| here.iter().any(|&g| s.face(g).surface == s.face(f).surface);
        let mut rho = f64::INFINITY;
        for f in s.face_ids() {
            if here.contains(&f) || same_surface(f) {
                continue;
            }
            let surf = &s.face(f).surface;
            let foot = surf.eval(surf.project(p));
            if ix.point_in_face(f, foot) != KWhere::Outside {
                rho = rho.min(foot.distance(p));
            }
        }
        for e in s.edge_ids() {
            if at.contains(&e) || seam(s, e) {
                continue;
            }
            rho = rho.min(to_edge(s, e, p));
        }
        for w in s.vertex_ids() {
            if w != v && !edges_at(s, w).iter().all(|&e| seam(s, e) || picked.contains(&e)) {
                rho = rho.min(s.vertex(w).point.distance(p));
            }
        }
        out.push(Zone {
            at: p,
            rho: 0.999 * rho,
            wedges,
            cap,
        });
    }
    Some(out)
}

/// Points of a face, by drawing parameters inside its loops, with the face's outward normal.
fn face_points(b: &Body, f: FaceId, want: usize, rng: &mut Rng) -> Vec<(Point3, Vec3)> {
    let loops = forge_topo::uv::face_uv_loops(b, f);
    let mut lo = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for l in &loops {
        for p in &l.points {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    let s = &b.face(f).surface;
    let mut out = Vec::with_capacity(want);
    for _ in 0..60 * want {
        if out.len() == want {
            break;
        }
        let uv = Point2::new(rng.range(lo.x, hi.x), rng.range(lo.y, hi.y));
        if forge_topo::uv::point_in_loops(uv, &loops)
            && let Some(n) = b.face_normal(f, uv)
        {
            out.push((s.eval(uv), n.get()));
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '"' && *c != '\\' && (*c as u32) >= 32)
        .collect()
}

fn filled(w: Where) -> Option<bool> {
    match w {
        Where::Body | Where::Fill => Some(true),
        Where::Air => Some(false),
        Where::Unknown => None,
    }
}

fn kernel_in(ix: &Index, p: Point3) -> Option<bool> {
    match ix.classify(p) {
        KWhere::Inside => Some(true),
        KWhere::Outside => Some(false),
        KWhere::On => None,
    }
}

fn one(fc: &str, fg: &str, id: &str, samples: usize) -> String {
    let fc_src = std::fs::read_to_string(format!("{fc}/{id}.fc")).expect("the .fc part");
    let lines: Vec<&str> = fc_src
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    let body = Field::Union(vec![solid(lines[0]), solid(lines[1])]);
    let r: f64 = lines[2].split_whitespace().nth(1).expect("r").parse().expect("r");
    let src = std::fs::read_to_string(format!("{fg}/{id}.fg")).expect("the .fg part");
    // the part up to its fillet
    let keep = src
        .lines()
        .position(|l| l.starts_with("round = "))
        .expect("a round");
    let src: String = src.lines().take(keep + 1).map(|l| format!("{l}\n")).collect();
    let m = forge_lang::fuzz::build(&src);
    let Some(k) = m
        .features
        .iter()
        .position(|f| matches!(f, Feature::Fillet { .. }))
    else {
        return format!("{{\"id\":\"{id}\",\"label\":\"refused\",\"why\":\"no fillet built\"}}");
    };
    let Some(s) = m.body_after(k) else {
        return format!("{{\"id\":\"{id}\",\"label\":\"refused\",\"why\":\"no body\"}}");
    };
    if let Some(e) = m.error() {
        return format!(
            "{{\"id\":\"{id}\",\"label\":\"refused\",\"why\":\"{}\"}}",
            esc(&forge_lang::fuzz::message(e))
        );
    }
    let Some(after) = m.body_after(k + 1) else {
        return format!("{{\"id\":\"{id}\",\"label\":\"refused\",\"why\":\"no result\"}}");
    };
    let problems = forge_topo::check(&after).len();
    let picked: BTreeSet<EdgeId> = s
        .edge_ids()
        .filter(|&e| !seam(&s, e) && forge_ops::fillet::fillet_kind(&s, e) == Some(false))
        .collect();
    let Some(zones) = zones(&s, &picked) else {
        return format!(
            "{{\"id\":\"{id}\",\"label\":\"unjudged\",\"why\":\"a face the zones cannot carry on\"}}"
        );
    };
    let spec = Spec {
        body,
        r,
        zones,
        search: Search::for_radius(r),
    };
    let added = match (forge_topo::mass::mass(&after), forge_topo::mass::mass(&s)) {
        (Ok(a), Ok(b)) => a.volume - b.volume,
        _ => f64::NAN,
    };
    let ix = Index::new(&after);
    let delta = (1e-3 * r).max(1e-4);
    // the obstacle must be the kernel's body before the fillet: held point by point
    let mut region = forge_math::Box3::EMPTY;
    for &e in &picked {
        region = region.hull(&s.edge_box(e));
    }
    let region = region.grow(2.0 * r);
    {
        let sx = Index::new(&s);
        let mut rng = Rng::new(17);
        let mut off = 0usize;
        for _ in 0..2000 {
            let x = Point3::new(
                rng.range(region.x.lo, region.x.hi),
                rng.range(region.y.lo, region.y.hi),
                rng.range(region.z.lo, region.z.hi),
            );
            let d = spec.body.sd(x);
            if d.abs() < delta {
                continue;
            }
            if kernel_in(&sx, x).is_some_and(|k| k != (d < 0.0)) {
                off += 1;
            }
        }
        if off > 0 {
            return format!(
                "{{\"id\":\"{id}\",\"label\":\"unjudged\",\"why\":\"the .fc solids differ from the kernel's body at {off} of 2000 points\"}}"
            );
        }
    }
    let mut rng = Rng::new(
        0x5eed
            ^ id.bytes()
                .fold(0u64, |h, c| h.wrapping_mul(31).wrapping_add(c as u64)),
    );
    // a disagreement both answers hold at δ round it
    let robust = |p: Point3, kernel: bool, oracle: bool| {
        let d = [
            Vec3::new(delta, 0.0, 0.0),
            Vec3::new(-delta, 0.0, 0.0),
            Vec3::new(0.0, delta, 0.0),
            Vec3::new(0.0, -delta, 0.0),
            Vec3::new(0.0, 0.0, delta),
            Vec3::new(0.0, 0.0, -delta),
        ];
        d.iter()
            .all(|&o| kernel_in(&ix, p + o) == Some(kernel) && filled(spec.classify(p + o)) == Some(oracle))
    };
    let (mut over, mut under, mut unknown, mut judged) = (0usize, 0usize, 0usize, 0usize);
    let mut worst: Vec<String> = Vec::new();
    // either side of each new face
    let old: BTreeSet<_> = s.face_ids().map(|f| s.face(f).name).collect();
    let new: Vec<FaceId> = after
        .face_ids()
        .filter(|&f| !old.contains(&after.face(f).name))
        .collect();
    let per = (samples / 4 / new.len().max(1)).max(8);
    for &f in &new {
        for (p, n) in face_points(&after, f, per, &mut rng) {
            for (x, inside) in [(p - n * delta, true), (p + n * delta, false)] {
                if kernel_in(&ix, x) != Some(inside) {
                    continue;
                }
                judged += 1;
                match filled(spec.classify(x)) {
                    None => unknown += 1,
                    Some(o) if o == inside => {}
                    Some(o) => {
                        if robust(x, inside, o) {
                            if inside {
                                over += 1
                            } else {
                                under += 1
                            }
                            if worst.len() < 3 {
                                worst.push(format!("[{:.4},{:.4},{:.4}]", x.x, x.y, x.z));
                            }
                        }
                    }
                }
            }
        }
    }
    // random points round the picked edges, each held against the kernel's own answer; a point
    // of the body before that the kernel's result leaves out is `lost` (the result, or its
    // classification, dropped body material: not the fillet's shape)
    let mut lost = 0usize;
    for _ in 0..samples {
        let x = Point3::new(
            rng.range(region.x.lo, region.x.hi),
            rng.range(region.y.lo, region.y.hi),
            rng.range(region.z.lo, region.z.hi),
        );
        let w = spec.classify(x);
        let (Some(o), Some(kin)) = (filled(w), kernel_in(&ix, x)) else {
            if w == Where::Unknown {
                unknown += 1;
            }
            continue;
        };
        judged += 1;
        if o != kin && robust(x, kin, o) {
            if w == Where::Body {
                lost += 1;
            } else if kin {
                over += 1
            } else {
                under += 1
            }
            if worst.len() < 6 {
                worst.push(format!("[{:.4},{:.4},{:.4}]", x.x, x.y, x.z));
            }
        }
    }
    // the fill's volume, by the oracle alone (cheap away from the candidates 0 < d < r)
    let vol_samples: usize = std::env::var("VOL_SAMPLES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);
    let (mut fill_n, mut fill_unknown) = (0usize, 0usize);
    for _ in 0..vol_samples {
        let x = Point3::new(
            rng.range(region.x.lo, region.x.hi),
            rng.range(region.y.lo, region.y.hi),
            rng.range(region.z.lo, region.z.hi),
        );
        match spec.classify(x) {
            Where::Fill => fill_n += 1,
            Where::Unknown => fill_unknown += 1,
            _ => {}
        }
    }
    let vbox = (region.x.hi - region.x.lo) * (region.y.hi - region.y.lo) * (region.z.hi - region.z.lo);
    let pf = fill_n as f64 / vol_samples as f64;
    let fill = pf * vbox;
    let sigma = (pf * (1.0 - pf) / vol_samples as f64).sqrt() * vbox;
    let fill_unknown = fill_unknown as f64 / vol_samples as f64 * vbox;
    let label = if problems > 0 {
        "invalid"
    } else if over + under > 0 {
        "wrong"
    } else if lost > 0 {
        "unsure"
    } else {
        "right"
    };
    format!(
        "{{\"id\":\"{id}\",\"label\":\"{label}\",\"r\":{r},\"added\":{added},\"fill\":{fill:.4},\"fill_sigma\":{sigma:.4},\"fill_unknown\":{fill_unknown:.4},\"lost\":{lost},\"judged\":{judged},\"over\":{over},\"under\":{under},\"unknown\":{unknown},\"zones\":{},\"new_faces\":{},\"problems\":{problems},\"at\":[{}]}}",
        spec.zones.len(),
        new.len(),
        worst.join(",")
    )
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (fc, fg, out) = (&a[1], &a[2], &a[3]);
    let ids: Vec<String> = if a.len() > 4 {
        a[4..].to_vec()
    } else {
        let mut v: Vec<String> = std::fs::read_dir(fg)
            .expect("the .fg parts")
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str()?.strip_suffix(".fg").map(String::from))
            .collect();
        v.sort();
        v
    };
    let samples: usize = std::env::var("SAMPLES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    let jobs: usize = std::env::var("JOBS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let next = AtomicUsize::new(0);
    let lines = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|sc| {
        for _ in 0..jobs {
            sc.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(id) = ids.get(i) else { break };
                    let line = one(fc, fg, id, samples);
                    eprintln!("{line}");
                    lines.lock().expect("lines").push(line);
                }
            });
        }
    });
    let mut lines = lines.into_inner().expect("lines");
    lines.sort();
    std::fs::write(out, lines.join("\n") + "\n").expect("written");
}

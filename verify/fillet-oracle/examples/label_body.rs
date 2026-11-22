//! Every fillet of a set of .fg parts labelled against the oracle, with the body before the fillet
//! as the obstacle: label_body OUT.jsonl FILE.fg..
//!
//! Unlike label_pipe, the obstacle here is a kernel body. Its distance is the least distance to
//! its faces: the surface's foot where the face holds it, else the face's edges. That rests on the
//! kernel's projections and trims, so these labels are only as certified as those are. A label
//! carries `"obstacle":"kernel"` to say so.
//!
//! The spec is restricted to the rounded edges (§1.3): every edge the fillet rounded is taken as
//! picked, that is, every sharp edge of the body before whose name the result no longer has. Within
//! 3r of an unpicked concave edge, or of a vertex where rounded and unrounded edges meet (the vertex
//! table), a point the oracle fills and the kernel leaves is not counted. The restricted fill there
//! is less than the closing's. Within 3r of such a vertex, a point the kernel fills and the oracle
//! clears is not counted either, because the table's mitre lies outside the closing. Everything
//! else is judged as in label_pipe: either side of each new face at δ, and random points round the
//! rounded edges, each held at δ round it.
//!
//! One fillet is one radius class of one kind. A fillet of several radii, or of both kinds, is
//! `unjudged`.
use forge_bool::{Index, Where as KWhere};
use forge_fillet_oracle::{Covered, Obstacle, Search, covered};
use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::{Box3, Point2, Point3, Vec3, rng::Rng};
use forge_model::Feature;
use forge_topo::{Body, EdgeId, FaceId};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A kernel body as the obstacle: the unsigned distance to its faces, its side asked only where a
/// witness needs it. `inverted` takes the complement (a convex round's opening).
struct Kernel<'a> {
    b: &'a Body,
    ix: Index<'a>,
    boxes: Vec<Box3>,
    inverted: bool,
}

/// The tolerance the kernel's projections and trims stand to.
const KERNEL_ERR: f64 = 1e-7;

impl<'a> Kernel<'a> {
    fn new(b: &'a Body, inverted: bool) -> Self {
        let ix = Index::new(b);
        let boxes = b.face_ids().map(|f| b.face_box(f)).collect();
        Kernel {
            b,
            ix,
            boxes,
            inverted,
        }
    }

    fn face_distance(&self, f: FaceId, p: Point3, best: f64) -> f64 {
        let s = &self.b.face(f).surface;
        let foot = s.eval(s.project(p));
        let d = foot.distance(p);
        if d < best && self.ix.point_in_face(f, foot) != KWhere::Outside {
            return d;
        }
        let mut m = best;
        for &l in &self.b.face(f).loops {
            for c in self.b.lp(l).coedges() {
                let x = self.b.edge(self.b.coedge(c).edge);
                let q = x.curve.eval(x.curve.closest(p, x.range));
                m = m.min(q.distance(p));
            }
        }
        m
    }

    fn distance(&self, p: Point3) -> f64 {
        let mut order: Vec<(f64, usize)> = self
            .boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.is_empty())
            .map(|(i, b)| {
                let c = |v: f64, lo: f64, hi: f64| (lo - v).max(v - hi).max(0.0);
                let (dx, dy, dz) = (
                    c(p.x, b.x.lo, b.x.hi),
                    c(p.y, b.y.lo, b.y.hi),
                    c(p.z, b.z.lo, b.z.hi),
                );
                ((dx * dx + dy * dy + dz * dz).sqrt(), i)
            })
            .collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut best = f64::INFINITY;
        let ids: Vec<FaceId> = self.b.face_ids().collect();
        for (lower, i) in order {
            if lower >= best {
                break;
            }
            best = best.min(self.face_distance(ids[i], p, best));
        }
        best
    }

    /// Inside the obstacle (the body, or its complement when inverted).
    fn inside(&self, p: Point3) -> Option<bool> {
        let k = match self.ix.classify(p) {
            KWhere::Inside => true,
            KWhere::Outside => false,
            KWhere::On => return None,
        };
        Some(k != self.inverted)
    }
}

impl Obstacle for Kernel<'_> {
    fn sd(&self, p: Point3) -> f64 {
        let d = self.distance(p);
        match self.inside(p) {
            Some(true) => -d,
            Some(false) => d,
            None => 0.0,
        }
    }
    fn err(&self, _: Point3) -> f64 {
        KERNEL_ERR
    }
    fn upper(&self, p: Point3) -> f64 {
        self.distance(p) + KERNEL_ERR
    }
    fn clear_at(&self, p: Point3, need: f64) -> bool {
        self.distance(p) - KERNEL_ERR >= need && self.inside(p) == Some(false)
    }
}

fn faces_of(b: &Body, e: EdgeId) -> Vec<FaceId> {
    b.edge(e).coedges.iter().map(|&c| b.coedge_face(c)).collect()
}

fn seam(b: &Body, e: EdgeId) -> bool {
    let fs = faces_of(b, e);
    fs.len() == 2 && (fs[0] == fs[1] || b.face(fs[0]).surface == b.face(fs[1]).surface)
}

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

fn kin(ix: &Index, p: Point3) -> Option<bool> {
    match ix.classify(p) {
        KWhere::Inside => Some(true),
        KWhere::Outside => Some(false),
        KWhere::On => None,
    }
}

fn to_edge(b: &Body, e: EdgeId, p: Point3) -> f64 {
    let x = b.edge(e);
    x.curve.eval(x.curve.closest(p, x.range)).distance(p)
}

/// One fillet: the body before and after, its radius; the label as a JSON object's fields.
///
/// With `dump` (DUMP=DIR), a fillet with counted disagreements writes DIR/NAME.pts, one counted point
/// a line (`x y z over|under before`, `before` 1 where `Index` puts it in the body before), and the
/// bodies before and after as DIR/NAME.before.step and DIR/NAME.after.step, for a classifier other
/// than forge-bool's `Index` (research/fillets/oracle/occt_body.py).
fn judge(
    s: &Body,
    after: &Body,
    r: f64,
    per_face: usize,
    randoms: usize,
    seed: u64,
    dump: Option<&str>,
) -> String {
    let names: BTreeSet<_> = after.edge_ids().map(|e| after.edge(e).name).collect();
    let rounded: Vec<EdgeId> = s
        .edge_ids()
        .filter(|&e| !seam(s, e) && !names.contains(&s.edge(e).name))
        .collect();
    let kinds: BTreeSet<Option<bool>> = rounded
        .iter()
        .map(|&e| forge_ops::fillet::fillet_kind(s, e))
        .collect();
    let convex = match kinds.iter().collect::<Vec<_>>()[..] {
        [Some(k)] => *k,
        _ => return format!("\"label\":\"unjudged\",\"why\":\"kinds {kinds:?}\""),
    };
    let picked: BTreeSet<EdgeId> = rounded.iter().copied().collect();
    // where the restricted spec is not the closing: unpicked concave edges (for a concave round;
    // convex for a convex one) and vertices where rounded and unrounded edges meet
    let guards: Vec<EdgeId> = s
        .edge_ids()
        .filter(|&e| {
            !picked.contains(&e) && !seam(s, e) && forge_ops::fillet::fillet_kind(s, e) == Some(convex)
        })
        .collect();
    let mut table = Vec::new();
    for v in s.vertex_ids() {
        let at: Vec<EdgeId> = s
            .edge_ids()
            .filter(|&e| s.edge(e).start == v || s.edge(e).end == v)
            .collect();
        if at.iter().any(|e| picked.contains(e)) && at.iter().any(|e| !picked.contains(e) && !seam(s, *e)) {
            table.push(s.vertex(v).point);
        }
    }
    let near_guard = |x: Point3| guards.iter().any(|&e| to_edge(s, e, x) < 3.0 * r);
    let near_table = |x: Point3| table.iter().any(|v| v.distance(x) < 3.0 * r);
    let obstacle = Kernel::new(s, convex);
    let sx = Index::new(s);
    let ix = Index::new(after);
    let delta = (1e-3 * r).max(1e-5);
    let search = Search {
        tau: 1e-7,
        smallest: 0.02 * delta,
        budget: 200_000,
    };
    // the spec's answer: in the result (body kept, or filled) or not; None unknown
    let spec = |x: Point3| -> Option<bool> {
        let inside = kin(&sx, x)?;
        if inside != convex {
            // concave: in the body; convex: outside it, where nothing changes
            return Some(inside);
        }
        match covered(&obstacle, x, r, &search) {
            // a clear ball covers it: air for a concave round, material kept for a convex one
            Covered::Yes(_) => Some(convex),
            Covered::No => Some(!convex),
            Covered::Unknown => None,
        }
    };
    let mut rng = Rng::new(seed);
    let (mut over, mut under, mut unknown, mut judged, mut excused) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut at = Vec::new();
    let mut dumped: Vec<String> = Vec::new();
    let mut count = |x: Point3, k: bool, o: bool, at: &mut Vec<String>| {
        if k == o {
            return;
        }
        // held at δ round it
        let d = [
            Vec3::new(delta, 0.0, 0.0),
            Vec3::new(-delta, 0.0, 0.0),
            Vec3::new(0.0, delta, 0.0),
            Vec3::new(0.0, -delta, 0.0),
            Vec3::new(0.0, 0.0, delta),
            Vec3::new(0.0, 0.0, -delta),
        ];
        if !d
            .iter()
            .all(|&o2| kin(&ix, x + o2) == Some(k) && spec(x + o2) == Some(o))
        {
            return;
        }
        // `more`: the kernel keeps material where the spec has none. The whole closing (or opening)
        // changes more than the restricted spec does near an unpicked edge of the round's own kind:
        // a concave spec filling where the kernel left air, a convex one removing where the kernel
        // kept material. Only those are excused near such an edge. Near a table vertex, both are.
        let more = k && !o;
        let spec_changes_more = if convex { more } else { !more };
        let excuse = near_table(x) || (spec_changes_more && near_guard(x));
        if excuse {
            excused += 1;
            return;
        }
        if more {
            over += 1
        } else {
            under += 1
        }
        dumped.push(format!(
            "{} {} {} {} {}",
            x.x,
            x.y,
            x.z,
            if more { "over" } else { "under" },
            u8::from(kin(&sx, x) == Some(true))
        ));
        if at.len() < 4 {
            at.push(format!("[{:.4},{:.4},{:.4}]", x.x, x.y, x.z));
        }
    };
    let old: BTreeSet<_> = s.face_ids().map(|f| s.face(f).name).collect();
    let new: Vec<FaceId> = after
        .face_ids()
        .filter(|&f| !old.contains(&after.face(f).name))
        .collect();
    for &f in &new {
        for (p, n) in face_points(after, f, per_face, &mut rng) {
            for (x, inside) in [(p - n * delta, true), (p + n * delta, false)] {
                if kin(&ix, x) != Some(inside) {
                    continue;
                }
                judged += 1;
                match spec(x) {
                    None => unknown += 1,
                    Some(o) => count(x, inside, o, &mut at),
                }
            }
        }
    }
    let per_edge = (randoms / rounded.len().max(1)).max(4);
    for &e in &rounded {
        let b = s.edge_box(e).grow(2.0 * r);
        for _ in 0..per_edge {
            let x = Point3::new(
                rng.range(b.x.lo, b.x.hi),
                rng.range(b.y.lo, b.y.hi),
                rng.range(b.z.lo, b.z.hi),
            );
            let (Some(k), o) = (kin(&ix, x), spec(x)) else {
                continue;
            };
            let Some(o) = o else {
                unknown += 1;
                continue;
            };
            judged += 1;
            count(x, k, o, &mut at);
        }
    }
    if let (Some(name), false) = (dump, dumped.is_empty()) {
        let _ = std::fs::write(format!("{name}.pts"), dumped.join("\n") + "\n");
        for (b, w) in [(s, "before"), (after, "after")] {
            if let Ok(e) = forge_step::export(b, &forge_step::Options::default()) {
                let _ = std::fs::write(format!("{name}.{w}.step"), e.text);
            }
        }
    }
    let label = if forge_topo::check(after)
        .iter()
        .any(|p| !matches!(p, forge_topo::Problem::DuplicateName { .. }))
    {
        "invalid"
    } else if over + under > 0 {
        "wrong"
    } else {
        "right"
    };
    format!(
        "\"label\":\"{label}\",\"obstacle\":\"kernel\",\"r\":{r},\"convex\":{convex},\"rounded\":{},\"judged\":{judged},\"over\":{over},\"under\":{under},\"excused\":{excused},\"unknown\":{unknown},\"at\":[{}]",
        rounded.len(),
        at.join(",")
    )
}

fn one(path: &str, per_face: usize, randoms: usize) -> Vec<String> {
    let id = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string();
    let Ok(src) = std::fs::read_to_string(path) else {
        return vec![format!("{{\"id\":\"{id}\",\"label\":\"unread\"}}")];
    };
    let m = forge_lang::fuzz::build(&src);
    let mut out = Vec::new();
    for (k, f) in m.features.iter().enumerate() {
        let Feature::Fillet { edges, setbacks, .. } = f else {
            continue;
        };
        let head = format!("{{\"id\":\"{id}\",\"feature\":{k}");
        if k >= m.built() {
            let why = m
                .error()
                .map(|e| esc(&forge_lang::fuzz::message(e)))
                .unwrap_or_default();
            out.push(format!("{head},\"label\":\"refused\",\"why\":\"{why}\"}}"));
            break;
        }
        let radii: BTreeSet<u64> = edges
            .iter()
            .filter_map(|e| e.1.constant())
            .map(f64::to_bits)
            .collect();
        let constant = edges.iter().all(|e| e.1.constant().is_some());
        if radii.len() != 1 || !constant || !setbacks.is_empty() {
            out.push(format!(
                "{head},\"label\":\"unjudged\",\"why\":\"not one constant radius\"}}"
            ));
            continue;
        }
        let Some(r) = edges[0].1.constant() else { continue };
        let (Some(s), Some(a)) = (m.body_after(k), m.body_after(k + 1)) else {
            out.push(format!("{head},\"label\":\"unjudged\",\"why\":\"no body\"}}"));
            continue;
        };
        let seed = id
            .bytes()
            .fold(k as u64, |h, c| h.wrapping_mul(31).wrapping_add(c as u64));
        let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let dump = std::env::var("DUMP").ok().map(|d| format!("{d}/{id}.{k}"));
            judge(&s, &a, r, per_face, randoms, seed, dump.as_deref())
        }))
        .unwrap_or_else(|_| "\"label\":\"unjudged\",\"why\":\"panicked\"".into());
        out.push(format!("{head},{body}}}"));
    }
    out
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = &a[1];
    let files: Vec<String> = if a.len() == 3 && a[2].ends_with(".txt") {
        std::fs::read_to_string(&a[2])
            .expect("a list")
            .lines()
            .map(String::from)
            .collect()
    } else {
        a[2..].to_vec()
    };
    let per_face: usize = std::env::var("PER_FACE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(12);
    let randoms: usize = std::env::var("RANDOMS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let jobs: usize = std::env::var("JOBS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    std::panic::set_hook(Box::new(|_| {}));
    let next = AtomicUsize::new(0);
    let lines = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|sc| {
        for _ in 0..jobs {
            sc.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(f) = files.get(i) else { break };
                    for l in one(f, per_face, randoms) {
                        eprintln!("{l}");
                        lines.lock().expect("lines").push(l);
                    }
                }
            });
        }
    });
    let mut lines = lines.into_inner().expect("lines");
    lines.sort();
    std::fs::write(out, lines.join("\n") + "\n").expect("written");
}

//! A rolling-ball blend checked against the part before it, with no use of how the blend was
//! made: at points of each new blend face, a ball of the asked radius sits on the face's inner
//! side and touches at least two of the old part's faces or edges, and the faces either side of
//! a seam meant to be smooth share a normal.

use forge_bool::{Index, Where};
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::rng::Rng;
use forge_math::{Point2, Point3, UnitVec3, Vec3};
use forge_topo::{Body, EdgeId, FaceId, Name};
use std::collections::BTreeSet;

/// What one fillet's audit found.
#[derive(Clone, Debug, Default)]
pub struct Audit {
    /// Ball-surface faces new in the result, and points tried on them.
    pub faces: usize,
    pub points: usize,
    /// Over all points, the worst gap between the ball and its second-best support.
    pub worst_gap: f64,
    /// The worst point, and the radius it was held to.
    pub worst_at: Option<Point3>,
    /// Points whose ball rests on fewer than two supports within `GAP`.
    pub unsupported: usize,
    /// Worst sine of the angle across a seam meant to be smooth (`SMOOTH`).
    pub worst_kink: f64,
    pub kink_at: Option<Point3>,
}

/// How far a ball may stand off a support: above the rounding a traced spine leaves, well under
/// the tolerance on a point of a surface.
pub const GAP: f64 = 2e-6;
/// Under this sine a seam is taken to be meant to be smooth.
pub const SMOOTH: f64 = 0.02;

/// The radii a fillet asked for: a list, or where any varies, the range they span.
#[derive(Clone, Debug)]
pub enum Radii {
    List(Vec<f64>),
    Range(f64, f64),
}

impl Radii {
    pub fn of<'a>(laws: impl Iterator<Item = &'a forge_ops::Radius>) -> Radii {
        let (mut list, mut varies) = (Vec::new(), false);
        for r in laws {
            match r {
                forge_ops::Radius::Constant(x) => list.push(*x),
                forge_ops::Radius::Linear(x, y) => {
                    varies = true;
                    list.extend([*x, *y]);
                }
                forge_ops::Radius::Points(p) => {
                    varies = true;
                    list.extend(p.iter().map(|q| q.1));
                }
            }
        }
        if varies {
            let lo = list.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = list.iter().copied().fold(0.0, f64::max);
            return Radii::Range(lo, hi);
        }
        list.sort_by(f64::total_cmp);
        list.dedup();
        Radii::List(list)
    }
}

/// Whether a surface can be a rolling ball's: a blend has one radius of curvature across it.
fn ball_surface(s: &Surface) -> bool {
    match s {
        Surface::Cylinder(_) | Surface::Torus(_) | Surface::Sphere(_) => true,
        Surface::Procedural(p) => p
            .as_any()
            .is_some_and(|a| a.is::<forge_geom::blend::Pipe>() || a.is::<forge_geom::varblend::VarBlend>()),
        _ => false,
    }
}

/// Points of a face, by drawing parameters inside its loops.
fn face_points(b: &Body, f: FaceId, want: usize, rng: &mut Rng) -> Vec<(Point3, UnitVec3)> {
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
    for _ in 0..40 * want {
        if out.len() == want {
            break;
        }
        let uv = Point2::new(rng.range(lo.x, hi.x), rng.range(lo.y, hi.y));
        if forge_topo::uv::point_in_loops(uv, &loops)
            && let Some(n) = b.face_normal(f, uv)
        {
            out.push((s.eval(uv), n));
        }
    }
    out
}

/// A support's distance from `c` to `r`: faces by their surfaces (the ball may run past a face's
/// end, onto the surface carried on), edges by their curves.
fn gaps(before: &Body, after: &Body, own: FaceId, new: &[FaceId], c: Point3, r: f64) -> Vec<f64> {
    let reach = forge_math::Box3::from_point(c).grow(3.0 * r);
    let mut out = Vec::new();
    // a blend may rest on another of the fillet's own, one kind rounding first
    for &f in new {
        if f == own || !after.face_box(f).overlaps(&reach) {
            continue;
        }
        let s = &after.face(f).surface;
        out.push((s.eval(s.project(c)).distance(c) - r).abs());
    }
    for f in before.face_ids() {
        if !before.face_box(f).overlaps(&reach) {
            continue;
        }
        let s = &before.face(f).surface;
        let foot = s.eval(s.project(c));
        out.push((foot.distance(c) - r).abs());
    }
    for e in before.edge_ids() {
        if !before.edge_box(e).overlaps(&reach) {
            continue;
        }
        let x = before.edge(e);
        let foot = x.curve.eval(x.curve.closest(c, x.range));
        out.push((foot.distance(c) - r).abs());
    }
    out.sort_by(f64::total_cmp);
    out
}

/// One fillet's check: `after` holds the blends made from `before`, at radii `r`.
pub fn audit(before: &Body, after: &Body, r: &Radii, seed: u64) -> Audit {
    let old: BTreeSet<Name> = before.face_ids().map(|f| before.face(f).name).collect();
    let mut rng = Rng::new(seed);
    let mut out = Audit::default();
    let mut seams: BTreeSet<EdgeId> = BTreeSet::new();
    let new: Vec<FaceId> = after
        .face_ids()
        .filter(|&f| !old.contains(&after.face(f).name) && ball_surface(&after.face(f).surface))
        .collect();
    for &f in &new {
        let face = after.face(f);
        out.faces += 1;
        for &l in &face.loops {
            seams.extend(after.lp(l).coedges().map(|c| after.coedge(c).edge));
        }
        for (p, n) in face_points(after, f, 12, &mut rng) {
            out.points += 1;
            // a canal surface's section need not be square to its normal: its own ball answers,
            // and that ball is still held against the supports
            let var = match &face.surface {
                Surface::Procedural(q) => q
                    .as_any()
                    .and_then(|a| a.downcast_ref::<forge_geom::varblend::VarBlend>())
                    .map(|v| v.ball(q.project(p).x)),
                _ => None,
            };
            let gap = match var {
                Some((c, radius)) => {
                    let g = gaps(before, after, f, &new, c, radius);
                    (p.distance(c) - radius)
                        .abs()
                        .max(g.get(1).copied().unwrap_or(f64::INFINITY))
                }
                None => {
                    // a surface with one radius of its own: that, which the fillet must have asked for
                    let own = match &face.surface {
                        Surface::Cylinder(k) => Some(k.radius),
                        Surface::Torus(k) => Some(k.minor),
                        Surface::Sphere(k) => Some(k.radius),
                        _ => None,
                    };
                    let asked = match (own, r) {
                        (Some(x), Radii::List(l)) => l.iter().any(|&y| (x - y).abs() <= GAP).then_some(x),
                        (Some(x), Radii::Range(lo, hi)) => (x >= lo - GAP && x <= hi + GAP).then_some(x),
                        _ => None,
                    };
                    match (own, asked) {
                        (Some(_), None) => f64::INFINITY,
                        (_, Some(x)) => ball_gap(before, after, f, &new, p, n.get(), &Radii::List(vec![x])),
                        _ => ball_gap(before, after, f, &new, p, n.get(), r),
                    }
                }
            };
            if gap > out.worst_gap {
                out.worst_gap = gap;
                out.worst_at = Some(p);
            }
            if gap > GAP {
                out.unsupported += 1;
            }
        }
    }
    for e in seams {
        let x = after.edge(e);
        let [c0, c1] = x.coedges[..] else { continue };
        let (f0, f1) = (after.coedge_face(c0), after.coedge_face(c1));
        if f0 == f1 {
            continue;
        }
        for i in 0..=8 {
            let p = x.curve.eval(x.range.lo + x.range.width() * i as f64 / 8.0);
            let n = |f: FaceId| after.face_normal(f, after.face(f).surface.project(p));
            let Some((a, b)) = n(f0).zip(n(f1)) else { continue };
            let sine = a.cross(b).length();
            if a.dot(b.get()) > 0.0 && sine < SMOOTH && sine > out.worst_kink {
                out.worst_kink = sine;
                out.kink_at = Some(p);
            }
        }
    }
    out
}

/// The gap between a ball on either side of the point and its second-best support, the better
/// side's, at the radius in `r` that suits it best.
fn ball_gap(before: &Body, after: &Body, own: FaceId, new: &[FaceId], p: Point3, n: Vec3, r: &Radii) -> f64 {
    let at = |side: f64, r: f64| {
        let g = gaps(before, after, own, new, p + n * (side * r), r);
        g.get(1).copied().unwrap_or(f64::INFINITY)
    };
    let mut best = f64::INFINITY;
    for side in [-1.0, 1.0] {
        let (lo, hi) = match r {
            Radii::List(l) => {
                for &x in l {
                    best = best.min(at(side, x));
                }
                continue;
            }
            Radii::Range(lo, hi) => (*lo, *hi),
        };
        // a varying radius: the second-best gap is V-shaped about the point's own radius
        let grid = 24;
        let mut low = (f64::INFINITY, lo);
        for i in 0..=grid {
            let x = lo + (hi - lo) * i as f64 / grid as f64;
            let g = at(side, x);
            if g < low.0 {
                low = (g, x);
            }
        }
        // golden section round it
        let step = (hi - lo) / grid as f64;
        let (mut a, mut b) = (low.1 - step, low.1 + step);
        let phi = 0.618_033_988_749_894_8;
        for _ in 0..40 {
            let (c, d) = (b - phi * (b - a), a + phi * (b - a));
            if at(side, c) < at(side, d) {
                b = d;
            } else {
                a = c;
            }
        }
        best = best.min(at(side, 0.5 * (a + b)));
    }
    best
}

/// What points near a fillet say, against the part before it and with no use of how it was made.
#[derive(Clone, Debug, Default)]
pub struct Points {
    pub tried: usize,
    /// Points the fillet changed that no ball surface accounts for, or that it left unchanged
    /// though one does; a point on a patched corner is not counted.
    pub wrong: usize,
    pub worst: Option<Point3>,
    /// Of `sampled` points drawn evenly in `reach`, how many the fillet changed, and that box's
    /// volume: an estimate of the volume it changed, to hold against the masses.
    pub sampled: usize,
    pub changed: usize,
    pub reach_volume: f64,
}

/// The ball's centre for point `p` over a swept surface: its axis or circle's nearest point.
fn spine_near(s: &Surface, p: Point3) -> Option<Point3> {
    match s {
        Surface::Cylinder(c) => {
            let z = c.frame.z().get();
            Some(c.frame.origin + z * z.dot(p - c.frame.origin))
        }
        Surface::Torus(t) => {
            let l = t.frame.point_to_local(p);
            let rho = (l.x * l.x + l.y * l.y).sqrt();
            if rho <= 0.0 {
                return None;
            }
            let k = t.major / rho;
            Some(t.frame.point_to_world(Point3::new(l.x * k, l.y * k, 0.0)))
        }
        Surface::Sphere(sp) => Some(sp.frame.origin),
        _ => None,
    }
}

/// Where a new round's rim passes along an old face that runs on into its ball: the ball rests on
/// that rim only where the face beyond falls away from it. A rim is an old edge along the round (a
/// line down a cylinder, a circle round a torus); the face beyond is asked a step towards the foot
/// of the ball's centre on it: leading onto the face, with the centre on the ball's side, it is in the way.
pub fn ridges(before: &Body, after: &Body) -> Vec<Point3> {
    let old: std::collections::BTreeSet<_> = before.face_ids().map(|f| before.face(f).name).collect();
    let mut out = Vec::new();
    for f in after.face_ids() {
        let face = after.face(f);
        let radius = match &face.surface {
            Surface::Cylinder(c) => c.radius,
            Surface::Torus(t) => t.minor,
            _ => continue,
        };
        if old.contains(&face.name) {
            continue;
        }
        for &lp in &face.loops {
            for c in after.lp(lp).coedges() {
                let e = after.coedge(c).edge;
                let x = after.edge(e);
                let along = match (&face.surface, &x.curve) {
                    (Surface::Cylinder(s), Curve::Line(k)) => {
                        k.dir.get().cross(s.frame.z().get()).length() < 1e-9
                    }
                    (Surface::Torus(s), Curve::Circle(k)) => {
                        k.frame.z().get().cross(s.frame.z().get()).length() < 1e-9
                    }
                    _ => false,
                };
                let Some(other) = after.partner(c).map(|p| (p, after.coedge_face(p))) else {
                    continue;
                };
                // two new rounds may meet at a ridge where no ball rests on both: only an old face blocks
                if !along || !old.contains(&after.face(other.1).name) {
                    continue;
                }
                let d = x.curve.deriv(x.range.mid());
                let Some(centre) = spine_near(&face.surface, d.p) else {
                    continue;
                };
                let s = &after.face(other.1).surface;
                let uv = s.project(centre);
                let foot = s.eval(uv);
                let Some(n) = after.face_normal(other.1, uv) else {
                    continue;
                };
                let Some(towards) = (foot - d.p).normalize() else {
                    continue;
                };
                // the centre on the ball's own side of that face: in the material for a convex round
                let Some(own) = after.face_normal(f, face.surface.project(d.p)) else {
                    continue;
                };
                let convex = own.get().dot(d.p - centre) > 0.0;
                let above = (centre - foot).dot(n.get());
                let step = s.eval(s.project(d.p + towards.get() * (1e-3 * radius)));
                let loops = forge_topo::uv::face_uv_loops(after, other.1);
                // a rim the part had: a new edge there is the round cut off by a wall it passes through.
                // Tangent there to the face it took, the ball is as wide as that face and rests on it.
                let was = before.edge_ids().find(|&g| {
                    let y = before.edge(g);
                    y.curve.eval(y.curve.closest(d.p, y.range)).distance(d.p) <= forge_math::tol::LINEAR
                });
                let was_edge = was.is_some_and(|g| {
                    !before.edge(g).coedges.iter().any(|&k| {
                        let w = before.coedge_face(k);
                        let sw = &before.face(w).surface;
                        before
                            .face_normal(w, sw.project(d.p))
                            .is_some_and(|m| m.get().cross(own.get()).length() < 1e-6)
                    })
                });
                if was_edge
                    && (if convex { above < 0.0 } else { above > 0.0 })
                    && foot.distance(centre) < radius - forge_math::tol::LINEAR
                    && forge_topo::uv::point_in_loops(s.project(step), &loops)
                {
                    out.push(d.p);
                }
            }
        }
    }
    out
}

/// Random points near the blends of a fillet, each judged by the ball surfaces. A point the part
/// held and the fillet took away (or the reverse, on a concave round) must lie beyond a blend
/// face from that face's ball centre. Only cylinders, tori and spheres can answer; a fillet with
/// other new faces has those faces' boxes left out: a patched corner is its kernel's own choice.
pub fn points(before: &Body, after: &Body, want: usize, seed: u64) -> Points {
    let old: BTreeSet<Name> = before.face_ids().map(|f| before.face(f).name).collect();
    let (mut blends, mut others) = (Vec::new(), Vec::new());
    // a patch standing where two blends should mitre is answered for like any other change
    let needless = needless_patches(before, after);
    for f in after.face_ids() {
        let face = after.face(f);
        if old.contains(&face.name) {
            continue;
        }
        match face.surface {
            Surface::Cylinder(_) | Surface::Torus(_) | Surface::Sphere(_) => blends.push(f),
            _ if needless.contains(&corners_box(after, f).center()) => {}
            // a patch bulges past its edges
            _ => others.push(corners_box(after, f).grow(0.5 * corners_box(after, f).diagonal())),
        }
    }
    let mut out = Points::default();
    let mut reach = forge_math::Box3::EMPTY;
    for &f in &blends {
        reach = reach.hull(&after.face_box(f));
    }
    if blends.is_empty() {
        return out;
    }
    let (ib, ia) = (Index::new(before), Index::new(after));
    let mut r = Rng::new(seed ^ 0xB1E);
    // the corners of the blend faces, where they meet what ends them
    let ends: Vec<forge_math::Box3> = blends
        .iter()
        .flat_map(|&f| {
            let lps: Vec<_> = after.face(f).loops.to_vec();
            lps.into_iter()
                .flat_map(|l| after.lp(l).coedges().collect::<Vec<_>>())
        })
        .map(|c| {
            let p = after.vertex(after.coedge_start(c)).point;
            forge_math::Box3::from_point(p).grow(0.25 * reach_size(after, &blends))
        })
        .collect();
    out.reach_volume = reach.x.width() * reach.y.width() * reach.z.width();
    let pick = |r: &mut Rng, b: &forge_math::Box3| {
        Point3::new(
            r.range(b.x.lo, b.x.hi),
            r.range(b.y.lo, b.y.hi),
            r.range(b.z.lo, b.z.hi),
        )
    };
    for k in 0..want {
        // half round each blend (grown by a fifth), the rest anywhere near them
        let even = k % 3 == 1;
        let b = if k % 3 == 0 {
            let f = after.face_box(blends[k / 3 % blends.len()]);
            f.grow(0.2 * f.diagonal())
        } else if even || ends.is_empty() {
            reach
        } else {
            ends[k / 3 % ends.len()]
        };
        let p = pick(&mut r, &b);
        if others.iter().any(|o| o.contains(p)) {
            continue;
        }
        let (was, now) = (ib.classify(p), ia.classify(p));
        if matches!(was, Where::On) || matches!(now, Where::On) {
            continue;
        }
        out.tried += 1;
        let changed = (was == Where::Inside) != (now == Where::Inside);
        if even {
            out.sampled += 1;
            out.changed += usize::from(changed);
        }
        // beyond a blend face from its centre, along the ball's radius through the point
        let mut clear = false;
        // (beyond a face's stretch of its surface, beyond the face as trimmed)
        let (mut may, mut must) = (false, false);
        for &f in &blends {
            let s = &after.face(f).surface;
            let Some(c) = spine_near(s, p) else { continue };
            let Some(d) = (p - c).normalize() else { continue };
            let radius = match s {
                Surface::Cylinder(k) => k.radius,
                Surface::Torus(k) => k.minor,
                Surface::Sphere(k) => k.radius,
                _ => continue,
            };
            let dist = p.distance(c);
            let q = c + d.get() * radius;
            if (dist - radius).abs() <= 1e-3 {
                clear = true;
                continue;
            }
            // a convex round takes what the part held, a concave one fills what it did not; a
            // point the ball covers is neither. It lies in the wedge: nothing of another class
            // before it, and further on the class changes
            let wedge = (was == Where::Inside) == after.face(f).same_sense
                && dist > radius
                && (1..5).all(|i| ib.classify(q.lerp(p, i as f64 / 5.0)) == was)
                && (1..=12).any(|i| ib.classify(p + d.get() * (i as f64 * radius / 3.0)) != was);
            let inside_face = !matches!(ia.point_in_face(f, q), Where::Outside);
            if wedge && (inside_face || in_span(after, f, q)) {
                may = true;
                // past a cut with another blend a wedge may or may not go: which, the corner's
                // kind says, so only what is beyond the face as it stands, and no further from the
                // centre than the old corner (where the tangents at the feet cross), must have changed
                must |= dist <= corner_reach(after, f, radius) && inside_face;
            }
        }
        if clear {
            continue;
        }
        if (changed && !may) || (!changed && must) {
            out.wrong += 1;
            out.worst.get_or_insert(p);
        }
    }
    out
}

/// How far from the ball's centre the old corner can be: the tangents at the ends of the arc the
/// face spans cross at the radius over the cosine of half that arc.
fn corner_reach(b: &Body, f: FaceId, radius: f64) -> f64 {
    let loops = forge_topo::uv::face_uv_loops(b, f);
    // the arc the angles cover: a whole turn less the widest gap among them
    let width = |pick: fn(&Point2) -> f64| {
        let tau = forge_math::dmath::TAU;
        let mut a: Vec<f64> = loops
            .iter()
            .flat_map(|l| l.points.iter().map(pick))
            .map(|x| x.rem_euclid(tau))
            .collect();
        a.sort_by(f64::total_cmp);
        let gap = a.windows(2).map(|w| w[1] - w[0]).fold(
            a.first().zip(a.last()).map_or(0.0, |(p, q)| p + tau - q),
            f64::max,
        );
        tau - gap
    };
    let arc = match &b.face(f).surface {
        Surface::Cylinder(_) => width(|p| p.x),
        Surface::Torus(_) => width(|p| p.y),
        // a corner's ball: three tangent planes square to each other at the widest
        _ => 1.91,
    };
    let half = forge_math::dmath::cos(0.5 * arc.min(2.6));
    1.001 * radius / half.max(0.05)
}

/// Whether `q`, on a face's surface, lies within the box of parameters its loops span.
fn in_span(b: &Body, f: FaceId, q: Point3) -> bool {
    let loops = forge_topo::uv::face_uv_loops(b, f);
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for l in &loops {
        for p in &l.points {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    let s = &b.face(f).surface;
    let uv = s.project(q);
    let d = s.domain();
    let turns = |p: Option<f64>| match p {
        Some(p) => vec![0.0, -p, p],
        None => vec![0.0],
    };
    turns(d.u.period).iter().any(|&du| {
        turns(d.v.period)
            .iter()
            .any(|&dv| (lo.x..=hi.x).contains(&(uv.x + du)) && (lo.y..=hi.y).contains(&(uv.y + dv)))
    })
}

/// The narrower size of the fillet's blends: how far round a corner is worth sampling.
fn reach_size(after: &Body, blends: &[FaceId]) -> f64 {
    blends
        .iter()
        .map(|&f| match &after.face(f).surface {
            Surface::Cylinder(k) => k.radius,
            Surface::Torus(k) => k.minor,
            Surface::Sphere(k) => k.radius,
            _ => 0.0,
        })
        .fold(0.0, f64::max)
        .max(1e-3)
        * 4.0
}

/// Worst distance of a new corner face's boundary from its neighbours' surfaces: every edge of a
/// patch lies on both its faces, and worst sine across those meant to be smooth.
pub fn patch_seams(before: &Body, after: &Body) -> (f64, f64) {
    let old: BTreeSet<Name> = before.face_ids().map(|f| before.face(f).name).collect();
    let (mut off, mut kink) = (0.0f64, 0.0f64);
    for f in after.face_ids() {
        let face = after.face(f);
        if old.contains(&face.name) || ball_surface(&face.surface) {
            continue;
        }
        for &l in &face.loops {
            for c in after.lp(l).coedges() {
                let x = after.edge(after.coedge(c).edge);
                let [c0, c1] = x.coedges[..] else { continue };
                let (f0, f1) = (after.coedge_face(c0), after.coedge_face(c1));
                for i in 0..=8 {
                    let p = x.curve.eval(x.range.lo + x.range.width() * i as f64 / 8.0);
                    let at = |g: FaceId| {
                        let s = &after.face(g).surface;
                        let uv = s.project(p);
                        (s.eval(uv).distance(p), after.face_normal(g, uv))
                    };
                    let ((d0, n0), (d1, n1)) = (at(f0), at(f1));
                    off = off.max(d0).max(d1);
                    if let Some((a, b)) = n0.zip(n1) {
                        let sine = a.cross(b).length();
                        if a.dot(b.get()) > 0.0 && sine < SMOOTH {
                            kink = kink.max(sine);
                        }
                    }
                }
            }
        }
    }
    (off, kink)
}

/// The box of a face's edges' points: a corner patch's own box is its whole surface's.
fn corners_box(b: &Body, f: FaceId) -> forge_math::Box3 {
    let mut out = forge_math::Box3::EMPTY;
    for &l in &b.face(f).loops {
        for c in b.lp(l).coedges() {
            out = out.hull(&forge_math::Box3::from_points(b.edge_points(b.coedge(c).edge, 8)));
        }
    }
    out
}

/// New corner patches standing where at most two blends meet (three faces there, the third edge
/// sharp): such a corner is a mitre, which needs no patch. A setback asks for one.
pub fn needless_patches(before: &Body, after: &Body) -> Vec<Point3> {
    let old: BTreeSet<Name> = before.face_ids().map(|f| before.face(f).name).collect();
    let kept: BTreeSet<Name> = after.edge_ids().map(|e| after.edge(e).name).collect();
    let mut out = Vec::new();
    for f in after.face_ids() {
        let face = after.face(f);
        if old.contains(&face.name) || ball_surface(&face.surface) {
            continue;
        }
        let c = corners_box(after, f).center();
        let Some(v) = before.vertex_ids().min_by(|&a, &b| {
            before
                .vertex(a)
                .point
                .distance(c)
                .total_cmp(&before.vertex(b).point.distance(c))
        }) else {
            continue;
        };
        let at: Vec<EdgeId> = before
            .edge_ids()
            .filter(|&e| before.edge(e).start == v || before.edge(e).end == v)
            .collect();
        let rounded = at
            .iter()
            .filter(|&&e| !kept.contains(&before.edge(e).name))
            .count();
        if at.len() == 3 && rounded <= 2 {
            out.push(c);
        }
    }
    out
}

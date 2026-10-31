//! The local-imprint boolean: the part changes only where the tool meets it, each decision made
//! in closed form, with no face split by tracing regions and no point-in-face test per region.
//! It runs beside the general boolean (`boolean.rs`), never instead of it: a case it does not
//! cover returns false before touching the part, and the general boolean runs.
//!
//! Covered so far: an extruded profile (one loop of lines and arcs, or a whole circle) whose
//! section lies inside, and clear of the boundary of, each planar face square to its axis that it
//! passes through, and which no other face of the part comes near (thru holes, pockets, bosses,
//! slots that meet nothing else). Along the axis the part's material inside the tool is then a set
//! of slabs between those faces, so the faces the tool passes gain a hole, the wall stands where
//! the material changes, and a cap closes a wall that ends inside the tool.

use crate::boolean::{Op, across, line_box, on_edge, settle, vertex_names};
use crate::classify::{circle_in_face, extend_query, loop_in_face};
use forge_geom::intersect::{Contact, CurveSurface, SurfaceSurface, curve_surface, plane_cylinder};
use forge_geom::{Curve, CurveGeom, Cylinder, Surface};
use forge_math::{Box3, Interval, Point3, UnitVec3, tol};
use forge_topo::{Body, CoedgeId, EdgeId, FaceId, Name, ShellId, VertexId};

/// How far apart things must be for this path to decide them; anything nearer goes to the
/// general boolean.
const CLEAR: f64 = 10.0 * tol::LINEAR;

/// Which boolean runs: FORGE_IMPRINT=on takes this path where it can, =check also runs the
/// general boolean and panics where they differ. Off by default.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    Check,
}

static MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(u8::MAX);

pub fn mode() -> Mode {
    let m = MODE.load(std::sync::atomic::Ordering::Relaxed);
    let m = if m == u8::MAX {
        let m = match std::env::var("FORGE_IMPRINT").as_deref() {
            Ok("on") => 1,
            Ok("check") => 2,
            _ => 0,
        };
        MODE.store(m, std::sync::atomic::Ordering::Relaxed);
        m
    } else {
        m
    };
    [Mode::Off, Mode::On, Mode::Check][m as usize]
}

/// Sets the mode for the whole process (tests).
pub fn set_mode(m: Mode) {
    MODE.store(m as u8, std::sync::atomic::Ordering::Relaxed);
}

static TAKEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How many booleans this path has done so far in the process.
pub fn taken() -> usize {
    TAKEN.load(std::sync::atomic::Ordering::Relaxed)
}

/// A tool as `forge_ops::extrude` builds it: one profile loop swept straight along `z`, its
/// walls in profile order (a whole circle is one wall, its seam both of the wall's sides).
struct Tool {
    z: UnitVec3,
    origin: Point3,
    /// Heights of the bottom and top caps above `origin`.
    h: [f64; 2],
    caps: [FaceId; 2],
    walls: Vec<Wall>,
    /// The one wall's cylinder, for a whole circle.
    round: Option<Cylinder>,
}

struct Wall {
    face: FaceId,
    bottom: EdgeId,
    top: EdgeId,
    /// Whether the bottom edge runs along the profile.
    fwd: bool,
    /// The upright edges at the wall's start and end along the profile, and their coedges in
    /// the wall's loop.
    sides: [EdgeId; 2],
    side_uses: [CoedgeId; 2],
}

fn tool(b: &Body) -> Option<Tool> {
    let mut walls = Vec::new();
    let mut caps = Vec::new();
    for f in b.face_ids() {
        let face = b.face(f);
        let [l] = face.loops[..] else { return None };
        let cs = &b.lp(l).coedges;
        if cs.len() != 4 {
            caps.push(f);
            continue;
        }
        let u = |i: usize| (b.coedge(cs[i]).edge, b.coedge(cs[i]).forward);
        let ((bottom, fwd), (end, true), (top, back), (start, false)) = (u(0), u(1), u(2), u(3)) else {
            caps.push(f);
            continue;
        };
        let upright = |e: EdgeId| matches!(b.edge(e).curve, Curve::Line(_));
        if back == fwd || !upright(start) || !upright(end) {
            caps.push(f);
            continue;
        }
        walls.push(Wall {
            face: f,
            bottom,
            top,
            fwd,
            sides: [start, end],
            side_uses: [cs[3], cs[1]],
        });
    }
    let n = walls.len();
    if n == 0 || caps.len() != 2 || (0..n).any(|k| walls[k].sides[1] != walls[(k + 1) % n].sides[0]) {
        return None;
    }
    let Curve::Line(l) = &b.edge(walls[0].sides[0]).curve else {
        return None;
    };
    let z = l.dir;
    for w in &walls {
        let ok = match &b.face(w.face).surface {
            Surface::Plane(p) => p.normal_vec().dot(z.get()).abs() <= 1e-12,
            Surface::Cylinder(c) => c.frame.z().dot(z.get()).abs() >= 1.0 - 1e-12,
            _ => false,
        };
        let Curve::Line(s) = &b.edge(w.sides[0]).curve else {
            return None;
        };
        if !ok || s.dir.dot(z.get()) < 1.0 - 1e-12 {
            return None;
        }
    }
    // caps square to the axis, facing out of the tool: the bottom one down
    let mut ends = [None; 2];
    for &f in &caps {
        let face = b.face(f);
        let Surface::Plane(pl) = &face.surface else {
            return None;
        };
        let out = pl.normal_vec().dot(z.get()) * if face.same_sense { 1.0 } else { -1.0 };
        if out.abs() < 1.0 - 1e-12 {
            return None;
        }
        ends[usize::from(out > 0.0)] = Some((f, pl.frame.origin));
    }
    let [Some((bottom, origin)), Some((top, at))] = ends else {
        return None;
    };
    let round = match (&walls[..], &b.face(walls[0].face).surface) {
        ([w], Surface::Cylinder(c)) if w.fwd && b.face(w.face).same_sense => Some(*c),
        _ => None,
    };
    Some(Tool {
        z,
        origin,
        h: [0.0, (at - origin).dot(z.get())],
        caps: [bottom, top],
        walls,
        round,
    })
}

/// Where the tool's profile, swept, passes through a planar face of A square to it: one curve
/// per wall, between the uprights it meets.
struct Hit {
    f: FaceId,
    h: f64,
    /// The face's outward normal points up the axis.
    up: bool,
    /// Per wall: its curve, that curve's range, and whether it starts at the wall's start side.
    segs: Vec<(Curve, Interval, bool)>,
    /// Per upright (wall k's start side): the vertex there and its tolerance.
    corners: Vec<(Point3, f64)>,
}

/// Whether face `f` stays clear of the tool (swept both ways along its axis), judged by the
/// face's surface or its box against the profile's.
fn clear_of(a: &Body, b: &Body, f: FaceId, t: &Tool) -> bool {
    let z = t.z.get();
    if let Some(cyl) = &t.round {
        let (fr, r) = (&cyl.frame, cyl.radius);
        let apart = match &a.face(f).surface {
            // apart, or the tool wholly inside the other cylinder
            Surface::Cylinder(c) if c.frame.z().dot(z).abs() >= 1.0 - 1e-12 => {
                let off = c.frame.origin - fr.origin;
                let d = (off - z * off.dot(z)).length();
                d > r + c.radius + CLEAR || d + r < c.radius - CLEAR
            }
            Surface::Plane(p) if p.normal_vec().dot(z).abs() <= 1e-12 => {
                p.signed_distance(fr.origin).abs() > r + CLEAR
            }
            _ => false,
        };
        if apart {
            return true;
        }
    }
    // the boxes in a frame along the axis
    let fr = match &t.round {
        Some(cyl) => cyl.frame,
        None => forge_math::Frame::from_z(t.origin, t.z),
    };
    let local = |bx: &Box3| {
        let mut l = Box3::EMPTY;
        for x in [bx.x.lo, bx.x.hi] {
            for y in [bx.y.lo, bx.y.hi] {
                for w in [bx.z.lo, bx.z.hi] {
                    l.include(fr.point_to_local(Point3::new(x, y, w)));
                }
            }
        }
        l
    };
    let l = local(&a.face_box(f));
    let lift = (fr.origin - t.origin).dot(z);
    if l.z.hi + lift < t.h[0] - CLEAR || l.z.lo + lift > t.h[1] + CLEAR {
        return true;
    }
    if let Some(cyl) = &t.round {
        let dx = l.x.lo.max(-l.x.hi).max(0.0);
        let dy = l.y.lo.max(-l.y.hi).max(0.0);
        return dx * dx + dy * dy > (cyl.radius + CLEAR) * (cyl.radius + CLEAR);
    }
    let p = local(&b.face_box(t.caps[0])).grow(CLEAR);
    !l.x.overlaps(&p.x) || !l.y.overlaps(&p.y)
}

/// The boolean of `a` with `b` done in place when `b` is a tool this path covers; false, with
/// `a` untouched, otherwise.
pub(crate) fn column(a: &mut Body, b: &Body, op: Op) -> bool {
    match plan(a, b, op) {
        Ok((t, hits, wall_at)) => {
            build(a, b, &t, &hits, &wall_at, op == Op::Subtract);
            TAKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if crate::boolean::debug_on() {
                eprintln!("imprint: taken");
            }
            true
        }
        Err(why) => {
            if crate::boolean::debug_on() {
                eprintln!("imprint: declined: {why}");
            }
            false
        }
    }
}

type Plan = (Tool, Vec<Hit>, Vec<bool>);

/// What the tool does to `a`, or why this path leaves it to the general boolean.
fn plan(a: &Body, b: &Body, op: Op) -> Result<Plan, &'static str> {
    if op == Op::Intersect {
        return Err("intersect");
    }
    let t = tool(b).ok_or("not an extruded profile")?;
    let z = t.z.get();
    let bb = match &t.round {
        // the tool's box: its two end disks'
        Some(cyl) => {
            let r = cyl.radius;
            let reach = |a: f64| r * (1.0 - a * a).max(0.0).sqrt() + tol::LINEAR;
            let half = Point3::new(reach(z.x), reach(z.y), reach(z.z)) - Point3::ORIGIN;
            let mut bb = Box3::EMPTY;
            for h in t.h {
                let c = cyl.frame.origin + z * ((t.origin - cyl.frame.origin).dot(z) + h);
                bb.include(c - half);
                bb.include(c + half);
            }
            bb
        }
        None => b.face_box(t.caps[0]).hull(&b.face_box(t.caps[1])),
    };
    let mut hits: Vec<Hit> = Vec::new();
    for f in a.faces_where(|x| x.overlaps(&bb)) {
        let face = a.face(f);
        let pl = match &face.surface {
            Surface::Plane(pl) if pl.normal_vec().dot(z).abs() >= 1.0 - 1e-12 => pl,
            _ => {
                if clear_of(a, b, f, &t) {
                    continue;
                }
                return Err("a face not square to the axis comes near");
            }
        };
        let h = (pl.frame.origin - t.origin).dot(z);
        if h < t.h[0] - CLEAR || h > t.h[1] + CLEAR {
            continue;
        }
        if h < t.h[0] + CLEAR || h > t.h[1] - CLEAR {
            return Err("a face on a cap's plane");
        }
        // where the uprights cross the face's plane
        let crossing = |e: EdgeId| {
            let Curve::Line(l) = &b.edge(e).curve else {
                unreachable!("uprights are lines")
            };
            l.eval(-pl.signed_distance(l.origin) / pl.normal_vec().dot(l.dir.get()))
        };
        // each wall's curve in the plane (one, a line or a circle, square to the axis)
        let mut curves: Vec<Curve> = Vec::with_capacity(t.walls.len());
        for w in &t.walls {
            let cs = match (&t.round, &b.face(w.face).surface) {
                (Some(cyl), _) => plane_cylinder(pl, cyl),
                (None, s) => crate::ssi::closed_form(&Surface::Plane(*pl), s),
            };
            let SurfaceSurface::Curves(mut cs) = cs else {
                return Err("a wall meets the plane in no curve");
            };
            if cs.len() != 1 {
                return Err("a wall meets the plane in no curve");
            }
            curves.push(cs.swap_remove(0).0);
        }
        let inside = match (&t.round, &curves[..]) {
            (Some(_), [Curve::Circle(circle)]) => circle_in_face(a, f, circle, CLEAR),
            _ => {
                let up = forge_math::Frame::WORLD.with_origin(Point3::ORIGIN + z * h);
                let pieces: Vec<(Curve, Interval)> = t
                    .walls
                    .iter()
                    .map(|w| (b.edge(w.bottom).curve.transformed(&up), b.edge(w.bottom).range))
                    .collect();
                loop_in_face(a, f, &pieces, CLEAR)
            }
        };
        match inside {
            None => return Err("a face's boundary comes near the profile"),
            Some(true) => {}
            Some(false) => {
                // the general boolean cuts an upright where the face's box holds the crossing,
                // even off the face: a vertex this path would not make
                let bx = a.face_box(f).grow(CLEAR);
                if t.walls.iter().any(|w| bx.contains(crossing(w.sides[0]))) {
                    return Err("an upright crosses a face's box off the face");
                }
                continue;
            }
        }
        let up = (pl.normal_vec().dot(z) > 0.0) == face.same_sense;
        let (segs, corners) = section(a, b, &t, f, pl, curves).ok_or("no section")?;
        hits.push(Hit {
            f,
            h,
            up,
            segs,
            corners,
        });
    }
    if hits.is_empty() {
        return Err("no face crossed");
    }
    hits.sort_by(|x, y| x.h.total_cmp(&y.h));
    // the material in each stretch between hits (and the caps): below a face facing up
    for w in hits.windows(2) {
        if w[0].up == w[1].up {
            return Err("faces out of turn");
        }
    }
    let n = hits.len();
    let solid: Vec<bool> = (0..=n)
        .map(|k| if k < n { hits[k].up } else { !hits[n - 1].up })
        .collect();
    let fill = op == Op::Union;
    let wall_at: Vec<bool> = solid.iter().map(|&m| m != fill).collect();
    Ok((t, hits, wall_at))
}

/// The corners at the low and high parameter ends of wall `k`'s curve (of `n`), which starts at
/// the wall's start side or its end side.
fn ends(n: usize, k: usize, starts: bool) -> (usize, usize) {
    match (n, starts) {
        (1, _) => (0, 0),
        (_, true) => (k, (k + 1) % n),
        (_, false) => ((k + 1) % n, k),
    }
}

/// The tool's section by planar face `f` as the general boolean finds it: each wall's curve
/// with the plane, cut where it meets the uprights (the curve against the surface across each
/// upright), each corner settled on the curves through it, in the order that boolean meets them.
type Section = (Vec<(Curve, Interval, bool)>, Vec<(Point3, f64)>);

fn section(
    a: &Body,
    b: &Body,
    t: &Tool,
    f: FaceId,
    pl: &forge_geom::Plane,
    curves: Vec<Curve>,
) -> Option<Section> {
    let n = t.walls.len();
    let a_box = a.face_box(f);
    // per wall: the curve, the trim's range for it, and its hits on the start and end sides
    let mut found: Vec<(Curve, [(f64, Point3); 2])> = Vec::with_capacity(n);
    // per corner: the first cut found there
    let mut first: Vec<Option<Point3>> = vec![None; n];
    for (k, (w, curve)) in t.walls.iter().zip(&curves).enumerate() {
        let range = match curve.domain().period {
            Some(p) => Interval::new(curve.domain().lo, curve.domain().lo + p),
            None => {
                let Curve::Line(line) = curve else { return None };
                line_box(line, &a_box.hull(&b.face_box(w.face)).grow(1.0))?
            }
        };
        let mut at = [(0.0, Point3::ORIGIN); 2];
        // the loop meets its end side first, then its start side (once, for a seam)
        for (i, side) in [(1, w.sides[1]), (0, w.sides[0])] {
            if i == 0 && n == 1 {
                at[0] = at[1];
                break;
            }
            let Curve::Line(l) = &b.edge(side).curve else {
                return None;
            };
            let hit = if n == 1 {
                // where the seam crosses the plane, in closed form: settled below, it lands
                // where the general boolean's cut against the plane across the seam lands
                let p = l.eval(-pl.signed_distance(l.origin) / pl.normal_vec().dot(l.dir.get()));
                (curve.closest(p, range), p)
            } else {
                // the curve against the surface across the upright, as the general boolean cuts
                let across = across(b, w.face, w.side_uses[i]).ok()?;
                let CurveSurface::Hits(hs) = curve_surface(curve, range, &across) else {
                    return None;
                };
                let h = hs.into_iter().find(|h| on_edge(b, side, h.point).is_some())?;
                if h.contact != Contact::Crossing {
                    return None;
                }
                (h.t, h.point)
            };
            if on_edge(b, side, hit.1).is_none() || !a_box.contains(hit.1) {
                return None;
            }
            at[i] = hit;
            first[(k + i) % n].get_or_insert(hit.1);
        }
        found.push((curve.clone(), at));
    }
    // each curve's stretch over its wall: from the side met at the lower parameter
    let mut segs = Vec::with_capacity(n);
    for (k, (curve, at)) in found.iter().enumerate() {
        let w = &t.walls[k];
        let span = match curve.domain().period {
            Some(p) if n == 1 => (Interval::new(at[0].0, at[0].0 + p), true),
            Some(p) => {
                let (lo, hi) = (at[0].0.min(at[1].0), at[0].0.max(at[1].0));
                // the stretch whose middle lies over the wall's bottom edge
                let bottom = b.edge(w.bottom);
                let drop = t.z.get() * ((pl.frame.origin - t.origin).dot(t.z.get()) - t.h[0]);
                let over = |s: Interval| {
                    let q = curve.eval(s.mid()) - drop;
                    bottom
                        .curve
                        .eval(bottom.curve.closest(q, bottom.range))
                        .distance(q)
                        <= 1e-6
                };
                let inner = Interval::new(lo, hi);
                let span = if over(inner) {
                    inner
                } else {
                    Interval::new(hi, lo + p)
                };
                (
                    span,
                    at[0].0 == span.lo || (at[0].0 + p == span.hi && at[0].0 != span.lo),
                )
            }
            None => {
                let starts = at[0].0 <= at[1].0;
                (Interval::new(at[0].0.min(at[1].0), at[0].0.max(at[1].0)), starts)
            }
        };
        segs.push((curve.clone(), span.0, span.1));
    }
    // corners settled on the uprights (once per cut there) and on the curves ending there
    let mut corners = Vec::with_capacity(n);
    for (c, p0) in first.iter().enumerate() {
        let side = b.edge(t.walls[c].sides[0]);
        let cuts = if n == 1 { 1 } else { 2 };
        // at most two cuts and two curve ends
        let mut through = [(&side.curve, side.range); 4];
        let mut at = cuts;
        // the curves' ends here, in wall order
        for (k, (curve, r, starts)) in segs.iter().enumerate() {
            let (c0, c1) = ends(n, k, *starts);
            let w = 1e-3 * r.width();
            for (corner, t0) in [(c0, r.lo), (c1, r.hi)] {
                if corner == c {
                    through[at] = (curve, Interval::new(t0 - w, t0 + w));
                    at += 1;
                }
            }
        }
        corners.push(settle((*p0)?, &through[..at]).ok()?);
    }
    // each curve's range from its settled ends
    for (k, s) in segs.iter_mut().enumerate() {
        let w = 1e-3 * s.1.width();
        let (c0, c1) = ends(n, k, s.2);
        let lo = s.0.closest(corners[c0].0, Interval::new(s.1.lo - w, s.1.lo + w));
        let hi = s.0.closest(corners[c1].0, Interval::new(s.1.hi - w, s.1.hi + w));
        s.1 = Interval::new(lo, hi);
    }
    Some((segs, corners))
}

/// The vertices and edges `build` has made so far, made on first use as the general boolean
/// makes them: B's by their ids, new vertices by number.
struct Made<'x> {
    b: &'x Body,
    /// B's vertices, then B's edges, then the new vertices.
    ids: Vec<Option<u32>>,
}

impl Made<'_> {
    fn b_vertex(&mut self, a: &mut Body, v: VertexId) -> VertexId {
        let x = self.b.vertex(v);
        VertexId(*self.ids[v.index()].get_or_insert_with(|| a.add_tolerant_vertex(x.point, x.tol, x.name).0))
    }
    /// A whole edge of B.
    fn b_edge(&mut self, a: &mut Body, e: EdgeId) -> EdgeId {
        let k = self.b.vertices.len() + e.index();
        if let Some(x) = self.ids[k] {
            return EdgeId(x);
        }
        let edge = self.b.edge(e);
        let (v0, v1) = (self.b_vertex(a, edge.start), self.b_vertex(a, edge.end));
        let x = a.add_edge(edge.curve.clone(), edge.range, v0, v1, edge.name);
        self.ids[k] = Some(x.0);
        x
    }
    fn new_vertex(&mut self, a: &mut Body, i: usize, at: (Point3, f64), name: Name) -> VertexId {
        let k = self.b.vertices.len() + self.b.edges.len() + i;
        VertexId(*self.ids[k].get_or_insert_with(|| a.add_tolerant_vertex(at.0, at.1, name).0))
    }
}

fn build(a: &mut Body, b: &Body, t: &Tool, hits: &[Hit], wall_at: &[bool], flip: bool) {
    let root = Name::feature(0);
    static CUT: std::sync::OnceLock<Name> = std::sync::OnceLock::new();
    let cut = *CUT.get_or_init(|| root.with_str("cut"));
    let nw = t.walls.len();
    let nh = hits.len();
    let z = t.z.get();
    let ends = |k: usize, starts: bool| ends(nw, k, starts);
    // named as the general boolean names a curve's piece: by the faces, then the edges it ends on
    let side_name = |c: usize| b.edge(t.walls[c].sides[0]).name;
    // hit i's wall-k curve is name i * nw + k
    let mut names: Vec<Name> = Vec::with_capacity(nh * nw);
    for h in hits {
        for (k, w) in t.walls.iter().enumerate() {
            let (c0, c1) = ends(k, h.segs[k].2);
            names.push(
                cut.with_set(vec![a.face(h.f).name, b.face(w.face).name])
                    .with(0)
                    .with_name(side_name(c0))
                    .with_name(side_name(c1))
                    .with(0),
            );
        }
    }
    // new vertices: hit i's corner c is point i * nw + c
    let points: Vec<Point3> = hits.iter().flat_map(|h| h.corners.iter().map(|c| c.0)).collect();
    let mut marks = Vec::with_capacity(2 * nw * nh);
    for (i, h) in hits.iter().enumerate() {
        for (k, name) in names[i * nw..(i + 1) * nw].iter().enumerate() {
            let (c0, c1) = ends(k, h.segs[k].2);
            marks.push(((i * nw + c0) as u32, *name));
            marks.push(((i * nw + c1) as u32, Name(name.0 ^ 1)));
        }
    }
    let vnames = vertex_names(&points, marks, root);
    let shell = ShellId(0);
    let mut made = Made {
        b,
        ids: vec![None; b.vertices.len() + b.edges.len() + points.len()],
    };
    let corner = |i: usize| hits[i / nw].corners[i % nw];
    // each hit face gains a hole, in face order, walked as the general boolean walks it
    let mut order: Vec<usize> = (0..nh).collect();
    order.sort_by_key(|&i| hits[i].f);
    let mut seg_edge = vec![None; nh * nw];
    for &i in &order {
        let h = &hits[i];
        let mut cycle = vec![(0, true)];
        let mut at = ends(0, h.segs[0].2).1;
        while cycle.len() < nw {
            let k = (0..nw)
                .find(|k| {
                    !cycle.iter().any(|c| c.0 == *k)
                        && (ends(*k, h.segs[*k].2).0 == at || ends(*k, h.segs[*k].2).1 == at)
                })
                .expect("a closed section");
            let (c0, c1) = ends(k, h.segs[k].2);
            cycle.push((k, c0 == at));
            at = if c0 == at { c1 } else { c0 };
        }
        let ccw = match &h.segs[0].0 {
            Curve::Circle(c) if nw == 1 => c.frame.z().dot(z) > 0.0,
            _ => ends(0, h.segs[0].2).0 == 0,
        };
        let hole: Vec<(usize, bool)> = if ccw == h.up {
            cycle.iter().rev().map(|&(k, d)| (k, !d)).collect()
        } else {
            cycle
        };
        let mut uses = Vec::with_capacity(nw);
        for (k, d) in hole {
            let (c0, c1) = ends(k, h.segs[k].2);
            let v0 = made.new_vertex(a, i * nw + c0, corner(i * nw + c0), vnames[i * nw + c0]);
            let v1 = made.new_vertex(a, i * nw + c1, corner(i * nw + c1), vnames[i * nw + c1]);
            let e = a.add_edge(h.segs[k].0.clone(), h.segs[k].1, v0, v1, names[i * nw + k]);
            seg_edge[i * nw + k] = Some(e);
            uses.push((e, d));
        }
        let lp = a.add_loop(h.f, &uses);
        extend_query(a, h.f, lp);
    }

    let flipped = |uses: &mut Vec<(EdgeId, bool)>| {
        if flip {
            uses.reverse();
            for u in uses.iter_mut() {
                u.1 = !u.1;
            }
        }
    };
    let walls = wall_at.iter().filter(|&&w| w).count();
    // the uprights' pieces between events (the caps, then each hit), made on first use
    let mut side_piece: Vec<Option<EdgeId>> = vec![None; (nh + 1) * nw];
    for f in b.face_ids() {
        let src = b.face(f);
        if let Some(k) = t.caps.iter().position(|&c| c == f) {
            if !wall_at[if k == 0 { 0 } else { nh }] {
                continue;
            }
            let face = a.add_face(shell, src.surface.clone(), src.same_sense != flip, src.name);
            let mut uses: Vec<(EdgeId, bool)> = b
                .lp(src.loops[0])
                .coedges
                .iter()
                .map(|&c| (made.b_edge(a, b.coedge(c).edge), b.coedge(c).forward))
                .collect();
            flipped(&mut uses);
            a.add_loop(face, &uses);
            continue;
        }
        let Some(k) = t.walls.iter().position(|w| w.face == f) else {
            continue;
        };
        let w = &t.walls[k];
        for i in 0..=nh {
            if !wall_at[i] {
                continue;
            }
            // the curve along the bottom (i) or top (i + 1) of this stretch, and which way it runs
            // from the wall's start side
            let along = |i: usize| -> (Option<(usize, usize)>, bool) {
                if i == 0 || i == nh + 1 {
                    return (None, w.fwd);
                }
                let s = &hits[i - 1].segs[k];
                let d = match (&s.0, nw) {
                    (Curve::Circle(c), 1) => (c.frame.z().dot(z) > 0.0) == w.fwd,
                    _ => ends(k, s.2).0 == k,
                };
                (Some((i - 1, k)), d)
            };
            let (lo, d_lo) = along(i);
            let (hi, d_hi) = along(i + 1);
            let mut piece = |a: &mut Body, made: &mut Made, j: usize| -> EdgeId {
                let c = (k + j) % nw;
                if let Some(e) = side_piece[c * (nh + 1) + i] {
                    return e;
                }
                let side = b.edge(t.walls[c].sides[0]);
                let mut end = |a: &mut Body, i: usize| -> (VertexId, f64, Name) {
                    if i == 0 || i == nh + 1 {
                        let v = if i == 0 { side.start } else { side.end };
                        let s = if i == 0 { side.range.lo } else { side.range.hi };
                        (made.b_vertex(a, v), s, b.vertex(v).name)
                    } else {
                        let p = (i - 1) * nw + c;
                        let id = made.new_vertex(a, p, corner(p), vnames[p]);
                        (id, side.curve.closest(points[p], side.range), vnames[p])
                    }
                };
                let (v0, s0, n0) = end(a, i);
                let (v1, s1, n1) = end(a, i + 1);
                let name = if walls == 1 {
                    side.name
                } else {
                    side.name.piece(n0.with_name(n1))
                };
                let e = a.add_edge(side.curve.clone(), Interval::new(s0, s1), v0, v1, name);
                side_piece[c * (nh + 1) + i] = Some(e);
                e
            };
            // the general boolean's walk round this stretch, from the first of its half-edges in
            // B's loop order: the bottom edge, else the end side going up
            let mut walk: Vec<(EdgeId, bool)> = Vec::with_capacity(4);
            for j in 0..4 {
                let j = if i > 0 { (j + 1) % 4 } else { j };
                walk.push(match j {
                    0 => match lo {
                        None => (made.b_edge(a, w.bottom), d_lo),
                        Some((h, k)) => (seg_edge[h * nw + k].expect("made with the hole"), d_lo),
                    },
                    1 => (piece(a, &mut made, 1), true),
                    2 => match hi {
                        None => (made.b_edge(a, w.top), !d_hi),
                        Some((h, k)) => (seg_edge[h * nw + k].expect("made with the hole"), !d_hi),
                    },
                    _ => (piece(a, &mut made, 0), false),
                });
            }
            let kept = (0..=nh).filter(|&x| wall_at[x]).count();
            let name = if kept == 1 {
                src.name
            } else {
                let least = walk
                    .iter()
                    .map(|&(e, d)| (a.edge(e).name, d))
                    .min()
                    .expect("four uses");
                src.name.piece(least.0.with(least.1 as u64))
            };
            flipped(&mut walk);
            let face = a.add_face(shell, src.surface.clone(), src.same_sense != flip, name);
            a.add_loop(face, &walk);
        }
    }
}

/// Where two bodies differ, for the check mode (None: the same). Entities are matched by name,
/// not by id or order: each face's surface and sense, its loops as cycles of (edge name, way),
/// each edge's curve kind and ends, each vertex's point, points to within 1e-9.
pub(crate) fn diff(x: &Body, y: &Body) -> Option<String> {
    let close = |p: Point3, q: Point3| p.distance(q) <= 1e-9 * (1.0 + p.distance(Point3::ORIGIN));
    type Loop = Vec<(Name, bool)>;
    let faces = |b: &Body| {
        let mut out: Vec<(Name, bool, String, Vec<Loop>)> = b
            .face_ids()
            .map(|f| {
                let face = b.face(f);
                let mut loops: Vec<Loop> = face
                    .loops
                    .iter()
                    .map(|&l| {
                        let mut c: Loop = b
                            .lp(l)
                            .coedges
                            .iter()
                            .map(|&c| (b.edge(b.coedge(c).edge).name, b.coedge(c).forward))
                            .collect();
                        let k = (0..c.len()).min_by_key(|&k| c[k]).unwrap_or(0);
                        c.rotate_left(k);
                        c
                    })
                    .collect();
                loops.sort();
                (face.name, face.same_sense, format!("{:?}", face.surface), loops)
            })
            .collect();
        out.sort_by(|p, q| (p.0, &p.3).cmp(&(q.0, &q.3)));
        out
    };
    let (fx, fy) = (faces(x), faces(y));
    if fx.len() != fy.len() {
        return Some(format!("{} faces vs {}", fx.len(), fy.len()));
    }
    for (p, q) in fx.iter().zip(&fy) {
        if p != q {
            return Some(format!("face {:?}: {:?} vs {:?}", p.0, p, q));
        }
    }
    // every edge the faces use, by name: its kind and its ends
    let edges = |b: &Body| {
        let mut out: Vec<(
            Name,
            Point3,
            Point3,
            Name,
            Name,
            std::mem::Discriminant<forge_geom::Curve>,
        )> = b
            .edge_ids()
            .map(|e| {
                let edge = b.edge(e);
                let (s, t) = (b.vertex(edge.start), b.vertex(edge.end));
                (
                    edge.name,
                    s.point,
                    t.point,
                    s.name,
                    t.name,
                    std::mem::discriminant(&edge.curve),
                )
            })
            .collect();
        out.sort_by_key(|e| (e.0, e.3, e.4));
        out
    };
    let (ex, ey) = (edges(x), edges(y));
    if ex.len() != ey.len() {
        return Some(format!("{} edges vs {}", ex.len(), ey.len()));
    }
    for (p, q) in ex.iter().zip(&ey) {
        if (p.0, p.3, p.4, p.5) != (q.0, q.3, q.4, q.5) || !close(p.1, q.1) || !close(p.2, q.2) {
            return Some(format!("edge {:?}: {:?} vs {:?}", p.0, p, q));
        }
    }
    None
}

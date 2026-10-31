//! The local-imprint boolean: the part changes only where the tool meets it, each decision made
//! in closed form, with no face split by tracing regions and no point-in-face test per region.
//! It runs beside the general boolean (`boolean.rs`), never instead of it: a case it does not
//! cover returns false before touching the part, and the general boolean runs.
//!
//! Covered so far: a round tool (an extruded circle) whose circle lies inside, and clear of the
//! boundary of, each planar face square to its axis that it passes through, and which no other
//! face of the part comes near (thru holes, pockets, bosses). Along the axis the part's material
//! inside the tool's cylinder is then a set of slabs between those faces, so the faces the tool
//! passes gain a hole, the wall stands where the material changes, and a cap closes a wall that
//! ends inside the tool.

use crate::boolean::{Op, settle, vertex_names};
use crate::classify::{circle_in_face, extend_query};
use forge_geom::intersect::{SurfaceSurface, plane_cylinder};
use forge_geom::{Curve, CurveGeom, Cylinder, Surface};
use forge_math::{Box3, Interval, Point3, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, ShellId, VertexId};

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

/// An extruded circle, as `forge_ops::extrude` builds it.
struct Tool {
    cyl: Cylinder,
    wall: FaceId,
    /// Bottom and top caps (the axis runs bottom to top), their circles, and the axis heights.
    caps: [FaceId; 2],
    circles: [EdgeId; 2],
    h: [f64; 2],
    seam: EdgeId,
}

fn tool(b: &Body) -> Option<Tool> {
    let mut wall = None;
    let mut planes = 0;
    for f in b.face_ids() {
        match b.face(f).surface {
            Surface::Cylinder(_) => wall = Some(f),
            Surface::Plane(_) => planes += 1,
            _ => return None,
        }
    }
    let wall = wall?;
    let face = b.face(wall);
    let Surface::Cylinder(cyl) = face.surface else {
        return None;
    };
    if planes != 2 || !face.same_sense || face.loops.len() != 1 {
        return None;
    }
    let uses: Vec<(EdgeId, bool)> = b
        .lp(face.loops[0])
        .coedges
        .iter()
        .map(|&c| (b.coedge(c).edge, b.coedge(c).forward))
        .collect();
    let [(e0, true), (seam, true), (e1, false), (s2, false)] = uses[..] else {
        return None;
    };
    if s2 != seam {
        return None;
    }
    let z = cyl.frame.z();
    let mut h = [0.0; 2];
    for (k, e) in [e0, e1].into_iter().enumerate() {
        let Curve::Circle(c) = &b.edge(e).curve else {
            return None;
        };
        if c.frame.z().dot(z.get()) < 1.0 - 1e-12 || c.radius != cyl.radius {
            return None;
        }
        h[k] = (c.frame.origin - cyl.frame.origin).dot(z.get());
    }
    let Curve::Line(l) = &b.edge(seam).curve else {
        return None;
    };
    if h[1] <= h[0] || l.dir.dot(z.get()) < 1.0 - 1e-12 {
        return None;
    }
    if b.edge(e0).start != b.edge(seam).start || b.edge(e1).start != b.edge(seam).end {
        return None;
    }
    let mut caps = [wall; 2];
    for f in b.face_ids().filter(|&f| f != wall) {
        let face = b.face(f);
        let [l] = face.loops[..] else { return None };
        let [c] = b.lp(l).coedges[..] else { return None };
        let k = [e0, e1].iter().position(|&e| e == b.coedge(c).edge)?;
        // square to the axis, facing out of the tool
        let Surface::Plane(pl) = &face.surface else {
            return None;
        };
        let out = pl.normal_vec().dot(z.get()) * if face.same_sense { 1.0 } else { -1.0 };
        if out.abs() < 1.0 - 1e-12 || (out > 0.0) != (k == 1) {
            return None;
        }
        caps[k] = f;
    }
    (caps[0] != wall && caps[1] != wall).then_some(Tool {
        cyl,
        wall,
        caps,
        circles: [e0, e1],
        h,
        seam,
    })
}

/// A planar face of A the tool's axis passes square through, its circle inside the face.
struct Hit {
    f: FaceId,
    h: f64,
    /// The face's outward normal points up the axis.
    up: bool,
    circle: forge_geom::Circle,
    /// Where the circle meets the tool's seam (and that vertex's tolerance), and its range.
    point: Point3,
    tol: f64,
    range: Interval,
}

/// Whether the cylinder (whole, both ways along its axis) stays clear of face `f`, judged by
/// the face's surface or its box.
fn clear_of(a: &Body, f: FaceId, t: &Tool, lohi: [f64; 2]) -> bool {
    let (fr, r) = (&t.cyl.frame, t.cyl.radius);
    let z = fr.z().get();
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
    // the face's box, in the tool's frame
    let b = a.face_box(f);
    let mut l = Box3::EMPTY;
    for x in [b.x.lo, b.x.hi] {
        for y in [b.y.lo, b.y.hi] {
            for w in [b.z.lo, b.z.hi] {
                l.include(fr.point_to_local(Point3::new(x, y, w)));
            }
        }
    }
    if l.z.hi < lohi[0] - CLEAR || l.z.lo > lohi[1] + CLEAR {
        return true;
    }
    let dx = l.x.lo.max(-l.x.hi).max(0.0);
    let dy = l.y.lo.max(-l.y.hi).max(0.0);
    dx * dx + dy * dy > (r + CLEAR) * (r + CLEAR)
}

/// The boolean of `a` with `b` done in place when `b` is a round tool this path covers; false,
/// with `a` untouched, otherwise.
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
    let t = tool(b).ok_or("not a round tool")?;
    let z = t.cyl.frame.z();
    // the tool's box: its two end disks'
    let bb = {
        let (o, r) = (t.cyl.frame.origin, t.cyl.radius);
        let reach = |a: f64| r * (1.0 - a * a).max(0.0).sqrt() + tol::LINEAR;
        let half = Point3::new(reach(z.get().x), reach(z.get().y), reach(z.get().z)) - Point3::ORIGIN;
        let mut bb = Box3::EMPTY;
        for h in t.h {
            let c = o + z.get() * h;
            bb.include(c - half);
            bb.include(c + half);
        }
        bb
    };
    let mut hits: Vec<Hit> = Vec::new();
    for f in a.faces_where(|x| x.overlaps(&bb)) {
        let face = a.face(f);
        let pl = match &face.surface {
            Surface::Plane(pl) if pl.normal_vec().dot(z.get()).abs() >= 1.0 - 1e-12 => pl,
            _ => {
                if clear_of(a, f, &t, t.h) {
                    continue;
                }
                return Err("a face not square to the axis comes near");
            }
        };
        let h = (pl.frame.origin - t.cyl.frame.origin).dot(z.get());
        if h < t.h[0] - CLEAR || h > t.h[1] + CLEAR {
            continue;
        }
        if h < t.h[0] + CLEAR || h > t.h[1] - CLEAR {
            return Err("a face on a cap's plane");
        }
        let SurfaceSurface::Curves(cs) = plane_cylinder(pl, &t.cyl) else {
            return Err("no circle");
        };
        let [(Curve::Circle(circle), _)] = &cs[..] else {
            return Err("no circle");
        };
        match circle_in_face(a, f, circle, CLEAR) {
            None => return Err("a face's boundary comes near the circle"),
            Some(true) => {}
            Some(false) => {
                // the general boolean cuts the tool's seam where the face's box holds the
                // crossing, even off the face: a vertex this path would not make
                let Curve::Line(seam) = &b.edge(t.seam).curve else {
                    return Err("seam");
                };
                let s = -pl.signed_distance(seam.origin) / pl.normal_vec().dot(seam.dir.get());
                if a.face_box(f).grow(CLEAR).contains(seam.eval(s)) {
                    return Err("the seam crosses a face's box off the face");
                }
                continue;
            }
        }
        let up = (pl.normal_vec().dot(z.get()) > 0.0) == face.same_sense;
        let (point, tol, range) = seam_point(b, &t, pl, &Curve::Circle(*circle)).ok_or("seam point")?;
        hits.push(Hit {
            f,
            h,
            up,
            circle: *circle,
            point,
            tol,
            range,
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

/// Where a hit's circle (on plane `pl`) meets the tool's seam, settled on both curves as the
/// general boolean settles its cut there, and the circle's range from that point round.
fn seam_point(b: &Body, t: &Tool, pl: &forge_geom::Plane, circle: &Curve) -> Option<(Point3, f64, Interval)> {
    let seam = b.edge(t.seam);
    let Curve::Line(line) = &seam.curve else {
        return None;
    };
    let at = line.eval(-pl.signed_distance(line.origin) / pl.normal_vec().dot(line.dir.get()));
    let period = circle.domain().period?;
    let t0 = circle.closest(at, Interval::new(0.0, period));
    let span = Interval::new(t0, t0 + period);
    let w = 1e-3 * span.width();
    let near = |s: f64| Interval::new(s - w, s + w);
    let curves = [
        (&seam.curve, seam.range),
        (circle, near(span.lo)),
        (circle, near(span.hi)),
    ];
    let (p, tol) = settle(at, &curves).ok()?;
    let range = Interval::new(circle.closest(p, near(span.lo)), circle.closest(p, near(span.hi)));
    Some((p, tol, range))
}

fn build(a: &mut Body, b: &Body, t: &Tool, hits: &[Hit], wall_at: &[bool], flip: bool) {
    let root = Name::feature(0);
    static CUT: std::sync::OnceLock<Name> = std::sync::OnceLock::new();
    let cut = *CUT.get_or_init(|| root.with_str("cut"));
    let wall_name = b.face(t.wall).name;
    let circles: Vec<Curve> = hits.iter().map(|h| Curve::Circle(h.circle)).collect();
    // named as the general boolean names a curve's piece: by the faces, then the edges it ends on
    let seam_name = b.edge(t.seam).name;
    let names: Vec<Name> = hits
        .iter()
        .map(|h| {
            cut.with_set(vec![a.face(h.f).name, wall_name])
                .with(0)
                .with_name(seam_name)
                .with_name(seam_name)
                .with(0)
        })
        .collect();
    let points: Vec<Point3> = hits.iter().map(|h| h.point).collect();
    let marks = names
        .iter()
        .enumerate()
        .flat_map(|(i, n)| [(i as u32, *n), (i as u32, Name(n.0 ^ 1))])
        .collect();
    let vnames = vertex_names(&points, marks, root);
    let shell = ShellId(0);
    let z = t.cyl.frame.z().get();
    let ccw = |c: &Curve| match c {
        Curve::Circle(k) => k.frame.z().dot(z) > 0.0,
        _ => unreachable!(),
    };

    // each hit face gains a hole, in face order
    let mut order: Vec<usize> = (0..hits.len()).collect();
    order.sort_by_key(|&i| hits[i].f);
    let mut edge_of = vec![EdgeId(0); hits.len()];
    let mut vertex_of = vec![VertexId(0); hits.len()];
    for &i in &order {
        let v = a.add_tolerant_vertex(points[i], hits[i].tol, vnames[i]);
        let e = a.add_edge(circles[i].clone(), hits[i].range, v, v, names[i]);
        let f = hits[i].f;
        let sense = a.face(f).same_sense;
        let fwd = match &a.face(f).surface {
            Surface::Plane(pl) => (pl.normal_vec().dot(hits[i].circle.frame.z().get()) > 0.0) != sense,
            _ => unreachable!(),
        };
        let lp = a.add_loop(f, &[(e, fwd)]);
        extend_query(a, f, lp);
        edge_of[i] = e;
        vertex_of[i] = v;
    }

    // B's entities a kept piece uses, made on first use
    let mut b_vertex: Vec<Option<VertexId>> = vec![None; b.vertices.len()];
    let mut vertex = |a: &mut Body, v: VertexId| {
        *b_vertex[v.index()].get_or_insert_with(|| {
            let x = b.vertex(v);
            a.add_tolerant_vertex(x.point, x.tol, x.name)
        })
    };
    let n = hits.len();
    let mut cap_edge = [None; 2];
    for f in b.face_ids() {
        let Some(k) = t.caps.iter().position(|&c| c == f) else {
            continue;
        };
        if !wall_at[if k == 0 { 0 } else { n }] {
            continue;
        }
        let src = b.face(f);
        let face = a.add_face(shell, src.surface.clone(), src.same_sense != flip, src.name);
        if !flip && let Some(bx) = src.bbox.get() {
            a.faces[face.index()].bbox.set(*bx);
        }
        let e = b.edge(t.circles[k]);
        let v = vertex(a, e.start);
        let ne = a.add_edge(e.curve.clone(), e.range, v, v, e.name);
        let fwd = b.coedge(b.lp(src.loops[0]).coedges[0]).forward;
        a.add_loop(face, &[(ne, fwd != flip)]);
        cap_edge[k] = Some(ne);
    }

    // the wall, one face per stretch where the material changes
    let seam = b.edge(t.seam);
    let walls = wall_at.iter().filter(|&&w| w).count();
    let vname = |k: usize| match k {
        0 => b.vertex(seam.start).name,
        k if k == n + 1 => b.vertex(seam.end).name,
        k => vnames[k - 1],
    };
    // seam parameters at each event: the caps, then each hit
    let s_at = |k: usize| match k {
        0 => seam.range.lo,
        k if k == n + 1 => seam.range.hi,
        k => seam.curve.closest(points[k - 1], seam.range),
    };
    for k in 0..=n {
        if !wall_at[k] {
            continue;
        }
        let (bottom, top) = (
            if k == 0 {
                cap_edge[0].expect("cap made")
            } else {
                edge_of[k - 1]
            },
            if k == n {
                cap_edge[1].expect("cap made")
            } else {
                edge_of[k]
            },
        );
        let curve_of = |i: usize| match i {
            0 => &b.edge(t.circles[0]).curve,
            i if i == n + 1 => &b.edge(t.circles[1]).curve,
            i => &circles[i - 1],
        };
        let (cb, ct) = (ccw(curve_of(k)), ccw(curve_of(k + 1)));
        let start = if k == 0 {
            vertex(a, seam.start)
        } else {
            vertex_of[k - 1]
        };
        let end = if k == n { vertex(a, seam.end) } else { vertex_of[k] };
        let piece_name = if walls == 1 {
            seam.name
        } else {
            seam.name.piece(vname(k).with_name(vname(k + 1)))
        };
        let sp = a.add_edge(
            seam.curve.clone(),
            Interval::new(s_at(k), s_at(k + 1)),
            start,
            end,
            piece_name,
        );
        // the general boolean's walk round this stretch of B's wall, from the first of its
        // half-edges in B's loop order: the bottom circle, else the seam going up
        let mut uses = [(bottom, cb), (sp, true), (top, !ct), (sp, false)];
        if k > 0 {
            uses.rotate_left(1);
        }
        if flip {
            uses.reverse();
            for u in &mut uses {
                u.1 = !u.1;
            }
        }
        let name = if walls == 1 {
            wall_name
        } else {
            let least = uses
                .iter()
                .map(|&(e, fwd)| (a.edge(e).name, fwd))
                .min()
                .expect("four uses");
            wall_name.piece(least.0.with(least.1 as u64))
        };
        let src = b.face(t.wall);
        let face = a.add_face(shell, src.surface.clone(), src.same_sense != flip, name);
        a.add_loop(face, &uses);
    }
}

/// Where two bodies differ, entity by entity, for the check mode (None: they are equal).
pub(crate) fn diff(x: &Body, y: &Body) -> Option<String> {
    if x == y {
        return None;
    }
    let first = |n: usize, same: &dyn Fn(usize) -> bool| (0..n).find(|&i| !same(i));
    let counts = |b: &Body| {
        (
            b.vertices.len(),
            b.edges.len(),
            b.coedges.len(),
            b.loops.len(),
            b.faces.len(),
        )
    };
    if counts(x) != counts(y) {
        return Some(format!("counts v/e/c/l/f {:?} vs {:?}", counts(x), counts(y)));
    }
    if let Some(i) = first(x.vertices.len(), &|i| x.vertices[i] == y.vertices[i]) {
        return Some(format!("vertex {i}: {:?} vs {:?}", x.vertices[i], y.vertices[i]));
    }
    if let Some(i) = first(x.edges.len(), &|i| x.edges[i] == y.edges[i]) {
        return Some(format!("edge {i}: {:?} vs {:?}", x.edges[i], y.edges[i]));
    }
    if let Some(i) = first(x.coedges.len(), &|i| x.coedges[i] == y.coedges[i]) {
        return Some(format!("coedge {i}: {:?} vs {:?}", x.coedges[i], y.coedges[i]));
    }
    if let Some(i) = first(x.loops.len(), &|i| x.loops[i] == y.loops[i]) {
        return Some(format!("loop {i}: {:?} vs {:?}", x.loops[i], y.loops[i]));
    }
    Some(match first(x.faces.len(), &|i| x.faces[i] == y.faces[i]) {
        Some(i) => format!("face {i}: {:?} vs {:?}", x.faces[i], y.faces[i]),
        None => "shells".into(),
    })
}

//! From a file's topology to what the kernel's bodies require. A file may leave a closed face's
//! seam out (a cylinder with two rims and no edge between), close a spline on itself, list a
//! plane's hole first. Each is put right here without moving geometry; what cannot be is refused.

use forge_geom::curve::{CurveDeriv, Domain1, ProcCurve, ProceduralCurve};
use forge_geom::{Circle, Curve, CurveGeom, Line, Surface, SurfaceGeom};
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec3, dmath, tol};
use forge_topo::{Body, EdgeId, Name, ShellId, VertexId};
use std::sync::Arc;

pub struct PEdge {
    pub curve: Curve,
    pub range: Interval,
    pub start: usize,
    pub end: usize,
    pub name: Name,
}

pub struct PFace {
    pub surface: Surface,
    pub same_sense: bool,
    pub name: Name,
    pub shell: usize,
    /// Each loop's uses of edges: (edge, walks it forward). The face on the loop's left.
    pub loops: Vec<Vec<(usize, bool)>>,
}

/// A body before it is one: everything by index, freely edited.
#[derive(Default)]
pub struct Proto {
    pub verts: Vec<(Point3, Name)>,
    /// Radii of balls vertices were left with, by vertex; missing or 0 for the usual.
    pub vtol: Vec<f64>,
    /// Edges that stayed off a surface: the two surfaces touch along them, so no exact curve can
    /// be found.
    pub tangent: usize,
    pub edges: Vec<PEdge>,
    pub faces: Vec<PFace>,
    pub shells: usize,
    /// Names made here: unique by a running count under a salt.
    pub salt: Name,
    pub made: u64,
}

impl Proto {
    fn fresh(&mut self) -> Name {
        self.made += 1;
        self.salt.with_str("heal").with(self.made)
    }

    pub fn start_of(&self, u: (usize, bool)) -> usize {
        if u.1 {
            self.edges[u.0].start
        } else {
            self.edges[u.0].end
        }
    }

    /// Splits edge `e` at parameter `t`, in every loop that uses it; returns the new vertex.
    fn split_edge(&mut self, e: usize, t: f64) -> usize {
        let p = self.edges[e].curve.eval(t);
        let name = self.fresh();
        self.verts.push((p, name));
        let v = self.verts.len() - 1;
        self.split_edge_at(e, t, v);
        v
    }

    fn split_edge_at(&mut self, e: usize, t: f64, v: usize) {
        let name = self.fresh();
        let (curve, range, end) = (
            self.edges[e].curve.clone(),
            self.edges[e].range,
            self.edges[e].end,
        );
        self.edges.push(PEdge {
            curve,
            range: Interval::new(t, range.hi),
            start: v,
            end,
            name,
        });
        let e2 = self.edges.len() - 1;
        self.edges[e].range = Interval::new(range.lo, t);
        self.edges[e].end = v;
        for f in &mut self.faces {
            for l in &mut f.loops {
                let mut i = 0;
                while i < l.len() {
                    if l[i].0 == e {
                        if l[i].1 {
                            l.insert(i + 1, (e2, true));
                        } else {
                            l[i] = (e2, false);
                            l.insert(i + 1, (e, false));
                        }
                        i += 1;
                    }
                    i += 1;
                }
            }
        }
    }

    /// A closed edge whose curve has no period (a spline ending where it starts) cannot be one
    /// edge in a body: it becomes two.
    pub fn split_closed_splines(&mut self) {
        for e in 0..self.edges.len() {
            let ed = &self.edges[e];
            if ed.start == ed.end && ed.curve.domain().period.is_none() {
                let t = ed.range.mid();
                self.split_edge(e, t);
            }
        }
    }

    /// Where an edge's samples lie on `s`, unwrapped from `from`.
    fn walk(&self, u: (usize, bool), s: &Surface, n: usize, out: &mut Vec<Point2>) {
        let e = &self.edges[u.0];
        let d = s.domain();
        for i in 0..=n {
            let k = if u.1 { i } else { n - i };
            let t = e.range.lo + e.range.width() * k as f64 / n as f64;
            let p = e.curve.eval(t);
            let uv = s.project(p);
            let uv = match out.last() {
                Some(&near) => unwrap(uv, near, d.u, d.v),
                None => uv,
            };
            out.push(uv);
        }
    }

    /// How a loop winds round a periodic surface: whole periods in u and in v.
    fn winding(&self, l: &[(usize, bool)], s: &Surface) -> (i64, i64) {
        let d = s.domain();
        let mut pts = Vec::new();
        for &u in l {
            self.walk(u, s, 16, &mut pts);
        }
        // through a pole the longitude jumps: such a loop bounds a lune, it does not wind
        if pts.iter().any(|&q| s.singular(q)) {
            return (0, 0);
        }
        let (Some(&a), Some(&b)) = (pts.first(), pts.last()) else {
            return (0, 0);
        };
        let w = |x: f64, p: Option<f64>| p.map_or(0, |p| (x / p).round() as i64);
        (w(b.x - a.x, d.u.period), w(b.y - a.y, d.v.period))
    }

    /// The parameter on loop `l` where u (or v, `dir` = 1) is `at` modulo the period: which use,
    /// and the curve parameter.
    fn find_on_loop(&self, l: &[(usize, bool)], s: &Surface, dir: usize, at: f64) -> Option<(usize, f64)> {
        let d = s.domain();
        let period = if dir == 0 { d.u.period } else { d.v.period }?;
        let val = |uv: Point2| if dir == 0 { uv.x } else { uv.y };
        let phase = |x: f64| {
            let r = (x - at) / period;
            (r - r.round()) * period
        };
        for (i, &u) in l.iter().enumerate() {
            let e = &self.edges[u.0];
            let at_t = |t: f64| val(s.project(e.curve.eval(t)));
            const N: usize = 32;
            for k in 0..N {
                let (t0, t1) = (
                    e.range.lo + e.range.width() * k as f64 / N as f64,
                    e.range.lo + e.range.width() * (k + 1) as f64 / N as f64,
                );
                let (a, b) = (phase(at_t(t0)), phase(at_t(t1)));
                // a sign change that is no wrap of the phase
                if (a <= 0.0) != (b <= 0.0) && (a - b).abs() < 0.5 * period {
                    let (mut lo, mut hi, mut fa) = (t0, t1, a);
                    for _ in 0..60 {
                        let m = 0.5 * (lo + hi);
                        let fm = phase(at_t(m));
                        if (fm <= 0.0) == (fa <= 0.0) {
                            lo = m;
                            fa = fm;
                        } else {
                            hi = m;
                        }
                    }
                    return Some((i, 0.5 * (lo + hi)));
                }
            }
        }
        None
    }

    /// Makes the loop start at vertex `v`, which it passes through.
    fn start_at(&self, l: &mut [(usize, bool)], v: usize) -> bool {
        match (0..l.len()).find(|&i| self.start_of(l[i]) == v) {
            Some(i) => {
                l.rotate_left(i);
                true
            }
            None => false,
        }
    }

    fn pick_vertex_on(&mut self, face: usize, li: usize, dir: usize, at: f64) -> Option<usize> {
        let s = self.faces[face].surface.clone();
        let l = self.faces[face].loops[li].clone();
        // an existing vertex at that parameter
        let period = {
            let d = s.domain();
            if dir == 0 { d.u.period } else { d.v.period }
        }?;
        for &u in &l {
            let v = self.start_of(u);
            let uv = s.project(self.verts[v].0);
            let x = if dir == 0 { uv.x } else { uv.y };
            let r = (x - at) / period;
            if ((r - r.round()) * period).abs() <= 1e-9 {
                return Some(v);
            }
        }
        let (i, t) = self.find_on_loop(&l, &s, dir, at)?;
        let e = l[i].0;
        if self.edges[e].start == self.edges[e].end {
            // a closed edge has its one vertex moved here
            let range = self.edges[e].range;
            let p = self.edges[e].curve.eval(t);
            let name = self.fresh();
            self.verts.push((p, name));
            let v = self.verts.len() - 1;
            let w = range.width();
            self.edges[e].range = Interval::new(t, t + w);
            self.edges[e].start = v;
            self.edges[e].end = v;
            return Some(v);
        }
        Some(self.split_edge(e, t))
    }

    /// The edge from `a` to `b` along an iso line of the surface.
    fn seam_edge(&mut self, s: &Surface, dir: usize, a: usize, b: usize) -> Option<usize> {
        let (pa, pb) = (self.verts[a].0, self.verts[b].0);
        let name = self.fresh();
        let (curve, range) = iso_curve(s, dir, pa, pb)?;
        self.edges.push(PEdge {
            curve,
            range,
            start: a,
            end: b,
            name,
        });
        Some(self.edges.len() - 1)
    }

    /// Seams for closed faces that have none. A face whose loops wind round its surface becomes
    /// one loop: both rims joined by a seam used both ways, or a rim and a seam to the pole.
    pub fn add_seams(&mut self) -> Result<(), String> {
        for f in 0..self.faces.len() {
            let s = self.faces[f].surface.clone();
            let d = s.domain();
            if d.u.period.is_none() && d.v.period.is_none() {
                continue;
            }
            let s = self.reframe_sphere(f, s);
            let windings: Vec<(i64, i64)> = self.faces[f].loops.iter().map(|l| self.winding(l, &s)).collect();
            let winding: Vec<usize> = (0..windings.len()).filter(|&i| windings[i] != (0, 0)).collect();
            match winding.len() {
                0 => {
                    if self.faces[f].loops.is_empty() {
                        self.whole_face(f)?;
                    }
                }
                1 => self.cap(f, winding[0], windings[winding[0]])?,
                2 => self.join(f, winding[0], winding[1], windings[winding[0]])?,
                n => return Err(format!("{n} loops wind round the face's surface")),
            }
        }
        Ok(())
    }

    /// A sphere's face holding a pole with no loop round it (a cap about another axis, seen from
    /// its big side) draws wrongly in parameters: the sphere gets the axis through its first hole.
    fn reframe_sphere(&mut self, f: usize, s: Surface) -> Surface {
        let Surface::Sphere(k) = &s else {
            return s;
        };
        let face = &self.faces[f];
        if face.loops.is_empty() {
            return s;
        }
        let same = face.same_sense;
        let mut any_outer = false;
        let mut centre = Vec3::ZERO;
        for (i, l) in face.loops.iter().enumerate() {
            if self.winding(l, &s) != (0, 0) {
                return s;
            }
            let mut pts = Vec::new();
            for &u in l {
                self.walk(u, &s, 16, &mut pts);
            }
            // through a pole it bounds a lune, as bodies draw them
            if pts.iter().any(|&q| s.singular(q)) {
                return s;
            }
            let mut area = 0.0;
            for j in 0..pts.len() {
                let (a, b) = (pts[j], pts[(j + 1) % pts.len()]);
                area += a.x * b.y - b.x * a.y;
            }
            any_outer |= (area > 0.0) == same;
            if i == 0 {
                let w = l.len().max(1) as f64;
                for &u in l {
                    let e = &self.edges[u.0];
                    let mid = e.curve.eval(e.range.mid()) - k.frame.origin;
                    centre += mid / w;
                }
            }
        }
        if any_outer {
            return s;
        }
        let Some(z) = centre.normalize() else {
            return s;
        };
        let n = Surface::Sphere(forge_geom::Sphere {
            frame: Frame::from_z(k.frame.origin, z),
            radius: k.radius,
        });
        self.faces[f].surface = n.clone();
        n
    }

    /// Two rims joined.
    fn join(&mut self, f: usize, a: usize, b: usize, w: (i64, i64)) -> Result<(), String> {
        // the seam runs across the direction the rims wind in
        let dir = usize::from(w.0 == 0);
        let s = self.faces[f].surface.clone();
        let va = self.start_of(self.faces[f].loops[a][0]);
        let uva = s.project(self.verts[va].0);
        let at = if dir == 0 { uva.x } else { uva.y };
        let Some(vb) = self.pick_vertex_on(f, b, dir, at) else {
            return Err("no place on the second rim for a seam".into());
        };
        // the rim may have been edited: look the loops up afresh
        let seam = self
            .seam_edge(&s, dir, va, vb)
            .ok_or("the surface has no seam curve")?;
        let (mut la, mut lb) = (self.faces[f].loops[a].clone(), self.faces[f].loops[b].clone());
        if !self.start_at(&mut la, va) || !self.start_at(&mut lb, vb) {
            return Err("a rim does not pass its seam's vertex".into());
        }
        let mut joined = la;
        joined.push((seam, true));
        joined.extend(lb);
        joined.push((seam, false));
        let (hi, lo) = (a.max(b), a.min(b));
        self.faces[f].loops.remove(hi);
        self.faces[f].loops[lo] = joined;
        Ok(())
    }

    /// One rim: a seam to the pole beyond it, on the face's side.
    fn cap(&mut self, f: usize, a: usize, w: (i64, i64)) -> Result<(), String> {
        let dir = usize::from(w.0 == 0);
        let s = self.faces[f].surface.clone();
        // walking +u with the natural normal, the face lies on +v
        let forward = if dir == 0 { w.0 > 0 } else { w.1 < 0 };
        let upward = forward == self.faces[f].same_sense;
        let Some(pole) = pole_of(&s, dir, upward) else {
            return Err(format!("a closed {} face with one rim and no pole", kind(&s)));
        };
        let name = self.fresh();
        self.verts.push((pole, name));
        let vp = self.verts.len() - 1;
        let va = self.start_of(self.faces[f].loops[a][0]);
        let seam = self
            .seam_edge(&s, dir, va, vp)
            .ok_or("the surface has no seam curve")?;
        let mut l = self.faces[f].loops[a].clone();
        l.push((seam, true));
        l.push((seam, false));
        self.faces[f].loops[a] = l;
        Ok(())
    }

    /// A surface closed all round with no edges: a sphere gets one meridian, pole to pole.
    fn whole_face(&mut self, f: usize) -> Result<(), String> {
        let s = self.faces[f].surface.clone();
        match &s {
            Surface::Sphere(_) => {
                let (a, b) = (
                    pole_of(&s, 0, false).ok_or("no pole")?,
                    pole_of(&s, 0, true).ok_or("no pole")?,
                );
                let (na, nb) = (self.fresh(), self.fresh());
                self.verts.push((a, na));
                self.verts.push((b, nb));
                let n = self.verts.len();
                let seam = self.seam_edge(&s, 0, n - 2, n - 1).ok_or("no seam curve")?;
                self.faces[f].loops = vec![vec![(seam, true), (seam, false)]];
                Ok(())
            }
            _ => Err(format!("a whole {} with no edges", kind(&s))),
        }
    }

    fn faces_of_edges(&self) -> Vec<Vec<usize>> {
        let mut out = vec![Vec::new(); self.edges.len()];
        for (f, face) in self.faces.iter().enumerate() {
            for l in &face.loops {
                for u in l {
                    if !out[u.0].contains(&f) {
                        out[u.0].push(f);
                    }
                }
            }
        }
        out
    }

    fn off(&self, e: usize, s: &Surface) -> f64 {
        let ed = &self.edges[e];
        let w = ed.range.width();
        (0..=8)
            .map(|k| s.distance(ed.curve.eval(ed.range.lo + w * k as f64 / 8.0)))
            .fold(0.0, f64::max)
    }

    /// Edges a file gives within its own tolerance but not within the kernel's get the curve
    /// where their two faces' surfaces really meet: in closed form with a plane, else as an
    /// intersection curve through the file's points pulled onto both. Faces that touch along
    /// the edge have no such curve, and it stays as it is. A file's error shifts the edge by no
    /// more than that error, and `limit` bounds it.
    pub fn tighten(&mut self, limit: f64) {
        let adj = self.faces_of_edges();
        for e in 0..self.edges.len() {
            if adj[e].len() != 2 {
                continue;
            }
            let (a, b) = (
                self.faces[adj[e][0]].surface.clone(),
                self.faces[adj[e][1]].surface.clone(),
            );
            let dev = self.off(e, &a).max(self.off(e, &b));
            if dev <= 0.5 * tol::LINEAR {
                continue;
            }
            if self.closed_form_edge(e, &a, &b, limit) || self.chain_edge(e, a, b, limit) {
                continue;
            }
            self.tangent += 1;
        }
        self.settle_vertices();
        self.collapse_slivers(limit);
    }

    /// An edge shorter than the balls of its ends, or than the tolerance, is a sliver a file's
    /// gaps left: its two ends become one vertex, and the loops close over it. Nothing moves
    /// more than the edge's length; at most `limit` in any case.
    fn collapse_slivers(&mut self, limit: f64) {
        self.vtol.resize(self.verts.len(), 0.0);
        let mut merged: Vec<usize> = (0..self.verts.len()).collect();
        fn find(m: &mut [usize], mut v: usize) -> usize {
            while m[v] != v {
                m[v] = m[m[v]];
                v = m[v];
            }
            v
        }
        let mut gone = vec![false; self.edges.len()];
        let mut any = false;
        for e in 0..self.edges.len() {
            let ed = &self.edges[e];
            let (a, b) = (find(&mut merged, ed.start), find(&mut merged, ed.end));
            if a == b && ed.start != ed.end {
                gone[e] = true;
                any = true;
                continue;
            }
            if a == b {
                continue;
            }
            let w = ed.range.width();
            let len: f64 = (0..4)
                .map(|k| {
                    ed.curve
                        .eval(ed.range.lo + w * k as f64 / 4.0)
                        .distance(ed.curve.eval(ed.range.lo + w * (k + 1) as f64 / 4.0))
                })
                .sum();
            let small = 2.0 * self.vtol[a].max(self.vtol[b]);
            if len <= tol::LINEAR.max(small.min(limit)) {
                merged[b] = a;
                gone[e] = true;
                any = true;
            }
        }
        if !any {
            return;
        }
        for f in &mut self.faces {
            for l in &mut f.loops {
                l.retain(|u| !gone[u.0]);
            }
            f.loops.retain(|l| !l.is_empty());
        }
        self.faces.retain(|f| !f.loops.is_empty());
        for ed in &mut self.edges {
            ed.start = find(&mut merged, ed.start);
            ed.end = find(&mut merged, ed.end);
        }
        self.settle_vertices();
    }

    fn closed_form_edge(&mut self, e: usize, a: &Surface, b: &Surface, limit: f64) -> bool {
        use forge_geom::intersect::{SurfaceSurface, plane_surface};
        let r = match (a, b) {
            (Surface::Plane(p), s) | (s, Surface::Plane(p)) => plane_surface(p, s),
            _ => return false,
        };
        let SurfaceSurface::Curves(curves) = r else {
            return false;
        };
        let ed = &self.edges[e];
        let w = ed.range.width();
        let samples: Vec<Point3> = (0..=8)
            .map(|k| ed.curve.eval(ed.range.lo + w * k as f64 / 8.0))
            .collect();
        let (ps, pe) = (self.verts[ed.start].0, self.verts[ed.end].0);
        let closed = ed.start == ed.end;
        for (c, _) in curves {
            let d = c.domain();
            let whole = if d.lo.is_finite() {
                Interval::new(d.lo, d.hi)
            } else {
                let far = 4.0 * tol::MODEL_HALF_SIZE;
                Interval::new(-far, far)
            };
            let fits = samples
                .iter()
                .all(|&p| c.eval(c.closest(p, whole)).distance(p) <= limit);
            if !fits {
                continue;
            }
            let (t0, t1) = (c.closest(ps, whole), c.closest(pe, whole));
            // along the old edge's way, or the new curve runs against it
            let (old, new) = (
                ed.curve.deriv(ed.range.lo + 0.5 * w).d1,
                c.deriv(c.closest(samples[4], whole)).d1,
            );
            let same = old.dot(new) >= 0.0;
            let range = match d.period {
                Some(p) if closed => Interval::new(t0, t0 + p),
                Some(p) => {
                    let (lo, mut hi) = if same { (t0, t1) } else { (t1, t0) };
                    while hi <= lo {
                        hi += p;
                    }
                    Interval::new(lo, hi)
                }
                None => Interval::new(t0.min(t1), t0.max(t1)),
            };
            self.edges[e].curve = c;
            self.edges[e].range = range;
            if !same {
                self.flip(e);
            }
            return true;
        }
        false
    }

    fn flip(&mut self, e: usize) {
        let ed = &mut self.edges[e];
        std::mem::swap(&mut ed.start, &mut ed.end);
        for f in &mut self.faces {
            for l in &mut f.loops {
                for u in l.iter_mut() {
                    if u.0 == e {
                        u.1 = !u.1;
                    }
                }
            }
        }
    }

    fn chain_edge(&mut self, e: usize, a: Surface, b: Surface, limit: f64) -> bool {
        use forge_geom::intcurve::{IntCurve, direction, refine};
        let ed = &self.edges[e];
        let closed = ed.start == ed.end;
        let w = ed.range.width();
        // a chain fine enough to lie within the kernel's tolerance before its Newton step
        let n = match &ed.curve {
            Curve::Procedural(p) => p.segments(ed.range, 1e-4).clamp(8, 2000),
            _ => 64,
        };
        let (mut pts, mut tans) = (Vec::with_capacity(n + 1), Vec::with_capacity(n + 1));
        let count = if closed { n } else { n + 1 };
        for k in 0..count {
            let t = ed.range.lo + w * k as f64 / n as f64;
            let d = ed.curve.deriv(t);
            let Some(p) = refine(&a, &b, d.p) else {
                return false;
            };
            if p.distance(d.p) > limit {
                return false;
            }
            // surfaces all but touching here would give a curve that wanders: leave the edge be
            let (na, nb) = (a.gradient(p), b.gradient(p));
            if na.cross(nb).length() < 0.02 * na.length() * nb.length() {
                return false;
            }
            let Some(g) = direction(&a, &b, p) else {
                return false;
            };
            let g = if g.dot(d.d1) < 0.0 { -g } else { g };
            if let Some(&q) = pts.last() {
                if g.dot(p - q) <= 0.0 {
                    return false;
                }
            }
            pts.push(p);
            tans.push(g);
        }
        let hi = (pts.len() - if closed { 0 } else { 1 }) as f64;
        // the chain's sag, from where its middles land
        let mut sag = 0.0_f64;
        for k in 0..pts.len() - 1 {
            let m = pts[k] + (pts[k + 1] - pts[k]) * 0.5;
            if let Some(q) = refine(&a, &b, m) {
                sag = sag.max(q.distance(m));
            }
        }
        let c = IntCurve::new(a, b, pts, tans, closed, sag);
        self.edges[e].curve = Curve::Intersection(Arc::new(c));
        self.edges[e].range = Interval::new(0.0, hi);
        true
    }

    /// Each vertex at the middle of its edges' ends, with a ball that holds them where that is
    /// more than the tolerance, and each edge's range reaching it.
    fn settle_vertices(&mut self) {
        self.vtol.resize(self.verts.len(), 0.0);
        let mut ends: Vec<Vec<Point3>> = vec![Vec::new(); self.verts.len()];
        for ed in &self.edges {
            ends[ed.start].push(ed.curve.eval(ed.range.lo));
            ends[ed.end].push(ed.curve.eval(ed.range.hi));
        }
        for (v, list) in ends.iter().enumerate() {
            if list.is_empty() {
                continue;
            }
            let p = self.verts[v].0;
            let worst = list.iter().map(|q| q.distance(p)).fold(0.0, f64::max);
            if worst <= tol::LINEAR {
                continue;
            }
            // the box's middle holds them within half the largest gap between any two
            let (mut lo, mut hi) = (list[0], list[0]);
            for q in list {
                lo = Point3::new(lo.x.min(q.x), lo.y.min(q.y), lo.z.min(q.z));
                hi = Point3::new(hi.x.max(q.x), hi.y.max(q.y), hi.z.max(q.z));
            }
            let m = lo + (hi - lo) * 0.5;
            let r = list.iter().map(|q| q.distance(m)).fold(0.0, f64::max);
            self.verts[v].0 = m;
            self.vtol[v] = if r > tol::LINEAR { r } else { 0.0 };
        }
    }

    /// Puts a plane's outer loop first, as bodies have it: the largest area.
    pub fn outer_first(&mut self) {
        for f in 0..self.faces.len() {
            if self.faces[f].loops.len() < 2 || !matches!(self.faces[f].surface, Surface::Plane(_)) {
                continue;
            }
            let s = self.faces[f].surface.clone();
            let mut best = (0usize, -1.0_f64);
            for (i, l) in self.faces[f].loops.iter().enumerate() {
                let mut pts = Vec::new();
                for &u in l {
                    self.walk(u, &s, 16, &mut pts);
                }
                let mut area = 0.0;
                for k in 0..pts.len() {
                    let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
                    area += a.x * b.y - b.x * a.y;
                }
                if area.abs() > best.1 {
                    best = (i, area.abs());
                }
            }
            self.faces[f].loops.swap(0, best.0);
        }
    }

    /// The body: vertices no edge uses are left out.
    pub fn into_body(self) -> Body {
        let mut body = Body::new();
        let mut map: Vec<Option<VertexId>> = vec![None; self.verts.len()];
        let used: Vec<bool> = {
            let mut u = vec![false; self.edges.len()];
            for f in &self.faces {
                for l in &f.loops {
                    for x in l {
                        u[x.0] = true;
                    }
                }
            }
            u
        };
        let mut emap: Vec<Option<EdgeId>> = vec![None; self.edges.len()];
        for (i, e) in self.edges.into_iter().enumerate() {
            if !used[i] {
                continue;
            }
            let vtol = &self.vtol;
            let mut at = |v: usize, body: &mut Body| {
                *map[v].get_or_insert_with(|| {
                    let t = vtol.get(v).copied().unwrap_or(0.0);
                    if t > 0.0 {
                        body.add_tolerant_vertex(self.verts[v].0, t, self.verts[v].1)
                    } else {
                        body.add_vertex(self.verts[v].0, self.verts[v].1)
                    }
                })
            };
            let (s, t) = (at(e.start, &mut body), at(e.end, &mut body));
            emap[i] = Some(body.add_edge(e.curve, e.range, s, t, e.name));
        }
        let shells: Vec<ShellId> = (0..self.shells.max(1)).map(|_| body.add_shell()).collect();
        for f in self.faces {
            let face = body.add_face(shells[f.shell], f.surface, f.same_sense, f.name);
            for l in &f.loops {
                let uses: Vec<(EdgeId, bool)> =
                    l.iter().filter_map(|&(e, w)| emap[e].map(|x| (x, w))).collect();
                body.add_loop(face, &uses);
            }
        }
        body
    }
}

fn kind(s: &Surface) -> &'static str {
    match s {
        Surface::Plane(_) => "plane",
        Surface::Cylinder(_) => "cylinder",
        Surface::Cone(_) => "cone",
        Surface::Sphere(_) => "sphere",
        Surface::Torus(_) => "torus",
        Surface::Procedural(_) => "spline surface",
    }
}

fn unwrap(uv: Point2, near: Point2, du: Domain1, dv: Domain1) -> Point2 {
    let fix = |x: f64, r: f64, p: Option<f64>| p.map_or(x, |p| x + p * ((r - x) / p).round());
    Point2::new(fix(uv.x, near.x, du.period), fix(uv.y, near.y, dv.period))
}

/// The surface's pole at the high or low end of the parameter a seam runs along (v for `dir` 0,
/// u for 1), if it has one.
fn pole_of(s: &Surface, dir: usize, up: bool) -> Option<Point3> {
    match s {
        Surface::Sphere(k) => {
            let z: Vec3 = k.frame.z().into();
            Some(k.frame.origin + z * (if up { k.radius } else { -k.radius }))
        }
        // a cone's only pole is its apex, at its low v
        Surface::Cone(k) if !up => {
            let z: Vec3 = k.frame.z().into();
            Some(k.frame.origin + z * k.apex_v())
        }
        Surface::Procedural(p) => {
            let d = p.domain();
            // collapsed there: the row of points is one point
            let at = |k: f64| {
                if dir == 0 {
                    s.eval(Point2::new(
                        d.u.lo + (d.u.hi - d.u.lo) * k,
                        if up { d.v.hi } else { d.v.lo },
                    ))
                } else {
                    s.eval(Point2::new(
                        if up { d.u.hi } else { d.u.lo },
                        d.v.lo + (d.v.hi - d.v.lo) * k,
                    ))
                }
            };
            let a = at(0.0);
            (1..8)
                .all(|k| at(k as f64 / 8.0).distance(a) <= tol::LINEAR)
                .then_some(a)
        }
        _ => None,
    }
}

/// The curve from `a` to `b` along the surface where its `dir` parameter stays fixed.
fn iso_curve(s: &Surface, dir: usize, a: Point3, b: Point3) -> Option<(Curve, Interval)> {
    match s {
        // along the axis or the slant: straight
        Surface::Cylinder(_) | Surface::Cone(_) if dir == 0 => {
            let d = (b - a).normalize()?;
            Some((
                Curve::Line(Line { origin: a, dir: d }),
                Interval::new(0.0, a.distance(b)),
            ))
        }
        Surface::Cylinder(k) => {
            // round the axis at b's height: no winding of v exists, so this is not reached
            let _ = k;
            None
        }
        Surface::Sphere(k) => {
            // a meridian: a circle through a and the poles, in their plane
            let z: Vec3 = k.frame.z().into();
            let uv = s.project(
                if a.distance(k.frame.origin + z * k.radius) > tol::LINEAR
                    && a.distance(k.frame.origin - z * k.radius) > tol::LINEAR
                {
                    a
                } else {
                    b
                },
            );
            let (su, cu) = dmath::sin_cos(uv.x);
            let (x, y): (Vec3, Vec3) = (k.frame.x().into(), k.frame.y().into());
            let rad = x * cu + y * su;
            let frame = Frame::from_zx(k.frame.origin, rad.cross(z).normalize()?, rad)?;
            let circle = Circle {
                frame,
                radius: k.radius,
            };
            arc(Curve::Circle(circle), a, b)
        }
        Surface::Torus(k) => {
            let uv = s.project(a);
            let z: Vec3 = k.frame.z().into();
            if dir == 0 {
                // the tube's circle at this u
                let (su, cu) = dmath::sin_cos(uv.x);
                let (x, y): (Vec3, Vec3) = (k.frame.x().into(), k.frame.y().into());
                let rad = x * cu + y * su;
                let frame = Frame::from_zx(k.frame.origin + rad * k.major, rad.cross(z).normalize()?, rad)?;
                arc(
                    Curve::Circle(Circle {
                        frame,
                        radius: k.minor,
                    }),
                    a,
                    b,
                )
            } else {
                // round the axis at this v
                let (sv, cv) = dmath::sin_cos(uv.y);
                let c = k.frame.origin + z * (k.minor * sv);
                let frame = Frame::from_zx(c, k.frame.z(), k.frame.x().into())?;
                arc(
                    Curve::Circle(Circle {
                        frame,
                        radius: k.major + k.minor * cv,
                    }),
                    a,
                    b,
                )
            }
        }
        Surface::Procedural(_) => {
            let (ua, ub) = (s.project(a), s.project(b));
            let (fixed, t0, t1) = if dir == 0 {
                (ua.x, ua.y, ub.y)
            } else {
                (ua.y, ua.x, ub.x)
            };
            let (lo, hi) = (t0.min(t1), t0.max(t1));
            let up = t1 >= t0;
            let c = Iso {
                s: s.clone(),
                dir,
                fixed,
                up,
                span: Interval::new(lo, hi),
            };
            let w = hi - lo;
            Some((Curve::Procedural(ProcCurve(Arc::new(c))), Interval::new(0.0, w)))
        }
        _ => None,
    }
}

/// The arc of `c` from `a` to `b` the way its parameter runs, or the curve reversed in effect:
/// here a circle's frame is chosen so that the shorter way is forward.
fn arc(c: Curve, a: Point3, b: Point3) -> Option<(Curve, Interval)> {
    let d = c.domain();
    let whole = Interval::new(d.lo, d.hi);
    let (t0, mut t1) = (c.closest(a, whole), c.closest(b, whole));
    if t1 <= t0 {
        t1 += d.period?;
    }
    if t1 - t0 > dmath::PI + 1e-9 {
        // the other way round: turn the circle over
        if let Curve::Circle(k) = &c {
            let flipped = Frame::from_zx(
                k.frame.origin,
                (-Vec3::from(k.frame.z())).normalize()?,
                k.frame.x().into(),
            )?;
            let n = Curve::Circle(Circle {
                frame: flipped,
                radius: k.radius,
            });
            let (s0, mut s1) = (n.closest(a, whole), n.closest(b, whole));
            if s1 <= s0 {
                s1 += d.period?;
            }
            return Some((n, Interval::new(s0, s1)));
        }
    }
    Some((c, Interval::new(t0, t1)))
}

/// A line of a spline surface where one parameter stays fixed, by construction exact. Runs over
/// [0, width], `up` saying which way along the free parameter.
#[derive(Debug)]
struct Iso {
    s: Surface,
    dir: usize,
    fixed: f64,
    up: bool,
    span: Interval,
}

impl Iso {
    fn uv(&self, t: f64) -> Point2 {
        let x = if self.up {
            self.span.lo + t
        } else {
            self.span.hi - t
        };
        if self.dir == 0 {
            Point2::new(self.fixed, x)
        } else {
            Point2::new(x, self.fixed)
        }
    }
}

impl ProceduralCurve for Iso {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let d = self.s.deriv(self.uv(t));
        let k = if self.up { 1.0 } else { -1.0 };
        if self.dir == 0 {
            CurveDeriv {
                p: d.p,
                d1: d.dv * k,
                d2: d.dvv,
            }
        } else {
            CurveDeriv {
                p: d.p,
                d1: d.du * k,
                d2: d.duu,
            }
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: 0.0,
            hi: self.span.width(),
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let uv = self.s.project(p);
        let x = if self.dir == 0 { uv.y } else { uv.x };
        let t = if self.up {
            x - self.span.lo
        } else {
            self.span.hi - x
        };
        t.clamp(range.lo, range.hi)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let (a, b) = (self.uv(range.lo), self.uv(range.hi));
        let uv = forge_math::Box2 {
            u: Interval::new(a.x.min(b.x), a.x.max(b.x)),
            v: Interval::new(a.y.min(b.y), a.y.max(b.y)),
        };
        self.s.bbox(uv)
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        // the most of its samples, with room
        let w = range.width();
        1.5 * (0..=16)
            .map(|k| self.deriv(range.lo + w * k as f64 / 16.0).d1.length())
            .fold(0.0, f64::max)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        let len = self.speed_bound(range) * range.width();
        let r = match &self.s {
            Surface::Procedural(p) => p.mesh_radius(),
            _ => 1.0,
        };
        let step = 2.0 * (2.0 * r.max(tol::LINEAR) * chord.max(1e-12)).sqrt();
        ((len / step).ceil() as usize).max(1)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Iso {
            s: self.s.transformed(f),
            dir: self.dir,
            fixed: self.fixed,
            up: self.up,
            span: self.span,
        }))
    }
}

//! A8's face arrangement per near plane, as `face_builder` builds it (the same segments, sub-edges,
//! cycles, holes and holders, so the same facet counts), made fast:
//!
//! - **Keys first.** A vertex is an input vertex `(body, v)` or the meet of three classes (sorted);
//!   equal keys are one point without arithmetic, and a meet lies on its own three planes.
//! - **Screened f64, then exact.** Every point carries an f64 shadow and an error bound per
//!   coordinate; each sign (a plane's side, the order along a line, an orientation, an angle) is
//!   decided in f64 where the bound settles it, else exactly. Exact points are made lazily.
//! - **No exact crossing tests.** An edge crosses a line's plane strictly inside it exactly when
//!   its ends are strictly on opposite sides (screened); only then is its meet made.
//! - **Per line, one sort.** The vertices on a line are sorted along it once; each segment's
//!   sub-edges are the gaps between its ends there.
//! - **Merging.** Different keys at one point (four planes or more through it) are merged: points
//!   no axis keeps apart (screened) are compared exactly.
//!
//! The counts of screens decided in f64 and of those that went exact are kept, per boolean.

use super::{Classes, GAMMA, In, NONE, PlaneF, Pts, crosses, crosses_f, meet_f, plane_f, side_err};
use forge_bool_core::Sgn;
use forge_math::exact::{Dy, DyPlane, DyV3, HPoint, meet3, orient2};
use forge_topo::{Body, VertexId};
use std::cell::{Cell, OnceCell};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Key {
    In(u32, u32),
    M([u32; 3]),
}

/// A point: its key, its f64 shadow and that shadow's bound per coordinate, the exact point once
/// asked for.
struct Pt {
    key: Key,
    /// classes known to hold the point exactly (its key's three, or an input vertex's round
    /// faces' classes when `vertex_at` checked it)
    on: u128,
    f: [f64; 3],
    e: f64,
    ex: OnceCell<HPoint>,
}

/// A class line `C ∩ K`: its direction exactly and in f64, with `|a_i b_j| + |a_j b_i|` per
/// component for the f64 direction's bound.
struct Line {
    d: DyV3,
    f: [f64; 3],
    ab: [f64; 3],
}

/// Screen tallies: (decided in f64, went exact), per kind.
#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct Tally {
    pub side: (u32, u32),
    pub along: (u32, u32),
    pub apart: (u32, u32),
    pub orient: (u32, u32),
    pub angle: (u32, u32),
    pub axis: (u32, u32),
    pub meet: (u32, u32),
    pub cross: (u32, u32),
}

struct Ctx<'a> {
    bodies: &'a [&'a Body],
    cls: &'a Classes,
    pts: &'a [Pts],
    snapped: &'a [DyPlane],
    cpf: Vec<PlaneF>,
    p: Vec<Pt>,
    /// per body, per vertex: its point's id, or NONE
    by_in: Vec<Vec<u32>>,
    /// the meets of the current plane `mc` with classes (k, h): `mtab[k * nc + h]` (k < h), NONE
    /// for not yet asked, DEP for dependent; `mtouched` to reset it for the next plane
    mc: u32,
    mtab: Vec<u32>,
    mtouched: Vec<u32>,
    t: Cell<Tally>,
    /// per face, the axis its in-face tests project along
    fax: Vec<usize>,
    /// per near face, its coedges' ends as point ids: `fe_pt[fe_start[g]..fe_start[g + 1]]`
    fe_start: Vec<u32>,
    fe_pt: Vec<(u32, u32)>,
}

fn bump(t: &Cell<Tally>, f: impl FnOnce(&mut Tally)) {
    let mut x = t.get();
    f(&mut x);
    t.set(x);
}

fn m3(p: &[f64; 3]) -> f64 {
    p[0].abs().max(p[1].abs()).max(p[2].abs())
}

impl<'a> Ctx<'a> {
    fn ex(&self, i: u32) -> &HPoint {
        let pt = &self.p[i as usize];
        pt.ex.get_or_init(|| match pt.key {
            Key::In(b, v) => self.pts[b as usize].ph(VertexId(v), self.snapped),
            Key::M([a, b, c]) => {
                let pl = &self.cls.planes;
                meet3(&pl[a as usize], &pl[b as usize], &pl[c as usize]).expect("an independent meet")
            }
        })
    }

    fn add_in(&mut self, b: usize, v: VertexId) -> u32 {
        let at = self.by_in[b][v.index()];
        if at != NONE {
            return at;
        }
        let f = self.pts[b].pf(v, self.snapped);
        // (`Pts::pf`: within 1e-13 of the exact point relative to its size)
        let e = 1.0000001e-13 * (1.0 + m3(&f));
        let id = self.p.len() as u32;
        let mut on = 0u128;
        if self.pts[b].held[v.index()] {
            for &g in self.pts[b].round(v) {
                let k = self.cls.of_face[g as usize];
                if k != NONE {
                    on |= 1u128 << k;
                }
            }
        }
        self.p.push(Pt {
            key: Key::In(b as u32, v.0),
            on,
            f,
            e,
            ex: OnceCell::new(),
        });
        self.by_in[b][v.index()] = id;
        id
    }

    /// Start plane `c`'s meets.
    fn plane_meets(&mut self, c: u32) {
        for &i in &self.mtouched {
            self.mtab[i as usize] = NONE;
        }
        self.mtouched.clear();
        self.mc = c;
    }

    /// The meet of the current plane with classes `a` and `b`, or None when they are dependent.
    fn add_m(&mut self, a: u32, b: u32) -> Option<u32> {
        const DEP: u32 = NONE - 1;
        let c = self.mc;
        let (a, b) = (a.min(b), a.max(b));
        if a == b || a == c || b == c {
            return None;
        }
        let nc = self.cpf.len();
        let slot = a as usize * nc + b as usize;
        match self.mtab[slot] {
            NONE => {}
            DEP => return None,
            id => return Some(id),
        }
        self.mtouched.push(slot as u32);
        let mut k = [a, b, c];
        k.sort_unstable();
        let (pa, pb, pc) = (
            &self.cpf[k[0] as usize],
            &self.cpf[k[1] as usize],
            &self.cpf[k[2] as usize],
        );
        let pl = &self.cls.planes;
        let (ea, eb, ec) = (&pl[k[0] as usize], &pl[k[1] as usize], &pl[k[2] as usize]);
        let got = match meet_f(pa, pb, pc) {
            Some((f, e)) if e <= 1e-13 * (1.0 + m3(&f)) => {
                bump(&self.t, |t| t.meet.0 += 1);
                Some((f, e, None))
            }
            _ => {
                bump(&self.t, |t| t.meet.1 += 1);
                // (dependent normals, the usual reason: settled without the meet)
                if ea.n.dot(&eb.n.cross(&ec.n)).is_zero() {
                    None
                } else {
                    meet3(ea, eb, ec).map(|h| {
                        let f = h.to_f64();
                        (f, 4.0 * f64::EPSILON * (1.0 + m3(&f)), Some(h))
                    })
                }
            }
        };
        let id = match got {
            None => DEP,
            Some((f, e, h)) => {
                let ex = OnceCell::new();
                if let Some(h) = h {
                    let _ = ex.set(h);
                }
                let on = (1u128 << k[0]) | (1u128 << k[1]) | (1u128 << k[2]);
                self.p.push(Pt {
                    key: Key::M(k),
                    on,
                    f,
                    e,
                    ex,
                });
                self.p.len() as u32 - 1
            }
        };
        self.mtab[slot] = id;
        (id != DEP).then_some(id)
    }

    /// Class `k`'s side at point `i`: -1, 0, 1.
    fn side(&self, k: u32, i: u32) -> i32 {
        let pt = &self.p[i as usize];
        if pt.on >> k & 1 == 1 {
            return 0;
        }
        let pff = &self.cpf[k as usize];
        let e = pff.n[0] * pt.f[0] + pff.n[1] * pt.f[1] + pff.n[2] * pt.f[2] - pff.c;
        let mag = pff.n[0].abs() * pt.f[0].abs()
            + pff.n[1].abs() * pt.f[1].abs()
            + pff.n[2].abs() * pt.f[2].abs()
            + pff.c.abs();
        let bound = GAMMA * mag + (pff.n[0].abs() + pff.n[1].abs() + pff.n[2].abs()) * pt.e;
        if e.abs() > bound {
            bump(&self.t, |t| t.side.0 += 1);
            return if e > 0.0 { 1 } else { -1 };
        }
        bump(&self.t, |t| t.side.1 += 1);
        let s = side_err(&self.cls.planes[k as usize], pff, &pt.f, pt.e, || {
            self.ex(i).clone()
        });
        match s {
            Sgn::Pos => 1,
            Sgn::Neg => -1,
            Sgn::Zero => 0,
        }
    }

    fn line(&self, c: u32, k: u32) -> Line {
        let (a, b) = (&self.cls.planes[c as usize].n, &self.cls.planes[k as usize].n);
        let d = a.cross(b);
        let (x, y) = (&self.cpf[c as usize].n, &self.cpf[k as usize].n);
        let f = [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ];
        let ab = [
            (x[1] * y[2]).abs() + (x[2] * y[1]).abs(),
            (x[2] * y[0]).abs() + (x[0] * y[2]).abs(),
            (x[0] * y[1]).abs() + (x[1] * y[0]).abs(),
        ];
        Line { d, f, ab }
    }

    /// The sign of `(p_i − p_j) · d` along line `l`.
    fn along(&self, i: u32, j: u32, l: &Line) -> i32 {
        if i == j {
            return 0;
        }
        let (p, q) = (&self.p[i as usize], &self.p[j as usize]);
        let mut v = 0.0;
        let mut b = 0.0;
        for k in 0..3 {
            let dk = p.f[k] - q.f[k];
            v += dk * l.f[k];
            b += (p.e + q.e) * (l.f[k].abs() + 1e-13 * l.ab[k]) + 1e-13 * dk.abs() * (l.f[k].abs() + l.ab[k]);
        }
        if v.abs() > b {
            bump(&self.t, |t| t.along.0 += 1);
            return if v > 0.0 { 1 } else { -1 };
        }
        bump(&self.t, |t| t.along.1 += 1);
        let (hp, hq) = (self.ex(i), self.ex(j));
        let pd = hp.p.dot(&l.d).mul(&hq.w);
        let qd = hq.p.dot(&l.d).mul(&hp.w);
        pd.sub(&qd).signum() * hp.w.signum() * hq.w.signum()
    }

    /// The same point.
    fn same(&self, i: u32, j: u32) -> bool {
        if i == j {
            return true;
        }
        let (p, q) = (&self.p[i as usize], &self.p[j as usize]);
        if p.key == q.key {
            return true;
        }
        let tol = (p.e + q.e) * (1.0 + 1e-12) + 1e-300;
        if (0..3).any(|k| (p.f[k] - q.f[k]).abs() > tol) {
            bump(&self.t, |t| t.apart.0 += 1);
            return false;
        }
        bump(&self.t, |t| t.apart.1 += 1);
        self.ex(i).same(self.ex(j))
    }

    /// The order of points `i`, `j` on axis `a`.
    fn cmp_axis(&self, i: u32, j: u32, a: usize) -> i32 {
        let (p, q) = (&self.p[i as usize], &self.p[j as usize]);
        let d = p.f[a] - q.f[a];
        if d.abs() > (p.e + q.e) * (1.0 + 1e-12) + 1e-300 {
            bump(&self.t, |t| t.axis.0 += 1);
            return if d > 0.0 { 1 } else { -1 };
        }
        bump(&self.t, |t| t.axis.1 += 1);
        self.ex(i).cmp_axis(self.ex(j), a)
    }

    /// The orientation of points `a`, `b`, `c` in the (s, t) projection.
    fn orient(&self, a: u32, b: u32, c: u32, s: usize, t: usize) -> i32 {
        let (pa, pb, pc) = (&self.p[a as usize], &self.p[b as usize], &self.p[c as usize]);
        let (bs, bt, cs, ct) = (
            pb.f[s] - pa.f[s],
            pb.f[t] - pa.f[t],
            pc.f[s] - pa.f[s],
            pc.f[t] - pa.f[t],
        );
        let v = bs * ct - bt * cs;
        let e = 2.0 * (pa.e + pb.e.max(pc.e)) * (1.0 + 1e-12);
        let bound = e * (bs.abs() + bt.abs() + cs.abs() + ct.abs())
            + e * e
            + 1e-14 * ((bs * ct).abs() + (bt * cs).abs());
        if v.abs() > bound {
            bump(&self.t, |x| x.orient.0 += 1);
            return if v > 0.0 { 1 } else { -1 };
        }
        bump(&self.t, |x| x.orient.1 += 1);
        orient2(self.ex(a), self.ex(b), self.ex(c), s, t)
    }

    /// Is `q` (between points `a` and `b`, their mean) inside face `g`? Parity over the face's
    /// coedges (`in_face`), screened per edge (`crosses_f`). The parity ray runs across the line
    /// `a`–`b` (along `t` when the line runs more along `s`), so it does not run along the face's
    /// edges on that line.
    fn mid_in(&self, a: u32, b: u32, g: usize) -> bool {
        let (pa, pb) = (&self.p[a as usize], &self.p[b as usize]);
        let qf = [
            0.5 * (pa.f[0] + pb.f[0]),
            0.5 * (pa.f[1] + pb.f[1]),
            0.5 * (pa.f[2] + pb.f[2]),
        ];
        let ax = self.fax[g];
        let (s, t) = ((ax + 1) % 3, (ax + 2) % 3);
        let (ds, dt) = ((pa.f[s] - pb.f[s]).abs(), (pa.f[t] - pb.f[t]).abs());
        let (s, t) = if ds > dt { (t, s) } else { (s, t) };
        let q: OnceCell<HPoint> = OnceCell::new();
        let mut n = 0;
        for &(ia, ib) in &self.fe_pt[self.fe_start[g] as usize..self.fe_start[g + 1] as usize] {
            let (fa, fb) = (&self.p[ia as usize].f, &self.p[ib as usize].f);
            let c = match crosses_f(qf, *fa, *fb, s, t) {
                Some(c) => {
                    bump(&self.t, |x| x.cross.0 += 1);
                    c
                }
                None => {
                    bump(&self.t, |x| x.cross.1 += 1);
                    let q = q.get_or_init(|| HPoint::mean(&[self.ex(a), self.ex(b)]));
                    crosses(q, self.ex(ia), self.ex(ib), s, t)
                }
            };
            n += c as usize;
        }
        n % 2 == 1
    }
}

/// The axis to project along (the normal's largest component; ties to the lower axis), and
/// whether that component is negative. (As `face_builder::axis_of`, on the f64 normal: a
/// normal's f64 components keep their order of size except at exact ties, which `to_f64` keeps.)
fn axis_of_f(p: &PlaneF) -> (usize, bool) {
    let n = p.n;
    let a = (0..3)
        .max_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs()).then(j.cmp(&i)))
        .unwrap();
    (a, n[a] < 0.0)
}

/// A segment on a line of plane `C`: its ends (point ids), the class of its line, and for a
/// boundary edge or a wall stretch the face of `C` (its position in `on[c]`) and its sense.
#[derive(Clone, Copy)]
struct Seg {
    a: u32,
    b: u32,
    line: u32,
    bound: (u32, i8),
}

/// A sub-edge: its ends (vertex positions in the plane's vertex list), its line, and the faces of
/// `C` (as a mask over positions in `on[c]`) it bounds with the face on its left, and on its right.
#[derive(Clone, Copy)]
struct Sub {
    u: u32,
    w: u32,
    line: u32,
    left: u64,
    right: u64,
}

/// Per class plane: (segments, vertices, facets), summed over near class planes, and the screen
/// tallies. `None` when a class holds more than 64 faces (the masks' width).
#[inline(never)]
pub(crate) fn counts(
    bodies: &[&Body],
    fs: &[In],
    cls: &Classes,
    pts: &[Pts],
    snapped: &[DyPlane],
) -> Option<((usize, usize, usize), Tally)> {
    let nc = cls.planes.len();
    let mut on: Vec<Vec<usize>> = vec![Vec::new(); nc];
    for (i, &c) in cls.of_face.iter().enumerate() {
        if c != NONE {
            on[c as usize].push(i);
        }
    }
    if on.iter().any(|o| o.len() > 64) {
        return None;
    }
    let mut gidx: Vec<Vec<usize>> = bodies.iter().map(|b| vec![usize::MAX; b.faces.len()]).collect();
    for (i, f) in fs.iter().enumerate() {
        gidx[f.body][f.face.index()] = i;
    }
    // per near face, its coedges: (start, end, class across or NONE)
    let mut fe_start: Vec<u32> = vec![0; fs.len() + 1];
    let mut fe: Vec<(VertexId, VertexId, u32)> = Vec::new();
    for (g, f) in fs.iter().enumerate() {
        if cls.of_face[g] != NONE {
            let b = bodies[f.body];
            for &lp in b.face(f.face).loops.iter() {
                for co in b.lp(lp).coedges() {
                    let across = b
                        .partner(co)
                        .map(|p| gidx[f.body][b.coedge_face(p).index()])
                        .filter(|&x| x != usize::MAX)
                        .map(|x| cls.of_face[x])
                        .unwrap_or(NONE);
                    fe.push((b.coedge_start(co), b.coedge_end(co), across));
                }
            }
        }
        fe_start[g + 1] = fe.len() as u32;
    }
    let edges_of = |g: usize| &fe[fe_start[g] as usize..fe_start[g + 1] as usize];
    // per class, the box of its faces
    let mut cbox: Vec<([f64; 3], [f64; 3])> = vec![([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]); nc];
    for (g, f) in fs.iter().enumerate() {
        let c = cls.of_face[g];
        if c != NONE {
            let bx = &mut cbox[c as usize];
            for k in 0..3 {
                bx.0[k] = bx.0[k].min(f.lo[k]);
                bx.1[k] = bx.1[k].max(f.hi[k]);
            }
        }
    }
    let is_wall = |k: u32| cls.walls.contains(&k);
    let mut cx = Ctx {
        bodies,
        cls,
        pts,
        snapped,
        cpf: cls.planes.iter().map(plane_f).collect(),
        p: Vec::with_capacity(256),
        by_in: pts.iter().map(|p| vec![NONE; p.f.borrow().len()]).collect(),
        mc: NONE,
        mtab: vec![NONE; cls.planes.len() * cls.planes.len()],
        mtouched: Vec::with_capacity(256),
        t: Cell::new(Tally::default()),
        fax: snapped.iter().map(|p| axis_of_f(&plane_f(p)).0).collect(),
        fe_start: Vec::new(),
        fe_pt: Vec::with_capacity(fe.len()),
    };
    let _ = cx.bodies;
    for (g, f) in fs.iter().enumerate() {
        for &(va, vb, _) in edges_of(g) {
            let (ia, ib) = (cx.add_in(f.body, va), cx.add_in(f.body, vb));
            cx.fe_pt.push((ia, ib));
        }
    }
    cx.fe_start = fe_start.clone();
    let (mut nseg, mut nvert, mut nfacet) = (0, 0, 0);
    // reused per plane
    let mut segs: Vec<Seg> = Vec::new();
    let mut ps: Vec<(u32, u128)> = Vec::new();
    let mut covered: Vec<bool> = Vec::new();
    for c in 0..nc {
        if on[c].is_empty() {
            continue;
        }
        let c32 = c as u32;
        cx.plane_meets(c32);
        segs.clear();
        // 1. boundary edges of the faces of C, each on C ∩ (the class across it)
        for (pos, &f) in on[c].iter().enumerate() {
            let b = fs[f].body;
            let along_c = fs[f].plane.n.dot(&cls.planes[c].n).signum();
            let sense = if bodies[b].face(fs[f].face).same_sense {
                along_c
            } else {
                -along_c
            };
            for &(va, vb, across) in edges_of(f) {
                if across == NONE || across == c32 {
                    continue;
                }
                let (a, bb) = (cx.add_in(b, va), cx.add_in(b, vb));
                segs.push(Seg {
                    a,
                    b: bb,
                    line: across,
                    bound: (pos as u32, sense as i8),
                });
            }
        }
        let nbound = segs.len();
        // 2. on each other line C ∩ K near C's faces, the stretches inside the faces of C and K
        let g = 4.0 * super::lin();
        let (clo, chi) = cbox[c];
        for k in 0..nc as u32 {
            if k as usize == c {
                continue;
            }
            let wall = is_wall(k);
            if !wall {
                let (klo, khi) = cbox[k as usize];
                if (0..3).any(|a| klo[a] > chi[a] + g || khi[a] < clo[a] - g) {
                    continue;
                }
            }
            let ln = cx.line(c32, k);
            if ln.d.x.is_zero() && ln.d.y.is_zero() && ln.d.z.is_zero() {
                continue; // parallel
            }
            ps.clear();
            // the points where the line meets a boundary edge of a face of C or of K, each with
            // the faces (bits: C's at their position in on[c], K's at 64 + theirs) whose boundary
            // gives it; `always`: faces whose boundary crosses the line at a point not listed
            let mut always: u128 = 0;
            let cross_edges = |faces: &[usize],
                               off: u32,
                               own: u32,
                               other: u32,
                               ps: &mut Vec<(u32, u128)>,
                               always: &mut u128,
                               cx: &mut Ctx| {
                for (pos, &f) in faces.iter().enumerate() {
                    let bit = 1u128 << (off + pos as u32);
                    let b = fs[f].body;
                    for &(va, vb, across) in edges_of(f) {
                        let (ia, ib) = (cx.add_in(b, va), cx.add_in(b, vb));
                        // (the face's vertices are on its own plane; on the line when on the other)
                        let (sa, sb) = (cx.side(other, ia), cx.side(other, ib));
                        if sa == 0 && cx.side(own, ia) == 0 {
                            ps.push((ia, bit));
                        }
                        if sb == 0 && cx.side(own, ib) == 0 {
                            ps.push((ib, bit));
                        }
                        if sa * sb >= 0 {
                            continue;
                        }
                        if across == NONE || across == k {
                            *always |= bit;
                            continue;
                        }
                        match cx.add_m(k, across) {
                            Some(x) => ps.push((x, bit)),
                            None => *always |= bit,
                        }
                    }
                }
            };
            cross_edges(&on[c], 0, c32, k, &mut ps, &mut always, &mut cx);
            if !wall {
                cross_edges(&on[k as usize], 64, k, c32, &mut ps, &mut always, &mut cx);
            }
            for &w in &cls.walls {
                if w != k
                    && let Some(x) = cx.add_m(k, w)
                {
                    ps.push((x, 0));
                }
            }
            // sorted along the line, repeats merged
            sort_along(&cx, &mut ps, &ln);
            ps.dedup_by(|p, q| {
                let same = cx.same(p.0, q.0);
                if same {
                    q.1 |= p.1;
                }
                same
            });
            let wi = cls.walls.iter().position(|&w| w == k);
            // per face, its answer on the current run of windows that touch none of its points
            let (mut known, mut ins): (u128, u128) = (0, 0);
            let inside =
                |a: u32, b: u32, adj: u128, bit: u128, g: usize, known: &mut u128, ins: &mut u128| -> bool {
                    if adj & bit != 0 || always & bit != 0 {
                        *known &= !bit;
                        return cx.mid_in(a, b, g);
                    }
                    if *known & bit == 0 {
                        if cx.mid_in(a, b, g) {
                            *ins |= bit;
                        } else {
                            *ins &= !bit;
                        }
                        *known |= bit;
                    }
                    *ins & bit != 0
                };
            // the windows on a boundary segment of C's on this line: a crossing there would
            // repeat it, so they are not looked at
            covered.clear();
            covered.resize(ps.len().saturating_sub(1), false);
            for g in &segs[..nbound] {
                if g.line != k {
                    continue;
                }
                let at = |x: u32| ps.iter().position(|&(y, _)| cx.same(x, y));
                if let (Some(i), Some(j)) = (at(g.a), at(g.b)) {
                    covered[i.min(j)..i.max(j)].fill(true);
                }
            }
            for w in 0..ps.len().saturating_sub(1) {
                if covered[w] {
                    continue;
                }
                let ((a, sa), (b, sb)) = (ps[w], ps[w + 1]);
                let adj = sa | sb;
                // (a face's answer does not carry past its own points)
                known &= !adj;
                let in_k = wall
                    || on[k as usize]
                        .iter()
                        .enumerate()
                        .any(|(pos, &h)| inside(a, b, adj, 1u128 << (64 + pos), h, &mut known, &mut ins));
                if !in_k {
                    continue;
                }
                match wi {
                    Some(wi) => {
                        // a wall bounds the faces of C it cuts, the inside of the box on its left +1
                        let pk = &cls.planes[k as usize];
                        let left = cls.planes[c].n.cross(&ln.d).dot(&pk.n).signum() * cx.along(b, a, &ln);
                        let sense = if (left > 0) == (cls.wall_in[wi] == Sgn::Pos) {
                            1
                        } else {
                            -1
                        };
                        for (pos, &f) in on[c].iter().enumerate() {
                            if inside(a, b, adj, 1u128 << pos, f, &mut known, &mut ins) {
                                segs.push(Seg {
                                    a,
                                    b,
                                    line: k,
                                    bound: (pos as u32, sense),
                                });
                            }
                        }
                    }
                    None => {
                        if on[c]
                            .iter()
                            .enumerate()
                            .any(|(pos, &f)| inside(a, b, adj, 1u128 << pos, f, &mut known, &mut ins))
                        {
                            segs.push(Seg {
                                a,
                                b,
                                line: k,
                                bound: (u32::MAX, 0),
                            });
                        }
                    }
                }
            }
        }
        nseg += segs.len();
        let xs = crossings(&mut cx, &segs);
        let (nv, nf) = plane(&cx, c32, &segs, &xs);
        nvert += nv;
        nfacet += nf;
    }
    Some(((nseg, nvert, nfacet), cx.t.get()))
}

fn sort_along(cx: &Ctx, ps: &mut [(u32, u128)], ln: &Line) {
    // insertion sort: few points, and most comparisons settled in f64
    for i in 1..ps.len() {
        let mut j = i;
        while j > 0 && cx.along(ps[j - 1].0, ps[j].0, ln) > 0 {
            ps.swap(j - 1, j);
            j -= 1;
        }
    }
}

/// The meets of segments on different lines of plane `C` whose f64 boxes meet: (i, j, point).
fn crossings(cx: &mut Ctx, segs: &[Seg]) -> Vec<(u32, u32, u32)> {
    let boxes: Vec<([f64; 3], [f64; 3])> = segs
        .iter()
        .map(|g| {
            let (p, q) = (&cx.p[g.a as usize], &cx.p[g.b as usize]);
            let e = p.e.max(q.e) * 2.0 + 1e-300;
            let mut lo = [0.0; 3];
            let mut hi = [0.0; 3];
            for k in 0..3 {
                lo[k] = p.f[k].min(q.f[k]) - e;
                hi[k] = p.f[k].max(q.f[k]) + e;
            }
            (lo, hi)
        })
        .collect();
    let mut out = Vec::new();
    for i in 0..segs.len() {
        for j in i + 1..segs.len() {
            if segs[i].line == segs[j].line {
                continue;
            }
            let (bi, bj) = (&boxes[i], &boxes[j]);
            if (0..3).any(|k| bi.0[k] > bj.1[k] || bj.0[k] > bi.1[k]) {
                continue;
            }
            if let Some(x) = cx.add_m(segs[i].line, segs[j].line) {
                out.push((i as u32, j as u32, x));
            }
        }
    }
    out
}

/// Plane `C`'s graph from its segments and their crossings; (vertices, facets).
#[allow(clippy::needless_range_loop)]
fn plane(cx: &Ctx, c: u32, segs: &[Seg], xs: &[(u32, u32, u32)]) -> (usize, usize) {
    let cls = cx.cls;
    let pc = &cls.planes[c as usize];
    let (ax, flip) = axis_of_f(&cx.cpf[c as usize]);
    let (s, t) = ((ax + 1) % 3, (ax + 2) % 3);
    // 1. the vertices: segment ends and crossings, merged; per line, its vertices
    let mut lines: Vec<u32> = segs.iter().map(|g| g.line).collect();
    lines.sort_unstable();
    lines.dedup();
    let li = |k: u32| lines.binary_search(&k).unwrap();
    let mut vs: Vec<u32> = Vec::new(); // point ids, one per vertex
    let vof = |id: u32, vs: &mut Vec<u32>| -> u32 {
        match vs.iter().position(|&x| cx.same(x, id)) {
            Some(i) => i as u32,
            None => {
                vs.push(id);
                vs.len() as u32 - 1
            }
        }
    };
    let mut on_line: Vec<Vec<u32>> = vec![Vec::new(); lines.len()];
    let mut ends: Vec<(u32, u32)> = Vec::with_capacity(segs.len());
    for g in segs {
        let (a, b) = (vof(g.a, &mut vs), vof(g.b, &mut vs));
        ends.push((a, b));
        on_line[li(g.line)].push(a);
        on_line[li(g.line)].push(b);
    }
    let lns: Vec<Line> = lines.iter().map(|&k| cx.line(c, k)).collect();
    // crossings within both segments
    for &(i, j, x) in xs {
        let (gs, gt) = (&segs[i as usize], &segs[j as usize]);
        let (ls, lt) = (&lns[li(gs.line)], &lns[li(gt.line)]);
        let within = |g: &Seg, l: &Line| cx.along(x, g.a, l) * cx.along(x, g.b, l) <= 0;
        if within(gs, ls) && within(gt, lt) {
            let v = vof(x, &mut vs);
            on_line[li(gs.line)].push(v);
            on_line[li(gt.line)].push(v);
        }
    }
    // per line, its vertices sorted along it
    for (k, ol) in on_line.iter_mut().enumerate() {
        ol.sort_unstable();
        ol.dedup();
        let l = &lns[k];
        for i in 1..ol.len() {
            let mut j = i;
            while j > 0 && cx.along(vs[ol[j - 1] as usize], vs[ol[j] as usize], l) > 0 {
                ol.swap(j - 1, j);
                j -= 1;
            }
        }
    }
    // 2. sub-edges: per line, the gaps between consecutive vertices a segment spans (ordered
    // along the line: u before w)
    let mut gap_sub: Vec<Vec<u32>> = on_line
        .iter()
        .map(|ol| vec![NONE; ol.len().saturating_sub(1)])
        .collect();
    let mut subs: Vec<Sub> = Vec::new();
    for (g, &(a, b)) in segs.iter().zip(&ends) {
        let k = li(g.line);
        let ol = &on_line[k];
        let (pa, pb) = (
            ol.iter().position(|&x| x == a).unwrap(),
            ol.iter().position(|&x| x == b).unwrap(),
        );
        let (lo, hi, fwd) = if pa <= pb { (pa, pb, true) } else { (pb, pa, false) };
        for q in lo..hi {
            let si = match gap_sub[k][q] {
                NONE => {
                    subs.push(Sub {
                        u: ol[q],
                        w: ol[q + 1],
                        line: g.line,
                        left: 0,
                        right: 0,
                    });
                    gap_sub[k][q] = subs.len() as u32 - 1;
                    subs.len() - 1
                }
                x => x as usize,
            };
            if g.bound.0 != u32::MAX {
                // the face on the segment's left when sense > 0, walking a → b
                let left_of_uw = (g.bound.1 > 0) == fwd;
                let m = 1u64 << g.bound.0;
                if left_of_uw {
                    subs[si].left |= m;
                } else {
                    subs[si].right |= m;
                }
            }
        }
    }
    let ns = subs.len();
    // 3. half-edges 2i (u → w) and 2i + 1 (w → u); per vertex, outgoing ones by angle
    let from = |h: usize| if h & 1 == 0 { subs[h / 2].u } else { subs[h / 2].w };
    let to = |h: usize| if h & 1 == 0 { subs[h / 2].w } else { subs[h / 2].u };
    // a half-edge's direction in (s, t), counter-clockwise about C's normal: the line's
    // direction, reversed for w → u (u is before w along the line)
    let dirs: Vec<(Dy, Dy, f64, f64)> = lns
        .iter()
        .map(|l| {
            let (x, mut y) = (l.d.get(s).clone(), l.d.get(t).clone());
            let (fx, mut fy) = (l.f[s], l.f[t]);
            if flip {
                y = y.neg();
                fy = -fy;
            }
            (x, y, fx, fy)
        })
        .collect();
    let hdir = |h: usize| (li(subs[h / 2].line), h & 1 == 1);
    let half = |h: usize| {
        let (k, rev) = hdir(h);
        let (x, y) = (dirs[k].0.signum(), dirs[k].1.signum());
        let (x, y) = if rev { (-x, -y) } else { (x, y) };
        if y > 0 || (y == 0 && x > 0) { 0 } else { 1 }
    };
    let cmp = |a: usize, b: usize| -> std::cmp::Ordering {
        let (ha, hb) = (half(a), half(b));
        if ha != hb {
            return ha.cmp(&hb);
        }
        let ((ka, ra), (kb, rb)) = (hdir(a), hdir(b));
        let sg = if ra == rb { 1 } else { -1 };
        let (da, db) = (&dirs[ka], &dirs[kb]);
        let v = da.2 * db.3 - da.3 * db.2;
        let bound = 4.0 * f64::EPSILON * ((da.2 * db.3).abs() + (da.3 * db.2).abs());
        let cr = if v.abs() > bound {
            bump(&cx.t, |t| t.angle.0 += 1);
            if v > 0.0 { 1 } else { -1 }
        } else {
            bump(&cx.t, |t| t.angle.1 += 1);
            da.0.mul(&db.1).sub(&da.1.mul(&db.0)).signum()
        } * sg;
        // a before b if a × b > 0
        0.cmp(&cr)
    };
    let nv = vs.len();
    let mut ostart = vec![0u32; nv + 1];
    for h in 0..2 * ns {
        ostart[from(h) as usize + 1] += 1;
    }
    for v in 0..nv {
        ostart[v + 1] += ostart[v];
    }
    let mut out = vec![0usize; 2 * ns];
    let mut fill = ostart.clone();
    for h in 0..2 * ns {
        let v = from(h) as usize;
        out[fill[v] as usize] = h;
        fill[v] += 1;
    }
    for v in 0..nv {
        out[ostart[v] as usize..ostart[v + 1] as usize].sort_by(|&x, &y| cmp(x, y));
    }
    let mut next = vec![0usize; 2 * ns];
    for h in 0..2 * ns {
        let v = to(h) as usize;
        let o = &out[ostart[v] as usize..ostart[v + 1] as usize];
        let i = o.iter().position(|&x| x == h ^ 1).unwrap();
        next[h] = o[(i + o.len() - 1) % o.len()];
    }
    // 4. cycles
    let mut cyc = vec![usize::MAX; 2 * ns];
    let mut cstart: Vec<usize> = Vec::new();
    let mut chs: Vec<usize> = Vec::with_capacity(2 * ns);
    for h in 0..2 * ns {
        if cyc[h] != usize::MAX {
            continue;
        }
        let id = cstart.len();
        cstart.push(chs.len());
        let mut x = h;
        while cyc[x] == usize::MAX {
            cyc[x] = id;
            chs.push(x);
            x = next[x];
        }
    }
    let ncy = cstart.len();
    cstart.push(chs.len());
    let cycle = |i: usize| &chs[cstart[i]..cstart[i + 1]];
    let pid = |v: u32| vs[v as usize];
    let ccw: Vec<bool> = (0..ncy)
        .map(|i| {
            let cy = cycle(i);
            let k = (0..cy.len())
                .min_by(|&x, &y| {
                    let (p, q) = (pid(from(cy[x])), pid(from(cy[y])));
                    cx.cmp_axis(p, q, s).cmp(&0).then(cx.cmp_axis(p, q, t).cmp(&0))
                })
                .unwrap();
            let prev = cy[(k + cy.len() - 1) % cy.len()];
            let o = cx.orient(pid(from(prev)), pid(from(cy[k])), pid(to(cy[k])), s, t);
            (if flip { -o } else { o }) > 0
        })
        .collect();
    // 5. components, and each clockwise cycle's face: the innermost counter-clockwise cycle of
    // another component round it, or the outside
    let mut comp: Vec<u32> = (0..nv as u32).collect();
    fn root(p: &mut [u32], x: u32) -> u32 {
        let mut x = x;
        while p[x as usize] != x {
            p[x as usize] = p[p[x as usize] as usize];
            x = p[x as usize];
        }
        x
    }
    for sb in &subs {
        let (a, b) = (root(&mut comp, sb.u), root(&mut comp, sb.w));
        comp[a.max(b) as usize] = a.min(b);
    }
    let croot: Vec<u32> = (0..ncy).map(|i| root(&mut comp, from(cycle(i)[0]))).collect();
    let in_cycle = |p: u32, cy: &[usize]| {
        let q = &cx.p[p as usize];
        let mut n = 0;
        for &h in cy {
            let (a, b) = (pid(from(h)), pid(to(h)));
            let c = match crosses_f(q.f, cx.p[a as usize].f, cx.p[b as usize].f, s, t) {
                Some(c) => c,
                None => crosses(cx.ex(p), cx.ex(a), cx.ex(b), s, t),
            };
            n += c as usize;
        }
        n % 2 == 1
    };
    const OUTSIDE: usize = usize::MAX - 1;
    let mut face_of = vec![usize::MAX; ncy];
    let mut holding: Vec<usize> = Vec::new();
    for i in 0..ncy {
        if ccw[i] {
            face_of[i] = i;
            continue;
        }
        let p = pid(from(cycle(i)[0]));
        holding.clear();
        holding.extend((0..ncy).filter(|&j| ccw[j] && croot[j] != croot[i] && in_cycle(p, cycle(j))));
        let inner = holding.iter().copied().find(|&j| {
            holding
                .iter()
                .all(|&k| k == j || in_cycle(pid(from(cycle(j)[0])), cycle(k)))
        });
        face_of[i] = inner.unwrap_or(OUTSIDE);
    }
    // 6. holders by propagation from the outside: crossing half-edge h's sub-edge from its left
    // to its right leaves the faces it bounds on its left and enters those on its right
    let cross = |set: u64, h: usize| {
        let sb = &subs[h / 2];
        let (l, r) = if h & 1 == 0 {
            (sb.left, sb.right)
        } else {
            (sb.right, sb.left)
        };
        (set & !l) | r
    };
    let face = |h: usize| face_of[cyc[h]];
    let mut holders: Vec<Option<u64>> = vec![None; ncy];
    let mut queue: Vec<(usize, u64)> = Vec::new();
    for h in 0..2 * ns {
        if face(h) == OUTSIDE {
            let g = face(h ^ 1);
            if g != OUTSIDE {
                queue.push((g, cross(0, h)));
            }
        }
    }
    // the half-edges by face, for the walk
    let mut by_face: Vec<Vec<usize>> = vec![Vec::new(); ncy];
    for h in 0..2 * ns {
        let f = face(h);
        if f != OUTSIDE && f != usize::MAX {
            by_face[f].push(h);
        }
    }
    while let Some((f, set)) = queue.pop() {
        if holders[f].is_some() {
            continue;
        }
        holders[f] = Some(set);
        for &h in &by_face[f] {
            let g = face(h ^ 1);
            if g != OUTSIDE && g != f && holders[g].is_none() {
                queue.push((g, cross(set, h)));
            }
        }
    }
    // 7. facets: faces holding a face of C, inside the region
    let wall_ok = |f: usize| {
        cycle(f).iter().all(|&h| {
            let p = pid(from(h));
            cls.walls.iter().zip(&cls.wall_in).all(|(&w, &si)| {
                let x = cx.side(w, p);
                x == 0 || (x > 0) == (si == Sgn::Pos)
            })
        })
    };
    let _ = pc;
    let nf = (0..ncy)
        .filter(|&f| ccw[f] && holders[f].is_some_and(|h| h != 0) && wall_ok(f))
        .count();
    (nv, nf)
}

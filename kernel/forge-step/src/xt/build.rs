//! A transmit file's solid bodies into the kernel's (XT Format Reference 4.3, 5.3): each body
//! where its instances place it, each solid region of it a solid. The topology goes to the same
//! healing and checks as a STEP file's.

use super::geom::{Geo, R, Xf, refuse};
use super::nodes::{Node, Store};
use crate::heal::{PEdge, PFace, Proto};
use crate::part21::Limits;
use crate::read::{ImportError, Imported, Refusal, Solid, Stats, edge_range, finish};
use forge_geom::CurveGeom;
use forge_math::{Interval, Point3};
use forge_topo::Name;
use std::collections::HashMap;

/// A face's loops: each its edges, with whether each is used along its direction.
type Loops = Vec<Vec<(usize, bool)>>;

/// Most nodes a chain or ring is followed through: past it the file loops.
const MAX_CHAIN: usize = 1 << 24;

/// A body where it stands: the body's node, its placement, and its name.
struct Placed {
    body: u32,
    xf: Xf,
    name: String,
    /// The instance placing it, for a refusal to name.
    instance: u32,
}

fn chain<'s>(s: &'s Store, first: u32, next: &'static str) -> impl Iterator<Item = Node<'s>> + 's {
    let mut at = first;
    let mut n = 0usize;
    std::iter::from_fn(move || {
        if at == 0 || n > MAX_CHAIN {
            return None;
        }
        n += 1;
        let node = s.node(at)?;
        at = node.p(next);
        Some(node)
    })
}

/// A name attribute (SDL/TYSA_NAME, or its unicode twin) on a node.
fn name_of(s: &Store, owner: u32) -> Option<String> {
    let n = s.node(owner)?;
    for a in chain(s, n.p("attributes"), "next") {
        if a.ty() != 81 {
            continue;
        }
        let def = s.node(a.p("definition"))?;
        let id = s.node(def.p("identifier"))?;
        let key: Vec<u8> = id.all("string").iter().map(|&c| c as u8).collect();
        let field = a.all("fields").first().and_then(|&f| s.node(f as u32));
        match (&key[..], field) {
            (b"SDL/TYSA_NAME", Some(v)) if v.ty() == 84 => {
                return Some(v.all("values").iter().map(|&c| c as u8 as char).collect());
            }
            (b"SDL/TYSA_UNAME", Some(v)) if v.ty() == 98 => {
                let w: Vec<u16> = v.all("values").iter().map(|&c| c as u16).collect();
                return Some(String::from_utf16_lossy(&w));
            }
            _ => {}
        }
    }
    None
}

/// The placement a TRANSFORM node makes (5.2.6), its precision node added.
fn transform(s: &Store, idx: u32) -> Result<Xf, Refusal> {
    let Some(t) = s.of(idx, 100) else {
        return refuse(idx, 100, "the instance's transform is missing");
    };
    let m = t.all("rotation_matrix");
    let v = t.all("translation_vector");
    if m.len() != 9 || v.len() != 3 || m.iter().chain(v).any(|x| !x.is_finite()) {
        return refuse(idx, 100, "its matrix or translation is null");
    }
    let mut r = [[m[0], m[1], m[2]], [m[3], m[4], m[5]], [m[6], m[7], m[8]]];
    let mut tv = [v[0], v[1], v[2]];
    if let Some(p) = s.node(t.p("precision")) {
        let (pm, pv) = (p.all("rotation_matrix"), p.all("translation_vector"));
        if pm.len() == 9 && pv.len() == 3 && pm.iter().chain(pv).all(|x| x.is_finite()) {
            for i in 0..3 {
                for j in 0..3 {
                    r[i][j] += pm[3 * i + j];
                }
                tv[i] += pv[i];
            }
        }
    }
    let k = t.f("scale");
    let k = if k.is_finite() && k != 0.0 { k } else { 1.0 };
    // x' = (R x + t) k, R by rows (5.2.6). The corpus settles it: the other way round stands the
    // VF-3 a metre too tall and takes its bolts out of their holes (research/parasolid).
    let xf = Xf { r, t: tv, k };
    Ok(xf)
}

/// Every body the file places, through its assemblies and instances.
fn placements(s: &Store, refusals: &mut Vec<Refusal>, place: bool) -> Vec<Placed> {
    let mut out = Vec::new();
    let Some(root) = s.root() else { return out };
    let mut seen_body = std::collections::HashSet::new();
    let mut part =
        |p: u32, xf: Xf, inst: u32, out: &mut Vec<Placed>, depth: u32, refusals: &mut Vec<Refusal>| {
            #[allow(clippy::too_many_arguments)]
            fn go(
                place: bool,
                s: &Store,
                p: u32,
                xf: Xf,
                inst: u32,
                out: &mut Vec<Placed>,
                depth: u32,
                refusals: &mut Vec<Refusal>,
                seen: &mut std::collections::HashSet<u32>,
            ) {
                if depth > 64 {
                    refusals.push(Refusal {
                        entity: p,
                        kind: "ASSEMBLY".into(),
                        why: "assemblies nested too deep".into(),
                    });
                    return;
                }
                let Some(n) = s.node(p) else { return };
                match n.ty() {
                    12 => {
                        seen.insert(p);
                        let name = name_of(s, inst).or_else(|| name_of(s, p)).unwrap_or_default();
                        out.push(Placed {
                            body: p,
                            xf,
                            name,
                            instance: inst,
                        })
                    }
                    10 => {
                        for i in chain(s, n.p("sub_instance"), "next_in_part") {
                            if i.ty() != 11 {
                                break;
                            }
                            if i.f("type") == 2.0 {
                                refusals.push(Refusal {
                                    entity: i.index,
                                    kind: "INSTANCE".into(),
                                    why: "a negative instance: a part taken away is not done".into(),
                                });
                                continue;
                            }
                            let inner = if i.p("transform") == 0 {
                                Ok(Xf { k: 1.0, ..Xf::MM })
                            } else {
                                transform(s, i.p("transform"))
                            };
                            let inner = if place { inner } else { Ok(Xf { k: 1.0, ..Xf::MM }) };
                            match inner {
                                Ok(t) => go(
                                    place,
                                    s,
                                    i.p("part"),
                                    xf.then(&t),
                                    i.index,
                                    out,
                                    depth + 1,
                                    refusals,
                                    seen,
                                ),
                                Err(r) => refusals.push(r),
                            }
                        }
                    }
                    _ => {}
                }
            }
            go(place, s, p, xf, inst, out, depth, refusals, &mut seen_body);
        };
    match root.ty() {
        12 | 10 => part(1, Xf::MM, 0, &mut out, 0, refusals),
        // several parts: a part list block or, from older writers, a pointer list block
        176 | 74 => {
            for e in root.all("entries") {
                let e = *e as u32;
                if e != 0 {
                    part(e, Xf::MM, 0, &mut out, 0, refusals);
                }
            }
        }
        // a partition: its world's bodies
        101 => {
            for b in chain(s, root.p("body"), "next") {
                part(b.index, Xf::MM, 0, &mut out, 0, refusals);
            }
        }
        _ => {}
    }
    out
}

/// One region's worth of a body as it is built: the proto, and the file's vertices and edges
/// already made.
struct Build<'g, 's> {
    g: &'g mut Geo<'s>,
    p: Proto,
    verts: HashMap<u32, usize>,
    /// Each file edge's proto edges, and whether they run against the file's.
    edges: HashMap<u32, (Vec<usize>, bool)>,
    stats: &'g mut Stats,
    salt: Name,
    /// Faces on blends, whose sense the faces they are tangent to settle.
    blends: Vec<usize>,
}

impl Build<'_, '_> {
    fn s(&self) -> &Store {
        self.g.s
    }

    fn vertex(&mut self, idx: u32) -> R<usize> {
        if let Some(&v) = self.verts.get(&idx) {
            return Ok(v);
        }
        let s = self.g.s;
        let Some(v) = s.of(idx, 18) else {
            return refuse(idx, 18, "the vertex is missing");
        };
        let Some(p) = s.of(v.p("point"), 29).and_then(|p| p.v("pvec")) else {
            return refuse(idx, 18, "its point is missing");
        };
        let at = self.g.xf.point(p);
        let tol = v.f("tolerance");
        self.p.verts.push((at, self.salt.with(u64::from(idx))));
        let i = self.p.verts.len() - 1;
        if tol.is_finite() && tol > 0.0 {
            self.p.vtol.resize(i, 0.0);
            self.p.vtol.push(self.g.xf.len(tol));
        }
        self.verts.insert(idx, i);
        Ok(i)
    }

    /// A file edge as proto edges in its order, and whether they run against the file's edge. A
    /// ring edge (a closed curve with no vertex) is made two halves: healing may give a closed
    /// edge's vertex another place for each face it bounds, and a rim shared by two closed faces
    /// would be pulled both ways.
    fn edge(&mut self, idx: u32) -> R<(Vec<usize>, bool)> {
        if let Some(e) = self.edges.get(&idx) {
            return Ok(e.clone());
        }
        let s = self.g.s;
        let Some(ed) = s.of(idx, 16) else {
            return refuse(idx, 16, "the edge is missing");
        };
        // the fins round the edge: the positive one's vertex is its end, the negative's its start
        let (mut start, mut end) = (0u32, 0u32);
        let mut fin_curves: Vec<u32> = Vec::new();
        let first = ed.p("fin");
        let mut at = first;
        for _ in 0..1024 {
            let Some(f) = s.of(at, 17) else { break };
            match f.c("sense") {
                b'+' if end == 0 => end = f.p("vertex"),
                b'-' if start == 0 => start = f.p("vertex"),
                _ => {}
            }
            if f.p("curve") != 0 {
                fin_curves.push(f.p("curve"));
            }
            at = f.p("other");
            if at == first || at == 0 {
                break;
            }
        }
        // the edge's curve and which way it runs along it; a tolerant edge's from a fin, the
        // first whose surface reads
        let (curve, against, _) = match ed.p("curve") {
            0 => {
                let mut got = None;
                let mut first_err = None;
                for &c in &fin_curves {
                    match self.g.curve(c) {
                        Ok(k) => {
                            got = Some((k, false, c));
                            break;
                        }
                        Err(e) => {
                            first_err.get_or_insert(e);
                        }
                    }
                }
                match (got, first_err) {
                    (Some(g), _) => g,
                    (None, Some(e)) => return Err(e),
                    (None, None) => return refuse(idx, 16, "no curve, on the edge or its fins"),
                }
            }
            c => (self.g.curve(c)?, against_curve(s, c), c),
        };
        let name = self.salt.with(u64::from(idx));
        if start == 0 && end == 0 {
            let d = curve.domain();
            let whole = match d.period {
                Some(p) => Interval::new(d.lo, d.lo + p),
                None => Interval::new(d.lo, d.hi),
            };
            if !(whole.width() > 0.0 && whole.width().is_finite()) {
                return refuse(idx, 16, "a ring edge on a curve that does not close");
            }
            let mid = whole.mid();
            let (p0, p1) = (curve.eval(whole.lo), curve.eval(mid));
            self.p
                .verts
                .push((p0, self.salt.with_str("ring0").with(u64::from(idx))));
            self.p
                .verts
                .push((p1, self.salt.with_str("ring1").with(u64::from(idx))));
            let (v0, v1) = (self.p.verts.len() - 2, self.p.verts.len() - 1);
            self.p.edges.push(PEdge {
                curve: curve.clone(),
                range: Interval::new(whole.lo, mid),
                start: v0,
                end: v1,
                name: name.with(0u64),
            });
            self.p.edges.push(PEdge {
                curve,
                range: Interval::new(mid, whole.hi),
                start: v1,
                end: v0,
                name: name.with(1u64),
            });
            let n = self.p.edges.len();
            self.stats.edges += 2;
            let e = (vec![n - 2, n - 1], against);
            self.edges.insert(idx, e.clone());
            return Ok(e);
        }
        if start == 0 || end == 0 {
            return refuse(idx, 16, "one vertex: an edge has both or neither");
        }
        let (a, b) = (self.vertex(start)?, self.vertex(end)?);
        // the proto edge runs the way its curve's parameter increases
        let (s0, t0, flipped) = if against { (b, a, true) } else { (a, b, false) };
        // on a curve closed in fact whose parameter does not wrap (a periodic B-curve), an edge
        // running with it past its closing point: two edges, met there
        let d = curve.domain();
        if d.period.is_none() && d.lo.is_finite() && d.hi.is_finite() && s0 != t0 {
            let lin = forge_math::tol::LINEAR;
            let (e0, e1) = (curve.eval(d.lo), curve.eval(d.hi));
            let (pa, pb) = (self.p.verts[s0].0, self.p.verts[t0].0);
            if e0.distance(e1) <= lin && pa.distance(e0) > lin && pb.distance(e0) > lin {
                let whole = Interval::new(d.lo, d.hi);
                let (ta, tb) = (curve.closest(pa, whole), curve.closest(pb, whole));
                if tb < ta {
                    let off = curve.eval(ta).distance(pa).max(curve.eval(tb).distance(pb));
                    self.stats.vertex_off_curve = self.stats.vertex_off_curve.max(off);
                    if off > lin {
                        self.stats.vertices_off += 1;
                    }
                    self.p
                        .verts
                        .push((e0, self.salt.with_str("closing").with(u64::from(idx))));
                    let vs = self.p.verts.len() - 1;
                    self.p.edges.push(PEdge {
                        curve: curve.clone(),
                        range: Interval::new(ta, d.hi),
                        start: s0,
                        end: vs,
                        name: name.with(0u64),
                    });
                    self.p.edges.push(PEdge {
                        curve,
                        range: Interval::new(d.lo, tb),
                        start: vs,
                        end: t0,
                        name: name.with(1u64),
                    });
                    let n = self.p.edges.len();
                    self.stats.edges += 2;
                    let e = (vec![n - 2, n - 1], flipped);
                    self.edges.insert(idx, e.clone());
                    return Ok(e);
                }
            }
        }
        let (range, off, back) = edge_range(&curve, self.p.verts[s0].0, self.p.verts[t0].0, s0 == t0);
        let (s0, t0, flipped) = if back {
            (t0, s0, !flipped)
        } else {
            (s0, t0, flipped)
        };
        self.stats.vertex_off_curve = self.stats.vertex_off_curve.max(off);
        if off > forge_math::tol::LINEAR {
            self.stats.vertices_off += 1;
        }
        self.p.edges.push(PEdge {
            curve,
            range,
            start: s0,
            end: t0,
            name,
        });
        let e = (vec![self.p.edges.len() - 1], flipped);
        self.stats.edges += 1;
        self.edges.insert(idx, e.clone());
        Ok(e)
    }

    /// A face, its normal out of the region when `out`.
    fn face(&mut self, f: &Node, shell: usize, out: bool) -> R<()> {
        let s = self.g.s;
        let idx = f.index;
        let (surface, agree) = self.g.surface(f.p("surface"))?;
        if self.g.is_blend(f.p("surface")) {
            self.blends.push(self.p.faces.len());
        }
        let same = (agree == (f.c("sense") != b'-')) == out;
        let mut loops = Vec::new();
        let mut poles: Vec<u32> = Vec::new();
        for l in chain(s, f.p("loop"), "next") {
            if l.ty() != 15 {
                return refuse(l.index, l.ty(), "not a loop");
            }
            let first = l.p("fin");
            let mut uses: Vec<(usize, bool)> = Vec::new();
            let mut at = first;
            let mut isolated = false;
            for k in 0..=MAX_CHAIN {
                if k == MAX_CHAIN {
                    return refuse(l.index, 15, "its fins do not come round");
                }
                let Some(fin) = s.of(at, 17) else {
                    return refuse(l.index, 15, "a fin is missing");
                };
                let e = fin.p("edge");
                if e == 0 {
                    // an isolated loop: a vertex alone (an apex or a pole); seams reach it in healing
                    isolated = true;
                    if fin.p("vertex") != 0 {
                        poles.push(fin.p("vertex"));
                    }
                    break;
                }
                let (pe, flipped) = self.edge(e)?;
                let fw = (fin.c("sense") == b'+') != flipped;
                if fw {
                    uses.extend(pe.iter().map(|&x| (x, true)));
                } else {
                    uses.extend(pe.iter().rev().map(|&x| (x, false)));
                }
                at = fin.p("forward");
                if at == first {
                    break;
                }
            }
            if isolated {
                continue;
            }
            if !out {
                uses.reverse();
                for u in &mut uses {
                    u.1 = !u.1;
                }
            }
            loops.push(uses);
        }
        // a spindle torus's cap, its rim and its pole on the axis: healing knows no pole of a
        // torus, so the meridian from the rim to the pole is made here
        if let (forge_geom::Surface::Torus(t), [pole], [rim]) = (&surface, &poles[..], &mut loops[..])
            && t.is_spindle()
        {
            let pv = self.vertex(*pole)?;
            if let Some((seam, from_rim)) = self.meridian(t, rim, pv) {
                rim.push((seam, from_rim));
                rim.push((seam, !from_rim));
            }
        }
        if loops.is_empty()
            && let forge_geom::Surface::Torus(t) = &surface
        {
            let bands = self.whole_torus(t, idx)?;
            for (k, mut band) in bands.into_iter().enumerate() {
                if !same {
                    for l in &mut band {
                        l.reverse();
                        for u in l.iter_mut() {
                            u.1 = !u.1;
                        }
                    }
                }
                self.p.faces.push(PFace {
                    surface: surface.clone(),
                    same_sense: same,
                    name: self.salt.with(u64::from(idx)).with_str("band").with(k as u64),
                    shell,
                    loops: band,
                });
            }
            self.stats.faces += 1;
            return Ok(());
        }
        self.p.faces.push(PFace {
            surface,
            same_sense: same,
            name: self.salt.with(u64::from(idx)),
            shell,
            loops,
        });
        self.stats.faces += 1;
        Ok(())
    }

    /// The arc of a spindle torus's tube circle from the rim loop's first vertex to the pole, the
    /// way that stays on the torus's used part (a + b cos v >= 0).
    /// The edge, and whether it runs from the rim's vertex.
    fn meridian(
        &mut self,
        t: &forge_geom::Torus,
        rim: &[(usize, bool)],
        pole: usize,
    ) -> Option<(usize, bool)> {
        use forge_geom::{Circle, Curve};
        use forge_math::{Frame, Interval, Vec3};
        let &(e0, fw0) = rim.first()?;
        let va = if fw0 {
            self.p.edges[e0].start
        } else {
            self.p.edges[e0].end
        };
        let (pa, pp) = (self.p.verts[va].0, self.p.verts[pole].0);
        let f = &t.frame;
        let l = f.point_to_local(pa);
        let r = Vec3::from(f.x()) * l.x + Vec3::from(f.y()) * l.y;
        let r = r.normalize()?;
        let z: Vec3 = f.z().into();
        let c = f.origin + Vec3::from(r) * t.major;
        let frame = Frame::from_zx(c, Vec3::from(r).cross(z).normalize()?, r.into())?;
        let circle = Curve::Circle(Circle {
            frame,
            radius: t.minor,
        });
        let at = |p: forge_math::Point3| {
            let q = frame.point_to_local(p);
            forge_math::dmath::atan2(q.y, q.x)
        };
        let (va_t, vp_t) = (at(pa), at(pp));
        // either way round: the one whose middle stays where the torus is used
        let tau = forge_math::dmath::TAU;
        let up = (vp_t - va_t).rem_euclid(tau);
        let mid_up = va_t + 0.5 * up;
        let used = |v: f64| t.major + t.minor * forge_math::dmath::cos(v) >= 0.0;
        let (start, end, range) = if used(mid_up) {
            (va, pole, Interval::new(va_t, va_t + up))
        } else {
            (pole, va, Interval::new(vp_t, vp_t + (tau - up)))
        };
        self.p.edges.push(PEdge {
            curve: circle,
            range,
            start,
            end,
            name: self.salt.with_str("meridian").with(va as u64),
        });
        let _ = end;
        Some((self.p.edges.len() - 1, start == va))
    }

    /// A whole torus face, which a file bounds with nothing, as two bands round the axis, out
    /// and in, met at its outer and inner equators: the circles round the axis at v = 0 and pi,
    /// each made two halves as a ring edge is. Each band's loops wind as a cylinder's rims do.
    fn whole_torus(&mut self, t: &forge_geom::Torus, idx: u32) -> R<[Loops; 2]> {
        use forge_geom::{Circle, Curve};
        use forge_math::Interval;
        if t.minor <= forge_math::tol::LINEAR || t.major <= t.minor + forge_math::tol::LINEAR {
            return refuse(idx, 14, "a whole torus that reaches its axis");
        }
        let pi = forge_math::dmath::PI;
        let mut rim = |radius: f64, k: u64| -> [usize; 2] {
            let c = Curve::Circle(Circle {
                frame: t.frame,
                radius,
            });
            let p0 = c.eval(0.0);
            let p1 = c.eval(pi);
            self.p
                .verts
                .push((p0, self.salt.with_str("rim0").with(u64::from(idx)).with(k)));
            self.p
                .verts
                .push((p1, self.salt.with_str("rim1").with(u64::from(idx)).with(k)));
            let (v0, v1) = (self.p.verts.len() - 2, self.p.verts.len() - 1);
            for (h, (r, a, b)) in [
                (Interval::new(0.0, pi), v0, v1),
                (Interval::new(pi, 2.0 * pi), v1, v0),
            ]
            .into_iter()
            .enumerate()
            {
                self.p.edges.push(PEdge {
                    curve: c.clone(),
                    range: r,
                    start: a,
                    end: b,
                    name: self
                        .salt
                        .with_str("rim")
                        .with(u64::from(idx))
                        .with(k)
                        .with(h as u64),
                });
            }
            let n = self.p.edges.len();
            [n - 2, n - 1]
        };
        let outer = rim(t.major + t.minor, 0);
        let inner = rim(t.major - t.minor, 1);
        let fw = |e: [usize; 2]| vec![(e[0], true), (e[1], true)];
        let back = |e: [usize; 2]| vec![(e[1], false), (e[0], false)];
        // v from 0 to pi: up the outer equator's side, back along the inner; then on round
        Ok([vec![fw(outer), back(inner)], vec![fw(inner), back(outer)]])
    }

    /// A shell of a solid region: its back faces, whose normals point out of the solid.
    fn shell(&mut self, sh: &Node) -> R<()> {
        let s = self.g.s;
        let shell = self.p.shells;
        self.p.shells += 1;
        for f in chain(s, sh.p("face"), "next") {
            self.face(&f, shell, true)?;
        }
        if let Some(f) = chain(s, sh.p("front_face"), "next_front").next() {
            return refuse(
                f.index,
                14,
                "a face with solid on its front: a sheet inside a solid",
            );
        }
        let _ = self.s();
        Ok(())
    }
}

/// `place`: each body where its instances put it; else each where it stands in its own part, once
/// per instance.
/// Whether an edge runs against its curve: the curve's sense, and a trimmed curve's basis's too
/// (a trimmed curve is positive, and runs as its basis does).
fn against_curve(s: &Store, c: u32) -> bool {
    let mut against = false;
    let mut at = c;
    for _ in 0..16 {
        let Some(n) = s.node(at) else { break };
        against ^= n.c("sense") == b'-';
        if n.ty() != 133 {
            break;
        }
        at = n.p("basis_curve");
    }
    against
}

/// A rolling-ball face meets the faces it rounds tangentially, so outside it faces the way they
/// do along those edges: each blend face takes the sense its tangent neighbours agree on.
fn orient_blends(p: &mut Proto, blends: &[usize]) {
    use forge_geom::SurfaceGeom;
    if blends.is_empty() {
        return;
    }
    let unsure: std::collections::HashSet<usize> = blends.iter().copied().collect();
    let mut uses: HashMap<usize, Vec<usize>> = HashMap::new();
    for (f, face) in p.faces.iter().enumerate() {
        for l in &face.loops {
            for &(e, _) in l {
                uses.entry(e).or_default().push(f);
            }
        }
    }
    // du x dv, as faces are measured and drawn by: a pipe's `normal` need not be it
    let normal = |p: &Proto, f: usize, at: Point3| {
        let face = &p.faces[f];
        let d = face.surface.deriv(face.surface.project(at));
        let n = d.du.cross(d.dv).normalize()?;
        let n: forge_math::Vec3 = n.into();
        Some(if face.same_sense { n } else { -n })
    };
    let nf = p.faces.len();
    for &f in blends.iter().filter(|&&f| f < nf) {
        let mut vote = 0i32;
        for l in &p.faces[f].loops {
            for &(e, _) in l {
                let Some(others) = uses.get(&e) else { continue };
                for &g in others {
                    if g == f || unsure.contains(&g) {
                        continue;
                    }
                    let ed = &p.edges[e];
                    let at = ed.curve.eval(ed.range.mid());
                    if let (Some(a), Some(b)) = (normal(p, f, at), normal(p, g, at)) {
                        let d = a.dot(b);
                        if d.abs() > 0.9 {
                            vote += if d > 0.0 { 1 } else { -1 };
                        }
                    }
                }
            }
        }
        if vote < 0 {
            p.faces[f].same_sense = !p.faces[f].same_sense;
        }
    }
}

pub(crate) fn import(b: &[u8], lim: &Limits, place: bool) -> Result<Imported, ImportError> {
    let s = super::nodes::read(b, lim).map_err(ImportError::Xt)?;
    let mut refusals = Vec::new();
    let mut solids = Vec::new();
    let mut stats = Stats {
        units_mm: 1000.0,
        ..Stats::default()
    };
    let placed = placements(&s, &mut refusals, place);
    for (k, pl) in placed.iter().enumerate() {
        let Some(body) = s.of(pl.body, 12) else { continue };
        if !pl.xf.rigid(1e-6) {
            refusals.push(Refusal {
                entity: if pl.instance != 0 { pl.instance } else { pl.body },
                kind: "INSTANCE".into(),
                why: "placed by a transform that mirrors or skews: only rotations and moves are done".into(),
            });
            continue;
        }
        match body.int("body_type") {
            Some(1) => {}
            t => {
                let what = match t {
                    Some(2) => "a wire body",
                    Some(3) => "a sheet body",
                    Some(6) => "a general body",
                    _ => "a body of unknown type",
                };
                refusals.push(Refusal {
                    entity: pl.body,
                    kind: "BODY".into(),
                    why: format!("{what}: only solids are read"),
                });
                continue;
            }
        }
        let res = forge_math::tol::LINEAR.max(pl.xf.len(body.f("res_linear")).abs());
        let mut geo = Geo::new(&s, pl.xf);
        for region in chain(&s, body.p("region"), "next") {
            if region.ty() != 19 || region.c("type") != b'S' {
                continue;
            }
            let salt = Name::feature(u64::from(region.index))
                .with_str("xt")
                .with(k as u64);
            let mut b = Build {
                g: &mut geo,
                p: Proto {
                    salt,
                    ..Proto::default()
                },
                verts: HashMap::new(),
                edges: HashMap::new(),
                stats: &mut stats,
                salt,
                blends: Vec::new(),
            };
            let built = (|| {
                for sh in chain(&s, region.p("shell"), "next") {
                    b.shell(&sh)?;
                }
                Ok(())
            })();
            let blends = std::mem::take(&mut b.blends);
            let mut proto = b.p;
            if built.is_ok() {
                orient_blends(&mut proto, &blends);
            }
            if proto.vtol.len() < proto.verts.len() {
                proto.vtol.resize(proto.verts.len(), 0.0);
            }
            match built {
                Ok(()) => match finish(proto, res, lim, &mut stats) {
                    Ok((body, mass)) => solids.push(Solid {
                        entity: pl.body,
                        name: pl.name.clone(),
                        body,
                        mass,
                    }),
                    Err(why) => refusals.push(Refusal {
                        entity: region.index,
                        kind: "REGION".into(),
                        why,
                    }),
                },
                Err(r) => refusals.push(r),
            }
        }
    }
    let _ = Point3::ORIGIN;
    if solids.is_empty() && refusals.is_empty() {
        return Err(ImportError::NoSolids);
    }
    stats.uncertainty = forge_math::tol::LINEAR;
    let system = s
        .header
        .key("APPL")
        .map(|a| match s.header.key("FRU") {
            Some(v) if !v.is_empty() => format!("{a} ({v})"),
            _ => a.to_string(),
        })
        .unwrap_or_default();
    Ok(Imported {
        solids,
        refusals,
        schema: s.header.schema.clone(),
        system,
        stats,
    })
}

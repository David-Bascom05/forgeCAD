//! A frame of members: where their ends meet (nodes), which member trims which there (one rule),
//! the planes each end is cut on, how far each member's solid must run past its path to be cut
//! there, and the cut list, all exact from the section's support function.
//!
//! A member's frame along its path: z the path's tangent, y `up` (the normal of the sketch the
//! segment lies in), x = y × z (the path's left in its sketch). The section's (x, y) are those.
//!
//! The rule at a node. The members meeting there are ranked: members passing through it first,
//! then the ones named in `rank`, then the rest in layout order. The first two, where both end
//! there, meet by the node's corner (miter or butt; two meeting in line, square). Every other
//! member is butted to each member ranked before it. A butted member ends on the plane that
//! touches the other's outline square to the other's axis, on its own side, `gap` off it;
//! in a butt corner the first runs on to the plane touching the second's outline on its far
//! side. A miter cuts both on the plane bisecting their directions, each `gap / 2` back.

use crate::section::Section;
use forge_math::{Point3, UnitVec3, Vec2, Vec3, dmath, tol};

/// A member's path: a line, or an arc about `axis` (counter-clockwise looking down it) from
/// `start`, `sweep` > 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Path {
    Line {
        a: Point3,
        b: Point3,
    },
    Arc {
        center: Point3,
        axis: UnitVec3,
        start: Point3,
        sweep: f64,
    },
}

impl Path {
    pub fn len(&self) -> f64 {
        match *self {
            Path::Line { a, b } => a.distance(b),
            Path::Arc {
                center, start, sweep, ..
            } => center.distance(start) * sweep,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() <= tol::MIN_LENGTH
    }
    fn arc_axes(&self) -> Option<(Point3, f64, Vec3, Vec3, f64)> {
        match *self {
            Path::Arc {
                center,
                axis,
                start,
                sweep,
            } => {
                let r = center.distance(start);
                let e1 = (start - center) / r;
                let e2 = axis.cross(e1);
                Some((center, r, e1, e2, sweep))
            }
            _ => None,
        }
    }
    /// The point at parameter `t` (a length along a line, an angle along an arc).
    pub fn point(&self, t: f64) -> Point3 {
        match *self {
            Path::Line { a, b } => a + (b - a) * (t / a.distance(b)),
            _ => {
                let (c, r, e1, e2, _) = self.arc_axes().expect("arc");
                let (s, co) = dmath::sin_cos(t);
                c + (e1 * co + e2 * s) * r
            }
        }
    }
    /// The unit tangent at parameter `t`.
    pub fn tangent(&self, t: f64) -> Vec3 {
        match *self {
            Path::Line { a, b } => (b - a) / a.distance(b),
            _ => {
                let (_, _, e1, e2, _) = self.arc_axes().expect("arc");
                let (s, co) = dmath::sin_cos(t);
                e2 * co - e1 * s
            }
        }
    }
    /// The parameter at the path's end.
    pub fn end_t(&self) -> f64 {
        match *self {
            Path::Line { a, b } => a.distance(b),
            Path::Arc { sweep, .. } => sweep,
        }
    }
    pub fn ends(&self) -> [Point3; 2] {
        [self.point(0.0), self.point(self.end_t())]
    }
}

/// One member of a frame: its path, `up` (its sketch's normal, square to the path), and its
/// section placed in the member's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct MemberSpec {
    pub path: Path,
    pub up: UnitVec3,
    pub section: Section,
}

impl MemberSpec {
    /// The member's (x, y, z) at path parameter `t`.
    pub fn axes(&self, t: f64) -> (Vec3, Vec3, Vec3) {
        let z = self.path.tangent(t);
        let y: Vec3 = self.up.into();
        (y.cross(z), y, z)
    }
    /// How far the section reaches along world direction `w` at parameter `t`, from the path.
    pub fn support(&self, t: f64, w: Vec3) -> f64 {
        let (x, y, _) = self.axes(t);
        self.section.support(Vec2::new(w.dot(x), w.dot(y)))
    }
}

/// How two members meet at a node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Corner {
    Miter { gap: f64 },
    Butt { gap: f64 },
}

impl Corner {
    pub fn gap(self) -> f64 {
        match self {
            Corner::Miter { gap } | Corner::Butt { gap } => gap,
        }
    }
}

/// A plane an end is cut on: what lies beyond it along `normal` is taken away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    pub point: Point3,
    pub normal: UnitVec3,
}

/// Which end of a member.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum End {
    Start,
    End,
}

/// A member at a node: ending there, or passing through it at parameter `t`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum At {
    End(End),
    Through(f64),
}

/// Where members meet.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub point: Point3,
    /// The members there, ranked (the rule above).
    pub members: Vec<(usize, At)>,
    pub corner: Corner,
}

/// Why a frame cannot be made.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameError {
    /// A member whose path has no length.
    Empty { member: usize },
    /// `up` is not square to the path.
    NotSquare { member: usize },
    /// Butting `member` to `to`: they run in line, no face of one faces the other.
    ButtParallel { member: usize, to: usize },
    /// A miter of two members folding back onto each other.
    MiterTooSharp { member: usize, to: usize },
    /// Two members pass through one node: neither ends to be trimmed.
    Crossing { member: usize, to: usize },
    /// A cut plane running along the member, or facing back into it.
    CutAlong { member: usize, end: End },
    /// The cuts at the two ends meet inside the member.
    CutAway { member: usize },
    /// An arc member whose end cut lies too far round its arc to be reached within a quarter
    /// turn, or whose radius is no larger than its section's.
    ArcTooTight { member: usize },
    /// The rank names no member of the frame.
    NoSuchMember { index: usize },
}

/// The ends of every member: each end's cuts, by member.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub nodes: Vec<Node>,
    /// Per member, the cuts at its start and at its end.
    pub cuts: Vec<[Vec<Cut>; 2]>,
}

fn unit(v: Vec3) -> Option<UnitVec3> {
    (v.length() > 1e-12).then(|| v.normalize()).flatten()
}

/// Where a point lies on a path's interior, if within the tolerance of it: its parameter.
fn on_interior(p: &Path, q: Point3, eps: f64) -> Option<f64> {
    match *p {
        Path::Line { a, b } => {
            let l = a.distance(b);
            let d = (b - a) / l;
            let s = (q - a).dot(d);
            let foot = a + d * s;
            (s > eps && s < l - eps && foot.distance(q) <= eps).then_some(s)
        }
        Path::Arc { .. } => {
            let (c, r, e1, e2, sw) = p.arc_axes()?;
            let v = q - c;
            let (x, y) = (v.dot(e1), v.dot(e2));
            let off = v - e1 * x - e2 * y;
            let rho = (x * x + y * y).sqrt();
            let mut t = dmath::atan2(y, x);
            if t < 0.0 {
                t += dmath::TAU;
            }
            let ok = (rho - r).abs() <= eps && off.length() <= eps && t * r > eps && (sw - t) * r > eps;
            ok.then_some(t)
        }
    }
}

/// The frame's nodes and every member's end cuts. `corners`: the corner at a node at that point
/// (else `corner`); `rank`: members ranked before the rest at every node they share.
pub fn plan(
    members: &[MemberSpec],
    corner: Corner,
    corners: &[(Point3, Corner)],
    rank: &[usize],
) -> Result<Plan, FrameError> {
    let eps = tol::LINEAR;
    for (k, m) in members.iter().enumerate() {
        if m.path.is_empty() {
            return Err(FrameError::Empty { member: k });
        }
        for t in [0.0, m.path.end_t()] {
            if m.path.tangent(t).dot(m.up.into()).abs() > 1e-9 {
                return Err(FrameError::NotSquare { member: k });
            }
        }
    }
    if let Some(&index) = rank.iter().find(|&&r| r >= members.len()) {
        return Err(FrameError::NoSuchMember { index });
    }
    // ends within the tolerance of each other are one node (union-find: every pair decided
    // once, by one rule)
    let ends: Vec<(usize, End, Point3)> = members
        .iter()
        .enumerate()
        .flat_map(|(k, m)| {
            let [a, b] = m.path.ends();
            [(k, End::Start, a), (k, End::End, b)]
        })
        .collect();
    let mut root: Vec<usize> = (0..ends.len()).collect();
    fn find(root: &mut [usize], i: usize) -> usize {
        let mut i = i;
        while root[i] != i {
            root[i] = root[root[i]];
            i = root[i];
        }
        i
    }
    for i in 0..ends.len() {
        for j in i + 1..ends.len() {
            if ends[i].2.distance(ends[j].2) <= eps {
                let (a, b) = (find(&mut root, i), find(&mut root, j));
                root[a.max(b)] = a.min(b);
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut seen: Vec<(usize, usize)> = Vec::new();
    for i in 0..ends.len() {
        let r = find(&mut root, i);
        match seen.iter().find(|s| s.0 == r) {
            Some(&(_, g)) => groups[g].push(i),
            None => {
                seen.push((r, groups.len()));
                groups.push(vec![i]);
            }
        }
    }
    let place = |k: usize| rank.iter().position(|&r| r == k).unwrap_or(rank.len());
    let mut nodes = Vec::new();
    for g in groups {
        let point = ends[g[0]].2;
        let mut at: Vec<(usize, At)> = g.iter().map(|&i| (ends[i].0, At::End(ends[i].1))).collect();
        for (k, m) in members.iter().enumerate() {
            if at.iter().any(|a| a.0 == k) {
                continue;
            }
            if let Some(t) = on_interior(&m.path, point, eps) {
                at.push((k, At::Through(t)));
            }
        }
        at.sort_by_key(|&(k, a)| {
            (
                !matches!(a, At::Through(_)),
                place(k),
                k,
                matches!(a, At::End(End::End)),
            )
        });
        let corner = corners
            .iter()
            .find(|c| c.0.distance(point) <= eps)
            .map_or(corner, |c| c.1);
        nodes.push(Node {
            point,
            members: at,
            corner,
        });
    }
    let mut cuts: Vec<[Vec<Cut>; 2]> = members.iter().map(|_| [Vec::new(), Vec::new()]).collect();
    for n in &nodes {
        node_cuts(members, n, &mut cuts)?;
    }
    for k in 0..members.len() {
        reach(&members[k], &cuts[k], k)?;
    }
    Ok(Plan { nodes, cuts })
}

/// The parameter of a member at a node.
fn param(m: &MemberSpec, a: At) -> f64 {
    match a {
        At::End(End::Start) => 0.0,
        At::End(End::End) => m.path.end_t(),
        At::Through(t) => t,
    }
}

/// The direction from the node into the member (along the path for its start, back for its
/// end), and the member's axis there.
fn into(m: &MemberSpec, a: At) -> (Vec3, Vec3) {
    let t = param(m, a);
    let z = m.path.tangent(t);
    match a {
        At::End(End::End) => (-z, z),
        _ => (z, z),
    }
}

fn slot(a: At) -> usize {
    match a {
        At::End(End::End) => 1,
        _ => 0,
    }
}

fn node_cuts(members: &[MemberSpec], n: &Node, cuts: &mut [[Vec<Cut>; 2]]) -> Result<(), FrameError> {
    let ms = &n.members;
    let p = n.point;
    let gap = n.corner.gap();
    // `b` ends on the plane touching `a`'s outline on b's side, gap off it
    let butt = |b: (usize, At), a: (usize, At), gap: f64| -> Result<Cut, FrameError> {
        let (ub, _) = into(&members[b.0], b.1);
        let (_, aa) = into(&members[a.0], a.1);
        let w = unit(ub - aa * ub.dot(aa)).ok_or(FrameError::ButtParallel { member: b.0, to: a.0 })?;
        let h = members[a.0].support(param(&members[a.0], a.1), w.into());
        Ok(Cut {
            point: p + w * (h + gap),
            normal: -w,
        })
    };
    let mut done = 1;
    if let [(k1, At::End(e1)), (k2, At::End(e2)), ..] = ms[..] {
        done = 2;
        let (u1, _) = into(&members[k1], At::End(e1));
        let (u2, _) = into(&members[k2], At::End(e2));
        let in_line = (u1 + u2).length() <= 1e-9;
        match n.corner {
            _ if in_line => {
                let w = unit(u1).expect("a unit direction");
                cuts[k1][slot(At::End(e1))].push(Cut {
                    point: p + w * (gap / 2.0),
                    normal: -w,
                });
                cuts[k2][slot(At::End(e2))].push(Cut {
                    point: p - w * (gap / 2.0),
                    normal: w,
                });
            }
            Corner::Miter { gap } => {
                let m = unit(u1 - u2).ok_or(FrameError::MiterTooSharp { member: k2, to: k1 })?;
                cuts[k1][slot(At::End(e1))].push(Cut {
                    point: p + m * (gap / 2.0),
                    normal: -m,
                });
                cuts[k2][slot(At::End(e2))].push(Cut {
                    point: p - m * (gap / 2.0),
                    normal: m,
                });
            }
            Corner::Butt { gap } => {
                // the first runs on, flush with the second's far side
                let w =
                    unit(-u1 - u2 * (-u1).dot(u2)).ok_or(FrameError::ButtParallel { member: k2, to: k1 })?;
                let h = members[k2].support(param(&members[k2], At::End(e2)), w.into());
                cuts[k1][slot(At::End(e1))].push(Cut {
                    point: p + w * h,
                    normal: w,
                });
                let c = butt((k2, At::End(e2)), (k1, At::End(e1)), gap)?;
                cuts[k2][slot(At::End(e2))].push(c);
            }
        }
    }
    for j in done..ms.len() {
        for i in 0..j {
            if let At::Through(_) = ms[j].1 {
                return Err(FrameError::Crossing {
                    member: ms[j].0,
                    to: ms[i].0,
                });
            }
            let c = butt(ms[j], ms[i], gap)?;
            cuts[ms[j].0][slot(ms[j].1)].push(c);
        }
    }
    Ok(())
}

/// For a straight member from `a` along `d`: where the plane of a cut crosses the line through
/// section point offset `x`, as a length along `d`: (n.(q - a) - n.x) / n.d.
fn crossing(a: Point3, d: Vec3, c: &Cut, x: Vec3) -> f64 {
    let n: Vec3 = c.normal.into();
    (n.dot(c.point - a) - n.dot(x)) / n.dot(d)
}

/// The extreme crossings of a cut over the section, along `d` from `a`: (the far one, past
/// which nothing of the member is kept; the near one, short of which all of it is).
fn crossings(m: &MemberSpec, t: f64, a: Point3, d: Vec3, c: &Cut) -> (f64, f64) {
    let n: Vec3 = c.normal.into();
    let nd = n.dot(d);
    let base = n.dot(c.point - a);
    // n.x ranges over [-support(-n), support(n)] on the section
    let far = (base + m.support(t, -n)) / nd;
    let near = (base - m.support(t, n)) / nd;
    (far, near)
}

/// How far member `k`'s solid must run on past each end of its path to be cut there (lengths
/// for a line, angles for an arc), checking every cut crosses the member properly and the cuts
/// of its two ends leave something between them.
pub fn reach(m: &MemberSpec, cuts: &[Vec<Cut>; 2], k: usize) -> Result<[f64; 2], FrameError> {
    match m.path {
        Path::Line { a, b } => {
            let l = a.distance(b);
            let d = (b - a) / l;
            let mut ext = [0.0f64; 2];
            let mut last_start_near = f64::NEG_INFINITY;
            let mut first_end_near = f64::INFINITY;
            for (s, end) in [(0, End::Start), (1, End::End)] {
                let t = if s == 0 { 0.0 } else { l };
                for c in &cuts[s] {
                    let nd = Vec3::from(c.normal).dot(d);
                    let facing = if s == 0 { nd < -1e-9 } else { nd > 1e-9 };
                    if !facing {
                        return Err(FrameError::CutAlong { member: k, end });
                    }
                    let (far, near) = crossings(m, t, a, d, c);
                    if s == 0 {
                        ext[0] = ext[0].max(-far);
                        last_start_near = last_start_near.max(near);
                    } else {
                        ext[1] = ext[1].max(far - l);
                        first_end_near = first_end_near.min(near);
                    }
                }
            }
            let lo = if cuts[0].is_empty() { 0.0 } else { last_start_near };
            let hi = if cuts[1].is_empty() { l } else { first_end_near };
            if hi - lo <= tol::LINEAR {
                return Err(FrameError::CutAway { member: k });
            }
            Ok(ext)
        }
        Path::Arc { .. } => {
            let (o, r, e1, e2, sw) = m.path.arc_axes().expect("arc");
            let rho = m.section.radius();
            if r <= rho + tol::LINEAR {
                return Err(FrameError::ArcTooTight { member: k });
            }
            let mut ext = [0.0f64; 2];
            let mut span = [0.0, sw];
            for (s, end) in [(0, End::Start), (1, End::End)] {
                let from = if s == 0 { 0.0 } else { sw };
                // beyond the end: decreasing angle at the start, increasing at the end
                let dir = if s == 0 { -1.0 } else { 1.0 };
                for c in &cuts[s] {
                    let n: Vec3 = c.normal.into();
                    // the path's height over the plane: ca cos phi + cb sin phi + cc
                    let (ca, cb, cc) = (r * n.dot(e1), r * n.dot(e2), n.dot(o - c.point));
                    let df = |phi: f64| {
                        let (sn, cs) = dmath::sin_cos(phi);
                        dir * (cb * cs - ca * sn)
                    };
                    if df(from) <= 1e-9 * r {
                        return Err(FrameError::CutAlong { member: k, end });
                    }
                    // where the path is rho past the plane, going beyond the end (the root of
                    // amp cos(phi - phi0) = rho - cc where the height rises, nearest the end):
                    // nothing of the member is kept past it
                    let amp = (ca * ca + cb * cb).sqrt();
                    let phi0 = dmath::atan2(cb, ca);
                    let x = (rho - cc) / amp;
                    if x.abs() > 1.0 {
                        return Err(FrameError::ArcTooTight { member: k });
                    }
                    let w = dmath::acos(x);
                    let past = [phi0 + w, phi0 - w]
                        .into_iter()
                        .flat_map(|q| [q - dmath::TAU, q, q + dmath::TAU])
                        .filter(|&q| df(q) > 0.0)
                        .min_by(|x, y| (x - from).abs().total_cmp(&(y - from).abs()))
                        .ok_or(FrameError::ArcTooTight { member: k })?;
                    let by = dir * (past - from);
                    if by > dmath::FRAC_PI_2 {
                        return Err(FrameError::ArcTooTight { member: k });
                    }
                    ext[s] = ext[s].max(by.max(0.0));
                    let at = arc_cross(&m.path, c, from).ok_or(FrameError::ArcTooTight { member: k })?;
                    span[s] = if s == 0 { span[0].max(at) } else { span[1].min(at) };
                }
            }
            if span[1] - span[0] <= tol::LINEAR / r {
                return Err(FrameError::CutAway { member: k });
            }
            Ok(ext)
        }
    }
}

/// Where an arc path crosses a cut's plane: the crossing nearest parameter `near`.
pub fn arc_cross(p: &Path, c: &Cut, near: f64) -> Option<f64> {
    let (o, r, e1, e2, _) = p.arc_axes()?;
    let n: Vec3 = c.normal.into();
    let (ca, cb, cc) = (r * n.dot(e1), r * n.dot(e2), n.dot(o - c.point));
    let amp = (ca * ca + cb * cb).sqrt();
    let x = -cc / amp;
    if amp <= 0.0 || x.abs() > 1.0 {
        return None;
    }
    let phi0 = dmath::atan2(cb, ca);
    let w = dmath::acos(x);
    [phi0 + w, phi0 - w]
        .into_iter()
        .flat_map(|q| [q - dmath::TAU, q, q + dmath::TAU])
        .min_by(|x, y| (x - near).abs().total_cmp(&(y - near).abs()))
}

/// An end as the cut list gives it: each cut's angle off square and the direction (in the
/// section's x, y) towards the long point, both in degrees.
#[derive(Clone, Debug, PartialEq)]
pub struct EndCut {
    pub angle: f64,
    pub toward: Option<f64>,
}

/// One member's line of the cut list.
#[derive(Clone, Debug, PartialEq)]
pub struct Cutting {
    /// Along the path between the end cuts.
    pub length: f64,
    /// Long point to long point along the axis (straight members).
    pub overall: Option<f64>,
    /// An arc member's radius and the angle it turns between its end cuts (degrees).
    pub arc: Option<(f64, f64)>,
    pub ends: [Vec<EndCut>; 2],
}

fn deg(a: f64) -> f64 {
    a * 180.0 / dmath::PI
}

/// Whether two members cut alike, by the one tolerance rule: every length within
/// `tol::LINEAR`, every angle within the angle that moves the section's farthest point (`radius`
/// from the path) by `tol::LINEAR`; their ends matched either way round. Each end's cuts are
/// compared in order of angle.
pub fn alike(a: &Cutting, b: &Cutting, radius: f64) -> bool {
    let len = |x: f64, y: f64| (x - y).abs() <= tol::LINEAR;
    let ang = tol::LINEAR / radius.max(tol::LINEAR) * 180.0 / dmath::PI;
    let turn = |x: f64, y: f64| {
        let d = (x - y).rem_euclid(360.0);
        d.min(360.0 - d) <= ang
    };
    let opt = |x: Option<f64>, y: Option<f64>, f: &dyn Fn(f64, f64) -> bool| match (x, y) {
        (Some(x), Some(y)) => f(x, y),
        (None, None) => true,
        _ => false,
    };
    let sorted = |e: &[EndCut]| {
        let mut v = e.to_vec();
        v.sort_by(|p, q| p.angle.total_cmp(&q.angle));
        v
    };
    let end = |x: &[EndCut], y: &[EndCut]| {
        let (x, y) = (sorted(x), sorted(y));
        x.len() == y.len()
            && x.iter()
                .zip(&y)
                .all(|(p, q)| (p.angle - q.angle).abs() <= ang && opt(p.toward, q.toward, &turn))
    };
    len(a.length, b.length)
        && opt(a.overall, b.overall, &len)
        && match (a.arc, b.arc) {
            (Some((ra, ta)), Some((rb, tb))) => len(ra, rb) && (ta - tb).abs() <= ang,
            (None, None) => true,
            _ => false,
        }
        && ((end(&a.ends[0], &b.ends[0]) && end(&a.ends[1], &b.ends[1]))
            || (end(&a.ends[0], &b.ends[1]) && end(&a.ends[1], &b.ends[0])))
}

/// The member's cut-list line, exact from its path, section and cuts.
pub fn cutting(m: &MemberSpec, cuts: &[Vec<Cut>; 2]) -> Cutting {
    let end_t = m.path.end_t();
    let ends = [0, 1].map(|s| {
        let t = if s == 0 { 0.0 } else { end_t };
        let (x, y, z) = m.axes(t);
        // one plane given twice (a butt against two members' coplanar faces) is one cut
        // (the tolerance rule: the planes within `tol::LINEAR` of each other over the section)
        let rho = m.section.radius().max(tol::LINEAR);
        let same = |a: &Cut, b: &Cut| {
            let (na, nb): (Vec3, Vec3) = (a.normal.into(), b.normal.into());
            (na - nb).length() * rho <= tol::LINEAR && na.dot(b.point - a.point).abs() <= tol::LINEAR
        };
        cuts[s]
            .iter()
            .enumerate()
            .filter(|&(i, c)| !cuts[s][..i].iter().any(|b| same(b, c)))
            .map(|(_, c)| {
                let n: Vec3 = c.normal.into();
                let along = n.dot(z).abs().min(1.0);
                let angle = dmath::acos(along);
                // the long point: where n.x is least on the section, at either end
                let toward = (angle > 1e-12).then(|| deg(dmath::atan2(-n.dot(y), -n.dot(x))));
                EndCut {
                    angle: deg(angle),
                    toward,
                }
            })
            .collect()
    });
    match m.path {
        Path::Line { a, b } => {
            let l = a.distance(b);
            let d = (b - a) / l;
            let at0 = |c: &Cut| crossing(a, d, c, Vec3::new(0.0, 0.0, 0.0));
            let s0 = if cuts[0].is_empty() {
                0.0
            } else {
                cuts[0].iter().map(at0).fold(f64::NEG_INFINITY, f64::max)
            };
            let s1 = if cuts[1].is_empty() {
                l
            } else {
                cuts[1].iter().map(at0).fold(f64::INFINITY, f64::min)
            };
            let (x, y, _) = m.axes(0.0);
            let lo = extreme(m, a, d, &cuts[0], x, y, false).unwrap_or(0.0);
            let hi = extreme(m, a, d, &cuts[1], x, y, true).unwrap_or(l);
            Cutting {
                length: s1 - s0,
                overall: Some(hi - lo),
                arc: None,
                ends,
            }
        }
        Path::Arc { .. } => {
            let r = m.path.point(0.0).distance(match m.path {
                Path::Arc { center, .. } => center,
                _ => unreachable!(),
            });
            let f0 = if cuts[0].is_empty() {
                0.0
            } else {
                cuts[0]
                    .iter()
                    .filter_map(|c| arc_cross(&m.path, c, 0.0))
                    .fold(f64::NEG_INFINITY, f64::max)
            };
            let f1 = if cuts[1].is_empty() {
                end_t
            } else {
                cuts[1]
                    .iter()
                    .filter_map(|c| arc_cross(&m.path, c, end_t))
                    .fold(f64::INFINITY, f64::min)
            };
            Cutting {
                length: r * (f1 - f0),
                overall: None,
                arc: Some((r, deg(f1 - f0))),
                ends,
            }
        }
    }
}

/// The farthest the kept member reaches along `d` at its end (`hi`), or the least at its start,
/// over the whole section and every cut there: max (or min) over x of the min (or max) over the
/// cuts of their crossings. Exact: the optimum is where one cut alone is extreme (its support
/// point), or on the line in the section where two cuts' crossings are equal, at the outline.
fn extreme(m: &MemberSpec, a: Point3, d: Vec3, cuts: &[Cut], x: Vec3, y: Vec3, hi: bool) -> Option<f64> {
    if cuts.is_empty() {
        return None;
    }
    // each cut's crossing as a linear function of the section point (u, v): s = c0 + cu u + cv v
    let lin: Vec<(f64, f64, f64)> = cuts
        .iter()
        .map(|c| {
            let n: Vec3 = c.normal.into();
            let nd = n.dot(d);
            (n.dot(c.point - a) / nd, -n.dot(x) / nd, -n.dot(y) / nd)
        })
        .collect();
    // the kept reach at a point: min over cuts at the end, max at the start
    let at = |u: f64, v: f64| {
        let it = lin.iter().map(|l| l.0 + l.1 * u + l.2 * v);
        if hi {
            it.fold(f64::INFINITY, f64::min)
        } else {
            it.fold(f64::NEG_INFINITY, f64::max)
        }
    };
    let sign = if hi { 1.0 } else { -1.0 };
    let mut cands: Vec<(f64, f64)> = Vec::new();
    // a cut alone: its extreme over the outline in the direction it grows (or falls)
    for l in &lin {
        let w = Vec2::new(sign * l.1, sign * l.2);
        if let Some(p) = support_point(&m.section, w) {
            cands.push(p);
        }
    }
    // two cuts equal: their difference is zero along a line in the section; the reach along it
    // is linear, so the optimum is at its meetings with the outline
    for i in 0..lin.len() {
        for j in i + 1..lin.len() {
            let (a0, au, av) = (lin[i].0 - lin[j].0, lin[i].1 - lin[j].1, lin[i].2 - lin[j].2);
            cands.extend(line_meets(&m.section, a0, au, av));
        }
    }
    let best = cands
        .into_iter()
        .map(|(u, v)| sign * at(u, v))
        .fold(f64::NEG_INFINITY, f64::max);
    Some(sign * best)
}

/// A point of the outline farthest along `w` (where the support is attained).
fn support_point(s: &Section, w: Vec2) -> Option<(f64, f64)> {
    let mut best: Option<(f64, (f64, f64))> = None;
    let mut take = |p: forge_math::Point2| {
        let v = p.to_vec().dot(w);
        if best.is_none_or(|b| v > b.0) {
            best = Some((v, (p.x, p.y)));
        }
    };
    let wl = w.length();
    for seg in s.loops.first()? {
        take(seg.start());
        take(seg.end());
        if let Some((c, r, st, sw)) = arc_params(seg)
            && wl > 0.0
        {
            let wa = dmath::atan2(w.y, w.x);
            if on_arc(wa, st, sw) {
                take(c + w * (r / wl));
            }
        }
    }
    best.map(|b| b.1)
}

fn arc_params(s: &forge_ops::Segment) -> Option<(forge_math::Point2, f64, f64, f64)> {
    match *s {
        forge_ops::Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => Some((center, radius, start, sweep)),
        forge_ops::Segment::Circle { center, radius, ccw } => {
            Some((center, radius, 0.0, if ccw { dmath::TAU } else { -dmath::TAU }))
        }
        _ => None,
    }
}

fn on_arc(a: f64, start: f64, sweep: f64) -> bool {
    let (lo, w) = if sweep >= 0.0 {
        (start, sweep)
    } else {
        (start + sweep, -sweep)
    };
    (a - lo).rem_euclid(dmath::TAU) <= w
}

/// Where the line a0 + au u + av v = 0 meets the outline.
fn line_meets(s: &Section, a0: f64, au: f64, av: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let g = |p: forge_math::Point2| a0 + au * p.x + av * p.y;
    for seg in s.loops.first().into_iter().flatten() {
        if let Some((c, r, st, sw)) = arc_params(seg) {
            // the line's distance from the centre, and the chord it cuts
            let nl = (au * au + av * av).sqrt();
            if nl == 0.0 {
                continue;
            }
            let dist = g(c) / nl;
            if dist.abs() > r {
                continue;
            }
            let nrm = Vec2::new(au, av) / nl;
            let foot = c - nrm * dist;
            let half = (r * r - dist * dist).max(0.0).sqrt();
            for k in [-1.0, 1.0] {
                let p = foot + nrm.perp() * (k * half);
                if on_arc(dmath::atan2(p.y - c.y, p.x - c.x), st, sw) {
                    out.push((p.x, p.y));
                }
            }
        } else {
            let (p, q) = (seg.start(), seg.end());
            let (gp, gq) = (g(p), g(q));
            if gp == gq {
                if gp == 0.0 {
                    out.push((p.x, p.y));
                    out.push((q.x, q.y));
                }
                continue;
            }
            let t = gp / (gp - gq);
            if (0.0..=1.0).contains(&t) {
                let m = p.lerp(q, t);
                out.push((m.x, m.y));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut(length: f64, a: f64, toward: Option<f64>) -> Cutting {
        Cutting {
            length,
            overall: Some(length + 40.0),
            arc: None,
            ends: [
                vec![EndCut { angle: a, toward }],
                vec![EndCut {
                    angle: 0.0,
                    toward: None,
                }],
            ],
        }
    }

    #[test]
    fn alike_by_the_tolerance_rule() {
        let a = cut(600.0, 45.0, Some(180.0));
        // within LINEAR in length, either way round, toward across 0/360
        assert!(alike(
            &a,
            &cut(600.0 + 0.5 * tol::LINEAR, 45.0, Some(180.0)),
            30.0
        ));
        let mut b = cut(600.0, 45.0, Some(180.0));
        b.ends.swap(0, 1);
        assert!(alike(&a, &b, 30.0));
        assert!(alike(
            &cut(1.0, 10.0, Some(359.9999999999)),
            &cut(1.0, 10.0, Some(0.0)),
            30.0
        ));
        // past it: a length, an angle, a side
        assert!(!alike(
            &a,
            &cut(600.0 + 2.0 * tol::LINEAR, 45.0, Some(180.0)),
            30.0
        ));
        assert!(!alike(&a, &cut(600.0, 45.001, Some(180.0)), 30.0));
        assert!(!alike(&a, &cut(600.0, 45.0, Some(0.0)), 30.0));
    }
}

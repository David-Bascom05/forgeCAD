//! Object snaps: for a cursor and an aperture, the best point of each kind on a set of curves
//! (the caller's spatial query gives the candidates), ranked as AutoCAD's AutoSnap picks them:
//! points on the geometry (endpoint, intersection, midpoint, centre, quadrant, node, insertion,
//! perpendicular, tangent) nearest the cursor first; then the tracking snaps (apparent
//! intersection, extension, parallel); Nearest last.

use super::curve::{Curve, Seg, tangent_params};
use super::geom::{Bbox, PI, TAU, V2};
use super::intersect::{Ext, intersect};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SnapKind {
    Endpoint,
    Intersection,
    Midpoint,
    Center,
    Quadrant,
    Node,
    Insertion,
    Perpendicular,
    Tangent,
    ApparentIntersection,
    Extension,
    Parallel,
    Nearest,
}

impl SnapKind {
    pub const ALL: [SnapKind; 13] = [
        SnapKind::Endpoint,
        SnapKind::Intersection,
        SnapKind::Midpoint,
        SnapKind::Center,
        SnapKind::Quadrant,
        SnapKind::Node,
        SnapKind::Insertion,
        SnapKind::Perpendicular,
        SnapKind::Tangent,
        SnapKind::ApparentIntersection,
        SnapKind::Extension,
        SnapKind::Parallel,
        SnapKind::Nearest,
    ];
    pub fn bit(self) -> u16 {
        1 << (self as u16)
    }
    /// 0: on the geometry; 1: tracking; 2: nearest.
    fn tier(self) -> u8 {
        match self {
            SnapKind::ApparentIntersection | SnapKind::Extension | SnapKind::Parallel => 1,
            SnapKind::Nearest => 2,
            _ => 0,
        }
    }
}

/// What a snap candidate is.
#[derive(Clone, Debug, PartialEq)]
pub enum SnapGeom {
    Curve(Curve),
    /// A POINT entity.
    Node(V2),
    /// A TEXT, MTEXT, INSERT or ATTRIB's insertion point.
    Insertion(V2),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapItem {
    pub handle: u32,
    pub geom: SnapGeom,
}

/// A snap request.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapQuery {
    pub cursor: V2,
    /// The aperture's radius in drawing units.
    pub aperture: f64,
    /// The kinds on (`SnapKind::bit`s); 0 for all.
    pub kinds: u16,
    /// The point the command is drawing from: for perpendicular, tangent and parallel.
    pub base: Option<V2>,
    /// Handles whose ends or direction the cursor has acquired (paused over): extension and
    /// parallel track them. Empty: every candidate line and arc.
    pub acquired: Vec<u32>,
}

impl SnapQuery {
    pub fn new(cursor: V2, aperture: f64) -> SnapQuery {
        SnapQuery {
            cursor,
            aperture,
            kinds: 0,
            base: None,
            acquired: vec![],
        }
    }
    fn on(&self, k: SnapKind) -> bool {
        self.kinds == 0 || self.kinds & k.bit() != 0
    }
    fn acquired(&self, h: u32) -> bool {
        self.acquired.is_empty() || self.acquired.contains(&h)
    }
}

/// A snap: its kind, point, the entities it comes from, and its distance from the cursor (what
/// it is ranked by).
#[derive(Clone, Debug, PartialEq)]
pub struct Snap {
    pub kind: SnapKind,
    pub point: V2,
    pub handles: Vec<u32>,
    pub dist: f64,
}

struct Best {
    by_kind: Vec<Option<Snap>>,
}

impl Best {
    fn offer(&mut self, q: &SnapQuery, kind: SnapKind, p: V2, handles: &[u32], dist: Option<f64>) {
        if !q.on(kind) || !p.is_finite() {
            return;
        }
        let d = dist.unwrap_or_else(|| p.dist(q.cursor));
        if d > q.aperture {
            return;
        }
        let slot = &mut self.by_kind[kind as usize];
        let better = match slot {
            None => true,
            Some(s) => d < s.dist || (d == s.dist && handles < s.handles.as_slice()),
        };
        if better {
            *slot = Some(Snap {
                kind,
                point: p,
                handles: handles.to_vec(),
                dist: d,
            });
        }
    }
}

/// The midpoint by length.
fn mid_by_length(c: &Curve) -> V2 {
    c.point(c.param_at_length(c.length() / 2.0))
}

/// Endpoints, midpoints, centres and quadrants of a curve.
fn points_of(c: &Curve, f: &mut dyn FnMut(SnapKind, V2)) {
    let quad = |f: &mut dyn FnMut(SnapKind, V2), a: &super::curve::Arc| {
        for k in 0..4 {
            let ang = k as f64 * PI / 2.0;
            if a.param_of_angle(ang) <= a.len() {
                f(SnapKind::Quadrant, a.c + V2::polar(ang, a.r));
            }
        }
    };
    match c {
        Curve::Line { a, b } => {
            f(SnapKind::Endpoint, *a);
            f(SnapKind::Endpoint, *b);
            f(SnapKind::Midpoint, a.mid(*b));
        }
        Curve::Ray { at, .. } => f(SnapKind::Endpoint, *at),
        Curve::Xline { .. } => {}
        Curve::Circle { c, r } => {
            f(SnapKind::Center, *c);
            for k in 0..4 {
                f(SnapKind::Quadrant, *c + V2::polar(k as f64 * PI / 2.0, *r));
            }
        }
        Curve::Arc(a) => {
            f(SnapKind::Endpoint, a.start_pt());
            f(SnapKind::Endpoint, a.end_pt());
            f(SnapKind::Midpoint, a.point(a.len() / 2.0));
            f(SnapKind::Center, a.c);
            quad(f, a);
        }
        Curve::Ellipse(e) => {
            f(SnapKind::Center, e.c);
            for k in 0..4 {
                let ang = k as f64 * PI / 2.0;
                if e.is_full() || e.param_of_angle(ang) <= e.sweep.abs() {
                    f(SnapKind::Quadrant, e.at_angle(ang));
                }
            }
            if !e.is_full() {
                f(SnapKind::Endpoint, e.point(0.0));
                f(SnapKind::Endpoint, e.point(e.sweep.abs()));
                f(SnapKind::Midpoint, mid_by_length(c));
            }
        }
        Curve::Poly(p) => {
            for &v in &p.pts {
                f(SnapKind::Endpoint, v);
            }
            for s in p.segs() {
                match s {
                    Seg::Line(a, b) => f(SnapKind::Midpoint, a.mid(b)),
                    Seg::Arc(a) => {
                        f(SnapKind::Midpoint, a.point(a.len() / 2.0));
                        f(SnapKind::Center, a.c);
                        quad(f, &a);
                    }
                }
            }
        }
        Curve::Spline(_) | Curve::Offset(_) => {
            if !c.is_closed() {
                f(SnapKind::Endpoint, c.start());
                f(SnapKind::Endpoint, c.end());
                f(SnapKind::Midpoint, mid_by_length(c));
            }
        }
    }
}

/// The extension of a curve's ends: lines along their direction, arcs round their circle.
fn extension_point(c: &Curve, p: V2) -> Option<V2> {
    match c {
        Curve::Line { .. } | Curve::Ray { .. } => {
            let (t, q) = c.closest_unbounded(p);
            let (lo, hi) = c.domain();
            (t < lo || t > hi).then_some(q)
        }
        Curve::Arc(a) => {
            let t = a.param_of_angle((p - a.c).angle());
            (t > a.len() && t < TAU).then(|| a.c + (p - a.c).unit_or_zero() * a.r)
        }
        Curve::Poly(pl) if !pl.closed && pl.nseg() > 0 => {
            let first = pl.seg(0).curve().reversed();
            let last = pl.seg(pl.nseg() - 1).curve();
            [first, last]
                .iter()
                .filter_map(|s| match s {
                    Curve::Line { a, b } => {
                        let d = *b - *a;
                        let t = (p - *a).dot(d) / d.len2();
                        (t > 1.0).then(|| a.lerp(*b, t))
                    }
                    Curve::Arc(_) => extension_point(s, p),
                    _ => None,
                })
                .min_by(|x, y| x.dist(p).total_cmp(&y.dist(p)))
        }
        _ => None,
    }
}

/// A line-like direction for parallel tracking.
fn direction_of(c: &Curve) -> Option<V2> {
    match c {
        Curve::Line { a, b } => (*b - *a).unit(),
        Curve::Ray { dir, .. } | Curve::Xline { dir, .. } => Some(*dir),
        _ => None,
    }
}

/// The best snap of each kind within the aperture, ranked.
pub fn snap(items: &[SnapItem], q: &SnapQuery) -> Vec<Snap> {
    let mut best = Best {
        by_kind: vec![None; SnapKind::ALL.len()],
    };
    let cur = q.cursor;
    let ap = q.aperture;
    // curves near the cursor
    let mut near: Vec<(u32, &Curve, Bbox)> = vec![];
    for it in items {
        match &it.geom {
            SnapGeom::Node(p) => best.offer(q, SnapKind::Node, *p, &[it.handle], None),
            SnapGeom::Insertion(p) => best.offer(q, SnapKind::Insertion, *p, &[it.handle], None),
            SnapGeom::Curve(c) => {
                let bb = c.bbox();
                let h = [it.handle];
                // tracking snaps reach past the curve
                if q.acquired(it.handle) {
                    if q.on(SnapKind::Extension)
                        && let Some(p) = extension_point(c, cur)
                    {
                        best.offer(q, SnapKind::Extension, p, &h, None);
                    }
                    if q.on(SnapKind::Parallel)
                        && let (Some(b), Some(d)) = (q.base, direction_of(c))
                    {
                        let p = b + d * (cur - b).dot(d);
                        best.offer(q, SnapKind::Parallel, p, &h, None);
                    }
                }
                if bb.dist(cur) > ap {
                    // a centre is found by its own position too
                    if let Curve::Circle { c: o, .. } | Curve::Arc(super::curve::Arc { c: o, .. }) = c {
                        best.offer(q, SnapKind::Center, *o, &h, None);
                    }
                    continue;
                }
                let (_, foot) = c.closest(cur);
                let d_curve = foot.dist(cur);
                points_of(c, &mut |k, p| {
                    if k == SnapKind::Center {
                        // the centre snaps while the cursor is over the curve or the centre
                        let dc = p.dist(cur);
                        let d = if dc <= ap {
                            dc
                        } else if d_curve <= ap {
                            ap
                        } else {
                            f64::INFINITY
                        };
                        best.offer(q, k, p, &h, Some(d));
                    } else {
                        best.offer(q, k, p, &h, None);
                    }
                });
                if d_curve <= ap {
                    best.offer(q, SnapKind::Nearest, foot, &h, Some(d_curve));
                    near.push((it.handle, c, bb));
                }
                if let Some(b) = q.base {
                    if q.on(SnapKind::Perpendicular) {
                        for t in c.feet(b) {
                            best.offer(q, SnapKind::Perpendicular, c.point(t), &h, None);
                        }
                    }
                    if q.on(SnapKind::Tangent) {
                        for t in tangent_params(c, b) {
                            best.offer(q, SnapKind::Tangent, c.point(t), &h, None);
                        }
                    }
                }
            }
        }
    }
    // intersections among the curves under the aperture
    if q.on(SnapKind::Intersection) || q.on(SnapKind::ApparentIntersection) {
        let window = Bbox::of([cur]).grow(ap);
        for i in 0..near.len() {
            for j in i + 1..near.len() {
                let (ha, a, _) = near[i];
                let (hb, b, _) = near[j];
                let ext = if q.on(SnapKind::ApparentIntersection) {
                    Ext::Both
                } else {
                    Ext::None
                };
                for h in intersect(a, b, ext).points {
                    if !window.contains(h.p) {
                        continue;
                    }
                    let kind = if h.on_both() {
                        SnapKind::Intersection
                    } else {
                        SnapKind::ApparentIntersection
                    };
                    best.offer(q, kind, h.p, &[ha.min(hb), ha.max(hb)], None);
                }
            }
        }
    }
    let mut out: Vec<Snap> = best.by_kind.into_iter().flatten().collect();
    out.sort_by(|x, y| {
        x.kind
            .tier()
            .cmp(&y.kind.tier())
            .then(x.dist.total_cmp(&y.dist))
            .then(x.kind.cmp(&y.kind))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::geom::v2;

    fn items() -> Vec<SnapItem> {
        vec![
            SnapItem {
                handle: 1,
                geom: SnapGeom::Curve(Curve::line(v2(0.0, 0.0), v2(10.0, 0.0))),
            },
            SnapItem {
                handle: 2,
                geom: SnapGeom::Curve(Curve::line(v2(5.0, -5.0), v2(5.0, 5.0))),
            },
            SnapItem {
                handle: 3,
                geom: SnapGeom::Curve(Curve::circle(v2(20.0, 0.0), 3.0)),
            },
            SnapItem {
                handle: 4,
                geom: SnapGeom::Node(v2(0.0, 1.0)),
            },
        ]
    }

    #[test]
    fn ranking() {
        let it = items();
        // by the crossing: intersection first, then the midpoint it coincides with, nearest last
        let s = snap(&it, &SnapQuery::new(v2(5.2, 0.1), 1.0));
        assert_eq!(s[0].kind, SnapKind::Intersection);
        assert!(s[0].point.dist(v2(5.0, 0.0)) < 1e-12 && s[0].handles == vec![1, 2]);
        assert_eq!(s.last().unwrap().kind, SnapKind::Nearest);
        assert!(s.iter().any(|x| x.kind == SnapKind::Midpoint));
        // by the line's end: the endpoint, then the node beside it
        let s = snap(&it, &SnapQuery::new(v2(0.2, 0.3), 1.0));
        assert_eq!(s[0].kind, SnapKind::Endpoint);
        assert_eq!(s[1].kind, SnapKind::Node);
        // only some kinds
        let mut q = SnapQuery::new(v2(0.2, 0.3), 1.0);
        q.kinds = SnapKind::Node.bit();
        let s = snap(&it, &q);
        assert!(s.len() == 1 && s[0].kind == SnapKind::Node);
    }

    #[test]
    fn circle_snaps() {
        let it = items();
        let s = snap(&it, &SnapQuery::new(v2(23.1, 0.2), 0.5));
        assert_eq!(s[0].kind, SnapKind::Quadrant);
        assert!(s[0].point.dist(v2(23.0, 0.0)) < 1e-12);
        // over the circle: its centre too, ranked after the quadrant
        assert!(
            s.iter()
                .any(|x| x.kind == SnapKind::Center && x.point.dist(v2(20.0, 0.0)) < 1e-12)
        );
        // tangent and perpendicular from a base point
        let mut q = SnapQuery::new(v2(20.0, 3.0), 1.0);
        q.base = Some(v2(20.0, 10.0));
        q.kinds = SnapKind::Tangent.bit() | SnapKind::Perpendicular.bit();
        let s = snap(&it, &q);
        assert!(
            s.iter()
                .any(|x| x.kind == SnapKind::Perpendicular && x.point.dist(v2(20.0, 3.0)) < 1e-12)
        );
        let mut q = SnapQuery::new(v2(22.0, 2.0), 1.5);
        q.base = Some(v2(20.0, 10.0));
        q.kinds = SnapKind::Tangent.bit();
        let s = snap(&it, &q);
        let t = s[0].point;
        assert!(((t - v2(20.0, 0.0)).dot(t - v2(20.0, 10.0))).abs() < 1e-9);
    }

    #[test]
    fn tracking_snaps() {
        let it = items();
        // past the line's end
        let s = snap(&it, &SnapQuery::new(v2(13.0, 0.2), 0.5));
        let e = s.iter().find(|x| x.kind == SnapKind::Extension).unwrap();
        assert!(e.point.dist(v2(13.0, 0.0)) < 1e-12);
        // the two lines' extensions meet
        let l = vec![
            SnapItem {
                handle: 7,
                geom: SnapGeom::Curve(Curve::line(v2(0.0, 0.0), v2(4.0, 0.0))),
            },
            SnapItem {
                handle: 8,
                geom: SnapGeom::Curve(Curve::line(v2(5.0, 1.0), v2(5.0, 5.0))),
            },
        ];
        let s = snap(&l, &SnapQuery::new(v2(4.8, 0.3), 1.5));
        let a = s
            .iter()
            .find(|x| x.kind == SnapKind::ApparentIntersection)
            .unwrap();
        assert!(a.point.dist(v2(5.0, 0.0)) < 1e-12);
        // parallel to the acquired line through the base
        let mut q = SnapQuery::new(v2(3.0, 5.2), 0.5);
        q.base = Some(v2(0.0, 5.0));
        q.acquired = vec![7];
        q.kinds = SnapKind::Parallel.bit();
        let s = snap(&l, &q);
        assert!(s[0].point.dist(v2(3.0, 5.0)) < 1e-12);
    }

    #[test]
    fn many_curves_fast_enough() {
        let mut it = vec![];
        for k in 0..2000 {
            let x = (k % 50) as f64;
            let y = (k / 50) as f64;
            it.push(SnapItem {
                handle: k,
                geom: SnapGeom::Curve(Curve::line(v2(x, y), v2(x + 0.7, y + 0.3))),
            });
        }
        let s = snap(&it, &SnapQuery::new(v2(10.1, 10.05), 0.2));
        assert_eq!(s[0].kind, SnapKind::Endpoint);
    }
}

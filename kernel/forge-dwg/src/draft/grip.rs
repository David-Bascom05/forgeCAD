//! Grips: the points AutoCAD shows on a selected entity (a line's ends and middle, a circle's
//! centre and quadrants, an arc's ends, middle and centre, a polyline's vertices and segment
//! middles, a spline's control points, an insertion point) and what dragging one does: an end
//! moves, a middle or centre moves the whole, a quadrant changes the radius.

use super::curve::{Arc, Curve, Poly};
use super::entity::{read_entity, transform_data, write_curve};
use super::geom::{Affine, PI, V2};
use crate::types::Data;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A line's or arc's start and end.
    Start,
    End,
    /// A line's or arc's middle: moves it whole (a line), or bends it through (an arc).
    Mid,
    Center,
    /// A circle's or ellipse's quadrant k (0 at angle 0, counter-clockwise).
    Quadrant(u8),
    /// A polyline's vertex k, or a spline's control point k.
    Vertex(usize),
    /// A polyline segment's middle: the segment moves.
    SegMid(usize),
    /// Text, a block reference, a point: moves it.
    Insertion,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grip {
    pub at: V2,
    pub role: Role,
}

fn g(at: V2, role: Role) -> Grip {
    Grip { at, role }
}

/// An entity's grips.
pub fn grips(d: &Data) -> Vec<Grip> {
    match d {
        Data::Text(t) => return vec![g(V2::from(t.insert), Role::Insertion)],
        Data::MText(t) => return vec![g(V2::from(t.insert), Role::Insertion)],
        Data::Insert(i) => return vec![g(V2::from(i.at), Role::Insertion)],
        Data::Point(p) => return vec![g(V2::from(p.at), Role::Insertion)],
        Data::DimLinear(x) => return vec![g(V2::from(x.dim.text_mid), Role::Insertion)],
        Data::DimAligned(x) => return vec![g(V2::from(x.dim.text_mid), Role::Insertion)],
        Data::DimRadius(x) => return vec![g(V2::from(x.dim.text_mid), Role::Insertion)],
        Data::DimDiameter(x) => return vec![g(V2::from(x.dim.text_mid), Role::Insertion)],
        _ => {}
    }
    let Ok(f) = read_entity(d) else { return vec![] };
    match &f.curve {
        Curve::Line { a, b } => vec![g(*a, Role::Start), g(a.mid(*b), Role::Mid), g(*b, Role::End)],
        Curve::Circle { c, r } => {
            let mut v = vec![g(*c, Role::Center)];
            for k in 0..4u8 {
                v.push(g(*c + V2::polar(k as f64 * PI / 2.0, *r), Role::Quadrant(k)));
            }
            v
        }
        Curve::Arc(a) => vec![
            g(a.start_pt(), Role::Start),
            g(a.point(0.5), Role::Mid),
            g(a.end_pt(), Role::End),
            g(a.c, Role::Center),
        ],
        Curve::Ellipse(e) => {
            let mut v = vec![g(e.c, Role::Center)];
            if e.is_full() {
                let mn = e.minor();
                for (k, q) in [e.major, mn, -e.major, -mn].into_iter().enumerate() {
                    v.push(g(e.c + q, Role::Quadrant(k as u8)));
                }
            } else {
                v.push(g(e.point(e.start), Role::Start));
                v.push(g(e.point(e.start + e.sweep), Role::End));
            }
            v
        }
        Curve::Poly(p) => {
            let mut v: Vec<Grip> = p
                .pts
                .iter()
                .enumerate()
                .map(|(k, q)| g(*q, Role::Vertex(k)))
                .collect();
            for k in 0..p.nseg() {
                v.push(g(p.seg(k).point(0.5), Role::SegMid(k)));
            }
            v
        }
        Curve::Spline(n) => n
            .ctrl
            .iter()
            .enumerate()
            .map(|(k, q)| g(*q, Role::Vertex(k)))
            .collect(),
        Curve::Ray { at, .. } | Curve::Xline { at, .. } => vec![g(*at, Role::Insertion)],
        Curve::Offset(_) => vec![],
    }
}

/// The entity with grip `role` (at `from`) dragged to `to`. None when that can't be.
pub fn drag(d: &Data, role: Role, from: V2, to: V2) -> Option<Data> {
    let mv = || transform_data(d, &Affine::translate(to - from), false);
    if role == Role::Insertion {
        return mv();
    }
    let f = read_entity(d).ok()?;
    let c = match (&f.curve, role) {
        (Curve::Line { b, .. }, Role::Start) => Curve::line(to, *b),
        (Curve::Line { a, .. }, Role::End) => Curve::line(*a, to),
        (Curve::Line { .. }, Role::Mid) | (_, Role::Center) => return mv(),
        (Curve::Circle { c, .. }, Role::Quadrant(_)) => Curve::circle(*c, to.dist(*c)),
        (Curve::Arc(a), Role::Start) => Curve::Arc(Arc::from_3_points(to, a.point(0.5), a.end_pt())?),
        (Curve::Arc(a), Role::End) => Curve::Arc(Arc::from_3_points(a.start_pt(), a.point(0.5), to)?),
        (Curve::Arc(a), Role::Mid) => Curve::Arc(Arc::from_3_points(a.start_pt(), to, a.end_pt())?),
        (Curve::Ellipse(e), Role::Quadrant(k)) => {
            let mut e = *e;
            let r = to.dist(e.c);
            if k % 2 == 0 {
                let b = e.b();
                e.major = e.major.unit()? * r;
                e.ratio = b / r;
                if e.ratio > 1.0 {
                    // the minor axis is the major now
                    let mn = e.minor();
                    e.major = mn;
                    e.ratio = r / b;
                }
            } else {
                e.ratio = r / e.a();
                if e.ratio > 1.0 {
                    let mn = e.minor().unit()? * r;
                    let a = e.a();
                    e.major = mn;
                    e.ratio = a / r;
                }
            }
            Curve::Ellipse(e)
        }
        (Curve::Poly(p), Role::Vertex(k)) => {
            let mut p: Poly = p.clone();
            *p.pts.get_mut(k)? = to;
            Curve::Poly(p)
        }
        (Curve::Poly(p), Role::SegMid(k)) => {
            let mut p: Poly = p.clone();
            let n = p.pts.len();
            let dl = to - from;
            p.pts[k] += dl;
            p.pts[(k + 1) % n] += dl;
            Curve::Poly(p)
        }
        (Curve::Spline(s), Role::Vertex(k)) => {
            let mut s = s.clone();
            *s.ctrl.get_mut(k)? = to;
            Curve::Spline(s)
        }
        (Curve::Ellipse(_), Role::Start | Role::End) => return None,
        _ => return None,
    };
    write_curve(&c, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::entity::to_data;
    use crate::draft::geom::v2;

    #[test]
    fn a_lines_grips() {
        let d = to_data(&Curve::line(v2(0.0, 0.0), v2(10.0, 0.0)), 0.0).unwrap();
        let gs = grips(&d);
        assert_eq!(gs.len(), 3);
        let e = drag(&d, Role::End, v2(10.0, 0.0), v2(10.0, 5.0)).unwrap();
        assert_eq!(
            read_entity(&e).unwrap().curve,
            Curve::line(v2(0.0, 0.0), v2(10.0, 5.0))
        );
        let m = drag(&d, Role::Mid, v2(5.0, 0.0), v2(5.0, 1.0)).unwrap();
        assert_eq!(
            read_entity(&m).unwrap().curve,
            Curve::line(v2(0.0, 1.0), v2(10.0, 1.0))
        );
    }

    #[test]
    fn a_circles_quadrant_sets_its_radius() {
        let d = to_data(&Curve::circle(v2(1.0, 1.0), 2.0), 0.0).unwrap();
        let q = grips(&d)[1];
        let e = drag(&d, q.role, q.at, v2(1.0, 6.0)).unwrap();
        assert_eq!(read_entity(&e).unwrap().curve, Curve::circle(v2(1.0, 1.0), 5.0));
    }
}

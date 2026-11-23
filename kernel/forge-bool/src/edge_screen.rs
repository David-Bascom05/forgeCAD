//! Step 2's screens for `on_edge` (research/decide-once/curved-fallback-rates.md, "Step 2"):
//! which of today's answers a proven screen or an exact key decides, and which stay open. Compiled
//! for tests and the decision log; `on_edge` itself is unchanged.
//!
//! * (a) a hit computed on the edge's own carrier (`plane_meet`, B:2523): on the carrier by
//!   provenance (S0-a);
//! * (b) a hit on face-surface ∩ across-surface (B:2833): on that set by provenance (S0-b); this
//!   edge's branch when `d ≤ ε/2 < sep − ε/2`, `sep` the separation of the set's components, a
//!   theorem per pair kind whose case is selected by exact keys;
//! * (c) the range: inside by a margin of ε from both end vertices, or the end vertex itself when
//!   within ε/2 of it and the other end is more than 2ε away.
use crate::boolean::on_edge;
use forge_geom::{CurveGeom, Surface};
use forge_math::{Point3, UnitVec3};
use forge_topo::{Body, EdgeId};

/// ε: twice the hit's distance bound. Pending the IEEE bound of the hit computations
/// (`plane_meet`, `curve_surface_in`); every "on" measured is within 1.9e-7, every "off" on
/// another branch at least 0.22 away.
pub const EPS: f64 = 2e-6;

/// The rounding bound of a dot product of two unit vectors (`dot_err`, Screen/Coincide.lean:
/// 10εM + 4η with M ≤ 3), with room.
pub const DOT_ERR: f64 = 1e-13;

/// The separation of the components of two surfaces' intersection set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sep {
    /// One component: nothing to choose.
    One,
    /// At least this far apart.
    Gap(f64),
    /// No separation theorem applies (or its case is not keyed exactly): an open guess site.
    Open(&'static str),
}

/// A unit vector exactly along a coordinate axis: its index.
fn axis(v: UnitVec3) -> Option<usize> {
    let w = v.get();
    let c = [w.x, w.y, w.z];
    let mut k = None;
    let mut zeros = 0;
    let mut i = 0;
    while i < 3 {
        if c[i] == 0.0 {
            zeros += 1;
        } else if c[i].abs() == 1.0 {
            k = Some(i);
        }
        i += 1;
    }
    if zeros == 2 { k } else { None }
}

fn coord(p: Point3, k: usize) -> f64 {
    [p.x, p.y, p.z][k]
}

/// `Sep` for a face's surface and the surface across an edge, by kind. Cases are selected on
/// exact keys (axis-aligned data compared bit for bit); anything else with two possible
/// components is open.
pub fn separation(s: &Surface, a: &Surface) -> Sep {
    use Surface::*;
    match (s, a) {
        (Plane(_), Plane(_)) | (Plane(_), Sphere(_)) | (Sphere(_), Plane(_)) | (Sphere(_), Sphere(_)) => {
            Sep::One
        }
        (Plane(p), Cylinder(c)) | (Cylinder(c), Plane(p)) => match (axis(p.frame.z()), axis(c.frame.z())) {
            // the plane square to the axis: a circle
            (Some(i), Some(j)) if i == j => Sep::One,
            // the plane parallel to the axis: two lines 2√(r² − h²) apart (one at tangency)
            (Some(i), Some(_)) => {
                let h = (coord(p.frame.origin, i) - coord(c.frame.origin, i)).abs();
                let g2 = c.radius * c.radius - h * h;
                if g2 > 0.0 {
                    Sep::Gap(2.0 * g2.sqrt())
                } else {
                    Sep::Open("plane tangent to a cylinder")
                }
            }
            // the plane not parallel to the axis (n·z ≠ 0, screened: the computed dot exceeds its
            // rounding bound, `dot_err`): one ellipse or circle
            _ if p.frame.z().dot(c.frame.z()).abs() > DOT_ERR => Sep::One,
            _ => Sep::Open("plane parallel to the cylinder axis, not axis-aligned"),
        },
        (Cylinder(c), Cylinder(d)) => match (axis(c.frame.z()), axis(d.frame.z())) {
            (Some(i), Some(j)) if i == j => {
                // parallel axes: two lines, the chord of the two section circles apart
                let (u, v) = ((i + 1) % 3, (i + 2) % 3);
                let (du, dv) = (
                    coord(d.frame.origin, u) - coord(c.frame.origin, u),
                    coord(d.frame.origin, v) - coord(c.frame.origin, v),
                );
                let dist = (du * du + dv * dv).sqrt();
                let (r1, r2) = (c.radius, d.radius);
                let x = (dist * dist + r1 * r1 - r2 * r2) / (2.0 * dist);
                let h2 = r1 * r1 - x * x;
                if dist > 0.0 && h2 > 0.0 {
                    Sep::Gap(2.0 * h2.sqrt())
                } else {
                    Sep::Open("parallel cylinders touching")
                }
            }
            _ => Sep::Open("cylinders, axes not parallel"),
        },
        (Plane(p), Cone(c)) | (Cone(c), Plane(p)) => match (axis(p.frame.z()), axis(c.frame.z())) {
            (Some(i), Some(j)) if i == j => Sep::One,
            // square to the axis (|n·z| = 1 within the dot's rounding, screened): one circle
            _ if p.frame.z().dot(c.frame.z()).abs() > 1.0 - DOT_ERR => {
                Sep::Open("plane and cone, nearly square to the axis")
            }
            _ => Sep::Open("plane and cone, not square to the axis"),
        },
        (Plane(p), Torus(t)) | (Torus(t), Plane(p)) => match (axis(p.frame.z()), axis(t.frame.z())) {
            // square to the axis: two concentric circles 2√(r² − h²) apart
            (Some(i), Some(j)) if i == j => {
                let h = (coord(p.frame.origin, i) - coord(t.frame.origin, i)).abs();
                let g2 = t.minor * t.minor - h * h;
                if g2 > 0.0 {
                    Sep::Gap(2.0 * g2.sqrt())
                } else {
                    Sep::Open("plane tangent to a torus")
                }
            }
            // through the axis: two meridian circles 2(R − r) apart, for a ring torus
            (Some(i), Some(_)) if coord(p.frame.origin, i) == coord(t.frame.origin, i) => {
                if t.major > t.minor {
                    Sep::Gap(2.0 * (t.major - t.minor))
                } else {
                    Sep::Open("horn or spindle torus")
                }
            }
            _ => Sep::Open("plane and torus, spiric or oblique"),
        },
        _ => Sep::Open("no separation theorem for this pair"),
    }
}

/// How a hit on an edge was made.
#[derive(Clone, Copy)]
pub enum Made<'a> {
    /// On the edge's own carrier (B:2523).
    OwnCarrier,
    /// On the face's surface ∩ the surface across the edge (B:2833).
    Across(&'a Surface, &'a Surface),
}

/// The class of one `on_edge` answer: decided (with the screened answer), or open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Class {
    Decided(bool),
    Open(&'static str),
}

/// Step 2's decision for `on_edge(body, e, p)` with the hit made as `made`.
pub fn classify(body: &Body, e: EdgeId, p: Point3, made: Made) -> Class {
    let edge = body.edge(e);
    let s = edge.curve.closest(p, edge.range);
    let d = edge.curve.eval(s).distance(p);
    // (b) the branch. Far from the edge's carrier the hit is not on it, whatever the pair: the
    // screen `d > ε/2` proves the distance nonzero. Near it, the hit is on this edge's branch when
    // the other components lie farther than ε (a separation theorem), or by provenance on the
    // edge's own carrier. (The surface across an edge is built in floats, so the edge's carrier
    // and the exact intersection of the stored surfaces differ by rounding: "on" is δ-faithful,
    // the hit within ε/2 of the edge, not on it exactly.)
    if d > 0.5 * EPS {
        return Class::Decided(false);
    }
    if let Made::Across(f, a) = made {
        match separation(f, a) {
            Sep::One => {}
            Sep::Gap(g) if 0.5 * EPS < g - 0.5 * EPS => {}
            Sep::Gap(_) => return Class::Open("branches within the screen's band"),
            Sep::Open(why) => return Class::Open(why),
        }
    }
    // (c) the range. A closed circle or ellipse over its whole period has none to test (its
    // parameter is periodic: `closest` may answer outside the stored range, an angle 2π off).
    let periodic = matches!(
        edge.curve,
        forge_geom::Curve::Circle(_) | forge_geom::Curve::Ellipse(_)
    );
    if edge.start == edge.end && periodic && edge.range.hi - edge.range.lo >= std::f64::consts::TAU - 1e-9 {
        return Class::Decided(true);
    }
    let (p0, p1) = (body.vertex(edge.start).point, body.vertex(edge.end).point);
    let (d0, d1) = (p.distance(p0), p.distance(p1));
    let len = p0.distance(p1);
    let inside = s > edge.range.lo && s < edge.range.hi;
    if d0 > EPS && d1 > EPS {
        Class::Decided(inside)
    } else if (d0 <= 0.5 * EPS || d1 <= 0.5 * EPS) && (len > 2.0 * EPS || edge.start == edge.end) {
        Class::Decided(true)
    } else {
        Class::Open("near an end, within the screen's band")
    }
}

/// (decision log) one `on_edge` call classified, beside today's answer, to stderr under
/// FORGE_DECISIONS_DUMP: `screen2 SITE CLASS TODAY`.
pub fn log(body: &Body, e: EdgeId, p: Point3, made: Made, site: &str) {
    if std::env::var_os("FORGE_DECISIONS_DUMP").is_none() {
        return;
    }
    let today = on_edge(body, e, p).is_some();
    let kind = |s: &Surface| match s {
        Surface::Plane(_) => "plane",
        Surface::Cylinder(_) => "cyl",
        Surface::Cone(_) => "cone",
        Surface::Sphere(_) => "sphere",
        Surface::Torus(_) => "torus",
        _ => "other",
    };
    let pair = match made {
        Made::OwnCarrier => "own".to_string(),
        Made::Across(f, a) => format!("{}-{}", kind(f), kind(a)),
    };
    match classify(body, e, p, made) {
        Class::Decided(b) => {
            eprintln!("screen2 {site} {pair} decided {b} today {today}");
            if b != today {
                let edge = body.edge(e);
                let s = edge.curve.closest(p, edge.range);
                let d = edge.curve.eval(s).distance(p);
                let (p0, p1) = (body.vertex(edge.start).point, body.vertex(edge.end).point);
                eprintln!(
                    "screen2-diff d {d:e} s {s} range {:?} d0 {:e} d1 {:e} closed {} curve {}",
                    edge.range,
                    p.distance(p0),
                    p.distance(p1),
                    edge.start == edge.end,
                    match edge.curve {
                        forge_geom::Curve::Line(_) => "line",
                        forge_geom::Curve::Circle(_) => "circle",
                        forge_geom::Curve::Ellipse(_) => "ellipse",
                        _ => "other",
                    }
                );
            }
        }
        Class::Open(why) => eprintln!("screen2 {site} {pair} open '{why}' today {today}"),
    }
}

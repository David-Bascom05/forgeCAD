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
use forge_math::Point3;
use forge_topo::{Body, EdgeId};

/// ε: twice the hit's distance bound. Pending the IEEE bound of the hit computations
/// (`plane_meet`, `curve_surface_in`); every "on" measured is within 1.9e-7, every "off" on
/// another branch at least 0.22 away.
pub const EPS: f64 = 2e-6;

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

// ---------------------------------------------------------------------------------------------
// Exact signs (error-free transformations: TwoSum, Dekker's TwoProduct). Exact for finite inputs
// whose products neither overflow nor underflow: true within `tol::INPUT_SIZE`.
// ---------------------------------------------------------------------------------------------

/// `a + b = s + e` exactly.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    let e = (a - (s - bb)) + (b - bb);
    (s, e)
}

/// Dekker's split: `a = hi + lo`, each half of the significand.
fn split(a: f64) -> (f64, f64) {
    let c = 134_217_729.0 * a;
    let hi = c - (c - a);
    (hi, a - hi)
}

/// `a · b = p + e` exactly (Dekker; no fused multiply-add, which native and wasm round apart).
fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    let e = al * bl - (((p - ah * bh) - al * bh) - ah * bl);
    (p, e)
}

/// The exact sign of the sum of `terms`: grown into a nonoverlapping expansion (each new term
/// carried through every component by TwoSum, the rounding errors kept), whose most significant
/// nonzero component has the sum's sign.
fn sum_sign(terms: &[f64]) -> i32 {
    let mut ex: Vec<f64> = Vec::with_capacity(terms.len());
    let mut i = 0;
    while i < terms.len() {
        let mut q = terms[i];
        let mut next: Vec<f64> = Vec::with_capacity(ex.len() + 1);
        let mut k = 0;
        while k < ex.len() {
            let (s, e) = two_sum(q, ex[k]);
            if e != 0.0 {
                next.push(e);
            }
            q = s;
            k += 1;
        }
        if q != 0.0 {
            next.push(q);
        }
        ex = next;
        i += 1;
    }
    match ex.last() {
        Some(x) if *x > 0.0 => 1,
        Some(x) if *x < 0.0 => -1,
        _ => 0,
    }
}

fn v3(v: forge_math::Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

/// The exact sign of `u · v`.
fn dot_sign(u: [f64; 3], v: [f64; 3]) -> i32 {
    let mut t = [0.0; 6];
    let mut i = 0;
    while i < 3 {
        let (p, e) = two_prod(u[i], v[i]);
        t[2 * i] = p;
        t[2 * i + 1] = e;
        i += 1;
    }
    sum_sign(&t)
}

/// The exact sign of `n · (a − b)` (no rounded difference).
fn dot_diff_sign(n: [f64; 3], a: [f64; 3], b: [f64; 3]) -> i32 {
    let mut t = [0.0; 12];
    let mut i = 0;
    while i < 3 {
        let (p, e) = two_prod(n[i], a[i]);
        let (q, f) = two_prod(n[i], b[i]);
        t[4 * i] = p;
        t[4 * i + 1] = e;
        t[4 * i + 2] = -q;
        t[4 * i + 3] = -f;
        i += 1;
    }
    sum_sign(&t)
}

/// `u × v = 0` exactly (u, v parallel).
fn parallel(u: [f64; 3], v: [f64; 3]) -> bool {
    let c = |a: f64, b: f64, c: f64, d: f64| {
        let (p, e) = two_prod(a, b);
        let (q, f) = two_prod(c, d);
        sum_sign(&[p, e, -q, -f])
    };
    c(u[1], v[2], u[2], v[1]) == 0 && c(u[2], v[0], u[0], v[2]) == 0 && c(u[0], v[1], u[1], v[0]) == 0
}

/// A point's distance from a plane (unit normal n through p): |n · (o − p)|, rounded
/// (a screened value: the separations it feeds are compared with ε at a margin far above it).
fn plane_offset(n: [f64; 3], o: [f64; 3], p: [f64; 3]) -> f64 {
    (n[0] * (o[0] - p[0]) + n[1] * (o[1] - p[1]) + n[2] * (o[2] - p[2])).abs()
}

/// `Sep` for a face's surface and the surface across an edge, by kind. Each case is selected by
/// exact signs (`dot_sign`, `dot_diff_sign`, `parallel`); the separations themselves are rounded
/// values compared with ε at a margin far above their rounding. Anything else with two possible
/// components is open.
pub fn separation(s: &Surface, a: &Surface) -> Sep {
    use Surface::*;
    match (s, a) {
        (Plane(_), Plane(_)) | (Plane(_), Sphere(_)) | (Sphere(_), Plane(_)) | (Sphere(_), Sphere(_)) => {
            Sep::One
        }
        (Plane(p), Cylinder(c)) | (Cylinder(c), Plane(p)) => {
            let (n, z) = (v3(p.frame.z().get()), v3(c.frame.z().get()));
            if dot_sign(n, z) != 0 {
                // not parallel to the axis: one ellipse or circle
                Sep::One
            } else {
                // parallel to the axis: two lines 2√(r² − h²) apart (one at tangency)
                let h = plane_offset(n, v3(c.frame.origin.to_vec()), v3(p.frame.origin.to_vec()));
                let g2 = c.radius * c.radius - h * h;
                if g2 > 0.0 {
                    Sep::Gap(2.0 * g2.sqrt())
                } else {
                    Sep::Open("plane tangent to a cylinder")
                }
            }
        }
        (Cylinder(c), Cylinder(d)) => {
            let (zc, zd) = (v3(c.frame.z().get()), v3(d.frame.z().get()));
            if parallel(zc, zd) {
                // parallel axes: two lines, the chord of the two section circles apart
                let off = d.frame.origin - c.frame.origin;
                let a = c.frame.z().get();
                let perp = off - a * a.dot(off);
                let dist = perp.length();
                let (r1, r2) = (c.radius, d.radius);
                let x = (dist * dist + r1 * r1 - r2 * r2) / (2.0 * dist);
                let h2 = r1 * r1 - x * x;
                if dist > 0.0 && h2 > 0.0 {
                    Sep::Gap(2.0 * h2.sqrt())
                } else {
                    Sep::Open("parallel cylinders touching")
                }
            } else {
                Sep::Open("cylinders, axes not parallel")
            }
        }
        (Plane(p), Cone(c)) | (Cone(c), Plane(p)) => {
            if parallel(v3(p.frame.z().get()), v3(c.frame.z().get())) {
                Sep::One
            } else {
                Sep::Open("plane and cone, not square to the axis")
            }
        }
        (Plane(p), Torus(t)) | (Torus(t), Plane(p)) => {
            let (n, z) = (v3(p.frame.z().get()), v3(t.frame.z().get()));
            let (o, q) = (v3(t.frame.origin.to_vec()), v3(p.frame.origin.to_vec()));
            if parallel(n, z) {
                // square to the axis: two concentric circles 2√(r² − h²) apart
                let h = plane_offset(n, o, q);
                let g2 = t.minor * t.minor - h * h;
                if g2 > 0.0 {
                    Sep::Gap(2.0 * g2.sqrt())
                } else {
                    Sep::Open("plane tangent to a torus")
                }
            } else if dot_sign(n, z) == 0 && dot_diff_sign(n, o, q) == 0 {
                // through the axis: two meridian circles 2(R − r) apart, for a ring torus
                if t.major > t.minor {
                    Sep::Gap(2.0 * (t.major - t.minor))
                } else {
                    Sep::Open("horn or spindle torus")
                }
            } else {
                Sep::Open("plane and torus, spiric or oblique")
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_signs() {
        // 0.1 · 3 − 0.3 is not zero in f64 data: the exact sign says so
        assert_eq!(sum_sign(&[0.1, 0.1, 0.1, -0.3]), 1);
        assert_eq!(sum_sign(&[1e16, 1.0, -1e16]), 1);
        assert_eq!(sum_sign(&[1e16, -1e16, 0.0]), 0);
        // a tilted normal against its own axis' perpendicular: exactly zero or not
        let n = [0.6, 0.8, 0.0];
        assert_eq!(dot_sign(n, [0.0, 0.0, 1.0]), 0);
        assert_eq!(dot_sign(n, [-0.8, 0.6, 0.0]), 0);
        assert_ne!(
            dot_sign(
                [0.012999633836427844, 0.999915501190035, 0.0],
                [0.999915501190035, -0.01299963383642705, 0.0]
            ),
            0
        );
        assert_eq!(dot_diff_sign(n, [3.0, 4.0, 7.0], [3.0, 4.0, -2.0]), 0);
        assert_eq!(dot_diff_sign(n, [3.0, 4.0, 7.0], [0.0, 0.0, 7.0]), 1);
        assert!(
            parallel([0.0, 0.0, 1.0], [0.0, 0.0, -1.0]) && !parallel([0.0, 1e-300, 1.0], [0.0, 0.0, 1.0])
        );
    }
}

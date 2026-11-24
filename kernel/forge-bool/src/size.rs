//! `InputSize`: the magnitude bounds the boolean's proven floating-point screens assume. It is the
//! `Size` of verify/forge-bool-proof/ForgeBoolProof/Statement.lean (ledger row E.size), and
//! research/decide-once/old-path-statements.md §E collects the bounds.
//!
//! A precondition a proof relies on is checked in debug builds only (research/PROOF-STANDARD.md), so
//! each operand is checked at the boolean's entries under `debug_assertions` and release pays
//! nothing. How release keeps bodies inside it (by construction, or a release refusal) is open:
//! ledger row E.size.rs.

use forge_geom::Surface;
use forge_math::{Frame, Point3};
use forge_topo::Body;

/// The largest coordinate or radius an operand may carry: the binding bound of the screens
/// (`offp_margin`'s measures ≤ 1e8 give coordinates ≤ about 1.6e7, 16 km in mm). With a unit
/// normal, a plane's distance |n·o| ≤ √3·1.6e7, far inside the 9e9 its screens assume.
pub const INPUT_SIZE: f64 = 1.6e7;

/// The smallest nonzero stored surface number, 2^-480: the exact-sign keys multiply surface data
/// with Dekker's TwoProduct, which never underflows above it.
pub const INPUT_FLOOR: f64 = f64::from_bits((1023 - 480) << 52);

/// A surface number within `InputSize`: finite, at most `INPUT_SIZE`, and zero or at least
/// `INPUT_FLOOR` in magnitude.
fn stored_ok(x: f64) -> bool {
    x.is_finite() && x.abs() <= INPUT_SIZE && (x == 0.0 || x.abs() >= INPUT_FLOOR)
}

fn frame_ok(f: &Frame) -> bool {
    let o = f.origin;
    [o.x, o.y, o.z].into_iter().all(stored_ok)
        && [f.x(), f.y(), f.z()].iter().all(|a| {
            let v = a.get();
            [v.x, v.y, v.z].into_iter().all(stored_ok)
        })
}

/// Whether `body` is within `InputSize`: every live face's surface data (frame origin and axes,
/// radii; a procedural surface by its frame alone, the curved screens bounding the rest) is
/// `stored_ok`, and every vertex is finite with coordinates at most `INPUT_SIZE` (no floor: vertex
/// points are not multiplied by the keys). `Err` names the first entity outside it.
pub fn input_size_ok(body: &Body) -> Result<(), String> {
    for f in body.face_ids() {
        let s = &body.face(f).surface;
        let ok = match s {
            Surface::Plane(p) => frame_ok(&p.frame),
            Surface::Cylinder(c) => frame_ok(&c.frame) && stored_ok(c.radius),
            Surface::Cone(k) => frame_ok(&k.frame) && stored_ok(k.radius),
            Surface::Sphere(x) => frame_ok(&x.frame) && stored_ok(x.radius),
            Surface::Torus(t) => frame_ok(&t.frame) && stored_ok(t.major) && stored_ok(t.minor),
            Surface::Procedural(_) => true,
        };
        if !ok {
            return Err(format!("face {f:?}'s surface is outside InputSize: {s:?}"));
        }
    }
    let within = |x: f64| x.is_finite() && x.abs() <= INPUT_SIZE;
    for v in &body.vertices {
        let p: Point3 = v.point;
        if !(within(p.x) && within(p.y) && within(p.z)) {
            return Err(format!("a vertex is outside InputSize: {p:?}"));
        }
    }
    Ok(())
}

/// Debug builds: every operand is within `InputSize`. Release: nothing.
#[inline]
pub(crate) fn debug_check(_bodies: &[&Body]) {
    #[cfg(debug_assertions)]
    for b in _bodies {
        if let Err(why) = input_size_ok(b) {
            panic!("boolean operand outside InputSize: {why}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_is_two_to_the_minus_480() {
        assert_eq!(INPUT_FLOOR, 2f64.powi(-480));
    }

    #[test]
    fn stored_numbers() {
        assert!(stored_ok(0.0) && stored_ok(1.0) && stored_ok(-INPUT_SIZE) && stored_ok(INPUT_FLOOR));
        assert!(!stored_ok(f64::NAN) && !stored_ok(f64::INFINITY) && !stored_ok(1.7e7));
        assert!(!stored_ok(INPUT_FLOOR / 2.0) && !stored_ok(-1e-300));
    }
}

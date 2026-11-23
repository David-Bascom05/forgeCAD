//! One surface by exact keys (compiled for tests and the decision log; the default build is
//! unchanged): the proof's predicate for every `true` of today's coincidence tests
//! (`ssi::coincident`, `unify::same_surface`). Each key is an exact identity of the stored f64 data,
//! decided with error-free arithmetic, and proven to give one point set in
//! verify/forge-curved/ForgeCurved/CarrierKey.lean:
//! * planes: normals exactly parallel, the second origin exactly on the first plane (`plane_key`);
//! * cylinders: axes exactly parallel, the origins' difference exactly along the axis, equal radii
//!   (the general form of `cyl_key`, whose axis-aligned case `same_carrier` had);
//! * spheres: equal centres and radii (`sphere_key`);
//! * tori: equal centres, axes exactly parallel, equal radii.
//!
//! `false` says nothing (the pair stays with the tolerance). Where the key says `true` the
//! tolerance tests do too, their measures being exactly zero up to the rounding of their own
//! arithmetic, far under ANGULAR and LINEAR within `InputSize`: tested below on random data.
use crate::edge_screen::{dot_diff_sign, parallel, sum_sign, two_prod, v3};
use forge_geom::Surface;

/// `z × (a − b) = 0` exactly: every component's four products summed by an exact expansion.
fn cross_diff_zero(z: [f64; 3], a: [f64; 3], b: [f64; 3]) -> bool {
    let comp = |i: usize, j: usize| {
        // z_i (a_j − b_j) − z_j (a_i − b_i)
        let (p1, e1) = two_prod(z[i], a[j]);
        let (p2, e2) = two_prod(z[i], b[j]);
        let (p3, e3) = two_prod(z[j], a[i]);
        let (p4, e4) = two_prod(z[j], b[i]);
        sum_sign(&[p1, e1, -p2, -e2, -p3, -e3, p4, e4])
    };
    comp(1, 2) == 0 && comp(2, 0) == 0 && comp(0, 1) == 0
}

fn finite(v: [f64; 3]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// The exact key: the two stored surfaces are one point set.
pub fn same_carrier(a: &Surface, b: &Surface) -> bool {
    use Surface::*;
    match (a, b) {
        (Plane(p), Plane(q)) => {
            let (n, m) = (v3(p.frame.z().get()), v3(q.frame.z().get()));
            let (o, r) = (v3(p.frame.origin.to_vec()), v3(q.frame.origin.to_vec()));
            finite(n) && finite(m) && finite(o) && finite(r) && parallel(n, m) && dot_diff_sign(n, r, o) == 0
        }
        (Cylinder(c), Cylinder(d)) => {
            let (z, w) = (v3(c.frame.z().get()), v3(d.frame.z().get()));
            let (o, r) = (v3(c.frame.origin.to_vec()), v3(d.frame.origin.to_vec()));
            c.radius == d.radius
                && c.radius.is_finite()
                && finite(z)
                && finite(w)
                && finite(o)
                && finite(r)
                && parallel(z, w)
                && cross_diff_zero(z, r, o)
        }
        (Sphere(s), Sphere(t)) => {
            s.radius == t.radius
                && s.radius.is_finite()
                && finite(v3(s.frame.origin.to_vec()))
                && s.frame.origin == t.frame.origin
        }
        (Torus(s), Torus(t)) => {
            let (z, w) = (v3(s.frame.z().get()), v3(t.frame.z().get()));
            s.major == t.major
                && s.minor == t.minor
                && s.major.is_finite()
                && s.minor.is_finite()
                && finite(z)
                && finite(w)
                && finite(v3(s.frame.origin.to_vec()))
                && s.frame.origin == t.frame.origin
                && parallel(z, w)
        }
        _ => false,
    }
}

/// (decision log) a `true` of a coincidence test, keyed or not: `coincidence-key SITE yes|no`
/// under FORGE_DECISIONS_DUMP.
#[cfg(feature = "decisions")]
pub fn log(a: &Surface, b: &Surface, site: &str) {
    if std::env::var_os("FORGE_DECISIONS_DUMP").is_none() {
        return;
    }
    let k = |s: &Surface| match s {
        Surface::Plane(_) => "plane",
        Surface::Cylinder(_) => "cyl",
        Surface::Cone(_) => "cone",
        Surface::Sphere(_) => "sphere",
        Surface::Torus(_) => "torus",
        _ => "other",
    };
    let keyed = same_carrier(a, b);
    eprintln!(
        "coincidence-key {site} | {}-{} | {}",
        k(a),
        k(b),
        if keyed { "keyed" } else { "unkeyed" }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Cylinder, Plane, Sphere};
    use forge_math::rng::Rng;
    use forge_math::{Frame, Point3, UnitVec3, Vec3};

    fn plane(o: Point3, z: UnitVec3) -> Surface {
        Surface::Plane(Plane {
            frame: Frame::from_z(o, z),
        })
    }

    #[test]
    fn keys() {
        let a = plane(Point3::new(0.0, 0.0, 6.0), UnitVec3::Z);
        let b = plane(Point3::new(17.5, -3.0, 6.0), -UnitVec3::Z);
        let c = plane(Point3::new(0.0, 0.0, 6.0 + 1e-15), UnitVec3::Z);
        assert!(same_carrier(&a, &b) && crate::ssi::coincident(&a, &b));
        // an ulp off: no key (the tolerance decides, as before)
        assert!(!same_carrier(&a, &c) && crate::ssi::coincident(&a, &c));
        // a tilted plane and a point exactly on it: moved by twice the stored normal's exact
        // perpendicular (-n_y, n_x), and along z
        let n = Vec3::new(0.6, 0.8, 0.0).normalize().unwrap();
        let (nx, ny) = (n.get().x, n.get().y);
        let t1 = plane(Point3::new(0.0, 0.0, 3.0), n);
        let t2 = plane(Point3::new(-2.0 * ny, 2.0 * nx, 9.0), n);
        assert!(same_carrier(&t1, &t2) && crate::ssi::coincident(&t1, &t2));
        // decimal data on the decimal perpendicular is not on it exactly (0.6 and 0.8 are not dyadic)
        let t3 = plane(Point3::new(4.0, -3.0, 3.0), n);
        assert!(!same_carrier(&t1, &t3) && crate::ssi::coincident(&t1, &t3));
        let cy = |o: Point3, r: f64| {
            Surface::Cylinder(Cylinder {
                frame: Frame::from_z(o, UnitVec3::Z),
                radius: r,
            })
        };
        assert!(same_carrier(
            &cy(Point3::new(4.0, 2.0, 0.0), 3.0),
            &cy(Point3::new(4.0, 2.0, -9.0), 3.0)
        ));
        assert!(!same_carrier(
            &cy(Point3::new(4.0, 2.0, 0.0), 3.0),
            &cy(Point3::new(4.0, 2.5, 0.0), 3.0)
        ));
        let sp = |r: f64| {
            Surface::Sphere(Sphere {
                frame: Frame::from_z(Point3::new(1.0, 2.0, 3.0), UnitVec3::Z),
                radius: r,
            })
        };
        assert!(same_carrier(&sp(2.0), &sp(2.0)) && !same_carrier(&sp(2.0), &sp(2.5)));
    }

    /// Wherever the key holds, today's tolerance test says so too (the answer never changes):
    /// random frames, and a second plane through a point of the first moved along it exactly.
    #[test]
    fn key_implies_tolerance() {
        let mut rng = Rng::new(11);
        let mut keyed = 0;
        for _ in 0..20000 {
            let r = |rng: &mut Rng| rng.range(-200.0, 200.0);
            let o = Point3::new(r(&mut rng), r(&mut rng), r(&mut rng));
            let Some(z) = Vec3::new(r(&mut rng), r(&mut rng), r(&mut rng)).normalize() else {
                continue;
            };
            let f = Frame::from_z(o, z);
            // a point along the frame's x by a power of two: exact when it is
            let k = (rng.range(0.0, 8.0) as i32 - 4) as f64;
            let q = o + f.x().get() * k.exp2();
            for (s, t) in [
                (plane(o, z), plane(q, z)),
                (plane(o, z), plane(q, -z)),
                (
                    Surface::Cylinder(Cylinder {
                        frame: f,
                        radius: 5.0,
                    }),
                    Surface::Cylinder(Cylinder {
                        frame: Frame::from_z(o + z.get() * k.exp2(), z),
                        radius: 5.0,
                    }),
                ),
            ] {
                if same_carrier(&s, &t) {
                    keyed += 1;
                    assert!(crate::ssi::coincident(&s, &t));
                }
            }
        }
        assert!(keyed > 0);
    }
}

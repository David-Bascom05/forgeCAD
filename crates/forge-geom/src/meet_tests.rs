use super::*;
use crate::testing::*;
use crate::{Cone, Cylinder, Plane, Sphere};
use forge_math::rng::Rng;
use forge_math::{UnitVec3, Vec3};

/// A random analytic surface of the kind near the origin, sized to meet the others.
fn rand_surface(r: &mut Rng, kind: usize) -> Surface {
    let o = rand_point(r, 4.0);
    let z = rand_unit(r);
    let f = Frame::from_zx(o, z, rand_unit(r).get()).unwrap_or_else(|| Frame::from_z(o, z));
    match kind {
        0 => Surface::Plane(Plane::new(f)),
        1 => Surface::Cylinder(Cylinder {
            frame: f,
            radius: r.range(0.5, 6.0),
        }),
        2 => Surface::Cone(Cone::new(f, r.range(0.0, 4.0), r.range(0.1, 1.3)).unwrap()),
        3 => Surface::Sphere(Sphere {
            frame: f,
            radius: r.range(0.5, 8.0),
        }),
        _ => {
            let major = r.range(2.0, 8.0);
            Surface::Torus(Torus::new(f, major, r.range(0.3, major * 0.9)).unwrap())
        }
    }
}

/// Every chain point and chain midpoint of every curve on both surfaces (as tight as a
/// traced curve evaluates), and the chain unbroken: links no longer than the tracer's step
/// allows, each close to the curve between its ends.
fn check(a: &Surface, b: &Surface, res: &SurfaceSurface, step: f64, worst: &mut f64) -> Result<(), String> {
    if let SurfaceSurface::Curves(cs) = res {
        for (c, _) in cs {
            let Curve::Intersection(k) = c else { continue };
            let n = k.pts.len();
            for i in 0..k.segments() {
                for p in [k.pts[i], k.eval(i as f64 + 0.5)] {
                    let e = a.distance(p).max(b.distance(p));
                    *worst = worst.max(e);
                    if e > 2e-3 * forge_math::tol::LINEAR {
                        return Err(format!("{a:?} {b:?} point {i} of {n} off by {e}"));
                    }
                }
                let chord = k.pts[i].distance(k.pts[(i + 1) % n]);
                if chord > 1.5 * step + 1e-9 {
                    return Err(format!("{a:?} {b:?} jumps {chord} at {i} of {n}"));
                }
            }
            if k.sag > 0.5 * step {
                return Err(format!("{a:?} {b:?} sags {}", k.sag));
            }
        }
    }
    Ok(())
}

#[test]
fn random_pairs_lie_on_both() {
    let mut r = Rng::new(7001);
    let mut stats = std::collections::BTreeMap::new();
    let mut worst = 0.0;
    let mut bad = Vec::new();
    let region = Box3::from_points([Point3::new(-12.0, -12.0, -12.0), Point3::new(12.0, 12.0, 12.0)]);
    let step = 0.5 * region.diagonal() / 64.0;
    for i in 0..20_000 {
        let (ka, kb) = (i % 5, (i / 5) % 5);
        if ka == 0 && kb == 0 {
            continue;
        }
        let a = rand_surface(&mut r, ka);
        let b = rand_surface(&mut r, kb);
        let res = intersect(&a, &b, region);
        let key = match &res {
            None => "none",
            Some(SurfaceSurface::Curves(_)) => "curves",
            Some(SurfaceSurface::Points(_)) => "points",
            Some(SurfaceSurface::Empty) => "empty",
            Some(_) => "other",
        };
        *stats.entry((ka.min(kb), ka.max(kb), key)).or_insert(0) += 1;
        if let Some(res) = &res
            && let Err(e) = check(&a, &b, res, step, &mut worst)
        {
            if bad.is_empty() {
                eprintln!("pair {i}: {e}");
            }
            bad.push((i, ka.min(kb), ka.max(kb)));
            *stats.entry((ka.min(kb), ka.max(kb), "BAD")).or_insert(0) += 1;
        }
    }
    eprintln!("worst {worst:e}");
    for (k, v) in &stats {
        eprintln!("{k:?} {v}");
    }
    assert!(
        bad.is_empty(),
        "{} bad: {:?}",
        bad.len(),
        &bad[..bad.len().min(20)]
    );
}

#[test]
fn quartic_disc_matches_roots() {
    let rs = [1.0, 2.0, -3.0, 0.5];
    let mut c = vec![1.0];
    for &r in &rs {
        let mut n = vec![0.0; c.len() + 1];
        for (i, &a) in c.iter().enumerate() {
            n[i] -= a * r;
            n[i + 1] += a;
        }
        c = n;
    }
    let mut want = 1.0;
    for i in 0..4 {
        for j in i + 1..4 {
            want *= (rs[i] - rs[j]) * (rs[i] - rs[j]);
        }
    }
    let got = quartic_disc(&[c[0], c[1], c[2], c[3], c[4]]);
    assert!((got - want).abs() <= 1e-9 * want.abs(), "{got} {want}");
}

/// A plane through a torus seen in a thin box: the loop crosses it twice, near where the
/// tube angle wraps.
#[test]
fn plane_through_torus_in_a_thin_box() {
    let at = Point3::new(3.674114324581751, 35.07708152211284, -1.1555645229281977);
    let n = UnitVec3::new_unchecked(Vec3::new(-0.6849610339534025, -0.7285797018621132, 0.0));
    let p = Surface::Plane(Plane::new(Frame::from_z(at, n)));
    let o = Point3::new(0.9957889709814399, 27.84441628242665, 13.45908405070749);
    let f = Frame::from_zx(o, UnitVec3::new_unchecked(Vec3::new(0.0, 0.0, -1.0)), Vec3::X).unwrap();
    let t = Surface::Torus(Torus::new(f, 6.406743525031951, 2.5346124377017465).unwrap());
    let region = Box3::from_points([
        Point3::new(3.6740943245817506, 34.19423202528969, 4.517708087973794),
        Point3::new(4.613182853923694, 35.077101522112855, 22.400460013441187),
    ]);
    let Some(SurfaceSurface::Curves(cs)) = intersect(&p, &t, region) else {
        panic!("curves")
    };
    assert_eq!(cs.len(), 2);
    let mut worst = 0.0;
    check(
        &p,
        &t,
        &SurfaceSurface::Curves(cs),
        0.5 * region.diagonal() / 64.0,
        &mut worst,
    )
    .unwrap();
}

/// Two tubes of one radius whose spines lie in one plane (a mirrored face fillet against a
/// corner blend), both touching the plane under them: the discriminant hugs zero and gave
/// some 18000 events, each recounting the rest. Left to the tracer, at once.
#[test]
fn tangent_tubes_end_quickly() {
    let cf = Frame::from_axes(Point3::new(0.89, 0.0, 0.89), -Vec3::X, Vec3::Z, Vec3::Y).unwrap();
    let c = Surface::Cylinder(Cylinder {
        frame: cf,
        radius: 0.89,
    });
    let x = Vec3::new(0.9801980198019803, 0.198019801980198, 0.0);
    let tf = Frame::from_zx(
        Point3::new(5.5362376237623785, 41.397623762376234, 0.89),
        UnitVec3::Z,
        x,
    )
    .unwrap();
    let t = Surface::Torus(Torus::new(tf, 7.45, 0.89).unwrap());
    let region = Box3::from_points([Point3::new(-20.0, -20.0, -5.0), Point3::new(60.0, 60.0, 5.0)]);
    let at = std::time::Instant::now();
    assert!(intersect(&c, &t, region).is_none());
    assert!(at.elapsed().as_secs_f64() < 5.0);
}

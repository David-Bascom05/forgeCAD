use super::*;
use crate::CurveGeom;
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

/// A tube square to a torus's axis (a bored sweep across a bent one): the reflection across
/// the plane through the torus's axis and square to the tube maps both to themselves, so the
/// two pairs of roots meet at once and the discriminant only touches zero. The least sample
/// missed the touch, no event was found, and the pair came out Empty.
#[test]
fn tube_square_to_a_torus_axis() {
    let tf = Frame::from_zx(
        Point3::new(85.87, 16.46, 26.6),
        UnitVec3::new_unchecked(Vec3::new(0.0, 0.0, -1.0)),
        Vec3::new(-1.0, 0.0, 0.0),
    )
    .unwrap();
    let t = Surface::Torus(Torus::new(tf, 25.43, 5.35).unwrap());
    let cf = Frame::from_zx(Point3::new(60.44, 19.68, -2.29), UnitVec3::X, Vec3::Z).unwrap();
    let c = Surface::Cylinder(Cylinder {
        frame: cf,
        radius: 26.21,
    });
    let region = Box3::from_points([Point3::new(50.0, 0.0, 15.0), Point3::new(120.0, 40.0, 35.0)]);
    let res = intersect(&c, &t, region).expect("a closed form");
    let SurfaceSurface::Curves(cs) = &res else {
        panic!("curves, not {res:?}")
    };
    // through a point on both
    let p = Point3::new(55.93040261348267, 16.46, 23.721453246598887);
    assert!(
        cs.iter()
            .any(|(k, _)| k.eval(k.closest(p, k.full_range(100.0))).distance(p) < 1e-6)
    );
    let mut worst = 0.0;
    check(&c, &t, &res, 0.5 * region.diagonal() / 64.0, &mut worst).unwrap();
}

/// A ball 3·LINEAR off tangent inside a cylinder's wall along its equator (the curved reference's
/// band, cylinder_sphere/tangent_wall_equator moved 3·LINEAR along x): they meet in one thin loop
/// round the half of the wall the ball pokes through, its two hairpin tips at x = e/2, y = ±10
/// rounded with a radius of e. The chain dropped one tip as a sliver point, its link ran the curve
/// backwards and a boolean's volume came out 2 mm³ high (or did not settle). Every link turns less
/// than 30 degrees by the curve's own tangent, down to TURN_LEAST of the step, and passes both tips.
#[test]
fn ball_a_hair_through_a_wall_keeps_both_tips() {
    let e = 3.0 * forge_math::tol::LINEAR;
    let cyl = Surface::Cylinder(Cylinder {
        frame: Frame::from_z(Point3::new(0.0, 0.0, -20.0), UnitVec3::Z),
        radius: 10.0,
    });
    let ball = Surface::Sphere(Sphere {
        frame: Frame::from_z(Point3::new(e, 0.0, -10.0), UnitVec3::Z),
        radius: 10.0,
    });
    let region = Box3::from_points([Point3::new(-10.0, -10.0, -20.0), Point3::new(10.0, 10.0, 0.0)]);
    let Some(SurfaceSurface::Curves(cs)) = intersect(&cyl, &ball, region) else {
        panic!("one loop")
    };
    assert_eq!(cs.len(), 1);
    let Curve::Intersection(k) = &cs[0].0 else {
        panic!("a traced loop")
    };
    let step = 0.5 * region.diagonal() / 64.0;
    for y in [-10.0, 10.0] {
        let tip = Point3::new(0.5 * e, y * (1.0 - e * e / 400.0).sqrt(), -10.0);
        let near = k
            .pts
            .iter()
            .map(|p| p.distance(tip))
            .fold(f64::INFINITY, f64::min);
        assert!(near <= 1e-6, "the tip at y = {y} is {near} from the chain");
    }
    let n = k.pts.len();
    for i in 0..k.segments() {
        let j = (i + 1) % n;
        let chord = k.pts[i].distance(k.pts[j]);
        let cos = k.tans[i].dot(k.tans[j]);
        assert!(
            cos >= crate::intcurve::TURN_COS || chord <= step * crate::intcurve::TURN_LEAST,
            "link {i} of {n} turns to cos {cos} over {chord}"
        );
    }
}

/// A thin torus on a wide cylinder (fuzz seed 295, feature 69): the cylinder cuts the tube in two
/// loops, each about 3° of the cylinder's angle, and the discriminant's 64 samples saw one. The
/// root count over `thin_window` finds both.
#[test]
fn thin_torus_on_a_wide_cylinder_both_loops() {
    let o = Point3::new(57.513833943223545, 82.64151449414277, 31.529651297690172);
    let tf = Frame::from_axes(o, Vec3::X, Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0)).unwrap();
    let t = Surface::Torus(Torus {
        frame: tf,
        major: 1.018123715855934,
        minor: 0.22881392007971665,
    });
    let z = Vec3::new(-0.04760542051711398, 0.17499343179871532, 0.9834180305265392);
    let x = Vec3::new(-0.34481495645685883, 0.921132764696941, -0.18060198117801576);
    let cf = Frame::from_axes(
        Point3::new(64.28286796892466, 86.4342803264453, 17.574469684406452),
        x,
        z.cross(x),
        z,
    )
    .unwrap();
    let c = Surface::Cylinder(Cylinder {
        frame: cf,
        radius: 8.70706734376011,
    });
    let region = Box3::from_point(o).grow(1.018123715855934 + 0.22881392007971665 + 1e-6);
    let res = intersect(&c, &t, region).expect("a closed form");
    let SurfaceSurface::Curves(cs) = &res else {
        panic!("curves, not {res:?}")
    };
    assert_eq!(cs.len(), 2, "{res:?}");
    // one loop either side of the ring: torus angles about 42° and 227°
    let mut at: Vec<f64> = cs
        .iter()
        .map(|(k, _)| {
            let q = k.eval(k.domain().lo) - o;
            (forge_math::dmath::atan2(-q.y, q.x) * 180.0 / forge_math::dmath::PI).rem_euclid(360.0)
        })
        .collect();
    at.sort_by(f64::total_cmp);
    assert!(
        (35.0..50.0).contains(&at[0]) && (220.0..235.0).contains(&at[1]),
        "{at:?}"
    );
    let mut worst = 0.0;
    check(&c, &t, &res, 0.5 * region.diagonal() / 64.0, &mut worst).unwrap();
}

/// fuzz_keep seed 33, feature 72: a rod 57.8 in radius kept through a ring torus 6.6 by 1.2. The
/// walk round one loop came back to a point 5e-10 from its first, and the loop kept both: a closing
/// link of nothing, where the curve's speed fell to 3.5e-10 (0.2 elsewhere). A piece's mid there
/// gave the boolean a tangent of noise, the lean of a region the wrong sign, and a classification
/// that contradicted its neighbour's (Internal(InconsistentClassification)).
#[test]
fn closed_loops_have_no_link_of_nothing() {
    let t = Surface::Torus(Torus {
        frame: Frame::from_axes(
            Point3::new(109.91745700353877, 58.99420509978757, 23.82286340569159),
            Vec3::new(1.0, -0.0, 0.0),
            Vec3::new(0.0, -1.0, -0.0),
            Vec3::new(0.0, 0.0, -1.0),
        )
        .unwrap(),
        major: 6.6179520084548855,
        minor: 1.2449487326050606,
    });
    let c = Surface::Cylinder(Cylinder {
        frame: Frame::WORLD.with_origin(Point3::new(
            57.59035496778853,
            42.90033752958833,
            -24.223983633929485,
        )),
        radius: 57.79539718387872,
    });
    let region = Box3 {
        x: forge_math::Interval::new(102.05454626247882, 115.3857721516673),
        y: forge_math::Interval::new(51.13129435872762, 66.85711584084751),
        z: forge_math::Interval::new(15.959952664631643, 31.685774146751534),
    };
    for (a, b) in [(&t, &c), (&c, &t)] {
        let res = intersect(a, b, region).expect("a closed form");
        let SurfaceSurface::Curves(cs) = &res else {
            panic!("curves, not {res:?}")
        };
        assert_eq!(cs.len(), 2, "{res:?}");
        for (k, _) in cs {
            let d = k.domain();
            let speeds: Vec<f64> = (0..4000)
                .map(|i| k.deriv(d.lo + (d.hi - d.lo) * i as f64 / 4000.0).d1.length())
                .collect();
            let (lo, hi) = speeds
                .iter()
                .fold((f64::INFINITY, 0.0f64), |m, &v| (m.0.min(v), m.1.max(v)));
            // (a link of nothing gives a speed of rounding: 3.5e-10 of 0.2 here before the fix. A
            // short link the walk keeps where the curve turns, a thousandth of the step, still gives
            // a tangent, and is the meet's own rule)
            assert!(lo > 1e-6 * hi, "speed {lo:e} of {hi:e}");
            if let Curve::Intersection(ic) = k {
                let n = ic.pts.len();
                for i in 0..n {
                    let g = ic.pts[i].distance(ic.pts[(i + 1) % n]);
                    assert!(g > forge_math::tol::LINEAR, "link {i} of {n}: {g:e}");
                }
            }
        }
    }
}

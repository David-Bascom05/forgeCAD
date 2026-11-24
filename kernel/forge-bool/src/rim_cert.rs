//! A root of a surface along an edge certified unique within the edge's reach, for the test
//! oracle (C.2910's rim point, C.place's K1 RootUnique): along a circle or a line, the other
//! surface's implicit (a plane, a cylinder, a sphere or a torus) and its derivative are enclosed by
//! interval arithmetic from the stored data, and `krawczyk1` (forge-geom isolate.rs,
//! verify/krawczyk) proves exactly one root in the parameter interval the reach spans round the
//! point's foot. Compiled for tests and the decision log only: never run by the shipped build.
use forge_geom::isolate::krawczyk1;
use forge_geom::{Curve, Surface};
use forge_math::{Frame, Interval, Point3, Vec3};

type I3 = [Interval; 3];

fn iv(x: f64) -> Interval {
    Interval::point(x)
}
fn dot(a: I3, b: I3) -> Interval {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn v(x: Vec3) -> I3 {
    [iv(x.x), iv(x.y), iv(x.z)]
}
fn add(a: I3, b: I3) -> I3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: I3, k: Interval) -> I3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
/// `q - o` in `f`'s axes (`q` an enclosure, `o` the frame's origin).
fn local(f: &Frame, q: I3) -> I3 {
    let o = f.origin;
    let d = [q[0] - iv(o.x), q[1] - iv(o.y), q[2] - iv(o.z)];
    [
        dot(d, v(f.x().get())),
        dot(d, v(f.y().get())),
        dot(d, v(f.z().get())),
    ]
}

/// The edge's point and tangent over the parameter interval T, enclosed: a circle or a line.
fn point(c: &Curve, t: Interval) -> Option<(I3, I3)> {
    match c {
        Curve::Circle(c) => {
            let (ct, st) = (t.cos(), t.sin());
            let (x, y, r) = (v(c.frame.x().get()), v(c.frame.y().get()), iv(c.radius));
            let o = c.frame.origin;
            let q = add(
                [iv(o.x), iv(o.y), iv(o.z)],
                scale(add(scale(x, ct), scale(y, st)), r),
            );
            let dq = scale(add(scale(x, -st), scale(y, ct)), r);
            Some((q, dq))
        }
        Curve::Line(l) => {
            let (o, d) = (l.origin, v(l.dir.get()));
            Some((add([iv(o.x), iv(o.y), iv(o.z)], scale(d, t)), d))
        }
        _ => None,
    }
}

/// The surface's implicit F and its gradient at the enclosed point `q`, both in world axes'
/// terms reduced to (F, ∇F · dq): plane, cylinder, sphere, torus.
fn implicit(s: &Surface, q: I3, dq: I3) -> Option<(Interval, Interval)> {
    match s {
        Surface::Plane(p) => {
            let l = local(&p.frame, q);
            Some((l[2], dot(dq, v(p.frame.z().get()))))
        }
        Surface::Cylinder(c) => {
            let l = local(&c.frame, q);
            let dl = [dot(dq, v(c.frame.x().get())), dot(dq, v(c.frame.y().get()))];
            let r = iv(c.radius);
            Some((
                l[0] * l[0] + l[1] * l[1] - r * r,
                iv(2.0) * (l[0] * dl[0] + l[1] * dl[1]),
            ))
        }
        Surface::Sphere(sp) => {
            let l = local(&sp.frame, q);
            let r = iv(sp.radius);
            Some((dot(l, l) - r * r, iv(2.0) * dot(l, dq)))
        }
        Surface::Torus(t) => {
            // (|p|² + R² − r²)² − 4R² (px² + py²), p in the torus's axes
            let l = local(&t.frame, q);
            let f = &t.frame;
            let dl = [
                dot(dq, v(f.x().get())),
                dot(dq, v(f.y().get())),
                dot(dq, v(f.z().get())),
            ];
            let (big, small) = (iv(t.major), iv(t.minor));
            let w = dot(l, l) + big * big - small * small;
            let b4 = iv(4.0) * big * big;
            let g = w * w - b4 * (l[0] * l[0] + l[1] * l[1]);
            let dw = iv(2.0) * dot(l, dl);
            let dg = iv(2.0) * w * dw - b4 * iv(2.0) * (l[0] * dl[0] + l[1] * dl[1]);
            Some((g, dg))
        }
        _ => None,
    }
}

/// g(t) = F(edge(t)) and g'(t) enclosed over T.
fn g(c: &Curve, s: &Surface, t: Interval) -> Option<(Interval, Interval)> {
    let (q, dq) = point(c, t)?;
    implicit(s, q, dq)
}

/// Whether the other surface meets the edge (a circle or a line) exactly once within `reach` of
/// `p`'s foot on it, along its parameter (reach over the edge's speed each way): Krawczyk's test
/// on that interval. Its g' enclosure excluding 0 is `zero_unique_of_deriv_ne`'s hypothesis
/// (verify/forge-curved PlaceMerge.lean), so `rootUnique_of_param` gives RootUnique there.
pub fn root_unique(c: &Curve, s: &Surface, p: Point3, reach: f64) -> bool {
    let (t0, speed) = match c {
        Curve::Circle(c) => {
            let l = c.frame.point_to_local(p);
            (forge_math::dmath::atan2(l.y, l.x), c.radius)
        }
        Curve::Line(l) => ((p - l.origin).dot(l.dir.get()), 1.0),
        _ => return false,
    };
    let w = reach / speed;
    let x = Interval::new(t0 - w, t0 + w);
    let (Some((gm, _)), Some((_, dg))) = (g(c, s, iv(x.mid())), g(c, s, x)) else {
        return false;
    };
    krawczyk1(x, gm, dg)
}

/// C.2910's rim point: `root_unique` on a circle edge.
pub fn rim_unique(c: &forge_geom::Circle, s: &Surface, p: Point3, reach: f64) -> bool {
    root_unique(&Curve::Circle(*c), s, p, reach)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Circle, Cylinder, Plane};
    use forge_math::Frame;

    fn frame(o: [f64; 3], x: [f64; 3], y: [f64; 3], z: [f64; 3]) -> Frame {
        let v = |a: [f64; 3]| Vec3::new(a[0], a[1], a[2]);
        Frame::from_axes(Point3::new(o[0], o[1], o[2]), v(x), v(y), v(z)).expect("frame")
    }

    /// The 7 traced-branch ends on a rim the corpus meets (fuzz 3, 9, 11, 15 seq, each twice):
    /// the rim point is certified unique within LINEAR of each, as C.2910's lemma needs.
    #[test]
    fn corpus_rim_points_are_unique_within_reach() {
        let reach = forge_math::tol::LINEAR;
        // fuzz11
        let c = Circle {
            frame: frame(
                [7.929847423294825, 19.97065860328603, 39.278246874914345],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            radius: 14.557531148153451,
        };
        let s = Surface::Plane(Plane {
            frame: frame(
                [0.0, 98.87382463335877, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
                [-1.0, 0.0, 0.0],
            ),
        });
        assert!(
            rim_unique(
                &c,
                &s,
                Point3::new(0.0, 7.762496048146034, 39.27824674281932),
                reach
            ),
            "fuzz11"
        );
        // fuzz11
        let c = Circle {
            frame: frame(
                [7.929847423294825, 19.97065860328603, 39.278246874914345],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            radius: 14.557531148153451,
        };
        let s = Surface::Plane(Plane {
            frame: frame(
                [0.0, 98.87382463335877, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
                [-1.0, 0.0, 0.0],
            ),
        });
        assert!(
            rim_unique(
                &c,
                &s,
                Point3::new(0.0, 32.178821158426025, 39.27824692161699),
                reach
            ),
            "fuzz11"
        );
        // fuzz15
        let c = Circle {
            frame: frame(
                [5.427662599508642, 70.21033931774154, 32.26406536811501],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            radius: 6.969733708497163,
        };
        let s = Surface::Plane(Plane {
            frame: frame(
                [0.0, 119.98071854942444, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
                [-1.0, 0.0, 0.0],
            ),
        });
        assert!(
            rim_unique(
                &c,
                &s,
                Point3::new(0.0, 74.58271472171324, 32.264065410438704),
                reach
            ),
            "fuzz15"
        );
        // fuzz3
        let c = Circle {
            frame: frame(
                [110.88669010298408, 125.18745911711085, 1.5923447261161305],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            radius: 18.14112584893035,
        };
        let s = Surface::Cylinder(Cylinder {
            frame: frame(
                [102.42768534304938, 131.63412208031122, -0.031074021995117107],
                [0.3529922242017857, 0.9356262553247832, 0.0],
                [-0.9356262553247832, 0.3529922242017857, 0.0],
                [0.0, 0.0, 1.0],
            ),
            radius: 12.524553874451378,
        });
        assert!(
            rim_unique(
                &c,
                &s,
                Point3::new(92.81464508619882, 123.60580302996165, 1.592344391457317),
                reach
            ),
            "fuzz3"
        );
        // fuzz3
        let c = Circle {
            frame: frame(
                [110.88669010298408, 125.18745911711085, 1.5923447261161305],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            radius: 18.14112584893035,
        };
        let s = Surface::Cylinder(Cylinder {
            frame: frame(
                [102.42768534304938, 131.63412208031122, -0.031074021995117107],
                [0.3529922242017857, 0.9356262553247832, 0.0],
                [-0.9356262553247832, 0.3529922242017857, 0.0],
                [0.0, 0.0, 1.0],
            ),
            radius: 12.524553874451378,
        });
        assert!(
            rim_unique(
                &c,
                &s,
                Point3::new(107.61943062281296, 143.03193900742878, 1.5923449594358512),
                reach
            ),
            "fuzz3"
        );
        // fuzz9
        let c = Circle {
            frame: frame(
                [176.61703318906888, 127.29102276089463, -0.7968858297468429],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            radius: 28.7732492127348,
        };
        let s = Surface::Plane(Plane {
            frame: frame(
                [184.4493397933867, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
            ),
        });
        assert!(
            rim_unique(
                &c,
                &s,
                Point3::new(184.4493397933867, 99.60429591364705, -0.7968849938325867),
                reach
            ),
            "fuzz9"
        );
        // fuzz9
        let c = Circle {
            frame: frame(
                [176.61703318906888, 127.29102276089463, -0.7968858297468429],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            radius: 28.7732492127348,
        };
        let s = Surface::Plane(Plane {
            frame: frame(
                [184.4493397933867, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
            ),
        });
        assert!(
            rim_unique(
                &c,
                &s,
                Point3::new(184.4493397933867, 154.9777496081422, -0.7968867367100317),
                reach
            ),
            "fuzz9"
        );
    }

    /// C.place's K1 merges on analytic edges (fuzz11: a circle on a torus; fuzz25: a circle on a
    /// cylinder): both cuts' points within the reach of one root, unique there (RootUnique).
    #[test]
    fn k1_roots_are_unique_within_reach() {
        let reach = forge_math::tol::LINEAR;
        let c = Curve::Circle(forge_geom::Circle {
            frame: frame(
                [6.107868907256708, 18.124954070216912, 39.30910054667453],
                [-0.8516005092987792, 0.5241913510942927, 0.0],
                [-0.5241913510942927, -0.8516005092987792, 0.0],
                [0.0, 0.0, 1.0],
            ),
            radius: 10.826236066006198,
        });
        let s = Surface::Torus(forge_geom::Torus {
            frame: frame(
                [7.929847423294825, 19.97065860328603, 39.278246874914345],
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            major: 11.423369799639682,
            minor: 3.134161348513769,
        });
        for p in [
            Point3::new(14.987282368160411, 24.31877388897298, 39.30910054667453),
            Point3::new(14.987282371957644, 24.318773885872172, 39.309100546674586),
        ] {
            assert!(root_unique(&c, &s, p, reach), "fuzz11");
        }
        let c = Curve::Circle(forge_geom::Circle {
            frame: frame(
                [157.43066622875125, 86.87061528866299, 15.298280159114498],
                [-0.5506317739175499, 0.6787577824636791, 0.48589352979577843],
                [-0.8011306688784643, -0.59322981520166, -0.07917093998525482],
                [0.23450863722847534, -0.43285824365581077, 0.8704248617568607],
            ),
            radius: 3.198467012672654,
        });
        let s = Surface::Cylinder(Cylinder {
            frame: frame(
                [169.54192085587638, 95.29206331364055, 11.146996668710393],
                [-0.9564285803945236, 0.29196638608668, 0.0],
                [-0.29196638608668, -0.9564285803945236, 0.0],
                [0.0, 0.0, 1.0],
            ),
            radius: 17.942032741713966,
        });
        for p in [
            Point3::new(154.83228405587636, 85.01864498687019, 15.077356380134928),
            Point3::new(154.8322840590951, 85.01864498225716, 15.077356376973611),
        ] {
            assert!(root_unique(&c, &s, p, reach), "fuzz25");
        }
    }

    /// Two roots within the reach (a line through a unit ball, the reach 3): not certified.
    #[test]
    fn two_roots_within_reach_are_not_certified() {
        let line = Curve::Line(forge_geom::Line {
            origin: Point3::ORIGIN,
            dir: forge_math::UnitVec3::X,
        });
        let ball = Surface::Sphere(forge_geom::Sphere {
            frame: Frame::WORLD,
            radius: 1.0,
        });
        assert!(!root_unique(&line, &ball, Point3::new(1.0, 0.0, 0.0), 3.0));
        assert!(root_unique(&line, &ball, Point3::new(1.0, 0.0, 0.0), 0.5));
    }
}

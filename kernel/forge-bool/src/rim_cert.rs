//! The rim point of a traced branch ending on an edge (C.2910), certified for the test oracle: a
//! circle edge against a plane or a cylinder, the other surface's implicit along the circle is
//! g(t) = c0 + c1 cos t + c2 sin t + c3 cos 2t + c4 sin 2t, its coefficients enclosed from the
//! stored data, and `krawczyk1` (forge-geom isolate.rs, verify/krawczyk) proves exactly one root of
//! g in the parameter interval the edge's reach spans round the branch end's foot. Compiled for
//! tests and the decision log only: never run by the shipped build.
use forge_geom::isolate::krawczyk1;
use forge_geom::{Circle, Surface};
use forge_math::{Interval, Point3, Vec3};

fn iv(x: f64) -> Interval {
    Interval::point(x)
}
fn dot(a: [Interval; 3], b: [Interval; 3]) -> Interval {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn v(x: Vec3) -> [Interval; 3] {
    [iv(x.x), iv(x.y), iv(x.z)]
}
fn d(a: Point3, b: Point3) -> [Interval; 3] {
    [iv(a.x) - iv(b.x), iv(a.y) - iv(b.y), iv(a.z) - iv(b.z)]
}

/// g's coefficients for a circle on a plane or a cylinder, enclosed; None for other surfaces.
fn coeffs(c: &Circle, s: &Surface) -> Option<[Interval; 5]> {
    let (o, x, y, r) = (
        c.frame.origin,
        v(c.frame.x().get()),
        v(c.frame.y().get()),
        iv(c.radius),
    );
    let zero = iv(0.0);
    match s {
        Surface::Plane(p) => {
            let n = v(p.frame.z().get());
            Some([
                dot(d(o, p.frame.origin), n),
                r * dot(x, n),
                r * dot(y, n),
                zero,
                zero,
            ])
        }
        Surface::Cylinder(cy) => {
            // |q - a|^2 - ((q - a).d)^2 - rho^2 with q - a = w + r (x cos t + y sin t)
            let (w, dd) = (d(o, cy.frame.origin), v(cy.frame.z().get()));
            let (k, al, be) = (dot(w, dd), dot(x, dd), dot(y, dd));
            let (wx, wy) = (dot(w, x), dot(w, y));
            let r2 = r * r;
            let half = iv(0.5);
            Some([
                dot(w, w) + r2 - iv(cy.radius) * iv(cy.radius) - k * k - half * r2 * (al * al + be * be),
                iv(2.0) * r * (wx - k * al),
                iv(2.0) * r * (wy - k * be),
                -(half * r2 * (al * al - be * be)),
                -(r2 * al * be),
            ])
        }
        _ => None,
    }
}

/// g and g' enclosed over T.
fn g(c: &[Interval; 5], t: Interval) -> (Interval, Interval) {
    let (ct, st) = (t.cos(), t.sin());
    let t2 = t * 2.0;
    let (c2, s2) = (t2.cos(), t2.sin());
    (
        c[0] + c[1] * ct + c[2] * st + c[3] * c2 + c[4] * s2,
        c[2] * ct - c[1] * st + iv(2.0) * (c[4] * c2 - c[3] * s2),
    )
}

/// Whether the other surface meets the circle exactly once within `reach` (along the circle's
/// parameter, reach / radius each way) of `p`'s foot on it: Krawczyk's test on that interval.
pub fn rim_unique(c: &Circle, s: &Surface, p: Point3, reach: f64) -> bool {
    let Some(k) = coeffs(c, s) else {
        return false;
    };
    let l = c.frame.point_to_local(p);
    let t0 = forge_math::dmath::atan2(l.y, l.x);
    let w = reach / c.radius;
    let x = Interval::new(t0 - w, t0 + w);
    let (gm, _) = g(&k, iv(x.mid()));
    let (_, dg) = g(&k, x);
    krawczyk1(x, gm, dg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Cylinder, Plane};
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
}

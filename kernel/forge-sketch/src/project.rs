//! Projected references: a piece of the part's 3D geometry (a point, an axis, an edge) laid on a
//! sketch's plane, as the sketch entity that holds its image exactly (a point, line, circle, arc
//! or ellipse), or a fitted cubic within a tolerance for a curve no entity holds. Read afresh from
//! the part on every rebuild, so the sketch follows it: Forge's `project(..)` and the app's
//! Project Geometry both lay their sources flat through `flat`.

use forge_geom::{Curve, CurveGeom, Line};
use forge_math::{Frame, Interval, Point2, Point3, dmath, tol};

type R<T> = Result<T, String>;

/// A piece of the part's 3D geometry to lay on a sketch's plane.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Point(Point3),
    Axis(Line),
    Edge {
        curve: Curve,
        range: Interval,
        start: Point3,
        end: Point3,
        closed: bool,
    },
}

/// A source laid in the plane.
#[derive(Clone, Debug, PartialEq)]
pub enum Flat {
    Point(Point2),
    Line(Point2, Point2),
    Circle(Point2, f64),
    Arc {
        center: Point2,
        start: Point2,
        end: Point2,
        ccw: bool,
    },
    /// One end of its major axis at `major`, semi-minor `b`; an arc of it between `ends`.
    Ellipse {
        center: Point2,
        major: Point2,
        b: f64,
        ends: Option<(Point2, Point2, bool)>,
    },
    /// The cubic through these points, fitted to a curve no sketch entity holds exactly.
    Spline(Vec<Point2>, bool),
}

fn in_plane(plane: &Frame, p: Point3) -> Point2 {
    let q = plane.point_to_local(p);
    Point2::new(q.x, q.y)
}

/// A conic's frame seen edge-on: P(t) = O + a cos t x + b sin t y runs along one line of the plane,
/// so the result is that line from its least reach over the range to its most.
fn edge_on(plane: &Frame, f: &Frame, (a, b): (f64, f64), range: Interval, ends: (Point3, Point3)) -> R<Flat> {
    let Some(u) = plane.z().get().cross(f.z().get()).normalize() else {
        return Err("the curve's plane has no direction across the sketch's".into());
    };
    let (ca, cb) = (a * f.x().get().dot(u.get()), b * f.y().get().dot(u.get()));
    let amp = (ca * ca + cb * cb).sqrt();
    let c = in_plane(plane, f.origin);
    let (ux, uy) = (u.get().dot(plane.x().get()), u.get().dot(plane.y().get()));
    let along = |s: f64| Point2::new(c.x + s * ux, c.y + s * uy);
    let at = |t: f64| ca * dmath::cos(t) + cb * dmath::sin(t);
    let mut stops = vec![
        (at(range.lo), in_plane(plane, ends.0)),
        (at(range.hi), in_plane(plane, ends.1)),
    ];
    // the reach's turning points inside the range: the curve doubles back there
    let t0 = dmath::atan2(cb, ca);
    let mut n = ((range.lo - t0) / dmath::PI).ceil();
    while t0 + n * dmath::PI <= range.hi {
        let s = if (n as i64).rem_euclid(2) == 0 { amp } else { -amp };
        stops.push((s, along(s)));
        n += 1.0;
    }
    let lo = stops.iter().fold(stops[0], |m, s| if s.0 < m.0 { *s } else { m });
    let hi = stops.iter().fold(stops[0], |m, s| if s.0 > m.0 { *s } else { m });
    Ok(if hi.0 - lo.0 <= tol::LINEAR {
        Flat::Point(lo.1)
    } else {
        Flat::Line(lo.1, hi.1)
    })
}

/// A circle's or ellipse's image, O + cos t A + sin t B in the plane: an ellipse (A, B conjugate
/// semi-diameters), its axes where |A cos t + B sin t| is greatest and least.
fn conic(plane: &Frame, f: &Frame, (a, b): (f64, f64), ends: Option<(Point3, Point3)>) -> R<Flat> {
    let (px, py) = (plane.x().get(), plane.y().get());
    let (x, y) = (f.x().get() * a, f.y().get() * b);
    let (ax, ay, bx, by) = (x.dot(px), x.dot(py), y.dot(px), y.dot(py));
    let th = 0.5 * dmath::atan2(2.0 * (ax * bx + ay * by), ax * ax + ay * ay - bx * bx - by * by);
    let (s, c) = dmath::sin_cos(th);
    let (mx, my) = (ax * c + bx * s, ay * c + by * s);
    let (nx, ny) = (bx * c - ax * s, by * c - ay * s);
    let (ma, mb) = ((mx * mx + my * my).sqrt(), (nx * nx + ny * ny).sqrt());
    let center = in_plane(plane, f.origin);
    let ccw = ax * by - ay * bx > 0.0;
    let (ux, uy) = (mx / ma, my / ma);
    let Some((p, q)) = ends else {
        return Ok(Flat::Ellipse {
            center,
            major: Point2::new(center.x + mx, center.y + my),
            b: mb,
            ends: None,
        });
    };
    let (p, q) = (in_plane(plane, p), in_plane(plane, q));
    // each end along the axes, squared
    let uv = |r: Point2| {
        let (dx, dy) = (r.x - center.x, r.y - center.y);
        let (u, v) = (dx * ux + dy * uy, dy * ux - dx * uy);
        (u * u, v * v)
    };
    let ((u1, v1), (u2, v2)) = (uv(p), uv(q));
    let off = |a: f64, b: f64, (u, v): (f64, f64)| ((u / (a * a) + v / (b * b)).sqrt() - 1.0).abs() * a;
    // ends a boolean settled off the curve: the axes stretched the hair that puts both on it
    let (na, nb) = if off(ma, mb, (u1, v1)).max(off(ma, mb, (u2, v2))) <= 1e-11 {
        (ma, mb)
    } else {
        let det = u1 * v2 - u2 * v1;
        let (ia, ib) = if det.abs() > 1e-6 * (u1 + u2) * (v1 + v2) {
            ((v2 - v1) / det, (u1 - u2) / det)
        } else {
            // ends alike about an axis: one equation, the major kept
            let ia = 1.0 / (ma * ma);
            (ia, (1.0 - u1 * ia) / v1.max(1e-300))
        };
        let (na, nb) = (1.0 / ia.sqrt(), 1.0 / ib.sqrt());
        if !(na.is_finite() && nb.is_finite())
            || (na - ma).abs().max((nb - mb).abs()) > 10.0 * tol::LINEAR
            || off(na, nb, (u1, v1)).max(off(na, nb, (u2, v2))) > 1e-10
        {
            return Err("the ends of this edge are not on its curve, as projected".into());
        }
        (na, nb)
    };
    Ok(Flat::Ellipse {
        center,
        major: Point2::new(center.x + ux * na, center.y + uy * na),
        b: nb,
        ends: Some((p, q, ccw)),
    })
}

/// The cubic through points of a curve's image, as many as hold it within `tol` of the image both
/// ways (each found by search over the other's parameter), or why none does at 512 points.
fn fit(
    plane: &Frame,
    curve: &Curve,
    range: Interval,
    ends: (Point3, Point3),
    closed: bool,
    tol: f64,
) -> R<Flat> {
    let at = |t: f64| in_plane(plane, curve.eval(t));
    let near = |p: Point2, f: &dyn Fn(f64) -> Point2, lo: f64, hi: f64| {
        let d = |t: f64| f(t).distance(p);
        let k = (0..=16)
            .map(|i| lo + (hi - lo) * i as f64 / 16.0)
            .min_by(|a, b| d(*a).total_cmp(&d(*b)))
            .expect("samples");
        let w = (hi - lo) / 16.0;
        let (mut a, mut b) = ((k - w).max(lo), (k + w).min(hi));
        for _ in 0..60 {
            let (m1, m2) = (a + (b - a) / 3.0, b - (b - a) / 3.0);
            if d(m1) < d(m2) {
                b = m2;
            } else {
                a = m1;
            }
        }
        d(0.5 * (a + b)).min(d(k))
    };
    let mut n = 4;
    while n <= 512 {
        let ts: Vec<f64> = (0..=n)
            .map(|i| range.lo + range.width() * i as f64 / n as f64)
            .collect();
        let mut pts: Vec<Point2> = ts.iter().map(|&t| at(t)).collect();
        // the ends where the part has them, so neighbours share them
        (pts[0], pts[n]) = (in_plane(plane, ends.0), in_plane(plane, ends.1));
        let mut knots = vec![0.0];
        for i in 1..=n {
            knots.push(knots[i - 1] + pts[i].distance(pts[i - 1]));
        }
        // a closed curve's image is periodic, through its start again
        let made = if closed {
            pts[n] = pts[0];
            forge_geom::spline::PlaneSpline::through_periodic(pts.clone())
        } else {
            forge_geom::spline::PlaneSpline::through(pts.clone(), None, None)
        };
        let Some(sp) = made else {
            return Err("the curve's image doubles back on itself here; it has no spline through it".into());
        };
        let lo = sp.c.domain().lo;
        let on = |u: f64| {
            let q = sp.c.point(u);
            Point2::new(q.x, q.y)
        };
        let mut worst = 0.0f64;
        for i in 0..n {
            for j in 1..4 {
                let f = j as f64 / 4.0;
                let u = lo + knots[i] + (knots[i + 1] - knots[i]) * f;
                worst = worst.max(near(on(u), &at, ts[i], ts[i + 1]));
                let t = ts[i] + (ts[i + 1] - ts[i]) * f;
                worst = worst.max(near(at(t), &on, lo + knots[i], lo + knots[i + 1]));
            }
        }
        if worst <= tol {
            if closed {
                pts.pop();
            }
            return Ok(Flat::Spline(pts, closed));
        }
        n *= 2;
    }
    Err(format!(
        "no spline of 512 points or fewer holds this curve's image within {tol}: take a larger `approx`"
    ))
}

/// A source's exact image on the plane, or why it has none a sketch can hold. The tests of
/// parallel and edge-on go by how far the true image strays from what's made: at most LINEAR.
pub fn flat(plane: &Frame, s: &Source, approx: Option<f64>) -> R<Flat> {
    let z = plane.z().get();
    match s {
        Source::Point(p) => Ok(Flat::Point(in_plane(plane, *p))),
        Source::Axis(l) => {
            if l.dir.get().cross(z).length() <= tol::ANGULAR {
                return Ok(Flat::Point(in_plane(plane, l.origin)));
            }
            Err(
                "an axis has no ends, so it makes no line here unless seen end-on; project an edge along it"
                    .into(),
            )
        }
        Source::Edge {
            curve,
            range,
            start,
            end,
            closed,
        } => match curve {
            Curve::Line(_) => {
                let (a, b) = (in_plane(plane, *start), in_plane(plane, *end));
                Ok(if a.distance(b) <= tol::LINEAR {
                    Flat::Point(a)
                } else {
                    Flat::Line(a, b)
                })
            }
            Curve::Circle(k) => {
                let cos = k.frame.z().get().dot(z);
                if k.radius * cos.abs() <= tol::LINEAR {
                    return edge_on(plane, &k.frame, (k.radius, k.radius), *range, (*start, *end));
                }
                if k.radius * (1.0 - cos.abs()) > tol::LINEAR {
                    return conic(plane, &k.frame, (k.radius, k.radius), (!*closed).then_some((*start, *end)));
                }
                let center = in_plane(plane, k.frame.origin);
                if *closed {
                    return Ok(Flat::Circle(center, k.radius));
                }
                let (a, b) = (in_plane(plane, *start), in_plane(plane, *end));
                // ends a boolean settled off the circle: the centre moved the hair that keeps them equidistant
                let center = if (a.distance(center) - b.distance(center)).abs() > 1e-11 {
                    let (dx, dy) = (b.x - a.x, b.y - a.y);
                    let t = ((a.x + b.x) / 2.0 - center.x) * dx + ((a.y + b.y) / 2.0 - center.y) * dy;
                    let t = t / (dx * dx + dy * dy);
                    Point2::new(center.x + t * dx, center.y + t * dy)
                } else {
                    center
                };
                Ok(Flat::Arc {
                    center,
                    start: a,
                    end: b,
                    ccw: cos > 0.0,
                })
            }
            Curve::Ellipse(e) => {
                if e.major * e.frame.z().get().dot(z).abs() <= tol::LINEAR {
                    return edge_on(plane, &e.frame, (e.major, e.minor), *range, (*start, *end));
                }
                conic(plane, &e.frame, (e.major, e.minor), (!*closed).then_some((*start, *end)))
            }
            _ => match approx {
                Some(tol) => fit(plane, curve, *range, (*start, *end), *closed, tol),
                None => Err(
                    "this edge is a general curve: no sketch entity holds its image exactly; an `approx` tolerance TOL fits a spline within TOL mm"
                        .into(),
                ),
            },
        },
    }
}

/// Whether a source has no exact image on the plane, only a fitted one (with an `approx` tolerance).
pub fn needs_fit(plane: &Frame, s: &Source) -> bool {
    matches!(s, Source::Edge { curve, .. } if !matches!(curve, Curve::Line(_) | Curve::Circle(_) | Curve::Ellipse(_)))
        && flat(plane, s, None).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::Circle;
    use forge_math::Vec3;

    fn edge(curve: Curve, range: Interval, start: Point3, end: Point3) -> Source {
        Source::Edge {
            curve,
            range,
            start,
            end,
            closed: false,
        }
    }

    fn at_tilt(c: f64) -> Frame {
        let s = (1.0 - c * c).sqrt();
        Frame::from_z(Point3::ORIGIN, Vec3::new(0.0, s, c).normalize().unwrap())
    }

    #[test]
    fn what_a_sketch_has_no_curve_for_is_refused_and_edge_on_is_a_line() {
        let z = Frame::default();
        let e = forge_geom::Ellipse {
            frame: z,
            major: 6.0,
            minor: 3.0,
        };
        let (a, b) = (Point3::new(6.0, 0.0, 0.0), Point3::new(-6.0, 0.0, 0.0));
        let whole = Interval::new(0.0, dmath::TAU);
        let source = edge(Curve::Ellipse(e), whole, a, a);
        assert!(matches!(
            flat(&z, &source, None),
            Ok(Flat::Ellipse { b: 3.0, .. })
        ));
        // from the side (y up the sketch's normal), its major across
        let side = Frame::from_z(Point3::ORIGIN, Vec3::new(0.0, 1.0, 0.0).normalize().unwrap());
        let Flat::Line(p, q) = flat(&side, &source, None).unwrap() else {
            panic!("a line")
        };
        assert!((p.distance(q) - 12.0).abs() < 1e-12, "{p:?} {q:?}");
        // a half of it from the same side: from end to end, the far ends
        let half = edge(Curve::Ellipse(e), Interval::new(0.0, dmath::PI), a, b);
        let Flat::Line(p, q) = flat(&side, &half, None).unwrap() else {
            panic!("a line")
        };
        assert!((p.distance(q) - 12.0).abs() < 1e-12);
        let par = forge_geom::Parabola { frame: z, focal: 1.0 };
        let g = edge(Curve::Parabola(par), Interval::new(-1.0, 1.0), a, b);
        assert!(flat(&z, &g, None).unwrap_err().contains("general curve"));
        // a circle one hair off parallel is still it; further, an ellipse named by its tilt
        let k = Curve::Circle(Circle {
            frame: z,
            radius: 10.0,
        });
        let round = |c: f64| {
            flat(
                &at_tilt(c),
                &Source::Edge {
                    curve: k.clone(),
                    range: whole,
                    start: Point3::new(10.0, 0.0, 0.0),
                    end: Point3::new(10.0, 0.0, 0.0),
                    closed: true,
                },
                None,
            )
        };
        assert!(matches!(round(1.0 - 1e-7), Ok(Flat::Circle(..))));
        assert!(matches!(round(1.0 - 1e-5), Ok(Flat::Ellipse { ends: None, .. })));
        assert!(matches!(round(1e-7), Ok(Flat::Line(..))));
    }
}

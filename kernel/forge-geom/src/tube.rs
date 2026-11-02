//! What keeps a given distance from a curve: a ball rolling on a sharp rim that is no line or
//! circle. At distance 0 it is the rim itself as a surface of no thickness, whose offset is the
//! tube round it, as a cylinder of radius 0 is for a line.

use std::sync::Arc;

use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec3, dmath};

use crate::curve::{Curve, CurveGeom, Domain1};
use crate::surface::{Domain2, Proc, Procedural, SurfDeriv, Surface};

/// Sections per curve for what is asked of its whole length.
const SAMPLES: usize = 64;

#[derive(Clone, Debug)]
pub struct Tube {
    curve: Curve,
    range: Interval,
    radius: f64,
    /// What a section's angle is measured from: never along the curve.
    up: Vec3,
    /// The curve's tightest radius of curvature, sampled.
    tightest: f64,
}

impl Tube {
    /// The whole of the curve holding the edge over `edge` as a surface of no thickness (the
    /// ball's touch may lie past the edge's end, and only then is it seen to be); None where no
    /// one direction stands off its tangent all along.
    pub fn rail(curve: &Curve, edge: Interval) -> Option<Surface> {
        let d = curve.domain();
        let range = match d.period {
            Some(p) => Interval::new(edge.lo, edge.lo + p),
            None if d.lo.is_finite() && d.hi.is_finite() => {
                Interval::new(d.lo.min(edge.lo), d.hi.max(edge.hi))
            }
            None => edge,
        };
        let at = |i: usize| range.lo + range.width() * i as f64 / SAMPLES as f64;
        let mut tangents = Vec::with_capacity(SAMPLES + 1);
        let mut tightest = f64::INFINITY;
        for i in 0..=SAMPLES {
            let d = curve.deriv(at(i));
            let speed = d.d1.length();
            if speed <= 1e-12 {
                return None;
            }
            let bend = d.d1.cross(d.d2).length();
            if bend > 0.0 {
                tightest = tightest.min(speed * speed * speed / bend);
            }
            tangents.push(d.d1 / speed);
        }
        let leaning = |a: Vec3| tangents.iter().fold(0.0f64, |m, t| m.max(t.dot(a).abs()));
        // a plane curve's normal, where it has one, else whichever axis is farthest off
        let mid = curve.deriv(range.mid());
        let flat = mid.d1.cross(mid.d2).normalize().map(|n| n.get());
        let candidates = flat.into_iter().chain([
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ]);
        let mut up = None;
        for a in candidates {
            let l = leaning(a);
            if up.is_none_or(|(_, best)| l < best) {
                up = Some((a, l));
            }
        }
        let (up, lean) = up?;
        if lean > 0.99 {
            return None;
        }
        Some(Surface::Procedural(Proc(Arc::new(Tube {
            curve: curve.clone(),
            range,
            radius: 0.0,
            up,
            tightest,
        }))))
    }

    /// The same at distance `d` further out.
    pub fn grown(&self, d: f64) -> Option<Surface> {
        let radius = self.radius + d;
        (radius > 0.0).then(|| {
            Surface::Procedural(Proc(Arc::new(Tube {
                radius,
                ..self.clone()
            })))
        })
    }

    /// Point of the curve, unit tangent, and the section's two axes, with their rates along u.
    fn section(&self, u: f64) -> Section {
        let d = self.curve.deriv(u);
        let speed = d.d1.length();
        let t = d.d1 / speed;
        let t1 = (d.d2 - t * t.dot(d.d2)) / speed;
        let lean = t.dot(self.up);
        let a = self.up - t * lean;
        let a1 = -(t1 * lean + t * t1.dot(self.up));
        let len = a.length();
        let n = a / len;
        let n1 = (a1 - n * n.dot(a1)) / len;
        // so that du x dv points away from the curve
        let b = n.cross(t);
        let b1 = n1.cross(t) + n.cross(t1);
        Section {
            c: d.p,
            d1: d.d1,
            n,
            n1,
            b,
            b1,
        }
    }

    fn first(&self, uv: Point2) -> (Point3, Vec3, Vec3, Section) {
        let x = self.section(uv.x);
        let (s, k) = dmath::sin_cos(uv.y);
        let r = self.radius;
        (
            x.c + (x.n * k + x.b * s) * r,
            x.d1 + (x.n1 * k + x.b1 * s) * r,
            (x.b * k - x.n * s) * r,
            x,
        )
    }

    fn nearest(&self, p: Point3) -> (f64, Point3) {
        let u = self.curve.closest(p, self.range);
        (u, self.curve.eval(u))
    }
}

struct Section {
    c: Point3,
    d1: Vec3,
    n: Vec3,
    n1: Vec3,
    b: Vec3,
    b1: Vec3,
}

impl Procedural for Tube {
    fn kind(&self) -> &'static str {
        "offset"
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let (p, du, dv, x) = self.first(uv);
        let (s, k) = dmath::sin_cos(uv.y);
        let r = self.radius;
        let speed = x.d1.length();
        let h = 1e-6 / speed.max(1e-9);
        let (_, u1, _, _) = self.first(Point2::new(uv.x + h, uv.y));
        let (_, u0, _, _) = self.first(Point2::new(uv.x - h, uv.y));
        SurfDeriv {
            p,
            du,
            dv,
            duu: (u1 - u0) / (2.0 * h),
            duv: (x.b1 * k - x.n1 * s) * r,
            dvv: (x.n * k + x.b * s) * -r,
        }
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: Domain1 {
                lo: self.range.lo,
                hi: self.range.hi,
                period: None,
            },
            v: Domain1 {
                lo: 0.0,
                hi: 2.0 * std::f64::consts::PI,
                period: Some(2.0 * std::f64::consts::PI),
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let (u, q) = self.nearest(p);
        let x = self.section(u);
        let w = p - q;
        let v = dmath::atan2(w.dot(x.b), w.dot(x.n));
        let v = if v < 0.0 {
            v + 2.0 * std::f64::consts::PI
        } else {
            v
        };
        Point2::new(u, v)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        self.curve.bbox(uv.u).grow(self.radius)
    }
    fn implicit(&self, p: Point3) -> f64 {
        p.distance(self.nearest(p).1) - self.radius
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.implicit_gradient(p).1
    }
    fn implicit_gradient(&self, p: Point3) -> (f64, Vec3) {
        let (u, q) = self.nearest(p);
        let w = p - q;
        let l = w.length();
        if l > 0.0 {
            (l - self.radius, w / l)
        } else {
            (-self.radius, self.section(u).n)
        }
    }
    fn transformed(&self, f: &Frame) -> Proc {
        Proc(Arc::new(Tube {
            curve: self.curve.transformed(f),
            up: f.vec_to_world(self.up),
            ..self.clone()
        }))
    }
    fn uv_scale(&self) -> (f64, f64) {
        let d = self.curve.deriv(self.range.mid());
        (d.d1.length().max(1e-9), self.radius.max(1e-9))
    }
    fn mesh_radius(&self) -> f64 {
        self.radius.max(1e-9)
    }
    fn feature(&self) -> f64 {
        self.tightest.min(if self.radius > 0.0 {
            self.radius
        } else {
            f64::INFINITY
        })
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Ellipse;
    use crate::surface::SurfaceGeom;

    fn tube() -> Surface {
        let frame = Frame::from_zx(
            Point3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 0.6, 0.8).normalize().unwrap(),
            Vec3::X,
        )
        .unwrap();
        let curve = Curve::Ellipse(Ellipse {
            frame,
            major: 7.0,
            minor: 4.0,
        });
        let range = Interval::new(0.0, 2.0 * std::f64::consts::PI);
        let Surface::Procedural(p) = Tube::rail(&curve, range).unwrap() else {
            panic!()
        };
        let t =
            p.0.as_any()
                .and_then(|x| x.downcast_ref::<Tube>())
                .unwrap()
                .clone();
        t.grown(1.5).unwrap()
    }

    #[test]
    fn points_are_at_the_distance() {
        let s = tube();
        for i in 0..40 {
            let uv = Point2::new(0.07 + 0.15 * i as f64, 0.3 + 0.17 * i as f64);
            let p = s.eval(uv);
            assert!(s.implicit(p).abs() < 1e-7, "{}", s.implicit(p));
            let d = s.deriv(uv);
            let n = d.du.cross(d.dv).normalize().unwrap();
            assert!(n.dot(s.gradient(p)) > 1.0 - 1e-6);
            let back = s.project(p);
            assert!(s.eval(back).distance(p) < 1e-7);
        }
    }

    #[test]
    fn first_derivatives_match_differences() {
        let s = tube();
        let h = 1e-5;
        for i in 0..40 {
            let uv = Point2::new(0.07 + 0.15 * i as f64, 0.3 + 0.17 * i as f64);
            let d = s.deriv(uv);
            let du = (s.eval(Point2::new(uv.x + h, uv.y)) - s.eval(Point2::new(uv.x - h, uv.y))) / (2.0 * h);
            let dv = (s.eval(Point2::new(uv.x, uv.y + h)) - s.eval(Point2::new(uv.x, uv.y - h))) / (2.0 * h);
            assert!((d.du - du).length() < 1e-6);
            assert!((d.dv - dv).length() < 1e-6);
            let duv = {
                let a = s.deriv(Point2::new(uv.x, uv.y + h)).du;
                let b = s.deriv(Point2::new(uv.x, uv.y - h)).du;
                (a - b) / (2.0 * h)
            };
            assert!((d.duv - duv).length() < 1e-5);
        }
    }
}

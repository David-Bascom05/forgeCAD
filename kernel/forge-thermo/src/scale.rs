//! A body scaled about a point: the shape grown by the plastic's shrink. The analytic surfaces and
//! curves scale in closed form; an intersection curve is the same chain on the scaled surfaces;
//! a procedural surface or curve is wrapped, its points the original's scaled, so it stays exact.
use forge_geom::curve::{CurveDeriv, ProcCurve, ProceduralCurve};
use forge_geom::intcurve::IntCurve;
use forge_geom::surface::{Proc, Procedural, SurfDeriv};
use forge_geom::{
    Circle, Cone, Curve, Cylinder, Domain1, Domain2, Ellipse, Hyperbola, Line, Parabola, Plane, Sphere,
    Surface, Torus,
};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec3, dmath};
use forge_topo::{Body, Name};
use std::sync::Arc;

/// What has no scaled form: the face or edge's name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unscalable {
    Face(Name),
    Edge(Name),
}

fn frame(f: &Frame, c: Point3, k: f64) -> Frame {
    f.with_origin(c + (f.origin - c) * k)
}

fn surface(s: &Surface, c: Point3, k: f64) -> Option<Surface> {
    Some(match s {
        Surface::Plane(p) => Surface::Plane(Plane::new(frame(&p.frame, c, k))),
        Surface::Cylinder(y) => Surface::Cylinder(Cylinder {
            frame: frame(&y.frame, c, k),
            radius: y.radius * k,
        }),
        Surface::Cone(o) => Surface::Cone(Cone::new(
            frame(&o.frame, c, k),
            o.radius * k,
            dmath::atan2(o.sin_a(), o.cos_a()),
        )?),
        Surface::Sphere(s) => Surface::Sphere(Sphere {
            frame: frame(&s.frame, c, k),
            radius: s.radius * k,
        }),
        Surface::Torus(t) => Surface::Torus(Torus::new(frame(&t.frame, c, k), t.major * k, t.minor * k)?),
        Surface::Procedural(p) => Surface::Procedural(Proc(Arc::new(Scaled {
            inner: p.clone(),
            c,
            k,
        }))),
    })
}

/// A procedural surface or curve `inner` scaled by `k` about `c`, in its own parameters.
#[derive(Debug)]
struct Scaled<T> {
    inner: T,
    c: Point3,
    k: f64,
}

impl<T> Scaled<T> {
    fn to(&self, p: Point3) -> Point3 {
        self.c + (p - self.c) * self.k
    }
    fn from(&self, p: Point3) -> Point3 {
        self.c + (p - self.c) * (1.0 / self.k)
    }
    fn boxed(&self, b: Box3) -> Box3 {
        Box3::from_points([
            self.to(Point3::new(b.x.lo, b.y.lo, b.z.lo)),
            self.to(Point3::new(b.x.hi, b.y.hi, b.z.hi)),
        ])
    }
}

impl Procedural for Scaled<Proc> {
    fn kind(&self) -> &'static str {
        self.inner.kind()
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let d = self.inner.deriv(uv);
        let k = self.k;
        SurfDeriv {
            p: self.to(d.p),
            du: d.du * k,
            dv: d.dv * k,
            duu: d.duu * k,
            duv: d.duv * k,
            dvv: d.dvv * k,
        }
    }
    fn domain(&self) -> Domain2 {
        self.inner.domain()
    }
    fn project(&self, p: Point3) -> Point2 {
        self.inner.project(self.from(p))
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        self.boxed(self.inner.bbox(uv))
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.k * self.inner.implicit(self.from(p))
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.inner.gradient(self.from(p))
    }
    fn transformed(&self, f: &Frame) -> Proc {
        Proc(Arc::new(Scaled {
            inner: self.inner.transformed(f),
            c: f.point_to_world(self.c),
            k: self.k,
        }))
    }
    fn uv_scale(&self) -> (f64, f64) {
        let (u, v) = self.inner.uv_scale();
        (u * self.k, v * self.k)
    }
    fn mesh_radius(&self) -> f64 {
        self.k * self.inner.mesh_radius()
    }
    fn min_radius(&self) -> Option<f64> {
        self.inner.min_radius().map(|r| r * self.k)
    }
    fn feature(&self) -> f64 {
        self.k * self.inner.feature()
    }
}

impl ProceduralCurve for Scaled<Curve> {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let d = forge_geom::CurveGeom::deriv(&self.inner, t);
        CurveDeriv {
            p: self.to(d.p),
            d1: d.d1 * self.k,
            d2: d.d2 * self.k,
        }
    }
    fn domain(&self) -> Domain1 {
        forge_geom::CurveGeom::domain(&self.inner)
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        forge_geom::CurveGeom::closest(&self.inner, self.from(p), range)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        self.boxed(forge_geom::CurveGeom::bbox(&self.inner, range))
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Scaled {
            inner: forge_geom::CurveGeom::transformed(&self.inner, f),
            c: f.point_to_world(self.c),
            k: self.k,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.k
            * match &self.inner {
                Curve::Procedural(p) => p.speed_bound(range),
                Curve::Intersection(i) => i.speed_bound(),
                _ => 1.0,
            }
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        match &self.inner {
            Curve::Procedural(p) => p.segments(range, chord / self.k),
            Curve::Intersection(i) => i.segments(),
            _ => 1,
        }
    }
    fn kinks(&self, range: Interval) -> Vec<f64> {
        match &self.inner {
            Curve::Procedural(p) => p.kinks(range),
            _ => vec![],
        }
    }
}

/// The curve and its parameter range scaled: lengths along a line or a parabola's axis scale,
/// angles do not.
fn curve(e: &Curve, r: Interval, c: Point3, k: f64) -> Option<(Curve, Interval)> {
    let by_k = Interval::new(r.lo * k, r.hi * k);
    Some(match e {
        Curve::Line(l) => (
            Curve::Line(Line {
                origin: c + (l.origin - c) * k,
                dir: l.dir,
            }),
            by_k,
        ),
        Curve::Circle(o) => (
            Curve::Circle(Circle {
                frame: frame(&o.frame, c, k),
                radius: o.radius * k,
            }),
            r,
        ),
        Curve::Ellipse(o) => (
            Curve::Ellipse(Ellipse {
                frame: frame(&o.frame, c, k),
                major: o.major * k,
                minor: o.minor * k,
            }),
            r,
        ),
        Curve::Parabola(p) => (
            Curve::Parabola(Parabola {
                frame: frame(&p.frame, c, k),
                focal: p.focal * k,
            }),
            by_k,
        ),
        Curve::Hyperbola(h) => (
            Curve::Hyperbola(Hyperbola {
                frame: frame(&h.frame, c, k),
                a: h.a * k,
                b: h.b * k,
            }),
            r,
        ),
        Curve::Intersection(i) => (
            Curve::Intersection(Arc::new(IntCurve::new(
                surface(&i.a, c, k)?,
                surface(&i.b, c, k)?,
                i.pts.iter().map(|&p| c + (p - c) * k).collect(),
                i.tans.clone(),
                i.closed,
                i.sag * k,
            ))),
            r,
        ),
        Curve::Procedural(_) => (
            Curve::Procedural(ProcCurve(Arc::new(Scaled {
                inner: e.clone(),
                c,
                k,
            }))),
            r,
        ),
    })
}

/// `body` scaled by `k` (> 0) about `c`.
pub fn scaled(body: &Body, c: Point3, k: f64) -> Result<Body, Unscalable> {
    let mut out = body.clone();
    for f in &mut out.faces {
        if f.alive {
            f.surface = surface(&f.surface, c, k).ok_or(Unscalable::Face(f.name))?;
            f.bbox.clear();
            f.query.clear();
        }
    }
    for e in &mut out.edges {
        if e.alive {
            (e.curve, e.range) = curve(&e.curve, e.range, c, k).ok_or(Unscalable::Edge(e.name))?;
            e.bbox.clear();
        }
    }
    for v in &mut out.vertices {
        v.point = c + (v.point - c) * k;
        v.tol *= k.max(1.0);
    }
    out.hulls.clear();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Point2;
    use forge_ops::Profile;

    #[test]
    fn a_rounded_block_scales_exactly() {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rounded_rectangle(
                Point2::new(-20.0, -10.0),
                40.0,
                20.0,
                4.0,
                0,
            )],
        };
        let b = forge_ops::extrude(&p, 15.0, 7).unwrap();
        let k = 1.006;
        let s = scaled(&b, Point3::new(0.0, 0.0, 0.0), k).unwrap();
        assert!(forge_topo::check(&s).is_empty());
        let (m0, m1) = (
            forge_topo::mass::mass(&b).unwrap(),
            forge_topo::mass::mass(&s).unwrap(),
        );
        assert!(
            (m1.volume / m0.volume - k * k * k).abs() < 1e-9,
            "{} {}",
            m0.volume,
            m1.volume
        );
    }
}

//! The part's B-rep lowered to the checker's faces (forge_cam_geom::check::Face).
//!
//! The lowering is exact in kind and bounded in size: a plane keeps its own frame, so its points
//! are the B-rep's; its trim's vertices are projected in f64; a cylinder keeps its axis and
//! radius, its angle reparametrised by the rational half-angle map about the middle of its range.
//! The part itself is only defined to `tol::LINEAR` (its vertices and edges lie within that of
//! their faces), so every face carries mu = LINEAR plus the projection's rounding: a clearance is
//! proven by mu more, and a violation only when it is mu worse.
//!
//! Faces outside this domain (cones, spheres, tori, procedural surfaces, cylinders not bounded
//! by circles about their axis and lines along it) are refused by kind: `Unsupported`.
use forge_cam_geom::check::{Face, Surf, TSeg, Trim, V3};
use forge_cam_geom::ival::Ival;
use forge_geom::curve::{Curve, CurveGeom};
use forge_geom::surface::Surface;
use forge_math::{Point3, Vec3, dmath, tol};
use forge_topo::{Body, FaceId};

#[derive(Clone, Debug, PartialEq)]
pub enum LowerError {
    /// A face the checker cannot take yet: its index and why.
    Unsupported { face: usize, why: String },
}

fn v3(v: Vec3) -> V3 {
    [v.x, v.y, v.z]
}
fn p3(p: Point3) -> V3 {
    [p.x, p.y, p.z]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The bound on a lowered face's distance from the true one, for a part of size `scale`.
fn mu(scale: f64) -> f64 {
    tol::LINEAR + 64.0 * f64::EPSILON * (1.0 + scale)
}

/// Every face of the body, lowered (a full cylinder becomes two halves).
pub fn lower(body: &Body) -> Result<Vec<Face>, LowerError> {
    let mut out = Vec::new();
    for (k, fid) in body.face_ids().enumerate() {
        lower_face(body, fid, k, &mut out)?;
    }
    Ok(out)
}

/// The edges of a face, as (curve, range) pairs.
fn edges(body: &Body, fid: FaceId) -> Vec<(Curve, forge_math::Interval)> {
    let mut v = Vec::new();
    for &l in body.face(fid).loops.iter() {
        for c in body.lp(l).coedges() {
            let e = body.edge(body.coedge(c).edge);
            v.push((e.curve.clone(), e.range));
        }
    }
    v
}

fn lower_face(body: &Body, fid: FaceId, k: usize, out: &mut Vec<Face>) -> Result<(), LowerError> {
    let f = body.face(fid);
    let bb = body.face_box(fid);
    let scale = [bb.x.lo, bb.y.lo, bb.z.lo, bb.x.hi, bb.y.hi, bb.z.hi].iter().fold(0.0f64, |m, x| m.max(x.abs()));
    let m = mu(scale);
    let unsupported = |why: &str| LowerError::Unsupported { face: k, why: why.to_string() };
    match &f.surface {
        Surface::Plane(pl) => {
            let o = p3(pl.frame.origin);
            let (x, y, z) = (v3(pl.frame.x().get()), v3(pl.frame.y().get()), v3(pl.frame.z().get()));
            let proj = |p: V3| -> [f64; 2] {
                let d = sub(p, o);
                [dot(d, x), dot(d, y)]
            };
            let mut segs = Vec::new();
            let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
            let mut grow = |p: [f64; 2], r: f64| {
                for i in 0..2 {
                    lo[i] = lo[i].min(p[i] - r);
                    hi[i] = hi[i].max(p[i] + r);
                }
            };
            for (cv, range) in edges(body, fid) {
                match &cv {
                    Curve::Line(_) => {
                        let (a, b) = (proj(p3(cv.eval(range.lo))), proj(p3(cv.eval(range.hi))));
                        grow(a, 0.0);
                        grow(b, 0.0);
                        segs.push(TSeg::Line { a, b });
                    }
                    Curve::Circle(c) => {
                        let cz = v3(c.frame.z().get());
                        if (dot(cz, z).abs() - 1.0).abs() > 1e-12 {
                            return Err(unsupported("a circle edge not in its plane's orientation"));
                        }
                        let ccw = dot(cz, z) > 0.0;
                        let cc = proj(p3(c.frame.origin));
                        let w = range.hi - range.lo;
                        let n = ((w / (0.5 * std::f64::consts::PI)).ceil() as usize).max(1);
                        let mut prev = proj(p3(cv.eval(range.lo)));
                        for i in 1..=n {
                            let t = if i == n { range.hi } else { range.lo + w * i as f64 / n as f64 };
                            let p = proj(p3(cv.eval(t)));
                            segs.push(TSeg::Arc { a: prev, b: p, c: cc, r: c.radius, ccw });
                            prev = p;
                        }
                        grow(cc, c.radius);
                    }
                    _ => return Err(unsupported("a planar face bounded by a curve other than lines and circles")),
                }
            }
            let pad = m;
            out.push(Face::new(
                Surf::Plane { o, u: x, v: y },
                Ival::new(lo[0] - pad, hi[0] + pad),
                Ival::new(lo[1] - pad, hi[1] + pad),
                Some(Trim { segs }),
                m,
                k as u32,
            ));
            Ok(())
        }
        Surface::Cylinder(cy) => {
            let o = p3(cy.frame.origin);
            let (x, y, z) = (v3(cy.frame.x().get()), v3(cy.frame.y().get()), v3(cy.frame.z().get()));
            let (mut vlo, mut vhi) = (f64::INFINITY, f64::NEG_INFINITY);
            let mut seams: Vec<f64> = Vec::new();
            let mut mid_angle: Option<f64> = None;
            let angle = |p: V3| -> f64 {
                let d = sub(p, o);
                dmath::atan2(dot(d, y), dot(d, x))
            };
            for (cv, range) in edges(body, fid) {
                match &cv {
                    Curve::Circle(c) => {
                        let cz = v3(c.frame.z().get());
                        let co = sub(p3(c.frame.origin), o);
                        let off = [co[0] - dot(co, z) * z[0], co[1] - dot(co, z) * z[1], co[2] - dot(co, z) * z[2]];
                        if (dot(cz, z).abs() - 1.0).abs() > 1e-12 || dot(off, off).sqrt() > tol::LINEAR || (c.radius - cy.radius).abs() > tol::LINEAR {
                            return Err(unsupported("a cylinder edge that is not a circle about its axis"));
                        }
                        let v = dot(co, z);
                        vlo = vlo.min(v);
                        vhi = vhi.max(v);
                        if mid_angle.is_none() {
                            mid_angle = Some(angle(p3(cv.eval(0.5 * (range.lo + range.hi)))));
                        }
                    }
                    Curve::Line(l) => {
                        if (dot(v3(l.dir.get()), z).abs() - 1.0).abs() > 1e-12 {
                            return Err(unsupported("a cylinder edge line not along its axis"));
                        }
                        let (a, b) = (p3(cv.eval(range.lo)), p3(cv.eval(range.hi)));
                        seams.push(angle(a));
                        for p in [a, b] {
                            let v = dot(sub(p, o), z);
                            vlo = vlo.min(v);
                            vhi = vhi.max(v);
                        }
                    }
                    _ => return Err(unsupported("a cylinder edge other than a circle or a line")),
                }
            }
            seams.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
            // the angular range [u0, u1] (u1 − u0 ≤ 2π)
            let tau = std::f64::consts::TAU;
            let (u0, u1) = if seams.len() <= 1 {
                let s = seams.first().copied().unwrap_or(0.0);
                (s, s + tau)
            } else if seams.len() == 2 {
                let (a, b) = (seams[0].min(seams[1]), seams[0].max(seams[1]));
                let mid = mid_angle.ok_or_else(|| unsupported("a cylinder strip with no circle edge"))?;
                let m = if mid < a { mid + tau } else { mid };
                if m > a && m < b { (a, b) } else { (b, a + tau) }
            } else {
                return Err(unsupported("a cylinder face with more than two seams"));
            };
            // pieces of at most a half turn each, by the rational map about their middles
            let w = u1 - u0;
            let n = ((w / std::f64::consts::PI) - 1e-12).ceil().max(1.0) as usize;
            let half = 0.5 * w / n as f64;
            // t range: tan(half/2), padded outward by a relative 1e-12 (inside mu)
            let th = dmath::sin(0.5 * half) / dmath::cos(0.5 * half) * (1.0 + 1e-12);
            for i in 0..n {
                let um = u0 + half * (2 * i + 1) as f64;
                let (s, c) = dmath::sin_cos(um);
                let xm = [c * x[0] + s * y[0], c * x[1] + s * y[1], c * x[2] + s * y[2]];
                let ym = [-s * x[0] + c * y[0], -s * x[1] + c * y[1], -s * x[2] + c * y[2]];
                out.push(Face::new(
                    Surf::Cyl { o, x: xm, y: ym, ax: z, r: cy.radius },
                    Ival::new(-th, th),
                    Ival::new(vlo - m, vhi + m),
                    None,
                    m,
                    k as u32,
                ));
            }
            Ok(())
        }
        s => Err(unsupported(&format!("a {} face (this phase lowers planes and cylinders)", s.kind()))),
    }
}

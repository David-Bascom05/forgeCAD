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

/// Every face of the body, lowered (a full cylinder becomes four quarters).
pub fn lower(body: &Body) -> Result<Vec<Face>, LowerError> {
    let mut out = Vec::new();
    for (k, fid) in body.face_ids().enumerate() {
        lower_face(body, fid, k, &mut out)?;
    }
    Ok(out)
}

/// The edges of a face, as (curve, range, start point, end point): the ends are the B-rep's
/// vertices, shared bit for bit by the edges that meet there, so a trim built from them closes.
fn edges(body: &Body, fid: FaceId) -> Vec<(Curve, forge_math::Interval, V3, V3)> {
    let mut v = Vec::new();
    for &l in body.face(fid).loops.iter() {
        for c in body.lp(l).coedges() {
            let e = body.edge(body.coedge(c).edge);
            v.push((
                e.curve.clone(),
                e.range,
                p3(body.vertex(e.start).point),
                p3(body.vertex(e.end).point),
            ));
        }
    }
    v
}

fn lower_face(body: &Body, fid: FaceId, k: usize, out: &mut Vec<Face>) -> Result<(), LowerError> {
    let f = body.face(fid);
    let bb = body.face_box(fid);
    let scale = [bb.x.lo, bb.y.lo, bb.z.lo, bb.x.hi, bb.y.hi, bb.z.hi]
        .iter()
        .fold(0.0f64, |m, x| m.max(x.abs()));
    let m = mu(scale);
    let unsupported = |why: &str| LowerError::Unsupported {
        face: k,
        why: why.to_string(),
    };
    match &f.surface {
        Surface::Plane(pl) => {
            let o = p3(pl.frame.origin);
            let (x, y, z) = (
                v3(pl.frame.x().get()),
                v3(pl.frame.y().get()),
                v3(pl.frame.z().get()),
            );
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
            for (cv, range, vs, ve) in edges(body, fid) {
                match &cv {
                    Curve::Line(_) => {
                        let (a, b) = (proj(vs), proj(ve));
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
                        let mut prev = proj(vs);
                        for i in 1..=n {
                            let t = range.lo + w * i as f64 / n as f64;
                            let p = if i == n { proj(ve) } else { proj(p3(cv.eval(t))) };
                            segs.push(TSeg::Arc {
                                a: prev,
                                b: p,
                                c: cc,
                                r: c.radius,
                                ccw,
                            });
                            prev = p;
                        }
                        grow(cc, c.radius);
                    }
                    _ => {
                        return Err(unsupported(
                            "a planar face bounded by a curve other than lines and circles",
                        ));
                    }
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
        Surface::Cylinder(cy) => revolve(body, fid, k, m, cy.frame, Rev::Cyl(cy.radius), out),
        Surface::Cone(cn) => revolve(body, fid, k, m, cn.frame, Rev::Cone(cn.radius, cn.tan_a()), out),
        Surface::Torus(t) => revolve(body, fid, k, m, t.frame, Rev::Torus(t.major, t.minor), out),
        s => Err(unsupported(&format!(
            "a {} face (this phase lowers planes, cylinders, cones and tori)",
            s.kind()
        ))),
    }
}

/// A surface of revolution about its frame's z.
#[derive(Clone, Copy)]
enum Rev {
    Cyl(f64),
    /// Radius at v = 0, and its change per unit of v.
    Cone(f64, f64),
    /// Major and minor radius.
    Torus(f64, f64),
}

/// A face of a surface of revolution, bounded by circles about the axis and by seams (lines
/// along a generator, or a torus's tube circles): its parameter box, cut into pieces of at most a
/// quarter turn each, by the rational half-angle map about their middles.
fn revolve(
    body: &Body,
    fid: FaceId,
    k: usize,
    m: f64,
    fr: forge_math::Frame,
    rev: Rev,
    out: &mut Vec<Face>,
) -> Result<(), LowerError> {
    let unsupported = |why: &str| LowerError::Unsupported {
        face: k,
        why: why.to_string(),
    };
    let o = p3(fr.origin);
    let (x, y, z) = (v3(fr.x().get()), v3(fr.y().get()), v3(fr.z().get()));
    let angle = |p: V3| -> f64 {
        let d = sub(p, o);
        dmath::atan2(dot(d, y), dot(d, x))
    };
    let (mut vlo, mut vhi) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut phis: Vec<f64> = Vec::new();
    let mut seams: Vec<f64> = Vec::new();
    let mut mid_angle: Option<f64> = None;
    for (cv, range, vs, ve) in edges(body, fid) {
        match &cv {
            Curve::Circle(c) => {
                let cz = v3(c.frame.z().get());
                let co = sub(p3(c.frame.origin), o);
                let h = dot(co, z);
                let off = [co[0] - h * z[0], co[1] - h * z[1], co[2] - h * z[2]];
                let about_axis =
                    (dot(cz, z).abs() - 1.0).abs() <= 1e-12 && dot(off, off).sqrt() <= tol::LINEAR;
                if about_axis {
                    let ok = match rev {
                        Rev::Cyl(r) => (c.radius - r).abs() <= tol::LINEAR,
                        Rev::Cone(r0, kk) => (c.radius - (r0 + h * kk)).abs() <= tol::LINEAR,
                        Rev::Torus(big, r) => {
                            ((c.radius - big).powi(2) + h * h).sqrt() - r <= tol::LINEAR
                                && r - ((c.radius - big).powi(2) + h * h).sqrt() <= tol::LINEAR
                        }
                    };
                    if !ok {
                        return Err(unsupported("a circle edge off its surface"));
                    }
                    vlo = vlo.min(h);
                    vhi = vhi.max(h);
                    if let Rev::Torus(big, _) = rev {
                        phis.push(dmath::atan2(h, c.radius - big));
                    }
                    if mid_angle.is_none() {
                        mid_angle = Some(angle(p3(cv.eval(0.5 * (range.lo + range.hi)))));
                    }
                } else if matches!(rev, Rev::Torus(..)) {
                    // a tube circle: a seam at its centre's angle
                    seams.push(angle(p3(c.frame.origin)));
                } else {
                    return Err(unsupported("a circle edge that is not about the axis"));
                }
            }
            Curve::Line(_) => {
                // a generator: both ends at one angle
                let (aa, ab) = (angle(vs), angle(ve));
                if (aa - ab).abs() > 1e-9 || matches!(rev, Rev::Torus(..)) {
                    return Err(unsupported("a line edge that is not a generator"));
                }
                seams.push(aa);
                for p in [vs, ve] {
                    let v = dot(sub(p, o), z);
                    vlo = vlo.min(v);
                    vhi = vhi.max(v);
                }
            }
            _ => return Err(unsupported("an edge other than a circle or a line")),
        }
    }
    seams.sort_by(|a, b| a.partial_cmp(b).unwrap());
    seams.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let tau = std::f64::consts::TAU;
    let (u0, u1) = if seams.len() <= 1 {
        let s = seams.first().copied().unwrap_or(0.0);
        (s, s + tau)
    } else if seams.len() == 2 {
        let (a, b) = (seams[0], seams[1]);
        let mid = mid_angle.ok_or_else(|| unsupported("a strip with no circle edge"))?;
        let mm = if mid < a { mid + tau } else { mid };
        if mm > a && mm < b { (a, b) } else { (b, a + tau) }
    } else {
        return Err(unsupported("a face with more than two seams"));
    };
    // the second parameter: height for cylinders and cones, the tube angle's half tangent for tori
    let mut phi_mid = 0.0;
    let q = match rev {
        Rev::Torus(..) => {
            phis.sort_by(|a, b| a.partial_cmp(b).unwrap());
            phis.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            if phis.len() != 2 {
                return Err(unsupported(
                    "a torus face not bounded by two circles about its axis",
                ));
            }
            // the face's tube arc: between the two circles, the way that passes the face's own
            // middle (a sample point of an edge's interior is on the face's closure; the tube
            // seam, if any, is not needed: a face of a torus here spans less than half its tube)
            let (p0, mut p1) = (phis[0], phis[1]);
            if p1 - p0 > std::f64::consts::PI {
                // the short way passes through ±π
                p1 -= std::f64::consts::TAU;
            }
            let (lo, hi) = (p0.min(p1), p0.max(p1));
            if hi - lo >= std::f64::consts::PI {
                return Err(unsupported("a torus face over half its tube or more"));
            }
            phi_mid = 0.5 * (lo + hi);
            let hw = 0.5 * (hi - lo);
            let t = dmath::sin(0.5 * hw) / dmath::cos(0.5 * hw);
            Ival::new(-t * (1.0 + 1e-12) - m, t * (1.0 + 1e-12) + m)
        }
        _ => Ival::new(vlo - m, vhi + m),
    };
    let w = u1 - u0;
    // pieces of at most a quarter turn: a wall's section arc is then strictly under a half
    // turn, as slab::dist's arc candidates need (DistArc.lean)
    let n = ((w / std::f64::consts::FRAC_PI_2) - 1e-12).ceil().max(1.0) as usize;
    let half = 0.5 * w / n as f64;
    let th = dmath::sin(0.5 * half) / dmath::cos(0.5 * half) * (1.0 + 1e-12);
    for i in 0..n {
        let um = u0 + half * (2 * i + 1) as f64;
        let (s, c) = dmath::sin_cos(um);
        let xm = [c * x[0] + s * y[0], c * x[1] + s * y[1], c * x[2] + s * y[2]];
        let ym = [-s * x[0] + c * y[0], -s * x[1] + c * y[1], -s * x[2] + c * y[2]];
        let surf = match rev {
            Rev::Cyl(r) => Surf::Cyl {
                o,
                x: xm,
                y: ym,
                ax: z,
                r,
            },
            Rev::Cone(r0, kk) => Surf::Cone {
                o,
                x: xm,
                y: ym,
                ax: z,
                r0,
                k: kk,
            },
            Rev::Torus(big, r) => {
                let (sm, cm) = dmath::sin_cos(phi_mid);
                Surf::Torus {
                    o,
                    x: xm,
                    y: ym,
                    ax: z,
                    big,
                    r,
                    cm,
                    sm,
                }
            }
        };
        out.push(Face::new(surf, Ival::new(-th, th), q, None, m, k as u32));
    }
    Ok(())
}

/// The part turned over about the x axis, for a second setup: (x, y, z) ↦ (x, −y, h − z), with
/// `h` = its lowest plus its highest z so it keeps its height range. A proper rotation, so
/// every face keeps its side; the parameters are renamed so that the lowering's conventions
/// (planes' u × v outward, walls' u horizontal with v up, revolved faces about +z) still hold:
/// - planes: o' = T(o), u' = −R u, v' = −R v over (−p, −q): T(o + p u + q v);
/// - cylinders, cones and tori: x' = R x, y' = −R y, ax' = −R ax over (−p, −q) (the rational
///   half-angle map is odd, so t ↦ −t mirrors the angle), a cone's k' = −k, a torus's
///   middle tube angle mirrored (sm' = −sm).
///
/// R negates y and z, exactly; h − z rounds once, by which every face's mu grows.
pub fn turned(faces: &[Face], h: f64) -> Result<Vec<Face>, LowerError> {
    let r = |v: V3| [v[0], -v[1], -v[2]];
    let n = |v: V3| [-v[0], v[1], v[2]];
    let t = |o: V3| [o[0], -o[1], h - o[2]];
    let neg = |i: Ival| Ival::new(-i.hi, -i.lo);
    let mut out = Vec::with_capacity(faces.len());
    for (k, f) in faces.iter().enumerate() {
        let surf = match f.surf {
            Surf::Plane { o, u, v } => Surf::Plane {
                o: t(o),
                u: n(u),
                v: n(v),
            },
            Surf::Cyl { o, x, y, ax, r: rr } => Surf::Cyl {
                o: t(o),
                x: r(x),
                y: n(y),
                ax: n(ax),
                r: rr,
            },
            Surf::Cone { o, x, y, ax, r0, k } => Surf::Cone {
                o: t(o),
                x: r(x),
                y: n(y),
                ax: n(ax),
                r0,
                k: -k,
            },
            Surf::Torus {
                o,
                x,
                y,
                ax,
                big,
                r: rr,
                cm,
                sm,
            } => Surf::Torus {
                o: t(o),
                x: r(x),
                y: n(y),
                ax: n(ax),
                big,
                r: rr,
                cm,
                sm: -sm,
            },
            Surf::Sphere { .. } => {
                return Err(LowerError::Unsupported {
                    face: k,
                    why: "a sphere is not turned over in this phase".into(),
                });
            }
        };
        let trim = f.trim.as_ref().map(|tr| Trim {
            segs: tr
                .segs
                .iter()
                .map(|s| match *s {
                    TSeg::Line { a, b } => TSeg::Line {
                        a: [-a[0], -a[1]],
                        b: [-b[0], -b[1]],
                    },
                    TSeg::Arc { a, b, c, r, ccw } => TSeg::Arc {
                        a: [-a[0], -a[1]],
                        b: [-b[0], -b[1]],
                        c: [-c[0], -c[1]],
                        r,
                        ccw,
                    },
                })
                .collect(),
        });
        let mu = f.mu + 4.0 * f64::EPSILON * (h.abs() + f.bbox[2].lo.abs().max(f.bbox[2].hi.abs()));
        out.push(Face::new(surf, neg(f.p), neg(f.q), trim, mu, f.id));
    }
    Ok(out)
}

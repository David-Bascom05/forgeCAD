//! The part lowered for 3-axis surfacing: every face `lower::lower` takes, as it takes it; a
//! face it refuses (a sphere, a revolved face whose edges are not circles about its axis and
//! generators) as a *superset*: the whole sphere or torus, or a cylinder or cone between the
//! lowest and highest heights along its axis of the face's edges' boxes (height along the axis
//! is linear on each generator, so its extremes over the face are on its boundary).
//!
//! A superset is sound for every check (a clearance from more than the face is a clearance from
//! the face) and for the drop-cutter's upper bound; it is stricter: a concave face lowered whole
//! blocks the tool where the true face does not, and the op is then refused, never wrong. A
//! `Hit` found on the superset's extra part is not a real violation, so a refusal on such a part
//! names the face as lowered whole. Procedural faces are refused by kind.
use crate::lower::{LowerError, edges, lower_face, mu};
use forge_cam_geom::check::{Face, Surf, V3};
use forge_cam_geom::ival::Ival;
use forge_geom::curve::CurveGeom;
use forge_geom::surface::Surface;
use forge_math::dmath;
use forge_topo::Body;

/// Each face lowered, and the indices of the faces lowered as supersets.
pub fn lower_surfacing(body: &Body) -> Result<(Vec<Face>, Vec<usize>), LowerError> {
    let mut out = Vec::new();
    let mut whole = Vec::new();
    for (k, fid) in body.face_ids().enumerate() {
        let mut one = Vec::new();
        match lower_face(body, fid, k, &mut one) {
            Ok(()) => out.extend(one),
            Err(e) => {
                if superset(body, fid, k, &mut out)? {
                    whole.push(k);
                } else {
                    return Err(e);
                }
            }
        }
    }
    Ok((out, whole))
}

fn v3(v: forge_math::Vec3) -> V3 {
    [v.x, v.y, v.z]
}

/// The four quarter turns' frames (x at each quarter's middle) and the half-angle bound.
fn quarters(x: V3, y: V3) -> ([(V3, V3); 4], f64) {
    let mut fr = [([0.0; 3], [0.0; 3]); 4];
    for (i, f) in fr.iter_mut().enumerate() {
        let um = std::f64::consts::FRAC_PI_4 * (2 * i + 1) as f64;
        let (s, c) = dmath::sin_cos(um);
        *f = (
            [c * x[0] + s * y[0], c * x[1] + s * y[1], c * x[2] + s * y[2]],
            [-s * x[0] + c * y[0], -s * x[1] + c * y[1], -s * x[2] + c * y[2]],
        );
    }
    // the map's t is tan(angle / 2), the angle up to π/4 either side of the middle: tan(π/8)
    let th =
        dmath::sin(std::f64::consts::FRAC_PI_8) / dmath::cos(std::f64::consts::FRAC_PI_8) * (1.0 + 1e-12);
    (fr, th)
}

fn superset(body: &Body, fid: forge_topo::FaceId, k: usize, out: &mut Vec<Face>) -> Result<bool, LowerError> {
    let f = body.face(fid);
    let bb = body.face_box(fid);
    let scale = [bb.x.lo, bb.y.lo, bb.z.lo, bb.x.hi, bb.y.hi, bb.z.hi]
        .iter()
        .fold(0.0f64, |m, x| m.max(x.abs()));
    let m = mu(scale);
    let fr = match &f.surface {
        Surface::Sphere(s) => s.frame,
        Surface::Torus(t) => t.frame,
        Surface::Cylinder(c) => c.frame,
        Surface::Cone(c) => c.frame,
        _ => return Ok(false),
    };
    let o = v3(fr.origin.to_vec());
    let (fx, fy, fz) = (v3(fr.x().get()), v3(fr.y().get()), v3(fr.z().get()));
    let (qs, th) = quarters(fx, fy);
    let p = Ival::new(-th, th);
    match &f.surface {
        Surface::Sphere(s) => {
            // elevation over the whole sphere: q = tan(el / 2) ∈ [−1, 1]
            for (xm, ym) in qs {
                out.push(Face::new(
                    Surf::Sphere {
                        o,
                        x: xm,
                        y: ym,
                        ax: fz,
                        r: s.radius,
                    },
                    p,
                    Ival::new(-1.0 - 1e-12, 1.0 + 1e-12),
                    None,
                    m,
                    k as u32,
                ));
            }
        }
        Surface::Torus(t) => {
            // the tube in four quarters about its middles 0, π/2, π, 3π/2
            for (xm, ym) in qs {
                for j in 0..4 {
                    let (sm, cm) = dmath::sin_cos(std::f64::consts::FRAC_PI_2 * j as f64);
                    out.push(Face::new(
                        Surf::Torus {
                            o,
                            x: xm,
                            y: ym,
                            ax: fz,
                            big: t.major,
                            r: t.minor,
                            cm,
                            sm,
                        },
                        p,
                        Ival::new(-th, th),
                        None,
                        m,
                        k as u32,
                    ));
                }
            }
        }
        Surface::Cylinder(_) | Surface::Cone(_) => {
            // heights along the axis: the extremes of the edges' boxes' corners
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for (cv, range, _, _) in edges(body, fid) {
                let b = cv.bbox(range);
                for cx in [b.x.lo, b.x.hi] {
                    for cy in [b.y.lo, b.y.hi] {
                        for cz in [b.z.lo, b.z.hi] {
                            let h = (cx - o[0]) * fz[0] + (cy - o[1]) * fz[1] + (cz - o[2]) * fz[2];
                            lo = lo.min(h);
                            hi = hi.max(h);
                        }
                    }
                }
            }
            if lo.is_nan() || hi.is_nan() || lo > hi {
                return Ok(false);
            }
            let pad = m + 1e-9 * (1.0 + scale);
            let q = Ival::new(lo - pad, hi + pad);
            for (xm, ym) in qs {
                let surf = match &f.surface {
                    Surface::Cylinder(c) => Surf::Cyl {
                        o,
                        x: xm,
                        y: ym,
                        ax: fz,
                        r: c.radius,
                    },
                    Surface::Cone(c) => Surf::Cone {
                        o,
                        x: xm,
                        y: ym,
                        ax: fz,
                        r0: c.radius,
                        k: c.tan_a(),
                    },
                    _ => unreachable!(),
                };
                out.push(Face::new(surf, p, q, None, m, k as u32));
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

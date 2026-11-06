//! Entities to curves and back: reading a curve out of an entity's data in its plane, writing an
//! edited curve back as the entity type that holds it naturally (a trimmed circle as an ARC, a
//! trimmed ellipse as an elliptical ELLIPSE, an offset ellipse as a fitted SPLINE), and moving,
//! turning, scaling and mirroring entity data (curves, points, TEXT, MTEXT, INSERT).

use super::curve::{Arc, Curve, Ellipse, Offset, Poly, Seg};
use super::geom::{Affine, PI, TAU, Tol, V2, ocs_axes, v2, wrap, wrap_pi};
use super::nurbs::{Nurbs, Param};
use crate::file::Drawing;
use crate::types::{self, Data, Polyline2d, Vertex2d};

/// A curve read from an entity: in WCS XY, at height `z`.
#[derive(Clone, Debug, PartialEq)]
pub struct Flat {
    pub curve: Curve,
    pub z: f64,
}

/// Why an entity can't be edited as a 2D curve.
#[derive(Clone, Debug, PartialEq)]
pub enum NotEditable {
    /// Not a curve (text, a block reference, a hatch, ..).
    NotACurve,
    /// A curve, but not in a plane parallel to XY (its normal, or a 3D curve).
    NotInXy([f64; 3]),
    /// Data that doesn't describe a curve (no points, bad knots, ..).
    Degenerate,
}

/// The map from an entity's OCS XY to WCS XY, when its normal is +-Z: identity for +Z, a
/// mirror in x for -Z (the arbitrary-axis algorithm's OCS). With the factor turning an OCS
/// elevation into a WCS z.
pub fn ocs_map(normal: [f64; 3]) -> Result<(Affine, f64), NotEditable> {
    let l = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if l == 0.0 {
        return Ok((Affine::IDENTITY, 1.0));
    }
    let n = [normal[0] / l, normal[1] / l, normal[2] / l];
    if n[0].abs() > 1e-9 || n[1].abs() > 1e-9 {
        return Err(NotEditable::NotInXy(normal));
    }
    if n[2] > 0.0 {
        return Ok((Affine::IDENTITY, 1.0));
    }
    let (ax, ay, _) = ocs_axes(n);
    Ok((
        Affine {
            m: [[ax[0], ay[0]], [ax[1], ay[1]]],
            t: V2::ZERO,
        },
        n[2],
    ))
}

fn flat_z(zs: impl IntoIterator<Item = f64>, scale: f64) -> Result<f64, NotEditable> {
    let zs: Vec<f64> = zs.into_iter().collect();
    let z0 = zs.first().copied().unwrap_or(0.0);
    let tol = Tol::for_size(scale, scale.max(z0.abs())).lin;
    if zs.iter().all(|z| (z - z0).abs() <= tol) {
        Ok(z0)
    } else {
        Err(NotEditable::NotInXy([0.0, 0.0, 0.0]))
    }
}

/// The curve an entity's data holds, in WCS XY. A 2D POLYLINE needs its vertices: see
/// `read_polyline2d` (or `curve_of`).
pub fn read_entity(d: &Data) -> Result<Flat, NotEditable> {
    match d {
        Data::Line(l) => {
            let (a, b) = (V2::from(l.start), V2::from(l.end));
            let z = flat_z([l.start[2], l.end[2]], a.dist(b))?;
            Ok(Flat {
                curve: Curve::Line { a, b },
                z,
            })
        }
        Data::Ray(r) | Data::Xline(types::Xline(r)) => {
            let dir = V2::from(r.direction);
            let l = (dir.len2() + r.direction[2] * r.direction[2]).sqrt();
            if l == 0.0 {
                return Err(NotEditable::Degenerate);
            }
            if r.direction[2].abs() > 1e-9 * l {
                return Err(NotEditable::NotInXy(r.direction));
            }
            let at = V2::from(r.at);
            let dir = dir.unit().ok_or(NotEditable::Degenerate)?;
            let curve = if matches!(d, Data::Ray(_)) {
                Curve::Ray { at, dir }
            } else {
                Curve::Xline { at, dir }
            };
            Ok(Flat { curve, z: r.at[2] })
        }
        Data::Circle(c) => {
            let (m, zf) = ocs_map(c.extrusion)?;
            if c.radius <= 0.0 {
                return Err(NotEditable::Degenerate);
            }
            let curve = Curve::Circle {
                c: V2::from(c.center),
                r: c.radius,
            }
            .transformed(&m)
            .ok_or(NotEditable::Degenerate)?;
            Ok(Flat {
                curve,
                z: c.center[2] * zf,
            })
        }
        Data::Arc(a) => {
            let c = &a.circle;
            let (m, zf) = ocs_map(c.extrusion)?;
            if c.radius <= 0.0 {
                return Err(NotEditable::Degenerate);
            }
            let curve = Curve::arc(V2::from(c.center), c.radius, a.start, a.end)
                .transformed(&m)
                .ok_or(NotEditable::Degenerate)?;
            Ok(Flat {
                curve,
                z: c.center[2] * zf,
            })
        }
        Data::Ellipse(e) => {
            // centre and major axis in WCS; the minor axis is extrusion x major
            let n = e.extrusion;
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if l == 0.0 || n[0].abs() > 1e-9 * l || n[1].abs() > 1e-9 * l {
                return Err(NotEditable::NotInXy(n));
            }
            let major = V2::from(e.major);
            if major.len() == 0.0 || e.major[2].abs() > 1e-9 * major.len() {
                return Err(NotEditable::NotInXy(n));
            }
            let up = n[2] > 0.0;
            let minor = if up {
                major.perp() * e.ratio
            } else {
                -major.perp() * e.ratio
            };
            let mut sweep = e.end - e.start;
            if (sweep.abs() - TAU).abs() <= 1e-12 || sweep == 0.0 {
                sweep = TAU;
            } else {
                sweep = wrap(sweep);
                if sweep == 0.0 {
                    sweep = TAU;
                }
            }
            let el = Ellipse::from_conjugate(V2::from(e.center), major, minor, e.start, sweep)
                .ok_or(NotEditable::Degenerate)?;
            Ok(Flat {
                curve: Curve::Ellipse(el),
                z: e.center[2],
            })
        }
        Data::LwPolyline(p) => {
            let (m, zf) = ocs_map(p.normal)?;
            if p.points.is_empty() {
                return Err(NotEditable::Degenerate);
            }
            let pl = Poly {
                pts: p.points.iter().map(|&q| V2::from(q)).collect(),
                bulges: if p.bulges.len() == p.points.len() {
                    p.bulges.clone()
                } else {
                    vec![0.0; p.points.len()]
                },
                widths: if p.widths.len() == p.points.len() {
                    p.widths.clone()
                } else {
                    vec![]
                },
                const_width: p.const_width,
                closed: p.flag & 512 != 0,
            };
            let curve = Curve::Poly(pl).transformed(&m).ok_or(NotEditable::Degenerate)?;
            Ok(Flat {
                curve,
                z: p.elevation * zf,
            })
        }
        Data::Spline(s) => read_spline(s),
        Data::Polyline2d(_) => Err(NotEditable::Degenerate),
        _ => Err(NotEditable::NotACurve),
    }
}

/// The curve, when the entity is editable as one.
pub fn from_entity(d: &Data) -> Option<Curve> {
    read_entity(d).ok().map(|f| f.curve)
}

fn read_spline(s: &types::Spline) -> Result<Flat, NotEditable> {
    let size = s
        .points
        .iter()
        .chain(&s.fit)
        .map(|p| V2::from(*p).abs_max())
        .fold(0.0, f64::max);
    let z = flat_z(s.points.iter().chain(&s.fit).map(|p| p[2]), size)?;
    let p = s.degree.max(1) as usize;
    if !s.points.is_empty() && s.knots.len() >= s.points.len() + p + 1 {
        let mut ctrl: Vec<V2> = s.points.iter().map(|&q| V2::from(q)).collect();
        let mut knots = s.knots.clone();
        let mut weights = (s.weighted && s.weights.len() == ctrl.len()).then(|| s.weights.clone());
        knots.truncate(ctrl.len() + p + 1);
        // a periodic spline stored without its wrapped control points: wrap them
        if s.periodic && ctrl.len() > p && ctrl[0].dist(ctrl[ctrl.len() - p]) > 1e-12 * size.max(1.0) {
            let k0 = knots.clone();
            for i in 0..p {
                ctrl.push(ctrl[i]);
                if let Some(w) = &mut weights {
                    w.push(w[i]);
                }
                // the knot spacing repeats
                let next = knots[knots.len() - 1] + (k0[i + 1] - k0[i]);
                knots.push(next);
            }
        }
        let n = Nurbs {
            degree: p,
            knots,
            ctrl,
            weights,
            closed: false,
        };
        if !n.is_valid() {
            return Err(NotEditable::Degenerate);
        }
        let (a, b) = n.domain();
        let closed = (s.closed || s.periodic) && n.point(a).dist(n.point(b)) <= 1e-9 * size.max(1.0);
        return Ok(Flat {
            curve: Curve::Spline(Nurbs { closed, ..n }),
            z,
        });
    }
    if s.fit.len() >= 2 {
        let pts: Vec<V2> = s.fit.iter().map(|&q| V2::from(q)).collect();
        let tan = |t: [f64; 3]| {
            let v = V2::from(t);
            (v.len() > 0.0).then_some(v)
        };
        let param = match s.knot_param {
            1 => Param::SqrtChord,
            2 => Param::Uniform,
            _ => Param::Chord,
        };
        let n = Nurbs::interpolate(&pts, p.max(1), tan(s.begin_tangent), tan(s.end_tangent), param)
            .ok_or(NotEditable::Degenerate)?;
        return Ok(Flat {
            curve: Curve::Spline(n),
            z,
        });
    }
    Err(NotEditable::Degenerate)
}

/// A 2D POLYLINE with its vertices (in order): its curve. Spline-fit polylines read through their
/// fitted vertices (frame control points skipped).
pub fn read_polyline2d(p: &Polyline2d, verts: &[Vertex2d]) -> Result<Flat, NotEditable> {
    let (m, zf) = ocs_map(p.extrusion)?;
    let vs: Vec<&Vertex2d> = verts.iter().filter(|v| v.flags & 16 == 0).collect();
    if vs.is_empty() {
        return Err(NotEditable::Degenerate);
    }
    let widths: Vec<[f64; 2]> = vs.iter().map(|v| [v.start_width, v.end_width]).collect();
    let pl = Poly {
        pts: vs.iter().map(|v| V2::from(v.at)).collect(),
        bulges: vs.iter().map(|v| v.bulge).collect(),
        widths: if widths.iter().any(|w| w[0] != 0.0 || w[1] != 0.0) {
            widths
        } else {
            vec![]
        },
        const_width: if p.start_width == p.end_width {
            p.start_width
        } else {
            0.0
        },
        closed: p.flags & 1 != 0,
    };
    let curve = Curve::Poly(pl).transformed(&m).ok_or(NotEditable::Degenerate)?;
    Ok(Flat {
        curve,
        z: p.elevation * zf,
    })
}

/// The curve of an entity in a drawing (a 2D POLYLINE through its vertices).
pub fn curve_of(d: &Drawing, h: u32) -> Result<Flat, NotEditable> {
    let o = d.objects.get(&h).ok_or(NotEditable::Degenerate)?;
    if let Data::Polyline2d(p) = &o.data {
        let verts: Vec<Vertex2d> = d
            .owned_by(h)
            .iter()
            .filter_map(|v| match d.objects.get(v).map(|o| &o.data) {
                Some(Data::Vertex2d(x)) => Some(x.clone()),
                _ => None,
            })
            .collect();
        return read_polyline2d(p, &verts);
    }
    read_entity(&o.data)
}

/// The tolerance a fitted SPLINE (for an offset curve) keeps to: a part in 1e7 of its size.
pub fn fit_tol(c: &Curve) -> f64 {
    let b = c.bbox();
    (b.size() * 1e-7).max(Tol::of_box(&b).lin)
}

/// Fits an offset curve with a C1 cubic spline (Hermite pieces on the exact points and
/// derivatives, split until within `tol`), on the base's parameter.
pub fn fit_offset(o: &Offset, tol: f64) -> Option<Nurbs> {
    let c = Curve::Offset(o.clone());
    let ts = o.samples();
    let mut spans: Vec<(f64, f64)> = vec![];
    fn rec(c: &Curve, t0: f64, t1: f64, tol: f64, depth: u32, out: &mut Vec<(f64, f64)>) {
        let h = t1 - t0;
        let (p0, p3) = (c.point(t0), c.point(t1));
        let (d0, d1) = (c.deriv(t0) * (h / 3.0), c.deriv(t1) * (h / 3.0));
        let (p1, p2) = (p0 + d0, p3 - d1);
        let bez = |s: f64| {
            let r = 1.0 - s;
            p0 * (r * r * r) + p1 * (3.0 * r * r * s) + p2 * (3.0 * r * s * s) + p3 * (s * s * s)
        };
        let err = [0.2, 0.4, 0.5, 0.6, 0.8]
            .iter()
            .map(|&s| c.dist(bez(s)).min(bez(s).dist(c.point(t0 + h * s))))
            .fold(0.0, f64::max);
        if err <= tol || depth == 0 {
            out.push((t0, t1));
        } else {
            let tm = 0.5 * (t0 + t1);
            rec(c, t0, tm, tol, depth - 1, out);
            rec(c, tm, t1, tol, depth - 1, out);
        }
    }
    for w in ts.windows(2) {
        if w[1] > w[0] {
            rec(&c, w[0], w[1], tol, 16, &mut spans);
        }
    }
    if spans.is_empty() {
        return None;
    }
    let mut knots = vec![spans[0].0; 4];
    let mut ctrl = vec![c.point(spans[0].0)];
    for (k, &(t0, t1)) in spans.iter().enumerate() {
        let h = t1 - t0;
        let (p0, p3) = (c.point(t0), c.point(t1));
        ctrl.push(p0 + c.deriv(t0) * (h / 3.0));
        ctrl.push(p3 - c.deriv(t1) * (h / 3.0));
        ctrl.push(p3);
        if k + 1 < spans.len() {
            knots.extend([t1; 3]);
        } else {
            knots.extend([t1; 4]);
        }
    }
    let n = Nurbs {
        degree: 3,
        knots,
        ctrl,
        weights: None,
        closed: c.is_closed(),
    };
    n.is_valid().then_some(n)
}

fn spline_data(n: &Nurbs, z: f64) -> Data {
    let weighted = n.is_rational();
    Data::Spline(types::Spline {
        scenario: 1,
        flags1: 0,
        knot_param: 15,
        degree: n.degree as i32,
        rational: weighted,
        closed: n.closed,
        periodic: false,
        knot_tol: 1e-10,
        ctrl_tol: 1e-10,
        weighted,
        knots: n.knots.clone(),
        points: n.ctrl.iter().map(|p| p.arr3(z)).collect(),
        weights: if weighted {
            (0..n.n()).map(|i| n.w(i)).collect()
        } else {
            vec![]
        },
        ..types::Spline::default()
    })
}

/// New entity data for a curve at height `z`, of the type that holds it: LINE, RAY, XLINE,
/// CIRCLE, ARC (a whole one is a CIRCLE), ELLIPSE (whole or an arc), LWPOLYLINE, SPLINE (an
/// offset curve fitted within `fit_tol`).
pub fn to_data(c: &Curve, z: f64) -> Option<Data> {
    let up = [0.0, 0.0, 1.0];
    Some(match c {
        Curve::Line { a, b } => Data::Line(types::Line {
            start: a.arr3(z),
            end: b.arr3(z),
            ..types::Line::default()
        }),
        Curve::Ray { at, dir } => Data::Ray(types::Ray {
            at: at.arr3(z),
            direction: dir.arr3(0.0),
        }),
        Curve::Xline { at, dir } => Data::Xline(types::Xline(types::Ray {
            at: at.arr3(z),
            direction: dir.arr3(0.0),
        })),
        Curve::Circle { c, r } => Data::Circle(types::Circle {
            center: c.arr3(z),
            radius: *r,
            ..types::Circle::default()
        }),
        Curve::Arc(a) => {
            let circle = types::Circle {
                center: a.c.arr3(z),
                radius: a.r,
                ..types::Circle::default()
            };
            if a.len() >= TAU * (1.0 - 1e-14) {
                return Some(Data::Circle(circle));
            }
            let a = a.ccw();
            Data::Arc(types::Arc {
                circle,
                start: wrap(a.start),
                end: wrap(a.start + a.sweep),
            })
        }
        Curve::Ellipse(e) => {
            let e = if e.sweep < 0.0 { e.reversed() } else { *e };
            let (start, end) = if e.is_full() {
                (0.0, TAU)
            } else {
                (wrap(e.start), wrap(e.start) + e.sweep)
            };
            let end = if end > TAU { end - TAU } else { end };
            Data::Ellipse(types::Ellipse {
                center: e.c.arr3(z),
                major: e.major.arr3(0.0),
                extrusion: up,
                ratio: e.ratio.min(1.0),
                start,
                end,
            })
        }
        Curve::Poly(p) => {
            let bulges = if p.bulges.iter().any(|&b| b != 0.0) {
                let mut b = p.bulges.clone();
                b.resize(p.pts.len(), 0.0);
                b
            } else {
                vec![]
            };
            Data::LwPolyline(types::LwPolyline {
                flag: if p.closed { 512 } else { 0 },
                const_width: p.const_width,
                elevation: z,
                normal: up,
                points: p.pts.iter().map(|q| q.arr()).collect(),
                bulges,
                widths: if p.widths.len() == p.pts.len() {
                    p.widths.clone()
                } else {
                    vec![]
                },
                ..types::LwPolyline::default()
            })
        }
        Curve::Spline(n) => spline_data(n, z),
        Curve::Offset(o) => spline_data(&fit_offset(o, fit_tol(c))?, z),
    })
}

/// The data for a curve replacing an entity's, keeping what the curve doesn't say: thickness,
/// the plane's normal (an OCS entity on -Z stays on -Z), the height, a LINE's extrusion.
pub fn write_curve(c: &Curve, like: &Data) -> Option<Data> {
    let (normal, thickness, z) = match like {
        Data::Line(l) => (l.extrusion, l.thickness, l.start[2]),
        Data::Circle(x) => (x.extrusion, x.thickness, 0.0),
        Data::Arc(x) => (x.circle.extrusion, x.circle.thickness, 0.0),
        Data::LwPolyline(x) => (x.normal, x.thickness, 0.0),
        Data::Polyline2d(x) => (x.extrusion, x.thickness, 0.0),
        _ => ([0.0, 0.0, 1.0], 0.0, 0.0),
    };
    let z = read_entity(like).map(|f| f.z).unwrap_or(z);
    let (m, zf) = ocs_map(normal).unwrap_or((Affine::IDENTITY, 1.0));
    let ocs = m != Affine::IDENTITY;
    // OCS types are written in the OCS; the others in WCS
    let ocs_curve = if ocs {
        c.transformed(&m.inverse()?)?
    } else {
        c.clone()
    };
    let natural = to_data(c, z)?;
    let out = match natural {
        Data::Circle(_) | Data::Arc(_) | Data::LwPolyline(_) => {
            let mut d = to_data(&ocs_curve, z * zf)?;
            match &mut d {
                Data::Circle(x) => {
                    x.extrusion = normal;
                    x.thickness = thickness;
                }
                Data::Arc(x) => {
                    x.circle.extrusion = normal;
                    x.circle.thickness = thickness;
                }
                Data::LwPolyline(x) => {
                    x.normal = normal;
                    x.thickness = thickness;
                    if let Data::LwPolyline(l) = like {
                        x.flag |= l.flag & 256;
                    }
                }
                _ => {}
            }
            d
        }
        Data::Line(mut l) => {
            l.thickness = thickness;
            if let Data::Line(o) = like {
                l.extrusion = o.extrusion;
            }
            Data::Line(l)
        }
        d => d,
    };
    Some(out)
}

/// Writes a curve into an entity's data when the entity's type holds it (a line into a LINE, an
/// arc into an ARC, ..). False when it doesn't (a trimmed circle is an ARC now): replace the
/// entity with `write_curve`'s data instead.
pub fn apply_to(c: &Curve, data: &mut Data) -> bool {
    let Some(new) = write_curve(c, data) else {
        return false;
    };
    if std::mem::discriminant(&new) != std::mem::discriminant(data) {
        return false;
    }
    *data = new;
    true
}

/// Text after a map: (insertion, rotation, height scale, mirrored glyphs?) for MIRRTEXT.
fn text_frame(m: &Affine, rotation: f64, mirrtext: bool) -> Option<(f64, f64, bool)> {
    let k = m.similarity_scale(1e-9)?;
    let md = m.vec(V2::polar(rotation, 1.0));
    if !m.is_reflection() {
        return Some((md.angle(), k, false));
    }
    let r1 = md.angle();
    let r2 = (-md).angle();
    if mirrtext {
        // reflected: turned to the reversed direction and drawn backwards
        return Some((r2, k, true));
    }
    // readable: keep the reading direction nearest the original
    let pick = if wrap_pi(r1 - rotation).abs() <= wrap_pi(r2 - rotation).abs() {
        r1
    } else {
        r2
    };
    Some((pick, k, false))
}

fn map_text(t: &mut types::Text, m: &Affine, mirrtext: bool) -> Option<()> {
    let (om, _) = ocs_map(t.extrusion).ok()?;
    let inv = om.inverse()?;
    // the map in the text's OCS
    let mo = om.then(m).then(&inv);
    let (rot, k, backward) = text_frame(&mo, t.rotation, mirrtext)?;
    let reversed = wrap_pi(rot - mo.vec(V2::polar(t.rotation, 1.0)).angle()).abs() > PI / 2.0;
    let ins = mo.apply(V2::from(t.insert));
    let al = t.align_pt.map(|a| mo.apply(V2::from(a)));
    // aligned and fit text run from the insertion to the alignment point
    if reversed && matches!(t.halign, 3 | 5) && !backward {
        if let Some(a) = al {
            t.insert = a.arr();
            t.align_pt = Some(ins.arr());
        }
    } else {
        t.insert = ins.arr();
        t.align_pt = al.map(|a| a.arr());
    }
    t.rotation = wrap(rot);
    t.height *= k;
    if mo.is_reflection() {
        t.oblique = -t.oblique;
        if backward {
            t.generation ^= 2;
        }
    }
    Some(())
}

fn map_mtext_axes(m: &Affine, x_axis: [f64; 3]) -> V2 {
    let x = V2::from(x_axis).unit().unwrap_or(v2(1.0, 0.0));
    let mx = m.vec(x).unit_or_zero();
    if m.is_reflection() {
        // MTEXT stays readable: the direction of the two nearer the original
        if mx.dot(x) >= (-mx).dot(x) { mx } else { -mx }
    } else {
        mx
    }
}

/// Entity data after a 2D affine map (in WCS XY): curves (an arc stays an arc under a
/// similarity, its direction kept counter-clockwise; an ellipse or spline mirrored stays exact),
/// points, solids, TEXT and attributes (MIRRTEXT 0 keeps them readable, 1 mirrors them), MTEXT
/// (always readable), INSERT (rotation and scale; a mirror is a negative X scale), VERTEX (2D,
/// under a +Z polyline). None for what the map can't keep (text under a non-uniform scale, a
/// circle's entity type under one: use `Curve::transformed` and `write_curve`).
pub fn transform_data(d: &Data, m: &Affine, mirrtext: bool) -> Option<Data> {
    let sim = m.similarity_scale(1e-9);
    let mut out = d.clone();
    match &mut out {
        Data::Line(_) | Data::Ray(_) | Data::Xline(_) | Data::LwPolyline(_) => {
            let f = read_entity(d).ok()?;
            let c = f.curve.transformed(m)?;
            return write_curve(&c, d);
        }
        Data::Circle(_) | Data::Arc(_) => {
            sim?;
            let f = read_entity(d).ok()?;
            return write_curve(&f.curve.transformed(m)?, d);
        }
        Data::Ellipse(e) => {
            let f = read_entity(d).ok()?;
            let Curve::Ellipse(el) = f.curve.transformed(m)? else {
                return None;
            };
            let Data::Ellipse(x) = to_data(&Curve::Ellipse(el), e.center[2])? else {
                return None;
            };
            *e = x;
        }
        Data::Spline(s) => {
            let mp = |p: &mut [f64; 3]| {
                let q = m.apply(V2::from(*p));
                p[0] = q.x;
                p[1] = q.y;
            };
            let mv = |p: &mut [f64; 3]| {
                let q = m.vec(V2::from(*p));
                p[0] = q.x;
                p[1] = q.y;
            };
            s.points.iter_mut().for_each(mp);
            s.fit.iter_mut().for_each(mp);
            mv(&mut s.begin_tangent);
            mv(&mut s.end_tangent);
        }
        Data::Point(p) => {
            let q = m.apply(V2::from(p.at));
            p.at = q.arr3(p.at[2]);
        }
        Data::Solid(s) => {
            for c in &mut s.corners {
                *c = m.apply(V2::from(*c)).arr();
            }
        }
        Data::Trace(types::Trace(s)) => {
            for c in &mut s.corners {
                *c = m.apply(V2::from(*c)).arr();
            }
        }
        Data::Text(t) => map_text(t, m, mirrtext)?,
        Data::Attrib(a) => map_text(&mut a.text, m, mirrtext)?,
        Data::AttDef(a) => map_text(&mut a.attrib.text, m, mirrtext)?,
        Data::MText(t) => {
            let k = sim?;
            let x = map_mtext_axes(m, t.x_axis);
            let ins = m.apply(V2::from(t.insert));
            t.insert = ins.arr3(t.insert[2]);
            t.x_axis = x.arr3(0.0);
            t.rect_width *= k;
            t.rect_height *= k;
            t.text_height *= k;
            t.ext_width *= k;
            t.ext_height *= k;
            if let Some(c) = &mut t.context {
                c.insert = m.apply(V2::from(c.insert)).arr3(c.insert[2]);
                c.x_axis = map_mtext_axes(m, c.x_axis).arr3(0.0);
                c.rect_width *= k;
                c.rect_height *= k;
                c.ext_width *= k;
                c.ext_height *= k;
                c.column_width *= k;
                c.gutter *= k;
                c.column_heights.iter_mut().for_each(|h| *h *= k);
            }
        }
        Data::Insert(i) => map_insert(i, m)?,
        Data::MInsert(mi) => {
            let k = sim?;
            map_insert(&mut mi.insert, m)?;
            mi.col_spacing *= k;
            mi.row_spacing *= k;
        }
        Data::Vertex2d(v) => {
            let q = m.apply(V2::from(v.at));
            v.at = q.arr3(v.at[2]);
            if let Some(k) = sim {
                v.start_width *= k;
                v.end_width *= k;
                if m.is_reflection() {
                    v.bulge = -v.bulge;
                }
            } else if v.bulge != 0.0 {
                return None;
            }
        }
        Data::Polyline2d(p) => {
            if let Some(k) = sim {
                p.start_width *= k;
                p.end_width *= k;
            }
        }
        _ => return None,
    }
    Some(out)
}

fn map_insert(i: &mut types::Insert, m: &Affine) -> Option<()> {
    let (om, _) = ocs_map(i.extrusion).ok()?;
    let mo = om.then(m).then(&om.inverse()?);
    let (sx, sy) = (i.scale[0], i.scale[1]);
    let r = i.rotation;
    let c1 = mo.vec(V2::polar(r, sx));
    let c2 = mo.vec(V2::polar(r, sy).perp());
    let (l1, l2) = (c1.len(), c2.len());
    if l1 == 0.0 || l2 == 0.0 || c1.dot(c2).abs() > 1e-9 * l1 * l2 {
        return None;
    }
    // keep the Y scale's sign; the rotation from the Y column
    let sy2 = if sy < 0.0 { -l2 } else { l2 };
    let rot = (c2 / sy2).angle() - PI / 2.0;
    let sx2 = c1.dot(V2::polar(rot, 1.0));
    i.at = mo.apply(V2::from(i.at)).arr3(i.at[2]);
    i.rotation = wrap(rot);
    let k = (l1 / sx.abs() + l2 / sy.abs()) * 0.5;
    i.scale = [
        sx2,
        sy2,
        i.scale[2]
            * if mo.similarity_scale(1e-9).is_some() {
                k
            } else {
                1.0
            },
    ];
    Some(())
}

/// A curve's segments as separate lines and arcs (EXPLODE of a polyline), others as they are.
pub fn explode(c: &Curve) -> Vec<Curve> {
    match c {
        Curve::Poly(p) => p.segs().iter().map(Seg::curve).collect(),
        _ => vec![c.clone()],
    }
}

/// The arc as an ARC entity's (start, end) angles, counter-clockwise.
pub fn arc_angles(a: &Arc) -> (f64, f64) {
    let a = a.ccw();
    (wrap(a.start), wrap(a.start + a.sweep))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::str::Str;

    #[test]
    fn reads_and_writes_each_kind() {
        let arc = Data::Arc(types::Arc {
            circle: types::Circle {
                center: [1.0, 2.0, 3.0],
                radius: 4.0,
                ..types::Circle::default()
            },
            start: 0.5,
            end: 2.0,
        });
        let f = read_entity(&arc).unwrap();
        assert_eq!(f.z, 3.0);
        let Curve::Arc(a) = &f.curve else { panic!() };
        assert!((a.sweep - 1.5).abs() < 1e-15);
        let back = write_curve(&f.curve, &arc).unwrap();
        assert_eq!(back, arc);
        // trimmed, a circle becomes an arc
        let circle = Data::Circle(types::Circle {
            radius: 2.0,
            ..types::Circle::default()
        });
        let c = from_entity(&circle).unwrap();
        let piece = c.sub(0.0, 1.0).unwrap();
        let mut d = circle.clone();
        assert!(!apply_to(&piece, &mut d));
        assert!(matches!(write_curve(&piece, &circle), Some(Data::Arc(_))));
        // an ellipse arc round trip
        let el = Data::Ellipse(types::Ellipse {
            center: [50.0, 60.0, 0.0],
            major: [10.0, 5.0, 0.0],
            ratio: 0.5,
            start: 0.3,
            end: 2.0,
            ..types::Ellipse::default()
        });
        let c = from_entity(&el).unwrap();
        let Data::Ellipse(e2) = write_curve(&c, &el).unwrap() else {
            panic!()
        };
        let Data::Ellipse(e1) = &el else { panic!() };
        assert!((e2.start - e1.start).abs() < 1e-12 && (e2.end - e1.end).abs() < 1e-12);
        assert!(V2::from(e2.major).dist(V2::from(e1.major)) < 1e-12);
    }

    #[test]
    fn ocs_minus_z() {
        // an arc on -Z: counter-clockwise in its OCS, clockwise seen from +Z, x mirrored
        let arc = Data::Arc(types::Arc {
            circle: types::Circle {
                center: [5.0, 0.0, 2.0],
                radius: 1.0,
                extrusion: [0.0, 0.0, -1.0],
                ..types::Circle::default()
            },
            start: 0.0,
            end: PI / 2.0,
        });
        let f = read_entity(&arc).unwrap();
        assert_eq!(f.z, -2.0);
        assert!(f.curve.start().dist(v2(-6.0, 0.0)) < 1e-12);
        assert!(f.curve.end().dist(v2(-5.0, 1.0)) < 1e-12);
        let back = write_curve(&f.curve, &arc).unwrap();
        let (Data::Arc(x), Data::Arc(y)) = (&back, &arc) else {
            panic!()
        };
        assert_eq!(x.circle.extrusion, [0.0, 0.0, -1.0]);
        assert!((x.start - y.start).abs() < 1e-12 && (x.end - y.end).abs() < 1e-12);
        assert!(V2::from(x.circle.center).dist(v2(5.0, 0.0)) < 1e-12 && x.circle.center[2] == 2.0);
        // tilted: not editable
        let mut t = arc.clone();
        if let Data::Arc(a) = &mut t {
            a.circle.extrusion = [0.0, 1.0, 1.0];
        }
        assert!(matches!(read_entity(&t), Err(NotEditable::NotInXy(_))));
        let line3d = Data::Line(types::Line {
            end: [1.0, 1.0, 1.0],
            ..types::Line::default()
        });
        assert!(read_entity(&line3d).is_err());
        assert_eq!(
            read_entity(&Data::Text(types::Text::default())),
            Err(NotEditable::NotACurve)
        );
    }

    #[test]
    fn splines_read() {
        let s = Data::Spline(types::Spline {
            scenario: 1,
            degree: 2,
            rational: true,
            weighted: true,
            knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            points: vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0]],
            weights: vec![1.0, 0.7071, 1.0],
            ..types::Spline::default()
        });
        let c = from_entity(&s).unwrap();
        assert!(c.end().dist(v2(2.0, 0.0)) < 1e-12);
        let back = to_data(&c, 0.0).unwrap();
        assert_eq!(from_entity(&back).unwrap(), c);
        let fit = Data::Spline(types::Spline {
            scenario: 2,
            degree: 3,
            fit: vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0], [3.0, 1.0, 0.0]],
            ..types::Spline::default()
        });
        let c = from_entity(&fit).unwrap();
        assert!(c.dist(v2(1.0, 1.0)) < 1e-9 && c.dist(v2(2.0, 0.0)) < 1e-9);
    }

    #[test]
    fn offset_fits_a_spline() {
        let e = Ellipse {
            c: V2::ZERO,
            major: v2(10.0, 0.0),
            ratio: 0.6,
            start: 0.0,
            sweep: TAU,
        };
        let o = Curve::Offset(Offset {
            base: Box::new(Curve::Ellipse(e)),
            d: -2.0,
        });
        let Some(Data::Spline(s)) = to_data(&o, 0.0) else {
            panic!()
        };
        let c = from_entity(&Data::Spline(s)).unwrap();
        let tol = fit_tol(&o);
        for k in 0..400 {
            let p = c.point(c.domain().1 * k as f64 / 400.0);
            assert!((Curve::Ellipse(e).dist(p) - 2.0).abs() <= tol * 2.0);
        }
    }

    #[test]
    fn mirror_text() {
        let t = types::Text {
            insert: [10.0, 5.0],
            rotation: 0.0,
            height: 2.5,
            text: Str::new("ABC", false),
            ..types::Text::default()
        };
        let m = Affine::mirror(v2(0.0, 0.0), v2(0.0, 1.0));
        // MIRRTEXT 0: readable, same direction
        let Some(Data::Text(a)) = transform_data(&Data::Text(t.clone()), &m, false) else {
            panic!()
        };
        assert!(V2::from(a.insert).dist(v2(-10.0, 5.0)) < 1e-12);
        assert!(a.rotation.abs() < 1e-12 && a.generation == 0);
        // MIRRTEXT 1: drawn backwards
        let Some(Data::Text(b)) = transform_data(&Data::Text(t.clone()), &m, true) else {
            panic!()
        };
        assert!(V2::from(b.insert).dist(v2(-10.0, 5.0)) < 1e-12);
        assert!(b.rotation.abs() < 1e-12 && b.generation & 2 != 0);
        // about a horizontal line, MIRRTEXT 0 keeps it upright
        let h = Affine::mirror(v2(0.0, 0.0), v2(1.0, 0.0));
        let Some(Data::Text(c)) = transform_data(&Data::Text(t.clone()), &h, false) else {
            panic!()
        };
        assert!(c.rotation.abs() < 1e-12 && V2::from(c.insert).dist(v2(10.0, -5.0)) < 1e-12);
        // rotation and scale
        let r = Affine::rotate(V2::ZERO, PI / 2.0).then(&Affine::scale(V2::ZERO, 2.0));
        let Some(Data::Text(d)) = transform_data(&Data::Text(t), &r, false) else {
            panic!()
        };
        assert!((d.rotation - PI / 2.0).abs() < 1e-12 && (d.height - 5.0).abs() < 1e-12);
    }

    #[test]
    fn mirror_insert_and_mtext() {
        let i = types::Insert {
            at: [10.0, 0.0, 0.0],
            ..types::Insert::default()
        };
        let m = Affine::mirror(v2(0.0, 0.0), v2(0.0, 1.0));
        let Some(Data::Insert(j)) = transform_data(&Data::Insert(i.clone()), &m, false) else {
            panic!()
        };
        assert!(V2::from(j.at).dist(v2(-10.0, 0.0)) < 1e-12);
        assert!(
            (j.scale[0] + 1.0).abs() < 1e-12 && (j.scale[1] - 1.0).abs() < 1e-12 && j.rotation.abs() < 1e-12
        );
        let r = Affine::rotate(V2::ZERO, 0.5).then(&Affine::scale(V2::ZERO, 3.0));
        let Some(Data::Insert(k)) = transform_data(&Data::Insert(i), &r, false) else {
            panic!()
        };
        assert!(
            (k.rotation - 0.5).abs() < 1e-12
                && (k.scale[0] - 3.0).abs() < 1e-12
                && (k.scale[2] - 3.0).abs() < 1e-12
        );
        let t = types::MText::default();
        let Some(Data::MText(u)) = transform_data(&Data::MText(t), &m, true) else {
            panic!()
        };
        assert!(V2::from(u.x_axis).dist(v2(1.0, 0.0)) < 1e-12);
    }

    #[test]
    fn mirror_arc_and_ellipse_entities() {
        let arc = Data::Arc(types::Arc {
            circle: types::Circle {
                center: [0.0, 0.0, 0.0],
                radius: 1.0,
                ..types::Circle::default()
            },
            start: 0.0,
            end: PI / 2.0,
        });
        let m = Affine::mirror(v2(0.0, 0.0), v2(0.0, 1.0));
        let Some(Data::Arc(a)) = transform_data(&arc, &m, false) else {
            panic!()
        };
        // the mirrored quarter is from 90 to 180 degrees, counter-clockwise
        assert!((a.start - PI / 2.0).abs() < 1e-12 && (a.end - PI).abs() < 1e-12);
        let el = Data::Ellipse(types::Ellipse {
            major: [2.0, 0.0, 0.0],
            ratio: 0.5,
            start: 0.0,
            end: PI / 2.0,
            ..types::Ellipse::default()
        });
        let c0 = from_entity(&el).unwrap();
        let Some(e2) = transform_data(&el, &m, false) else {
            panic!()
        };
        let c1 = from_entity(&e2).unwrap();
        let mirrored = c0.transformed(&m).unwrap();
        for k in 0..=10 {
            let p = mirrored.point(mirrored.domain().1 * k as f64 / 10.0);
            assert!(c1.dist(p) < 1e-12);
        }
        assert!(c1.start().dist(v2(0.0, 1.0)) < 1e-12 && c1.end().dist(v2(-2.0, 0.0)) < 1e-12);
    }
}

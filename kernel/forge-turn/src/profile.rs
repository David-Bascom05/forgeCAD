//! The part's half-profile: a solid of revolution about the spindle axis (the part's z) is the set
//! of points whose (ρ, z), ρ = √(x² + y²), lies in a plane region, its half-profile. Turning works
//! on that region: every op is a path in the (ρ, z) half-plane, and every check is a 2D distance
//! there (check.rs says why a 2D distance bounds the 3D one).
//!
//! The lowering is exact in kind: each face of revolution gives its meridian, the curve its
//! points' (ρ, z) trace:
//! - a plane normal to the axis, a segment at its height;
//! - a cylinder about the axis, a segment at its radius;
//! - a cone about the axis, a sloped segment;
//! - a sphere centred on the axis, an arc about (0, centre height);
//! - a torus about the axis, an arc of its tube circle.
//!
//! A meridian's ends are read from the face's own circle edges (their radius and height), and two
//! faces meeting on a circle read the same edge: their ends are bit-equal, so the loop closes
//! exactly. Where the part's material touches the axis (a disc face, a cone's apex, a sphere's
//! pole), the profile is closed by segments along ρ = 0.
//!
//! The part is defined to `tol::LINEAR`, and an axis off the world z by at most that is taken as
//! the world z with the offset added to `mu`: every clearance is then proven by mu more.
//! Anything else (a flat, a cross hole, a keyway, a face about another axis) is not a turned
//! face: refused by kind, `NotTurned`, with the face's index.
use forge_cam_geom::region::{Loop, P2, Region, Seg, p2};
use forge_geom::curve::Curve;
use forge_geom::surface::Surface;
use forge_math::{Frame, Point3, tol};
use forge_topo::{Body, FaceId};

/// The half-profile: the region in the (ρ, z) half-plane (x = ρ ≥ 0, y = z), counter-clockwise
/// outside, and the bound `mu` on its distance from the true part's.
#[derive(Clone, Debug, PartialEq)]
pub struct HalfProfile {
    pub region: Region,
    pub mu: f64,
    /// [ρ_min, z_min, ρ_max, z_max].
    pub bbox: [f64; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProfileError {
    /// A face that is not a surface of revolution about the axis: its index, its kind and why.
    NotTurned { face: usize, kind: String, why: String },
    /// The meridians do not chain into closed loops (an end with no partner, at this point).
    Open { at: P2 },
    /// The meridians chain, but not into one valid region (crossing, wrong nesting).
    Invalid { why: String },
    Empty,
}

/// One face's meridian, with its ends in (ρ, z).
#[derive(Clone, Copy, Debug)]
struct Mer {
    seg: Seg,
}

/// How far a frame is from the world z axis (0 when it is the axis): the larger of its origin's
/// distance off the axis and its z's tilt times the size. None when not parallel within ANGULAR.
fn off_axis(f: &Frame, size: f64) -> Option<f64> {
    let z = f.z().get();
    if (z.z.abs() - 1.0).abs() > tol::ANGULAR || z.x.abs() > 1e-9 || z.y.abs() > 1e-9 {
        return None;
    }
    let o = f.origin;
    let off = (o.x * o.x + o.y * o.y).sqrt() + (z.x.abs() + z.y.abs()) * size;
    Some(off)
}

/// The (ρ, z) of a circle edge about the axis, and its distance off it.
fn circle_rz(c: &forge_geom::curve::Circle, size: f64) -> Option<(P2, f64)> {
    let off = off_axis(&c.frame, size)?;
    Some((p2(c.radius, c.frame.origin.z), off))
}

/// The ends a face's edges give: each circle edge's (ρ, z); each vertex lying on the axis (an
/// apex or a pole) its (0, z). Seam lines (in a meridian plane) give nothing of their own.
fn face_ends(body: &Body, fid: FaceId, size: f64, k: usize, kind: &str) -> Result<(Vec<P2>, f64), ProfileError> {
    let mut ends: Vec<P2> = Vec::new();
    let mut off = 0.0f64;
    let bad = |why: &str| ProfileError::NotTurned {
        face: k,
        kind: kind.to_string(),
        why: why.to_string(),
    };
    for &l in body.face(fid).loops.iter() {
        for c in body.lp(l).coedges() {
            let e = body.edge(body.coedge(c).edge);
            match &e.curve {
                Curve::Circle(ci) => {
                    let (q, o) = circle_rz(ci, size).ok_or_else(|| bad("a circle edge not about the axis"))?;
                    off = off.max(o);
                    if !ends.contains(&q) {
                        ends.push(q);
                    }
                }
                Curve::Line(_) => {
                    // a seam or a degenerate edge: it must lie in a plane through the axis (its
                    // two ends at the same angle, or one of them on the axis)
                    let (a, b) = (body.vertex(e.start).point, body.vertex(e.end).point);
                    let cr = a.x * b.y - a.y * b.x;
                    let scale = 1.0 + a.x.abs() + a.y.abs() + b.x.abs() + b.y.abs();
                    if cr.abs() > tol::LINEAR * scale {
                        return Err(bad("a straight edge off a meridian plane (a flat or a cut)"));
                    }
                }
                _ => return Err(bad("an edge that is neither a circle about the axis nor a seam")),
            }
            for v in [e.start, e.end] {
                let p: Point3 = body.vertex(v).point;
                let r = (p.x * p.x + p.y * p.y).sqrt();
                if r <= tol::LINEAR {
                    off = off.max(r);
                    let q = p2(0.0, p.z);
                    if !ends.iter().any(|x| x.x == 0.0 && (x.y - q.y).abs() <= tol::LINEAR) {
                        ends.push(q);
                    }
                }
            }
        }
    }
    Ok((ends, off))
}

fn line(a: P2, b: P2) -> Seg {
    Seg::Line { a, b }
}

/// The face's meridian. `ends` are its ends in (ρ, z) as its edges give them.
fn meridian(body: &Body, fid: FaceId, k: usize, size: f64) -> Result<(Option<Mer>, f64), ProfileError> {
    let f = body.face(fid);
    let kind = f.surface.kind().to_string();
    let bad = |why: &str| ProfileError::NotTurned {
        face: k,
        kind: kind.clone(),
        why: why.to_string(),
    };
    let (ends, eoff) = face_ends(body, fid, size, k, &kind)?;
    let two = |ends: &[P2]| -> Result<(P2, P2), ProfileError> {
        if ends.len() != 2 {
            return Err(bad(&format!("{} meridian ends, not 2", ends.len())));
        }
        Ok((ends[0], ends[1]))
    };
    let m = match &f.surface {
        Surface::Plane(pl) => {
            let off = off_axis(&pl.frame, size).ok_or_else(|| bad("a plane not normal to the axis"))?;
            let z = pl.frame.origin.z;
            let (a, b) = match ends.len() {
                1 => (p2(0.0, ends[0].y), ends[0]),
                2 => (ends[0], ends[1]),
                n => return Err(bad(&format!("{n} meridian ends, not 1 or 2"))),
            };
            let _ = z;
            (Some(Mer { seg: line(a, b) }), off)
        }
        Surface::Cylinder(cy) => {
            let off = off_axis(&cy.frame, size).ok_or_else(|| bad("a cylinder not about the axis"))?;
            let (a, b) = two(&ends)?;
            (Some(Mer { seg: line(a, b) }), off)
        }
        Surface::Cone(co) => {
            let off = off_axis(&co.frame, size).ok_or_else(|| bad("a cone not about the axis"))?;
            let (a, b) = two(&ends)?;
            (Some(Mer { seg: line(a, b) }), off)
        }
        Surface::Sphere(sp) => {
            let o = sp.frame.origin;
            let off = (o.x * o.x + o.y * o.y).sqrt();
            if off > tol::LINEAR {
                return Err(bad("a sphere whose centre is off the axis"));
            }
            let (a, b) = two(&ends)?;
            let c = p2(0.0, o.z);
            (Some(Mer { seg: arc_on_face(body, fid, a, b, c)? }), off)
        }
        Surface::Torus(to) => {
            let off = off_axis(&to.frame, size).ok_or_else(|| bad("a torus not about the axis"))?;
            let (a, b) = two(&ends)?;
            let c = p2(to.major, to.frame.origin.z);
            (Some(Mer { seg: arc_on_face(body, fid, a, b, c)? }), off)
        }
        Surface::Procedural(_) => return Err(bad("a procedural surface")),
    };
    Ok((m.0, m.1.max(eoff)))
}

/// The arc from a to b about c that the face covers: of the two arcs between the ends, the one
/// whose height range fits the face's box (both fitting: the one whose ρ range does too).
fn arc_on_face(body: &Body, fid: FaceId, a: P2, b: P2, c: P2) -> Result<Seg, ProfileError> {
    let bb = body.face_box(fid);
    let rho_hi = (bb.x.hi.abs().max(bb.x.lo.abs()).powi(2) + bb.y.hi.abs().max(bb.y.lo.abs()).powi(2)).sqrt();
    let slack = 1e-6 * (1.0 + rho_hi + bb.z.hi.abs() + bb.z.lo.abs());
    let fits = |ccw: bool| -> (bool, bool) {
        let (zl, zh, rh) = arc_range(a, b, c, ccw);
        (zl >= bb.z.lo - slack && zh <= bb.z.hi + slack, rh <= rho_hi + slack)
    };
    let (z1, r1) = fits(true);
    let (z2, r2) = fits(false);
    let ccw = match (z1 && r1, z2 && r2) {
        (true, false) => true,
        (false, true) => false,
        _ => {
            return Err(ProfileError::Invalid {
                why: "a meridian arc whose side cannot be told from the face's box".into(),
            });
        }
    };
    Ok(Seg::Arc { a, b, c, ccw })
}

/// The z range and the largest ρ of the arc from a to b about c (counter-clockwise or not).
fn arc_range(a: P2, b: P2, c: P2, ccw: bool) -> (f64, f64, f64) {
    let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
    let ang = |p: P2| (p.y - c.y).atan2(p.x - c.x);
    let (mut t0, mut t1) = (ang(a), ang(b));
    if !ccw {
        std::mem::swap(&mut t0, &mut t1);
    }
    while t1 <= t0 {
        t1 += std::f64::consts::TAU;
    }
    let (mut zl, mut zh) = (a.y.min(b.y), a.y.max(b.y));
    let mut rh = a.x.max(b.x);
    for (k, th) in [(0, 0.0), (1, 0.5), (2, 1.0), (3, 1.5)] {
        let _ = k;
        let mut t = th * std::f64::consts::PI;
        while t < t0 {
            t += std::f64::consts::TAU;
        }
        if t <= t1 {
            let (x, y) = (c.x + r * t.cos(), c.y + r * t.sin());
            zl = zl.min(y);
            zh = zh.max(y);
            rh = rh.max(x);
        }
    }
    (zl, zh, rh)
}

/// The half-profile of a body of revolution about its z axis.
pub fn half_profile(body: &Body) -> Result<HalfProfile, ProfileError> {
    let mut size = 0.0f64;
    for fid in body.face_ids() {
        let bb = body.face_box(fid);
        for v in [bb.x.lo, bb.x.hi, bb.y.lo, bb.y.hi, bb.z.lo, bb.z.hi] {
            size = size.max(v.abs());
        }
    }
    let mut mers: Vec<Seg> = Vec::new();
    let mut off = 0.0f64;
    for (k, fid) in body.face_ids().enumerate() {
        let (m, o) = meridian(body, fid, k, size)?;
        off = off.max(o);
        if let Some(m) = m {
            // a face of a half turn (revolve's split faces) gives the same meridian twice
            let dup = mers.iter().any(|s| same_seg(s, &m.seg));
            if !dup {
                mers.push(m.seg);
            }
        }
    }
    if mers.is_empty() {
        return Err(ProfileError::Empty);
    }
    // close along the axis: the meridian ends on ρ = 0, sorted by z, pair up (material lies
    // between the 1st and 2nd, the 3rd and 4th, ...)
    let mut axis: Vec<f64> = Vec::new();
    for s in &mers {
        for q in [s.start(), s.end()] {
            if q.x == 0.0 && !axis.contains(&q.y) {
                axis.push(q.y);
            }
        }
    }
    axis.sort_by(|a, b| a.total_cmp(b));
    if axis.len() % 2 != 0 {
        return Err(ProfileError::Invalid {
            why: format!("{} meridian ends on the axis, not an even number", axis.len()),
        });
    }
    let mut i = 0;
    while i < axis.len() {
        mers.push(line(p2(0.0, axis[i]), p2(0.0, axis[i + 1])));
        i += 2;
    }
    let loops = chain(&mers)?;
    let mut region = Region { loops };
    orient(&mut region);
    let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for l in &region.loops {
        for s in &l.segs {
            let (zl, zh, rh, rl) = seg_range(s);
            bbox[0] = bbox[0].min(rl);
            bbox[1] = bbox[1].min(zl);
            bbox[2] = bbox[2].max(rh);
            bbox[3] = bbox[3].max(zh);
        }
    }
    let mu = tol::LINEAR + off + 64.0 * f64::EPSILON * (1.0 + size);
    let mut cx = forge_cam_geom::real::Ctx::new();
    forge_cam_geom::region::validate(&mut cx, &region).map_err(|e| ProfileError::Invalid { why: format!("{e:?}") })?;
    Ok(HalfProfile { region, mu, bbox })
}

/// (z_lo, z_hi, ρ_hi, ρ_lo) of a piece.
pub(crate) fn seg_range(s: &Seg) -> (f64, f64, f64, f64) {
    match *s {
        Seg::Line { a, b } => (a.y.min(b.y), a.y.max(b.y), a.x.max(b.x), a.x.min(b.x)),
        Seg::Arc { a, b, c, ccw } => {
            let (zl, zh, rh) = arc_range(a, b, c, ccw);
            let (_, _, nrl) = arc_range(p2(-a.x, a.y), p2(-b.x, b.y), p2(-c.x, c.y), !ccw);
            (zl, zh, rh, -nrl)
        }
    }
}

fn same_seg(s: &Seg, t: &Seg) -> bool {
    s == t || *s == t.reversed()
}

/// Links the pieces into closed loops by their bit-equal ends (each end met by exactly two).
fn chain(segs: &[Seg]) -> Result<Vec<Loop>, ProfileError> {
    let n = segs.len();
    let mut used = vec![false; n];
    let mut loops = Vec::new();
    for s0 in 0..n {
        if used[s0] {
            continue;
        }
        used[s0] = true;
        let first = segs[s0].start();
        let mut lp = vec![segs[s0]];
        let mut at = segs[s0].end();
        while at != first {
            let mut found = None;
            for (j, s) in segs.iter().enumerate() {
                if used[j] {
                    continue;
                }
                if s.start() == at {
                    found = Some(*s);
                } else if s.end() == at {
                    found = Some(s.reversed());
                }
                if found.is_some() {
                    used[j] = true;
                    break;
                }
            }
            let s = found.ok_or(ProfileError::Open { at })?;
            at = s.end();
            lp.push(s);
        }
        loops.push(Loop { segs: lp });
    }
    Ok(loops)
}

/// Twice the signed area of a loop (arcs by their chord plus the circular segment).
pub(crate) fn area2(l: &Loop) -> f64 {
    let mut a = 0.0;
    for s in &l.segs {
        let (p, q) = (s.start(), s.end());
        a += p.x * q.y - q.x * p.y;
        if let Seg::Arc { a: s0, b: s1, c, ccw } = *s {
            let r2 = (s0.x - c.x).powi(2) + (s0.y - c.y).powi(2);
            let r = r2.sqrt();
            let mut t = (s1.y - c.y).atan2(s1.x - c.x) - (s0.y - c.y).atan2(s0.x - c.x);
            if ccw {
                while t <= 0.0 {
                    t += std::f64::consts::TAU;
                }
            } else {
                while t >= 0.0 {
                    t -= std::f64::consts::TAU;
                }
            }
            // the circular segment between chord and arc, signed by the sense
            a += r * r * (t - t.sin());
            let _ = r;
        }
    }
    a
}

/// Outer loops counter-clockwise, loops inside another clockwise (by nesting of first points
/// in f64; validation then checks it exactly).
fn orient(rg: &mut Region) {
    let n = rg.loops.len();
    let areas: Vec<f64> = rg.loops.iter().map(area2).collect();
    for i in 0..n {
        let p = rg.loops[i].segs[0].start();
        let mut depth = 0;
        for j in 0..n {
            if j != i && areas[j].abs() > areas[i].abs() && inside_f64(&rg.loops[j], p) {
                depth += 1;
            }
        }
        let want_ccw = depth % 2 == 0;
        if (areas[i] > 0.0) != want_ccw {
            rg.loops[i] = rg.loops[i].reversed();
        }
    }
}

/// Even-odd ray cast against the loop's chords (for nesting only).
fn inside_f64(l: &Loop, p: P2) -> bool {
    let mut inside = false;
    for s in &l.segs {
        let (a, b) = (s.start(), s.end());
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if x > p.x {
                inside = !inside;
            }
        }
    }
    inside
}

/// A closed polygon as a loop of lines (in the order given).
pub fn poly(pts: &[[f64; 2]]) -> Loop {
    let n = pts.len();
    Loop {
        segs: (0..n)
            .map(|i| Seg::Line {
                a: p2(pts[i][0], pts[i][1]),
                b: p2(pts[(i + 1) % n][0], pts[(i + 1) % n][1]),
            })
            .collect(),
    }
}

/// The rectangle [x0, x1] × [y0, y1], counter-clockwise.
pub fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop {
    poly(&[[x0, y0], [x1, y0], [x1, y1], [x0, y1]])
}

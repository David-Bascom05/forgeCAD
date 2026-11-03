//! Corner seams: two flanges off adjacent edges of one wall, their ends at the corner trimmed or
//! extended to meet with a gap. Overlap: the first joint's flange runs on over the second's end
//! to its far skin, the second stops the gap short of the first's near skin (swap the joints for
//! underlap). Closed: both meet at the plane bisecting their mid-surfaces, each the half gap
//! back from it. Every cut is normal to its wall, so where a seam plane is not square to a wall
//! the line kept is the one of its two skins' that keeps no material past the plane: no layer
//! ever crosses a seam (exact; a labelled convention, as a laser cuts it). The seam starts the
//! gap above each flange's tangent line, so the bend zones stay whole; the corner of the base
//! gets the shop rule's corner relief (`CornerKind::Closed` / `Overlap` in `build::corners`).
//! The walls' outlines are edited in folded terms, so a K edit moves no seam.
use crate::error::{JointId, SheetError, WallId};
use crate::graph::{CornerKind, JointKind, Sheet, straight};
use forge_math::{Point2, Vec2, Vec3, dmath, tol};
use forge_ops::{ProfileLoop, Segment};

/// What a seam asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seam {
    /// The first joint's flange laps over the second's end.
    Overlap { gap: f64 },
    /// Butted at the bisecting plane.
    Closed { gap: f64 },
}

/// Why a seam is refused; `joints` as asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeamError {
    /// The joints are not bends off adjacent edges of one wall meeting at a corner.
    NotACorner,
    /// The joint's far wall is not a plain flange (a rectangle off its mold line).
    NotAFlange { joint: JointId },
    /// The flange's tip already carries a joint: the seam would change its length.
    TipTaken { joint: JointId, on: JointId },
    /// A gap below the floor (`crate::hem::GAP_FLOOR`, a solid has no touching faces).
    Gap { gap: f64, limit: f64 },
    /// The seam plane runs along the flange (they are coplanar or parallel).
    Parallel { joint: JointId },
    /// The seam would leave the flange `length` long at its tip or past its far end.
    NoRoom { joint: JointId, length: f64 },
}

/// A wall's frame in its parent's 3D coordinates (reference skin z = 0).
#[derive(Clone, Copy, Debug)]
struct Frame3 {
    o: Vec3,
    x: Vec3,
    y: Vec3,
    z: Vec3,
}

impl Frame3 {
    fn at(&self, x: f64, y: f64, z: f64) -> Vec3 {
        self.o + self.x * x + self.y * y + self.z * z
    }
}

/// A half space n . p >= c.
#[derive(Clone, Copy, Debug)]
struct Half {
    n: Vec3,
    c: f64,
}

struct Flange {
    joint: JointId,
    /// Its outline from (0, 0) round, each point with the id of the segment it starts; `tip` the
    /// index of the tip's start.
    ring: Vec<(Point2, u64)>,
    tip: usize,
    wall: WallId,
    f: Frame3,
    /// Height (mold line to tip) and length along the mold line.
    h: f64,
    len: f64,
    /// The end at the corner: y = len (true) or y = 0.
    far: bool,
    /// Setback on the flange.
    s: f64,
}

fn v3(p: Point2) -> Vec3 {
    Vec3::new(p.x, p.y, 0.0)
}

impl Sheet {
    /// Seams the corner where bends `a` and `b` (off adjacent edges of one wall) meet.
    pub fn seam(&mut self, a: JointId, b: JointId, seam: Seam) -> Result<(), SheetError> {
        let feature = self.feature;
        let fail = |why| SheetError::Seam {
            feature,
            joints: [a, b],
            why,
        };
        let gap = match seam {
            Seam::Overlap { gap } | Seam::Closed { gap } => gap,
        };
        if gap.is_nan() || gap < crate::hem::GAP_FLOOR {
            return Err(fail(SeamError::Gap {
                gap,
                limit: crate::hem::GAP_FLOOR,
            }));
        }
        let t = self.gauge.thickness;
        let ja = *self.joint(a).ok_or(fail(SeamError::NotACorner))?;
        let jb = *self.joint(b).ok_or(fail(SeamError::NotACorner))?;
        if ja.a != jb.a {
            return Err(fail(SeamError::NotACorner));
        }
        let base = self.wall(ja.a).ok_or(fail(SeamError::NotACorner))?;
        let (Ok((p0, p1)), Ok((q0, q1))) = (straight(base, ja.on_a), straight(base, jb.on_a)) else {
            return Err(fail(SeamError::NotACorner));
        };
        // the corner: a's edge ends where b's starts, or the other way round
        let a_far = if p1.distance(q0) <= tol::LINEAR {
            true
        } else if q1.distance(p0) <= tol::LINEAR {
            false
        } else {
            return Err(fail(SeamError::NotACorner));
        };
        let flange =
            |j: &crate::graph::Joint, e0: Point2, e1: Point2, far: bool| -> Result<Flange, SheetError> {
                let JointKind::Bend { angle, radius } = j.kind else {
                    return Err(fail(SeamError::NotAFlange { joint: j.id }));
                };
                let w = self
                    .wall(j.b)
                    .ok_or(fail(SeamError::NotAFlange { joint: j.id }))?;
                let len = e0.distance(e1);
                // a flange's outline: lines only, its mold line segment 3 from (0, len) to (0, 0), its
                // tip segment 1 at x = h (either end may be seamed already)
                let pts: Vec<(Point2, u64)> = match &w.outline[..] {
                    [l] => l
                        .segments
                        .iter()
                        .map(|(g, id)| match g {
                            Segment::Line { a, .. } => Some((*a, *id)),
                            _ => None,
                        })
                        .collect::<Option<_>>()
                        .unwrap_or_default(),
                    _ => vec![],
                };
                let not = || fail(SeamError::NotAFlange { joint: j.id });
                let at = |id: u64| pts.iter().position(|p| p.1 == id);
                let (Some(m), Some(tip)) = (at(3), at(1)) else {
                    return Err(not());
                };
                let n = pts.len();
                let h = pts[tip].0.x;
                let mold_ok =
                    j.on_b == 3 && pts[m].0 == Point2::new(0.0, len) && pts[(m + 1) % n].0 == Point2::ORIGIN;
                if !mold_ok || pts[(tip + 1) % n].0.x != h {
                    return Err(not());
                }
                // from (0, 0) round: the near end up to the tip, the far end after it
                let ring: Vec<(Point2, u64)> = (0..n).map(|i| pts[(m + 1 + i) % n]).collect();
                let tip = ring.iter().position(|p| p.1 == 1).expect("found above");
                if let Some(on) = self
                    .joints
                    .iter()
                    .find(|k| k.id != j.id && ((k.a == j.b) || (k.b == j.b)))
                {
                    return Err(fail(SeamError::TipTaken {
                        joint: j.id,
                        on: on.id,
                    }));
                }
                let d = (e1 - e0) * (1.0 / len);
                let n = Vec2::new(d.y, -d.x);
                let (th, sg) = (angle.abs(), angle.signum());
                let (sn, cs) = dmath::sin_cos(th);
                let z = Vec3::new(0.0, 0.0, 1.0);
                let f = Frame3 {
                    o: v3(e0),
                    x: v3(Point2::ORIGIN + n * cs) + z * (sg * sn),
                    y: v3(Point2::ORIGIN + d),
                    z: v3(Point2::ORIGIN + n * (-sg * sn)) + z * cs,
                };
                Ok(Flange {
                    ring,
                    tip,
                    joint: j.id,
                    wall: j.b,
                    f,
                    h,
                    len,
                    far,
                    s: crate::build::setback_of(&self.gauge, radius, angle),
                })
            };
        let fa = flange(&ja, p0, p1, a_far)?;
        let fb = flange(&jb, q0, q1, !a_far)?;
        let mid = |f: &Flange| f.f.at(0.5 * f.h, 0.5 * f.len, -0.5 * t);
        let skin = |f: &Flange, z: f64| Half {
            n: f.f.z,
            c: f.f.z.dot(f.f.o) + z,
        };
        // a skin plane facing `from`'s middle: its half space holding it
        let facing = |h: Half, from: Vec3| {
            if h.n.dot(from) >= h.c {
                h
            } else {
                Half { n: -h.n, c: -h.c }
            }
        };
        let (ia, ib) = (mid(&fa), mid(&fb));
        let shift = |h: Half, by: f64| Half { n: h.n, c: h.c + by };
        let (ka, kb) = match seam {
            Seam::Overlap { gap } => {
                // a runs to b's far skin; b stops gap short of a's near skin
                let far = [0.0, -t]
                    .map(|z| facing(skin(&fb, z), ia))
                    .into_iter()
                    .max_by(|x, y| (x.n.dot(ia) - x.c).total_cmp(&(y.n.dot(ia) - y.c)))
                    .expect("two skins");
                let near = [0.0, -t]
                    .map(|z| facing(skin(&fa, z), ib))
                    .into_iter()
                    .min_by(|x, y| (x.n.dot(ib) - x.c).total_cmp(&(y.n.dot(ib) - y.c)))
                    .expect("two skins");
                (far, shift(near, gap))
            }
            Seam::Closed { gap } => {
                // the plane bisecting the mid-surfaces between the two flanges
                let (ma, mb) = (skin(&fa, -0.5 * t), skin(&fb, -0.5 * t));
                let g = |sg: f64| {
                    let n = ma.n + mb.n * sg;
                    let l = n.length();
                    Half {
                        n: n * (1.0 / l),
                        c: (ma.c + mb.c * sg) / l,
                    }
                };
                let pick = [g(-1.0), g(1.0)]
                    .into_iter()
                    .find(|h| (h.n.dot(ia) - h.c) * (h.n.dot(ib) - h.c) < 0.0)
                    .ok_or(fail(SeamError::Parallel { joint: a }))?;
                let (pa, pb) = (facing(pick, ia), facing(pick, ib));
                (shift(pa, 0.5 * gap), shift(pb, 0.5 * gap))
            }
        };
        let mut work = self.clone();
        for (f, k) in [(&fa, ka), (&fb, kb)] {
            let outline = trimmed(f, k, t, gap).map_err(fail)?;
            let w = work
                .walls
                .iter_mut()
                .find(|w| w.id == f.wall)
                .expect("found above");
            w.outline = vec![outline];
        }
        work.set_corner(
            a,
            b,
            match seam {
                Seam::Overlap { gap } => CornerKind::Overlap { gap },
                Seam::Closed { .. } => CornerKind::Closed,
            },
        );
        *self = work;
        Ok(())
    }
}

/// The flange's outline with its corner end cut along the half space `k`, from the gap above its
/// tangent line to its tip.
fn trimmed(f: &Flange, k: Half, t: f64, gap: f64) -> Result<ProfileLoop, SeamError> {
    // k on the wall: a x + b y + c0 + c1 z >= 0
    let (a, b) = (k.n.dot(f.f.x), k.n.dot(f.f.y));
    let toward = if f.far { 1.0 } else { -1.0 };
    // kept side must be away from the corner: b * toward < 0
    if b.abs() <= 1e-12 || b * toward >= 0.0 {
        return Err(SeamError::Parallel { joint: f.joint });
    }
    // the y where the seam crosses x, on whichever skin keeps less
    let y_at = |x: f64| {
        let ys = [0.0, -t].map(|z| (k.c - k.n.dot(f.f.at(0.0, 0.0, z)) - a * x) / b);
        if f.far { ys[0].min(ys[1]) } else { ys[0].max(ys[1]) }
    };
    let xs = f.s + gap;
    let (ys, yh) = (y_at(xs), y_at(f.h));
    // what is left at either end, from the far end
    let shortest = if f.far {
        ys.min(yh - f.ring[f.tip].0.y)
    } else {
        (f.len - ys).min(f.ring[f.tip + 1].0.y - yh)
    };
    if xs >= f.h - tol::LINEAR || shortest <= tol::LINEAR {
        return Err(SeamError::NoRoom {
            joint: f.joint,
            length: shortest.min(f.h - xs),
        });
    }
    let p = Point2::new;
    let (h, len) = (f.h, f.len);
    let near = &f.ring[..f.tip];
    let far = &f.ring[f.tip + 1..];
    let mut pts: Vec<(Point2, u64)> = Vec::new();
    if f.far {
        pts.extend_from_slice(near);
        pts.push((f.ring[f.tip].0, 1));
        pts.push((p(h, yh), 4));
        if (ys - len).abs() > tol::LINEAR {
            pts.push((p(xs, ys), 5));
        }
        pts.push((p(xs, len), 2));
        pts.push((p(0.0, len), 3));
    } else {
        pts.push((p(0.0, 0.0), 0));
        pts.push((p(xs, 0.0), 6));
        if ys.abs() > tol::LINEAR {
            pts.push((p(xs, ys), 7));
        }
        pts.push((p(h, yh), 1));
        pts.extend_from_slice(far);
    }
    let n = pts.len();
    Ok(ProfileLoop::new(
        (0..n)
            .map(|i| {
                (
                    Segment::Line {
                        a: pts[i].0,
                        b: pts[(i + 1) % n].0,
                    },
                    pts[i].1,
                )
            })
            .collect(),
    ))
}

/// Whether a flange's outline carries a seam's cut at either end (its seam segments' ids).
pub(crate) fn seamed(w: &crate::graph::Wall) -> bool {
    w.outline
        .iter()
        .flat_map(|l| &l.segments)
        .any(|s| matches!(s.1, 4 | 7))
}

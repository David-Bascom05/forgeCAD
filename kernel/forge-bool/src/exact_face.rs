//! Exact point-in-face pieces for flat faces: `vertex_at` (a vertex as the meet of the planes round
//! it, `exactOf`'s vertex), the exact parity crossing `crosses` and its screen `crosses_f` (proven
//! in verify/ieee754, Screen). Moved out of decide_once.rs so that today's boolean's exact
//! fallbacks (old-path-statements.md §1a) and decide-once (A8, the fallback) share one copy.

use forge_math::exact::{Dy, DyPlane, HPoint, meet3};

/// `Snap.vertexAt`: the first independent three of the planes meet in a point that every one of
/// them holds.
pub(crate) fn vertex_at(ps: &[&DyPlane]) -> Option<HPoint> {
    let mut p = None;
    'find: for a in ps {
        for b in ps {
            for c in ps {
                if let Some(q) = meet3(a, b, c) {
                    p = Some(q);
                    break 'find;
                }
            }
        }
    }
    let p = p?;
    ps.iter().all(|q| q.side(&p) == 0).then_some(p)
}

/// An exact rational `n / d`.
#[derive(Clone)]
pub(crate) struct Q {
    pub(crate) n: Dy,
    pub(crate) d: Dy,
}

impl Q {
    pub(crate) fn of(h: &HPoint, axis: usize) -> Q {
        Q {
            n: h.p.get(axis).clone(),
            d: h.w.clone(),
        }
    }
    pub(crate) fn sub(&self, o: &Q) -> Q {
        Q {
            n: self.n.mul(&o.d).sub(&o.n.mul(&self.d)),
            d: self.d.mul(&o.d),
        }
    }
    pub(crate) fn mul(&self, o: &Q) -> Q {
        Q {
            n: self.n.mul(&o.n),
            d: self.d.mul(&o.d),
        }
    }
    pub(crate) fn sgn(&self) -> i32 {
        self.n.signum() * self.d.signum()
    }
}

/// `Ref.crossesYZ` in axes `(s, t)`: does the ray from `q` toward +s (its origin perturbed to
/// `q + (ε, ε²)`) cross segment `ab`? Exact.
pub(crate) fn crosses(q: &HPoint, a: &HPoint, b: &HPoint, s: usize, t: usize) -> bool {
    let (qs, qt) = (Q::of(q, s), Q::of(q, t));
    let (a_s, at) = (Q::of(a, s), Q::of(a, t));
    let (bs, bt) = (Q::of(b, s), Q::of(b, t));
    let aa = at.sub(&qt).sgn() > 0;
    let ab = bt.sub(&qt).sgn() > 0;
    if aa == ab {
        return false;
    }
    let dz = bt.sub(&at);
    let d0 = bs.sub(&a_s).mul(&qt.sub(&at)).sub(&qs.sub(&a_s).mul(&dz));
    let sg = if d0.sgn() != 0 { d0.sgn() } else { -dz.sgn() };
    sg * dz.sgn() > 0
}

/// `crosses`, screened in f64: the answer where every sign it takes is plainly away from zero, else
/// `None` (the exact test decides). The f64 points are within a few ulps of the exact ones.
pub(crate) fn crosses_f(q: [f64; 3], a: [f64; 3], b: [f64; 3], s: usize, t: usize) -> Option<bool> {
    const E: f64 = 1e-12;
    // (every coordinate of the three points, the dropped axis too: `Pts::pf`'s bound is relative
    // to a point's largest coordinate)
    let m3 = |p: [f64; 3]| p[0].abs().max(p[1].abs()).max(p[2].abs());
    let scale = 1.0 + m3(q).max(m3(a)).max(m3(b));
    let above = |x: f64| -> Option<bool> {
        let d = x - q[t];
        if d.abs() > E * scale { Some(d > 0.0) } else { None }
    };
    let aa = above(a[t])?;
    let ab = above(b[t])?;
    if aa == ab {
        return Some(false);
    }
    // b above and a not: dz > 0; and the other way round
    let dz_pos = ab;
    let d0 = (b[s] - a[s]) * (q[t] - a[t]) - (q[s] - a[s]) * (b[t] - a[t]);
    let mag = ((b[s] - a[s]).abs() + (q[s] - a[s]).abs() + (q[t] - a[t]).abs() + (b[t] - a[t]).abs()) * scale;
    if d0.abs() <= E * mag {
        return None;
    }
    Some((d0 > 0.0) == dz_pos)
}

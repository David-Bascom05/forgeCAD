//! Certified events of a torus on a cylinder (`Form::Poly4`), by interval subdivision of the
//! (u, s) plane (curved-sites gap #7; compiled for tests and the decision log only, so the default
//! build is unchanged).
//!
//! `intersect` finds where the root count of F(u, .) changes by sampling the scaled quartic
//! discriminant at 64 places, which can miss a loop narrower than a sample (fuzz seed 295,
//! feature 69). Here the same equation F(u, s) = T(q(u, s)) is enclosed rigorously over boxes
//! U × S, with T the torus's implicit and q(u, s) the cylinder's point in the torus's frame, both
//! read from the stored data:
//! * a box where F excludes 0 holds no point of the curve;
//! * a box where ∂F/∂s excludes 0 holds no event: every root there is simple, so the root count
//!   cannot change inside it;
//! * every other box is split until its u-width is under `W_EVENT`. What is left encloses every
//!   event (every (u, s) with F = ∂F/∂s = 0), and nothing else is certain about it.
//!
//! The verdict compares those event clusters with the events the sampled search found: a cluster
//! no found event lies in is an event the search missed (`Missed`); running out of boxes is
//! `Undecided`. `Certified` proves no event was missed: between two found events the root count
//! is constant, so every loop the curve has is seen by the tracks.
use super::{Alg, Eqn};
use crate::surface::Surface;
use forge_math::{Interval, Vec3, dmath};

/// Below this u-width a box that excludes neither F nor ∂F/∂s is an event's.
const W_EVENT: f64 = 1e-6;
/// Boxes examined before giving up.
const BUDGET: usize = 200_000;

#[derive(Clone, Copy)]
struct IV3 {
    x: Interval,
    y: Interval,
    z: Interval,
}

impl IV3 {
    fn of(v: Vec3) -> IV3 {
        IV3 {
            x: Interval::point(v.x),
            y: Interval::point(v.y),
            z: Interval::point(v.z),
        }
    }
    fn add(self, o: IV3) -> IV3 {
        IV3 {
            x: self.x + o.x,
            y: self.y + o.y,
            z: self.z + o.z,
        }
    }
    fn scale(self, k: Interval) -> IV3 {
        IV3 {
            x: self.x * k,
            y: self.y * k,
            z: self.z * k,
        }
    }
    fn dot(self, o: IV3) -> Interval {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
}

/// The verdict on a sampled event search.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Verdict {
    /// Every event lies within `W_EVENT` of a found one.
    Certified { boxes: usize },
    /// Events the search missed, at these u (the middle of each cluster).
    Missed { at: Vec<f64>, boxes: usize },
    /// The budget ran out (a pair near tangent over a stretch): not decided.
    Undecided { boxes: usize },
    /// Not a torus on a cylinder.
    NotCovered,
}

/// The data of F(u, s) for a torus on a cylinder, in the torus's frame: the cylinder's point is
/// c0 + rho (cx cos u + cy sin u) + s cz.
struct Data {
    c0: IV3,
    cx: IV3,
    cy: IV3,
    cz: IV3,
    rho: Interval,
    k: Interval,
    big2: Interval,
    s_bound: f64,
}

fn data(eq: &Eqn) -> Option<Data> {
    let (Surface::Cylinder(cyl), &Alg::Torus { frame, big, r }) = (&eq.host, &eq.alg) else {
        return None;
    };
    let (tx, ty, tz) = (
        IV3::of(frame.x().get()),
        IV3::of(frame.y().get()),
        IV3::of(frame.z().get()),
    );
    let local = |v: IV3| IV3 {
        x: v.dot(tx),
        y: v.dot(ty),
        z: v.dot(tz),
    };
    let d = cyl.frame.origin - frame.origin;
    let c0 = local(IV3::of(d));
    let (cx, cy, cz) = (
        local(IV3::of(cyl.frame.x().get())),
        local(IV3::of(cyl.frame.y().get())),
        local(IV3::of(cyl.frame.z().get())),
    );
    // |q(u, s)| <= R + r on the torus; |q| >= |s| |z| - |c0| - rho (|x| + |y|), |z| about 1
    let reach = d.length() + cyl.radius * 2.0 + big + r;
    let s_bound = 2.0 * reach + 1.0;
    let big = Interval::point(big);
    let r = Interval::point(r);
    Some(Data {
        c0,
        cx,
        cy,
        cz,
        rho: Interval::point(cyl.radius),
        k: big.sqr() - r.sqr(),
        big2: big.sqr(),
        s_bound,
    })
}

/// F and ∂F/∂s enclosed over U × S.
fn enclose(d: &Data, u: Interval, s: Interval) -> (Interval, Interval) {
    let (su, cu) = (u.sin(), u.cos());
    let rad = d.cx.scale(cu).add(d.cy.scale(su));
    let q = d.c0.add(rad.scale(d.rho)).add(d.cz.scale(s));
    let qq = q.dot(q);
    let w = qq + d.k;
    let four_b2 = d.big2 * 4.0;
    let f = w.sqr() - four_b2 * (q.x.sqr() + q.y.sqr());
    // ∇T = 4 w q − 8 R² (qx, qy, 0); dq/ds = cz
    let g = IV3 {
        x: w * q.x * 4.0 - d.big2 * q.x * 8.0,
        y: w * q.y * 4.0 - d.big2 * q.y * 8.0,
        z: w * q.z * 4.0,
    };
    (f, g.dot(d.cz))
}

/// The events of F(u, .) on a torus–cylinder pair against `found` (u in [0, 2π)).
pub(crate) fn certify(eq: &Eqn, found: &[(f64, bool)]) -> Verdict {
    let Some(d) = data(eq) else {
        return Verdict::NotCovered;
    };
    let tau = dmath::TAU;
    let mut stack: Vec<(Interval, Interval)> = Vec::new();
    let (nu, ns) = (64, 8);
    for i in 0..nu {
        let u = Interval::new(tau * i as f64 / nu as f64, tau * (i + 1) as f64 / nu as f64);
        for j in 0..ns {
            let lo = -d.s_bound + 2.0 * d.s_bound * j as f64 / ns as f64;
            let hi = -d.s_bound + 2.0 * d.s_bound * (j + 1) as f64 / ns as f64;
            stack.push((u, Interval::new(lo, hi)));
        }
    }
    let mut boxes = 0;
    let mut events: Vec<Interval> = Vec::new();
    while let Some((u, s)) = stack.pop() {
        boxes += 1;
        if boxes > BUDGET {
            return Verdict::Undecided { boxes };
        }
        let (f, fs) = enclose(&d, u, s);
        if !f.contains_zero() || !fs.contains_zero() {
            continue;
        }
        if u.width() < W_EVENT {
            events.push(u);
            continue;
        }
        // split the wider side, measured against the other's scale
        if s.width() > 32.0 * u.width() * (1.0 + d.s_bound) {
            let (a, b) = s.split();
            stack.push((u, a));
            stack.push((u, b));
        } else {
            let (a, b) = u.split();
            stack.push((a, s));
            stack.push((b, s));
        }
    }
    // the event boxes' u, merged into clusters
    events.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    let mut clusters: Vec<Interval> = Vec::new();
    for e in events {
        match clusters.last_mut() {
            Some(c) if e.lo <= c.hi + W_EVENT => c.hi = c.hi.max(e.hi),
            _ => clusters.push(e),
        }
    }
    let near = |c: &Interval| {
        found.iter().any(|&(x, _)| {
            let x = x.rem_euclid(tau);
            [x - tau, x, x + tau]
                .iter()
                .any(|&y| y >= c.lo - 4.0 * W_EVENT && y <= c.hi + 4.0 * W_EVENT)
        })
    };
    let missed: Vec<f64> = clusters.iter().filter(|c| !near(c)).map(|c| c.mid()).collect();
    if missed.is_empty() {
        Verdict::Certified { boxes }
    } else {
        Verdict::Missed { at: missed, boxes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cylinder, Torus};
    use forge_math::{Frame, Point3, UnitVec3};

    /// fuzz seed 295, feature 69 (FALLBACK's repro): a thin torus on a wide tilted cylinder meets
    /// it in two loops round the tube, at torus angles about 42° and 229°. The sampled search
    /// finds one; the certifier must not certify that.
    #[test]
    fn thin_torus_loop_not_certified_when_missed() {
        let torus = Surface::Torus(
            Torus::new(
                Frame::from_zx(
                    Point3::new(57.513833943223545, 82.64151449414277, 31.529651297690172),
                    UnitVec3::new_unchecked(Vec3::new(0.0, 0.0, -1.0)),
                    Vec3::new(1.0, 0.0, 0.0),
                )
                .expect("frame"),
                1.018123715855934,
                0.22881392007971665,
            )
            .expect("torus"),
        );
        let z = Vec3::new(-0.04760542051711398, 0.17499343179871532, 0.9834180305265392);
        let x = Vec3::new(-0.34481495645685883, 0.921132764696941, -0.18060198117801576);
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::from_zx(
                Point3::new(64.28286796892466, 86.4342803264453, 17.574469684406452),
                UnitVec3::new_unchecked(z),
                x,
            )
            .expect("frame"),
            radius: 8.70706734376011,
        });
        let (host, other, form) = super::super::setup(&cyl, &torus).expect("a closed form");
        let eq = Eqn::new(host, Alg::of(&other).expect("alg"), form);
        let sampled = eq.events(64);
        let v = certify(&eq, &sampled);
        eprintln!("sampled {:?}\nverdict {v:?}", sampled);
        assert!(
            !matches!(v, Verdict::Certified { .. }),
            "{v:?} with {} sampled",
            sampled.len()
        );
    }
}

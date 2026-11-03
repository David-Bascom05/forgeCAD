//! What every option's answer is held against, none of it the option's own code.
use crate::cases::Case;
use crate::reference;
use crate::region::{Component, Region, holds, loop_of};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2};
use forge_ops::{Profile, ProfileError, ProfileLoop};

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// Right area, right points, every component a valid profile.
    Good,
    /// Right area, right points, and the boundary touches itself: 2D right, no solid (`pinches` points).
    Pinched { pinches: usize },
    /// The area differs from the reference by more than it can explain.
    WrongArea { got: f64, want: f64, limit: f64 },
    /// `wrong` of `of` sampled points disagree with the input.
    WrongPoints { wrong: usize, of: usize },
    /// Loops that the kernel's own profile check refuses.
    BadProfile { part: usize, why: ProfileError },
}

fn direct_inside(l: &[ProfileLoop], p: Point2) -> bool {
    // an independent even-odd: fine polygons of the input loops
    let mut odd = false;
    for x in l {
        let poly: Vec<Point2> = x.segments.iter().flat_map(|(s, _)| s.samples(200)).collect();
        let n = poly.len();
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && a.x + (b.x - a.x) * (p.y - a.y) / (b.y - a.y) > p.x {
                odd = !odd;
            }
        }
    }
    odd
}

fn near_boundary(l: &ProfileLoop, p: Point2, d: f64) -> bool {
    l.nearest(p).distance(p) <= d
}

fn profile_of(c: &Component) -> Result<Profile, ProfileError> {
    let mut loops = Vec::with_capacity(1 + c.holes.len());
    let mut id = 0u64;
    let mut push = |l: &[crate::region::Piece]| -> Option<()> {
        let mut x = loop_of(l)?;
        for s in &mut x.segments {
            s.1 = id;
            id += 1;
        }
        loops.push(x);
        Some(())
    };
    if push(&c.outer).is_none() {
        return Err(ProfileError::Empty);
    }
    for h in &c.holes {
        if push(h).is_none() {
            return Err(ProfileError::Empty);
        }
    }
    Ok(Profile {
        plane: Frame::WORLD,
        loops,
    })
}

/// The reference area of a case: rows fine enough that its own error is far below anything we test for.
pub fn truth(case: &Case, rows: usize) -> (f64, f64) {
    reference::area(&case.wall, &case.cuts, rows)
}

pub fn verdict(case: &Case, r: &Region, want: (f64, f64), seed: u64) -> Verdict {
    let got = r.area();
    // the reference's estimate, and what a boundary known to the tolerance can move
    let limit = 4.0 * want.1 + 1e-3;
    if (got - want.0).abs() > limit {
        return Verdict::WrongArea {
            got,
            want: want.0,
            limit,
        };
    }
    let mut rng = Rng::new(seed ^ 0xabc);
    let (mut wrong, of) = (0, 2000);
    let mut tested = 0;
    for _ in 0..of {
        let p = Point2::new(rng.range(-5.0, 305.0), rng.range(-5.0, 155.0));
        if case
            .wall
            .iter()
            .chain(case.cuts.iter())
            .any(|l| near_boundary(l, p, 0.05))
        {
            continue;
        }
        tested += 1;
        let truth = direct_inside(&case.wall, p)
            && !case
                .cuts
                .iter()
                .any(|c| direct_inside(std::slice::from_ref(c), p));
        let ours = r.parts.iter().any(|c| holds(c, p));
        if truth != ours {
            wrong += 1;
        }
    }
    if wrong > 0 {
        return Verdict::WrongPoints { wrong, of: tested };
    }
    let mut touches = r.pinches.len();
    for (i, c) in r.parts.iter().enumerate() {
        let p = match profile_of(c) {
            Ok(p) => p,
            Err(why) => return Verdict::BadProfile { part: i, why },
        };
        match p.normalized() {
            Ok(_) => {}
            Err(ProfileError::Pinched { .. }) => touches += 1,
            // its crossing test is on sampled polygons, so a loop touching itself reads as crossing
            Err(ProfileError::SelfIntersecting { .. }) if !r.pinches.is_empty() => touches += 1,
            Err(why) => {
                if std::env::var_os("SHOWBAD").is_some() {
                    for l in &p.loops {
                        for (s, id) in &l.segments {
                            println!("  {id}: {s:?}");
                        }
                    }
                }
                return Verdict::BadProfile { part: i, why };
            }
        }
    }
    if touches > 0 {
        Verdict::Pinched { pinches: touches }
    } else {
        Verdict::Good
    }
}

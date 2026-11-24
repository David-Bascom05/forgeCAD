//! Linking and ordering of 3D finishing passes.
//!
//! A fixed rule, the same shape as TOOLPATH-QUALITY's 2D linking (verify/forge-cam-quality): the
//! kind of link follows from the geometry alone (length and proven clearance), in the order
//! below. Links are new moves between the passes as generated (the passes themselves are not
//! changed), each proven free by the drop-cutter where it is made.
//!
//! From the end a of one pass to the start b of the next:
//! 1. **stay down**: a straight feed move a → b when it is at most `max_stay` long in xy and the
//!    CL height is proven at most `slack` above it all along (`seg3_below`; slack at most the
//!    op's inner tolerance, so the certifier's G1 holds);
//! 2. **lift**: else up to z_l = (the proven maximum CL height along a → b) + `lift`, across at
//!    z_l, and down to b, all at feed, when z_l is below the retract height;
//! 3. **retract**: else up to the retract height at feed, a rapid across, and a plunge to b.
//!
//! Ordering: greedy nearest start (an open pass may be reversed when the strategy allows it),
//! from the op's start; groups (e.g. waterline levels top down) are kept in order.
//!
//! Fitting (`path_fit`): each pass's points are fitted into longer lines and arcs by forge-cam's
//! one fitter (`forge_cam::fit`) before they are emitted, within the rule's chord tolerance, and
//! each fitted piece is kept only when the drop-cutter proves it as it proves a chord: a line by
//! `seg3_below`, an arc by `arc3_below`, both with the rule's slack. A fitted piece is within the
//! tolerance of the pass both ways, so the cusp it leaves is the pass's plus at most the
//! tolerance (the caller budgets for it).

use crate::drape::Pass;
use crate::drop::DropCutter;
use crate::path::Emit;
use forge_cam::fit::{self, Fit, Piece};
use forge_cam::ops::{OpPath, PMove, Phase};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rules {
    /// The longest stay-down link (xy), e.g. twice the stepover.
    pub max_stay: f64,
    /// The height a lifted link keeps above the CL surface.
    pub lift: f64,
    /// How far a stay-down link may dip below the CL surface (at most the op's inner
    /// tolerance; a link exactly on a face is not provable against its lowering's μ).
    pub slack: f64,
}

/// Fitting passes into longer moves before they are emitted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitRule {
    /// The chord tolerance and the pieces allowed.
    pub fit: Fit,
    /// How far a fitted piece may dip below the CL surface (at most the op's inner tolerance,
    /// as for the strategies' own chords).
    pub slack: f64,
}

/// What a linked path did, for the quality report.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LinkCount {
    pub stay: usize,
    pub lift: usize,
    pub retract: usize,
    /// With fitting: the fitted pieces' measured largest distance from their passes (mm;
    /// `fit::deviation`, sampled every 0.01 mm); 0 without.
    pub fit_dev: f64,
}

fn dxy(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1])).sqrt()
}

/// The vertex of `p` nearest `at` in xy (distance, index), the repeated closing vertex excluded.
fn nearest(p: &Pass, at: [f64; 3]) -> (f64, usize) {
    let mut bd = f64::INFINITY;
    let mut bi = 0;
    let mut i = 0;
    while i + 1 < p.len() {
        let d = dxy(at, p[i]);
        if d < bd {
            bd = d;
            bi = i;
        }
        i += 1;
    }
    (bd, bi)
}

/// Greedy nearest-start order within each group; `reversible` lets an open pass run backwards.
pub fn order(groups: Vec<Vec<Pass>>, start: [f64; 3], reversible: bool) -> Vec<Pass> {
    let mut out: Vec<Pass> = Vec::new();
    let mut at = start;
    for g in groups {
        let mut left: Vec<Option<Pass>> = g.into_iter().map(Some).collect();
        let mut n = left.iter().filter(|p| p.is_some()).count();
        while n > 0 {
            let mut best = usize::MAX;
            let mut rev = false;
            let mut bd = f64::INFINITY;
            let mut k = 0;
            while k < left.len() {
                if let Some(p) = &left[k]
                    && !p.is_empty()
                {
                    let closed = p.len() > 2 && p[0] == p[p.len() - 1];
                    // a closed pass can start at any vertex: its nearest
                    let (d0, _) = if closed {
                        nearest(p, at)
                    } else {
                        (dxy(at, p[0]), 0)
                    };
                    if d0 < bd {
                        bd = d0;
                        best = k;
                        rev = false;
                    }
                    if reversible && !closed {
                        let d1 = dxy(at, p[p.len() - 1]);
                        if d1 < bd {
                            bd = d1;
                            best = k;
                            rev = true;
                        }
                    }
                }
                k += 1;
            }
            if best == usize::MAX {
                break;
            }
            let mut p = left[best].take().unwrap();
            if rev {
                p.reverse();
            }
            if p.len() > 2 && p[0] == p[p.len() - 1] {
                let (_, i) = nearest(&p, at);
                if i > 0 {
                    // rotate the ring to start at vertex i (drop the repeated end, re-close)
                    p.pop();
                    p.rotate_left(i);
                    p.push(p[0]);
                }
            }
            at = p[p.len() - 1];
            out.push(p);
            n -= 1;
        }
    }
    out
}

/// The passes, in order, linked by the rule, as one path.
pub fn path(dc: &dyn DropCutter, ps: &[Pass], r: Rules, e: Emit) -> (OpPath, LinkCount) {
    path_fit(dc, ps, r, e, None)
}

/// A pass fitted by the rule: its cutting moves after its first point.
pub fn fit_pass(dc: &dyn DropCutter, p: &Pass, fr: FitRule) -> Vec<Piece> {
    fit::fit(p, fr.fit, &mut |a, pc| match *pc {
        Piece::Line { to } => dc.seg3_below(a, to, fr.slack),
        Piece::Arc { to, c, cw } => dc.arc3_below(a, to, c, cw, fr.slack),
    })
}

/// `path`, with every pass fitted first when `fit` is given (the passes in parallel, in a fixed
/// order).
pub fn path_fit(
    dc: &dyn DropCutter,
    ps: &[Pass],
    r: Rules,
    e: Emit,
    fit: Option<FitRule>,
) -> (OpPath, LinkCount) {
    let fitted: Option<Vec<Vec<Piece>>> = fit.map(|fr| {
        forge_math::par::map(ps.len(), |k| {
            if ps[k].len() < 2 {
                Vec::new()
            } else {
                fit_pass(dc, &ps[k], fr)
            }
        })
    });
    let mut moves = Vec::new();
    let mut count = LinkCount::default();
    if let Some(f) = &fitted {
        let dev = forge_math::par::map(ps.len(), |k| {
            if ps[k].len() < 2 {
                0.0
            } else {
                fit::deviation(&ps[k], &f[k], 0.01)
            }
        });
        count.fit_dev = dev.into_iter().fold(0.0, f64::max);
    }
    let start = match ps.first().and_then(|l| l.first()) {
        Some(p) => [p[0], p[1], e.h.clear],
        None => [0.0, 0.0, e.h.clear],
    };
    let mut prev: Option<[f64; 3]> = None;
    for (k, p) in ps.iter().enumerate() {
        if p.len() < 2 {
            continue;
        }
        let b = p[0];
        match prev {
            None => {
                moves.push(PMove::Rapid {
                    to: [b[0], b[1], e.h.retract],
                });
                moves.push(PMove::Line {
                    to: b,
                    f: e.feed.1,
                    phase: Phase::Plunge,
                });
            }
            Some(a) => link(dc, a, b, r, e, &mut moves, &mut count),
        }
        match &fitted {
            Some(f) => moves.extend(fit::to_moves(&f[k], e.feed.0, Phase::Cut)),
            None => {
                let mut i = 1;
                while i < p.len() {
                    moves.push(PMove::Line {
                        to: p[i],
                        f: e.feed.0,
                        phase: Phase::Cut,
                    });
                    i += 1;
                }
            }
        }
        prev = Some(p[p.len() - 1]);
    }
    if let Some(a) = prev {
        moves.push(PMove::Line {
            to: [a[0], a[1], e.h.retract],
            f: e.feed.1,
            phase: Phase::Link,
        });
        moves.push(PMove::Rapid {
            to: [a[0], a[1], e.h.clear],
        });
    }
    (
        OpPath {
            tool: e.tool,
            start,
            moves,
            rpm: e.rpm,
            coolant: e.coolant,
        },
        count,
    )
}

fn link(
    dc: &dyn DropCutter,
    a: [f64; 3],
    b: [f64; 3],
    r: Rules,
    e: Emit,
    moves: &mut Vec<PMove>,
    count: &mut LinkCount,
) {
    let ab = [[a[0], a[1]], [b[0], b[1]]];
    if dxy(a, b) <= r.max_stay && dc.seg3_below(a, b, r.slack) {
        count.stay += 1;
        moves.push(PMove::Line {
            to: b,
            f: e.feed.0,
            phase: Phase::Link,
        });
        return;
    }
    // the lowest proven-free height for the traverse
    let top = match dc.drop_seg(ab[0], ab[1]) {
        None => a[2].max(b[2]),
        Some(band) => band.hi.max(a[2]).max(b[2]),
    };
    let zl = top + r.lift;
    if zl < e.h.retract && dc.seg_below(ab[0], ab[1], zl) {
        count.lift += 1;
        moves.push(PMove::Line {
            to: [a[0], a[1], zl],
            f: e.feed.1,
            phase: Phase::Link,
        });
        moves.push(PMove::Line {
            to: [b[0], b[1], zl],
            f: e.feed.0,
            phase: Phase::Link,
        });
        moves.push(PMove::Line {
            to: b,
            f: e.feed.1,
            phase: Phase::Plunge,
        });
        return;
    }
    count.retract += 1;
    moves.push(PMove::Line {
        to: [a[0], a[1], e.h.retract],
        f: e.feed.1,
        phase: Phase::Link,
    });
    moves.push(PMove::Rapid {
        to: [b[0], b[1], e.h.retract],
    });
    moves.push(PMove::Line {
        to: b,
        f: e.feed.1,
        phase: Phase::Plunge,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;
    use crate::steep;
    use forge_cam::ops::Heights;

    #[test]
    fn steep_and_shallow_on_the_dome_mostly_stays_down() {
        let dc = Analytic {
            r: 3.0,
            planes: vec![(0.0, [-40.0, -40.0], [40.0, 40.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        };
        let h = std::f64::consts::FRAC_1_SQRT_2;
        let p = steep::Params {
            sin0: h,
            cos0: h,
            overlap: 1.0,
            dz: 1.0,
            stepover: 1.0,
            angle_deg: 0.0,
            top: 20.0,
            bottom: 0.0,
            grid: 1.0,
            tol: 0.01,
            leave: 0.01,
            contour: 0.01,
            area: None,
        };
        let (st, sh) = steep::steep_and_shallow(&dc, p).unwrap();
        let e = Emit {
            tool: 0,
            h: Heights {
                clear: 40.0,
                retract: 25.0,
            },
            feed: (1500.0, 500.0),
            rpm: 12000.0,
            coolant: forge_post::Coolant::Flood,
        };
        let ord = order(vec![st, sh], [0.0, 0.0, 40.0], true);
        let (_path, c) = path(
            &dc,
            &ord,
            Rules {
                max_stay: 2.0,
                lift: 0.5,
                slack: 0.0,
            },
            e,
        );
        let total = c.stay + c.lift + c.retract;
        assert!(total > 10);
        // fitted: fewer moves, each cutting line proven by the drop-cutter (a fitted piece at the
        // fit's slack, a kept input chord at the drape's, here the op's tol 0.01), within the
        // tolerance of the passes (the dome's drop-cutter has no arc test: lines only)
        let fr = FitRule {
            fit: Fit::arcs(0.002),
            slack: 0.005,
        };
        let r = Rules {
            max_stay: 2.0,
            lift: 0.5,
            slack: 0.0,
        };
        let (p0, _) = path(&dc, &ord, r, e);
        let (p1, c1) = path_fit(&dc, &ord, r, e, Some(fr));
        assert!(
            p1.moves.len() < p0.moves.len(),
            "{} vs {}",
            p1.moves.len(),
            p0.moves.len()
        );
        assert!(c1.fit_dev <= 0.002 * (1.0 + 1e-9), "{}", c1.fit_dev);
        let mut at = p1.start;
        for m in &p1.moves {
            match *m {
                PMove::Line {
                    to,
                    phase: Phase::Cut,
                    ..
                } => {
                    assert!(dc.seg3_below(at, to, 0.01), "{at:?} -> {to:?}");
                    at = to;
                }
                PMove::Arc { .. } => panic!("no arc test, no arcs"),
                PMove::Rapid { to } | PMove::Line { to, .. } => at = to,
                _ => {}
            }
        }
        // most links stay down or lift a little; full retracts are rare
        assert!(c.retract * 4 < total, "{c:?}");
        assert!(c.stay > 0, "{c:?}");
    }
}

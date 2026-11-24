//! From passes to a forge-cam path (`OpPath`), which forge-cam's checker certifies move by move
//! against the exact faces (refuse-only: it never edits the path).
//!
//! Linking here is the plain safe form: each pass is entered by a rapid at the retract height
//! over its first point, a straight plunge at plunge feed (free: the point is proven free at the
//! pass's height, so the vertical line above it is too), the pass, and a retract. Stay-down
//! linking is `link.rs`.

use crate::drape::Pass;
use crate::waterline::Loop;
use forge_cam::ops::{Heights, OpPath, PMove, Phase};

/// What an op links and cuts with.
#[derive(Clone, Copy, Debug)]
pub struct Emit {
    pub tool: usize,
    pub h: Heights,
    /// (cut, plunge) feeds, mm/min.
    pub feed: (f64, f64),
    pub rpm: f64,
    pub coolant: forge_post::Coolant,
}

/// Closed waterline loops, in the given order, as one path.
pub fn loops(ls: &[Loop], e: Emit) -> OpPath {
    let mut moves = Vec::new();
    let start = match ls.first().and_then(|l| l.pts.first()) {
        Some(p) => [p[0], p[1], e.h.clear],
        None => [0.0, 0.0, e.h.clear],
    };
    let mut k = 0;
    while k < ls.len() {
        let l = &ls[k];
        if l.pts.is_empty() {
            k += 1;
            continue;
        }
        let p0 = l.pts[0];
        moves.push(PMove::Rapid {
            to: [p0[0], p0[1], e.h.retract],
        });
        moves.push(PMove::Line {
            to: [p0[0], p0[1], l.z],
            f: e.feed.1,
            phase: Phase::Plunge,
        });
        let mut i = 1;
        while i <= l.pts.len() {
            let q = l.pts[i % l.pts.len()];
            moves.push(PMove::Line {
                to: [q[0], q[1], l.z],
                f: e.feed.0,
                phase: Phase::Cut,
            });
            i += 1;
        }
        // off the wall at feed: a rapid must keep the rapid clearance, a cutting-feed move only
        // the tolerance, and the pass ends against the material
        moves.push(PMove::Line {
            to: [p0[0], p0[1], e.h.retract],
            f: e.feed.1,
            phase: Phase::Link,
        });
        k += 1;
    }
    if let Some(PMove::Line { to, .. }) = moves.last().copied() {
        moves.push(PMove::Rapid {
            to: [to[0], to[1], e.h.clear],
        });
    }
    OpPath {
        tool: e.tool,
        start,
        moves,
        rpm: e.rpm,
        coolant: e.coolant,
    }
}

/// Open 3D passes, in the given order, as one path: each entered by a rapid over its first
/// point at the retract height and a plunge at plunge feed (free: the point is at or above its
/// CL height, and the vertical line above it is too), cut, and left by a retract at feed.
pub fn passes(ps: &[Pass], e: Emit) -> OpPath {
    let mut moves = Vec::new();
    let start = match ps.first().and_then(|l| l.first()) {
        Some(p) => [p[0], p[1], e.h.clear],
        None => [0.0, 0.0, e.h.clear],
    };
    for p in ps {
        if p.len() < 2 {
            continue;
        }
        let p0 = p[0];
        moves.push(PMove::Rapid {
            to: [p0[0], p0[1], e.h.retract],
        });
        moves.push(PMove::Line {
            to: p0,
            f: e.feed.1,
            phase: Phase::Plunge,
        });
        let mut i = 1;
        while i < p.len() {
            moves.push(PMove::Line {
                to: p[i],
                f: e.feed.0,
                phase: Phase::Cut,
            });
            i += 1;
        }
        let q = p[p.len() - 1];
        moves.push(PMove::Line {
            to: [q[0], q[1], e.h.retract],
            f: e.feed.1,
            phase: Phase::Link,
        });
    }
    if let Some(PMove::Line { to, .. }) = moves.last().copied() {
        moves.push(PMove::Rapid {
            to: [to[0], to[1], e.h.clear],
        });
    }
    OpPath {
        tool: e.tool,
        start,
        moves,
        rpm: e.rpm,
        coolant: e.coolant,
    }
}

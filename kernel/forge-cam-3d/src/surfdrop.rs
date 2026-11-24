//! The adapter onto CAM-3AXIS's drop-cutter (forge_cam::surf::Surfacer): its certified CL
//! heights (closed forms per face, branch and bound otherwise) and its column gap check, behind
//! this crate's `DropCutter` seam, so every strategy here runs on the lowered B-rep.
//!
//! - `drop` is `Surfacer::cl` (the generating column: the cutter grown by the leave, widened a
//!   little past its rim; at least the floor);
//! - the segment tests check the cutter's column grown by the leave (`gap_move`, then the
//!   column's undecided taken as not proven: the strategy splits the piece), so a chord keeps
//!   the leave as the points do.

use crate::drop::{Band, DropCutter, seg_band};
use forge_cam::surf::{Surfacer, XyGrid};
use forge_cam_geom::check::Move;
use forge_cam_geom::drop::{Column, Gap, gap_move};
use forge_cam_geom::ival::Ival;

/// Cells for one face's gap check in a generator's segment test.
const GAP_BUDGET: usize = 2_000;

pub struct SurfDrop<'s, 'a> {
    pub sf: &'s Surfacer<'a>,
    chk: Column,
    grid: XyGrid,
}

impl<'s, 'a> SurfDrop<'s, 'a> {
    pub fn new(sf: &'s Surfacer<'a>) -> Option<SurfDrop<'s, 'a>> {
        let chk = sf.check.grow(sf.leave)?;
        let grid = XyGrid::new(sf.faces, chk.r);
        Some(SurfDrop { sf, chk, grid })
    }

    /// Every face near the move proven at least `-limit` (≥ 0) below the grown column over it.
    fn clear(&self, a: [f64; 3], b: [f64; 3], limit: f64) -> bool {
        let r = self.chk.r;
        let idx = self.grid.near(
            a[0].min(b[0]) - r,
            a[1].min(b[1]) - r,
            a[0].max(b[0]) + r,
            a[1].max(b[1]) + r,
        );
        let mv = Move::Line { a, b };
        // the answer is "every face near is proven clear", the same in any order of the faces:
        // the face that refused this thread's last move first (the pieces of a split chord
        // and the next chords mostly fail on the same face), so a refusal costs one face
        let first = LAST_REFUSED.with(|c| c.get());
        if idx.contains(&first) && !self.face_clear(first as usize, &mv, limit) {
            return false;
        }
        let mut i = 0;
        while i < idx.len() {
            if idx[i] != first && !self.face_clear(idx[i] as usize, &mv, limit) {
                LAST_REFUSED.with(|c| c.set(idx[i]));
                return false;
            }
            i += 1;
        }
        true
    }

    /// Face `fi` proven at least `-limit` below the grown column over the move.
    fn face_clear(&self, fi: usize, mv: &Move, limit: f64) -> bool {
        let f = &self.sf.faces[fi];
        // the closed form for a ball cutter (contact.rs: sphere, cylinder, plane, torus,
        // cone), then the column's gap on a small budget: undecided is "not proven", and
        // the strategy splits the piece
        if matches!(
            forge_cam_geom::contact::clear(&self.sf.cutter, f, mv, self.sf.leave + limit),
            Some(forge_cam_geom::check::Verdict::Clear { .. })
        ) {
            return true;
        }
        match gap_move(&self.chk, f, mv, limit, self.sf.res, GAP_BUDGET).0 {
            Gap::Clear { .. } => true,
            Gap::Over { .. } => false,
            // undecided by the column: not proven. The generator splits the piece (short
            // pieces resolve by the closed forms), which is far cheaper than a branch and
            // bound here; the path's certification is the checker's, after
            Gap::Unknown => false,
        }
    }
}

std::thread_local! {
    /// The face index that refused this thread's last segment test (a hint only: see `clear`).
    static LAST_REFUSED: std::cell::Cell<u32> = const { std::cell::Cell::new(u32::MAX) };
}

impl DropCutter for SurfDrop<'_, '_> {
    fn drop(&self, x: f64, y: f64) -> Option<Band> {
        let c = self.sf.cl(x, y);
        Some(Band { lo: c.lo, hi: c.hi })
    }

    fn drop_seg(&self, a: [f64; 2], b: [f64; 2]) -> Option<Band> {
        seg_band(self, a, b, self.sf.res)
    }

    fn seg_below(&self, a: [f64; 2], b: [f64; 2], z: f64) -> bool {
        self.clear([a[0], a[1], z], [b[0], b[1], z], 0.0)
    }

    fn seg3_below(&self, a: [f64; 3], b: [f64; 3], slack: f64) -> bool {
        self.clear(a, b, -slack.max(0.0))
    }

    /// The arc by its chords: the arc's sweep cut into n equal angles Δ, each piece within its
    /// sagitta s ≤ r Δ²/8 (widened, at most a quarter of the slack) of its chord at the same
    /// fraction. The cutter's signed distance is 1-Lipschitz in its position, and the column
    /// grown by s holds the cutter anywhere within s of the chord: so each chord proven, per
    /// face, by the closed form at the slack less s, or by the grown column's gap at the slack,
    /// proves the arc. Undecided: not proven.
    fn arc3_below(&self, a: [f64; 3], to: [f64; 3], c: [f64; 2], cw: bool, slack: f64) -> bool {
        let r = ((a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2)).sqrt();
        let slack = slack.max(0.0);
        if !(r.is_finite() && r > 0.0 && to.iter().all(|v| v.is_finite()) && slack.is_finite() && slack > 0.0)
        {
            return false;
        }
        let tau = std::f64::consts::TAU;
        let a0 = forge_math::dmath::atan2(a[1] - c[1], a[0] - c[0]);
        let mut a1 = forge_math::dmath::atan2(to[1] - c[1], to[0] - c[0]);
        if cw {
            while a1 >= a0 {
                a1 -= tau;
            }
        } else {
            while a1 <= a0 {
                a1 += tau;
            }
        }
        let sweep = a1 - a0;
        // chords of sagitta at most slack/4 (Δ = √(2 slack / r)), at least one per quarter turn
        let dmax = (2.0 * slack / r).sqrt().min(0.5 * std::f64::consts::PI);
        let n = ((sweep.abs() / dmax).ceil() as usize).max(1);
        if n > 4096 {
            return false;
        }
        let d = sweep.abs() / n as f64;
        let sag = 1.01 * r * d * d / 8.0 + 1e-12 * (1.0 + r + c[0].abs().max(c[1].abs()));
        let Some(grown) = self.chk.grow(sag) else {
            return false;
        };
        let at = |k: usize| -> [f64; 3] {
            if k == 0 {
                return a;
            }
            if k == n {
                return to;
            }
            let t = k as f64 / n as f64;
            let (sn, cs) = forge_math::dmath::sin_cos(a0 + sweep * t);
            [c[0] + r * cs, c[1] + r * sn, a[2] + (to[2] - a[2]) * t]
        };
        let w = r + grown.r;
        let idx = self.grid.near(c[0] - w, c[1] - w, c[0] + w, c[1] + w);
        // outward-rounded, as `contact::clear_arc`
        let lim = Ival::pt(self.sf.leave).sub(Ival::pt(slack)).add(Ival::pt(sag)).hi;
        for k in 0..n {
            let mv = Move::Line {
                a: at(k),
                b: at(k + 1),
            };
            for &i in &idx {
                let f = &self.sf.faces[i as usize];
                if matches!(
                    forge_cam_geom::contact::clear(&self.sf.cutter, f, &mv, lim),
                    Some(forge_cam_geom::check::Verdict::Clear { .. })
                ) {
                    continue;
                }
                match gap_move(&grown, f, &mv, -slack, self.sf.res, GAP_BUDGET).0 {
                    Gap::Clear { .. } => {}
                    _ => return false,
                }
            }
        }
        true
    }

    fn extent(&self) -> [[f64; 2]; 2] {
        let r = self.sf.make.r;
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for f in self.sf.faces {
            lo = [lo[0].min(f.bbox[0].lo - r), lo[1].min(f.bbox[1].lo - r)];
            hi = [hi[0].max(f.bbox[0].hi + r), hi[1].max(f.bbox[1].hi + r)];
        }
        [lo, hi]
    }
}

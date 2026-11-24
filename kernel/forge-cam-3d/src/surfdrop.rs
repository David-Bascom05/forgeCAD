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
        let mut i = 0;
        while i < idx.len() {
            let f = &self.sf.faces[idx[i] as usize];
            // the closed form for a ball cutter (contact.rs: sphere, cylinder, plane, torus,
            // cone), then the column's gap on a small budget: undecided is "not proven", and
            // the strategy splits the piece
            if forge_cam_geom::contact::clear(&self.sf.cutter, f, &mv, self.sf.leave + limit).is_some() {
                i += 1;
                continue;
            }
            match gap_move(&self.chk, f, &mv, limit, self.sf.res, GAP_BUDGET).0 {
                Gap::Clear { .. } => {}
                Gap::Over { .. } => return false,
                // undecided by the column: not proven. The generator splits the piece (short
                // pieces resolve by the closed forms), which is far cheaper than a branch and
                // bound here; the path's certification is the checker's, after
                Gap::Unknown => return false,
            }
            i += 1;
        }
        true
    }
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

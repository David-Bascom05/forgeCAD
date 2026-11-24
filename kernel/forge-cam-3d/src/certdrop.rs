//! A drop-cutter for any faces the certifier takes, built only on its verdicts.
//!
//! This is the generic, certified fallback behind the `DropCutter` seam (CAM-3AXIS's Surfacer,
//! with its closed forms, is the primary; an adapter for it replaces this where it applies).
//! It needs no geometry of its own:
//!
//! - A vertical move of the tool from tip height z up to z_top that certifies **Clear** at limit
//!   `leave` proves that no tip height in [z, z_top] brings the body within `leave` of the face:
//!   the CL height (the highest such contact) is below z, or above z_top, which the caller rules
//!   out by starting z_top above the face's box.
//! - A **Hit** gives a witness instant s: the tip height z + s(z_top − z) is a proven contact, so
//!   the CL height is at least that.
//!
//! Bisection between the two gives, per face, a band [lo, hi] with lo from witnesses and hi from
//! clearances; an Undecided step keeps the current hi (sound: hi is proven) and stops. The CL
//! band is the max over the faces near the tool's footprint. The segment tests are a certify of
//! the move itself against those faces (Clear on every one).

use crate::drop::{Band, DropCutter};
use forge_cam_geom::check::{Face, Move, Verdict, certify};
use forge_cam_geom::tool::Body;

pub struct CertDrop<'a> {
    /// The tool's bodies, cutter first (all are held to the same `leave` here: the drop is the
    /// cutter's; holders are the path certifier's business).
    pub cutter: &'a Body,
    pub faces: &'a [Face],
    /// Stock to leave: the CL is where the cutter comes within `leave` of a face.
    pub leave: f64,
    /// Band width to stop at.
    pub tol: f64,
    /// Cell budget per certify call, and its resolution.
    pub budget: usize,
    pub res: f64,
}

impl CertDrop<'_> {
    fn near(&self, lo: [f64; 2], hi: [f64; 2]) -> Vec<usize> {
        let r = self.cutter.rmax;
        let mut v = Vec::new();
        let mut i = 0;
        while i < self.faces.len() {
            let b = &self.faces[i].bbox;
            if b[0].hi >= lo[0] - r && b[0].lo <= hi[0] + r && b[1].hi >= lo[1] - r && b[1].lo <= hi[1] + r {
                v.push(i);
            }
            i += 1;
        }
        v
    }

    fn clear(&self, f: &Face, mv: &Move, limit: f64) -> Verdict {
        certify(self.cutter, f, mv, limit, self.res, self.budget).0
    }

    /// The band of one face's CL height at (x, y), or None when the tool never meets it there.
    fn drop_face(&self, f: &Face, x: f64, y: f64) -> Option<Band> {
        let top = f.bbox[2].hi + 1.0;
        let bottom = f.bbox[2].lo - (self.cutter.z1 - self.cutter.z0) - 1.0;
        // anything at all above the bottom?
        let all = Move::Line {
            a: [x, y, bottom],
            b: [x, y, top],
        };
        let mut lo = match self.clear(f, &all, self.leave) {
            Verdict::Clear { .. } => return None,
            Verdict::Hit { s, .. } => bottom + s * (top - bottom),
            Verdict::Undecided { .. } => bottom,
        };
        let mut hi = top;
        let mut k = 0;
        while hi - lo > self.tol && k < 60 {
            let m = 0.5 * (lo + hi);
            let mv = Move::Line {
                a: [x, y, m],
                b: [x, y, hi],
            };
            match self.clear(f, &mv, self.leave) {
                Verdict::Clear { .. } => hi = m,
                Verdict::Hit { s, .. } => lo = lo.max(m + s * (hi - m)),
                Verdict::Undecided { .. } => break,
            }
            k += 1;
        }
        Some(Band { lo, hi })
    }

    fn move_clear(&self, mv: &Move, lo: [f64; 2], hi: [f64; 2], limit: f64) -> bool {
        let ids = self.near(lo, hi);
        let mut i = 0;
        while i < ids.len() {
            if !matches!(self.clear(&self.faces[ids[i]], mv, limit), Verdict::Clear { .. }) {
                return false;
            }
            i += 1;
        }
        true
    }
}

fn bx(a: [f64; 2], b: [f64; 2]) -> ([f64; 2], [f64; 2]) {
    ([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])])
}

impl DropCutter for CertDrop<'_> {
    fn drop(&self, x: f64, y: f64) -> Option<Band> {
        let ids = self.near([x, y], [x, y]);
        let mut out: Option<Band> = None;
        let mut i = 0;
        while i < ids.len() {
            if let Some(b) = self.drop_face(&self.faces[ids[i]], x, y) {
                out = Some(match out {
                    None => b,
                    Some(o) => o.max(b),
                });
            }
            i += 1;
        }
        out
    }

    fn drop_seg(&self, a: [f64; 2], b: [f64; 2]) -> Option<Band> {
        let da = self.drop(a[0], a[1]);
        let db = self.drop(b[0], b[1]);
        let lo = match (da, db) {
            (None, None) => None,
            (Some(p), None) | (None, Some(p)) => Some(p.lo),
            (Some(p), Some(q)) => Some(p.lo.max(q.lo)),
        };
        // the upper end: the lowest height, by bisection, at which the level move is proven
        // clear, above the ends' own bands
        let start = match (da, db) {
            (None, None) => f64::NEG_INFINITY,
            (Some(p), None) | (None, Some(p)) => p.hi,
            (Some(p), Some(q)) => p.hi.max(q.hi),
        };
        let (blo, bhi) = bx(a, b);
        let ids = self.near(blo, bhi);
        let mut top = f64::NEG_INFINITY;
        for &i in &ids {
            top = top.max(self.faces[i].bbox[2].hi + 1.0);
        }
        if ids.is_empty() {
            return lo.map(|l| Band { lo: l, hi: start });
        }
        let mut h = top;
        let mut l = if start.is_finite() { start } else { top - 1e3 };
        if self.seg_below(a, b, l) {
            h = l;
        } else {
            let mut k = 0;
            while h - l > self.tol && k < 60 {
                let m = 0.5 * (l + h);
                if self.seg_below(a, b, m) {
                    h = m;
                } else {
                    l = m;
                }
                k += 1;
            }
        }
        Some(Band {
            lo: lo.unwrap_or(f64::NEG_INFINITY),
            hi: h,
        })
    }

    fn seg_below(&self, a: [f64; 2], b: [f64; 2], z: f64) -> bool {
        let (lo, hi) = bx(a, b);
        self.move_clear(
            &Move::Line {
                a: [a[0], a[1], z],
                b: [b[0], b[1], z],
            },
            lo,
            hi,
            self.leave,
        )
    }

    fn seg3_below(&self, a: [f64; 3], b: [f64; 3], slack: f64) -> bool {
        let (lo, hi) = bx([a[0], a[1]], [b[0], b[1]]);
        self.move_clear(&Move::Line { a, b }, lo, hi, self.leave - slack)
    }

    fn extent(&self) -> [[f64; 2]; 2] {
        let r = self.cutter.rmax;
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for f in self.faces {
            lo = [lo[0].min(f.bbox[0].lo - r), lo[1].min(f.bbox[1].lo - r)];
            hi = [hi[0].max(f.bbox[0].hi + r), hi[1].max(f.bbox[1].hi + r)];
        }
        [lo, hi]
    }
}

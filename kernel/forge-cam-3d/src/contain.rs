//! Containment: where a strategy may cut. Every strategy's passes are clipped to it at their
//! vertices (a kept chord was proven where it was generated, so clipping keeps every guarantee).
//!
//! - `boundary`: an xy polygon the tool centre stays inside (none: the whole part);
//! - `z`: the band of heights the tool tip stays in;
//! - `slope`: the band of CL slopes (degrees from horizontal) a point must lie in, read from
//!   the slope map (steep.rs).

use crate::drape::Pass;
use crate::steep::{Mask, SlopeMap};

#[derive(Clone, Debug, PartialEq)]
pub struct Containment {
    pub boundary: Option<Vec<[f64; 2]>>,
    pub zmin: f64,
    pub zmax: f64,
    /// (tan of the least slope, tan of the greatest): [0, ∞] for any.
    pub tan_slope: (f64, f64),
}

impl Default for Containment {
    fn default() -> Self {
        Containment {
            boundary: None,
            zmin: f64::NEG_INFINITY,
            zmax: f64::INFINITY,
            tan_slope: (0.0, f64::INFINITY),
        }
    }
}

/// Whether p is inside the polygon (even-odd; points on the edge count as either).
pub fn inside(poly: &[[f64; 2]], p: [f64; 2]) -> bool {
    let n = poly.len();
    let mut c = false;
    let mut i = 0;
    let mut j = n.wrapping_sub(1);
    while i < n {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            c = !c;
        }
        j = i;
        i += 1;
    }
    c
}

impl Containment {
    /// The slope mask this containment needs (None when the slope band is everything).
    pub fn mask(&self, sm: &SlopeMap) -> Option<Mask> {
        if self.tan_slope.0 <= 0.0 && self.tan_slope.1 == f64::INFINITY {
            return None;
        }
        let mut on = vec![false; sm.tan.len()];
        let mut k = 0;
        while k < on.len() {
            let t = sm.tan[k];
            on[k] = !t.is_nan() && t >= self.tan_slope.0 && t <= self.tan_slope.1;
            k += 1;
        }
        Some(Mask {
            x0: sm.x0,
            y0: sm.y0,
            g: sm.g,
            nx: sm.nx,
            ny: sm.ny,
            on,
        })
    }

    fn holds(&self, q: [f64; 3], m: Option<&Mask>) -> bool {
        if q[2] < self.zmin || q[2] > self.zmax {
            return false;
        }
        if let Some(b) = &self.boundary
            && !inside(b, [q[0], q[1]])
        {
            return false;
        }
        match m {
            None => true,
            Some(m) => m.at([q[0], q[1]]),
        }
    }

    /// The runs of each pass that lie in the containment.
    pub fn clip(&self, ps: Vec<Pass>, m: Option<&Mask>) -> Vec<Pass> {
        let mut out = Vec::new();
        for p in ps {
            let mut cur: Pass = Vec::new();
            for q in p {
                if self.holds(q, m) {
                    cur.push(q);
                } else {
                    if cur.len() > 1 {
                        out.push(core::mem::take(&mut cur));
                    }
                    cur.clear();
                }
            }
            if cur.len() > 1 {
                out.push(cur);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_and_z() {
        let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert!(inside(&sq, [5.0, 5.0]) && !inside(&sq, [15.0, 5.0]));
        let c = Containment {
            boundary: Some(sq),
            zmin: 0.0,
            zmax: 5.0,
            ..Default::default()
        };
        let p = vec![
            [-1.0, 5.0, 1.0],
            [1.0, 5.0, 1.0],
            [3.0, 5.0, 1.0],
            [5.0, 5.0, 9.0],
            [7.0, 5.0, 1.0],
        ];
        let out = c.clip(vec![p], None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 2);
    }
}

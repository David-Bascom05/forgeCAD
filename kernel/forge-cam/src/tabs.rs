//! Holding tabs on a contour loop: short bridges left standing on the lowest levels so the part
//! stays joined to the stock.
//!
//! Method (plain, textbook): along the loop the tool centre lifts over each tab's interval, of
//! the tab's width plus the tool's diameter on the centre path (so the bridge left is `width`
//! wide where the loop is straight), to the tab's top; a rectangular tab goes straight up at the
//! interval's start and straight down at its end, a triangular one ramps up to the interval's
//! middle and down again. Tabs are placed at even spacing along the loop (the first half a
//! spacing from the loop's start), or at the loop's points nearest given points.
use crate::leads::seg_len;
use forge_cam_geom::region::{P2, Seg, p2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabShape {
    Rect,
    Triangle,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TabPlace {
    /// This many tabs per loop, evenly spaced.
    Count(usize),
    /// A tab every this length along the loop (at least one).
    Spacing(f64),
    /// A tab at each loop's point nearest each of these.
    At(Vec<[f64; 2]>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tabs {
    pub place: TabPlace,
    /// The bridge's width along the loop.
    pub width: f64,
    /// The bridge's height above the contour's bottom.
    pub height: f64,
    pub shape: TabShape,
}

/// The intervals [s0, s1] of arc length along a loop of length `total` that the tool centre
/// lifts over, sorted, each of `w` (the tab's width plus the tool's diameter). Refused when they
/// overlap each other or the loop's start (the plunge and the overlap are there): why.
pub fn intervals(segs: &[Seg], tabs: &Tabs, tool_d: f64, keep_clear: f64) -> Result<Vec<(f64, f64)>, String> {
    let total: f64 = segs.iter().map(seg_len).sum();
    let w = tabs.width + tool_d;
    let mut mids: Vec<f64> = match &tabs.place {
        TabPlace::Count(n) => (0..*n).map(|k| total * (k as f64 + 0.5) / *n as f64).collect(),
        TabPlace::Spacing(sp) => {
            let n = ((total / sp).floor() as usize).max(1);
            (0..n).map(|k| total * (k as f64 + 0.5) / n as f64).collect()
        }
        TabPlace::At(ps) => ps.iter().map(|q| param_nearest(segs, p2(q[0], q[1]))).collect(),
    };
    mids.sort_by(f64::total_cmp);
    let iv: Vec<(f64, f64)> = mids.iter().map(|m| (m - 0.5 * w, m + 0.5 * w)).collect();
    for k in 1..iv.len() {
        if iv[k].0 < iv[k - 1].1 {
            return Err(format!(
                "tabs overlap: each takes {w:.3} of the tool's path (width plus the tool's diameter) and the loop is {total:.3} long"
            ));
        }
    }
    for &(a, b) in &iv {
        if a < keep_clear || b > total {
            return Err(format!(
                "a tab ({w:.3} long on the tool's path, at {:.3} of {total:.3}) reaches the loop's start: move it or the `entry:`",
                0.5 * (a + b)
            ));
        }
    }
    Ok(iv)
}

/// The arc-length parameter of the loop's point nearest q.
fn param_nearest(segs: &[Seg], q: P2) -> f64 {
    let mut best = (f64::INFINITY, 0.0);
    let mut s0 = 0.0;
    for s in segs {
        let l = seg_len(s);
        let (d, t) = crate::leads::nearest_param(s, q);
        if d < best.0 {
            best = (d, s0 + t * l);
        }
        s0 += l;
    }
    best.1
}

/// The loop cut at the arc-length parameters `cuts` (sorted, inside the loop): its pieces in
/// order, between consecutive cuts (the first from the start, the last to the end).
pub fn cut(segs: &[Seg], cuts: &[f64]) -> Vec<Vec<Seg>> {
    let mut out: Vec<Vec<Seg>> = vec![vec![]];
    let mut s0 = 0.0;
    let mut k = 0;
    for s in segs {
        let l = seg_len(s);
        let mut rest = *s;
        let mut from = 0.0; // the parameter along s where `rest` starts
        while k < cuts.len() && cuts[k] < s0 + l {
            let t = cuts[k] - s0;
            if t <= from + 1e-9 {
                // at rest's start: a new piece begins here
                if !out.last().is_some_and(|p| p.is_empty()) {
                    out.push(vec![]);
                }
            } else {
                let p = crate::leads::point_at(s, t / l);
                let (h, tl) = crate::leads::split_at(&rest, p);
                if let Some(v) = out.last_mut() {
                    v.push(h);
                }
                out.push(vec![]);
                rest = tl;
                from = t;
            }
            k += 1;
        }
        if let Some(v) = out.last_mut() {
            v.push(rest);
        }
        s0 += l;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Seg> {
        let p = [p2(0.0, 0.0), p2(10.0, 0.0), p2(10.0, 10.0), p2(0.0, 10.0)];
        (0..4)
            .map(|i| Seg::Line {
                a: p[i],
                b: p[(i + 1) % 4],
            })
            .collect()
    }

    #[test]
    fn intervals_and_cut() {
        let t = Tabs {
            place: TabPlace::Count(2),
            width: 2.0,
            height: 1.0,
            shape: TabShape::Rect,
        };
        let iv = intervals(&square(), &t, 4.0, 0.0).unwrap();
        assert_eq!(iv, vec![(7.0, 13.0), (27.0, 33.0)]);
        let cuts: Vec<f64> = iv.iter().flat_map(|&(a, b)| [a, b]).collect();
        let pieces = cut(&square(), &cuts);
        assert_eq!(pieces.len(), 5);
        assert_eq!(pieces[1].first().unwrap().start(), p2(7.0, 0.0));
        assert_eq!(pieces[1].last().unwrap().end(), p2(10.0, 3.0));
        assert_eq!(pieces[3].first().unwrap().start(), p2(3.0, 10.0));
        // the pieces join end to end and cover the loop
        let all: Vec<Seg> = pieces.concat();
        for k in 1..all.len() {
            assert_eq!(all[k - 1].end(), all[k].start());
        }
        assert_eq!(all.last().unwrap().end(), p2(0.0, 0.0));
        let at = Tabs {
            place: TabPlace::At(vec![[5.0, 20.0]]),
            ..t.clone()
        };
        assert_eq!(intervals(&square(), &at, 4.0, 0.0).unwrap(), vec![(22.0, 28.0)]);
        let many = Tabs {
            place: TabPlace::Count(8),
            ..t
        };
        assert!(
            intervals(&square(), &many, 4.0, 0.0)
                .unwrap_err()
                .contains("overlap")
        );
    }
}

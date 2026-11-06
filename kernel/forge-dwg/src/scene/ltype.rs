//! Line type patterns laid along paths: dashes drawn, gaps skipped, dots as very short dashes.

use super::geom::P2;

fn len2(a: P2, b: P2) -> f64 {
    ((b[0] - a[0]) * (b[0] - a[0]) + (b[1] - a[1]) * (b[1] - a[1])).sqrt()
}

fn lerp(a: P2, b: P2, t: f64) -> P2 {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// The most dashes one path gets; past that the pattern is too fine to see and the path is
/// drawn solid (as AutoCAD does when a pattern is too dense).
const MAX_DASHES: f64 = 5_000.0;

/// A pattern scaled to world units: positive a dash, negative a gap, zero a dot.
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub dashes: Vec<f64>,
    /// The length a dot is drawn as.
    pub dot: f64,
}

impl Pattern {
    pub fn period(&self) -> f64 {
        self.dashes.iter().map(|d| d.abs()).sum()
    }
}

/// Lays the pattern along a path. `restarts` are the point indices where the pattern starts
/// again (a polyline's vertices without continuous generation); the pattern always starts at
/// the first point. Returns the dashes as segments (each pair of points one; a dash round a
/// corner is two), or None when the path should be drawn as it is (no pattern, or one too
/// fine to see).
pub fn dash(pts: &[P2], closed: bool, restarts: &[usize], pat: &Pattern) -> Option<Vec<P2>> {
    let period = pat.period();
    if pat.dashes.is_empty() || period <= 0.0 || !period.is_finite() || pts.len() < 2 {
        return None;
    }
    if pat.dashes.iter().all(|d| *d > 0.0) {
        return None;
    }
    let n = pts.len() + closed as usize;
    let at = |i: usize| pts[i % pts.len()];
    let mut total = 0.0;
    for i in 1..n {
        total += len2(at(i - 1), at(i));
    }
    if total / period > MAX_DASHES {
        return None;
    }
    let mut out = Vec::with_capacity(((total / period) as usize + 2) * 2 * pat.dashes.len());
    let mut ri = 0;
    let mut k = 0; // the pattern element
    let mut left = pat.dashes[0].abs(); // what is left of it
    let mut last = at(0);
    for i in 1..n {
        // the pattern starts again at a restart vertex
        while ri < restarts.len() && restarts[ri] < i - 1 {
            ri += 1;
        }
        if i > 1 && ri < restarts.len() && restarts[ri] == i - 1 {
            k = 0;
            left = pat.dashes[0].abs();
            last = at(i - 1);
        }
        let (a, b) = (at(i - 1), at(i));
        let l = len2(a, b);
        let mut t = 0.0; // along this segment
        while l - t > left {
            t += left;
            let p = lerp(a, b, t / l);
            let d = pat.dashes[k];
            if d == 0.0 {
                // a dot: a very short dash along the line
                if t + pat.dot <= l {
                    out.push(p);
                    out.push(lerp(a, b, (t + pat.dot) / l));
                } else {
                    out.push(lerp(a, b, (t - pat.dot).max(0.0) / l));
                    out.push(p);
                }
            } else if d > 0.0 {
                out.push(last);
                out.push(p);
            }
            k = (k + 1) % pat.dashes.len();
            left = pat.dashes[k].abs();
            last = p;
        }
        left -= l - t;
        if pat.dashes[k] > 0.0 {
            out.push(last);
            out.push(b);
        }
        last = b;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(d: &[P2]) -> Vec<(P2, P2)> {
        d.as_chunks::<2>().0.iter().map(|w| (w[0], w[1])).collect()
    }

    #[test]
    fn dashes_along_a_line() {
        let pat = Pattern {
            dashes: vec![2.0, -1.0],
            dot: 0.01,
        };
        let d = dash(&[[0.0, 0.0], [10.0, 0.0]], false, &[], &pat).unwrap();
        let s: Vec<(f64, f64)> = spans(&d).iter().map(|s| (s.0[0], s.1[0])).collect();
        assert_eq!(s, vec![(0.0, 2.0), (3.0, 5.0), (6.0, 8.0), (9.0, 10.0)]);
    }

    #[test]
    fn dashes_turn_corners() {
        let pat = Pattern {
            dashes: vec![3.0, -1.0],
            dot: 0.01,
        };
        let d = dash(&[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0]], false, &[], &pat).unwrap();
        // the first dash bends round the corner: two segments
        assert_eq!(
            spans(&d),
            vec![([0.0, 0.0], [2.0, 0.0]), ([2.0, 0.0], [2.0, 1.0])]
        );
        // restarting at the corner: each side its own pattern, from a dash
        let d = dash(&[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0]], false, &[0, 1], &pat).unwrap();
        assert_eq!(
            spans(&d),
            vec![([0.0, 0.0], [2.0, 0.0]), ([2.0, 0.0], [2.0, 2.0])]
        );
        // closed: the closing side too
        let d = dash(&[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0]], true, &[], &pat).unwrap();
        assert!(spans(&d).contains(&([2.0, 2.0], [0.0, 0.0])));
    }

    #[test]
    fn dots_and_gaps_first() {
        let pat = Pattern {
            dashes: vec![-1.0, 0.0],
            dot: 0.1,
        };
        let d = dash(&[[0.0, 0.0], [3.5, 0.0]], false, &[], &pat).unwrap();
        let at: Vec<f64> = spans(&d).iter().map(|s| s.0[0]).collect();
        assert_eq!(at, vec![1.0, 2.0, 3.0]);
        assert!(spans(&d).iter().all(|s| (s.1[0] - s.0[0] - 0.1).abs() < 1e-12));
    }

    #[test]
    fn too_fine_or_solid() {
        let pat = Pattern {
            dashes: vec![1e-6, -1e-6],
            dot: 0.0,
        };
        assert!(dash(&[[0.0; 2], [10.0, 0.0]], false, &[], &pat).is_none());
        let pat = Pattern {
            dashes: vec![1.0],
            dot: 0.0,
        };
        assert!(dash(&[[0.0; 2], [10.0, 0.0]], false, &[], &pat).is_none());
    }
}

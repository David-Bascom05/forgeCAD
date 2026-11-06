//! One character's shape in em units (cap height 1, origin on the baseline at the pen), and the
//! few things done to glyphs: boxes, placing one in another, arcs cut into chords.

use forge_math::dmath;

/// A character: strokes (single-line fonts) and even-odd filled rings (TrueType), in em units,
/// with the pen's advance.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Glyph {
    pub strokes: Vec<Vec<[f64; 2]>>,
    pub fills: Vec<Vec<[f64; 2]>>,
    pub advance: f64,
}

impl Glyph {
    /// The box round the strokes and rings, or None for an empty glyph (a space).
    pub fn bbox(&self) -> Option<([f64; 2], [f64; 2])> {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for p in self.strokes.iter().chain(&self.fills).flatten() {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo[0] <= hi[0]).then_some((lo, hi))
    }

    /// Adds `g`'s strokes and rings scaled by `s` (x, y) and then moved by `d`. The advance is
    /// left as it is.
    pub fn add(&mut self, g: &Glyph, s: [f64; 2], d: [f64; 2]) {
        let m = |p: &[f64; 2]| [p[0] * s[0] + d[0], p[1] * s[1] + d[1]];
        self.strokes
            .extend(g.strokes.iter().map(|l| l.iter().map(m).collect::<Vec<_>>()));
        self.fills
            .extend(g.fills.iter().map(|l| l.iter().map(m).collect::<Vec<_>>()));
    }

    /// The glyph moved by `d` and scaled by `s` about the origin first.
    pub fn transformed(&self, s: [f64; 2], d: [f64; 2]) -> Glyph {
        let mut g = Glyph {
            advance: self.advance * s[0].abs(),
            ..Glyph::default()
        };
        g.add(self, s, d);
        g
    }
}

/// How many chords keep a circular arc of radius `r` and sweep `sweep` (radians) within `tol`
/// of the curve.
pub fn arc_steps(r: f64, sweep: f64, tol: f64) -> usize {
    let r = r.abs();
    if r <= tol || r == 0.0 {
        return 1;
    }
    // The gap of a chord over angle a is r (1 - cos(a/2)) ~ r a^2 / 8.
    let a = (8.0 * tol / r).sqrt();
    ((sweep.abs() / a).ceil() as usize).clamp(1, 256)
}

/// Points along the arc about `c` of radius `r` from angle `a0` through `sweep` (radians, positive
/// anticlockwise), the start left out.
pub fn arc_points(c: [f64; 2], r: f64, a0: f64, sweep: f64, tol: f64, out: &mut Vec<[f64; 2]>) {
    let n = arc_steps(r, sweep, tol);
    let mut cs = Vec::with_capacity(n);
    dmath::sin_cos_steps(a0 + sweep / n as f64, sweep / n as f64, n, &mut cs);
    out.extend(cs.iter().map(|&(s, k)| [c[0] + r * k, c[1] + r * s]));
    // The end exactly where a single sin_cos puts it, so arcs meet what follows them.
    let (s, k) = dmath::sin_cos(a0 + sweep);
    if let Some(last) = out.last_mut() {
        *last = [c[0] + r * k, c[1] + r * s];
    }
}

/// A closed circle as a polyline.
pub fn circle(c: [f64; 2], r: f64, tol: f64) -> Vec<[f64; 2]> {
    let mut v = vec![[c[0] + r, c[1]]];
    arc_points(c, r, 0.0, dmath::TAU, tol, &mut v);
    let first = v[0];
    if let Some(last) = v.last_mut() {
        *last = first;
    }
    v
}

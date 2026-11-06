//! The geometry the builder needs: 3D affine transforms (an entity's OCS, an insert's placement,
//! a viewport's view), curves cut into chords (arcs and bulges, ellipses, NURBS, fit-point
//! splines), and clipping to a rectangle. Trig from forge-math's dmath only, so every target
//! cuts the same points.

use forge_math::dmath;

pub type P3 = [f64; 3];
pub type P2 = [f64; 2];

pub fn add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn mul(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn len(a: P3) -> f64 {
    dot(a, a).sqrt()
}
pub fn unit(a: P3) -> Option<P3> {
    let l = len(a);
    (l > 1e-300 && l.is_finite()).then(|| mul(a, 1.0 / l))
}

/// The OCS axes for an extrusion (normal), by AutoCAD's arbitrary axis algorithm.
pub fn ocs_axes(n: P3) -> (P3, P3, P3) {
    let az = unit(n).unwrap_or([0.0, 0.0, 1.0]);
    let ax = if az[0].abs() < 1.0 / 64.0 && az[1].abs() < 1.0 / 64.0 {
        cross([0.0, 1.0, 0.0], az)
    } else {
        cross([0.0, 0.0, 1.0], az)
    };
    let ax = unit(ax).unwrap_or([1.0, 0.0, 0.0]);
    let ay = unit(cross(az, ax)).unwrap_or([0.0, 1.0, 0.0]);
    (ax, ay, az)
}

/// An affine map of 3-space: rows of [x y z | t].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat(pub [[f64; 4]; 3]);

impl Mat {
    pub const ID: Mat = Mat([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]]);

    /// Columns: where x, y and z go, and the origin.
    pub fn from_cols(x: P3, y: P3, z: P3, o: P3) -> Mat {
        Mat([
            [x[0], y[0], z[0], o[0]],
            [x[1], y[1], z[1], o[1]],
            [x[2], y[2], z[2], o[2]],
        ])
    }
    /// OCS to WCS for an extrusion.
    pub fn ocs(n: P3) -> Mat {
        if n == [0.0, 0.0, 1.0] {
            return Mat::ID;
        }
        let (x, y, z) = ocs_axes(n);
        Mat::from_cols(x, y, z, [0.0; 3])
    }
    pub fn translate(t: P3) -> Mat {
        let mut m = Mat::ID;
        m.0[0][3] = t[0];
        m.0[1][3] = t[1];
        m.0[2][3] = t[2];
        m
    }
    pub fn scale(s: P3) -> Mat {
        Mat([
            [s[0], 0.0, 0.0, 0.0],
            [0.0, s[1], 0.0, 0.0],
            [0.0, 0.0, s[2], 0.0],
        ])
    }
    /// About z, counter-clockwise.
    pub fn rotate_z(a: f64) -> Mat {
        let (s, c) = dmath::sin_cos(a);
        Mat([[c, -s, 0.0, 0.0], [s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]])
    }
    /// self after `o`: (self * o)(p) = self(o(p)).
    pub fn then_from(&self, o: &Mat) -> Mat {
        let a = &self.0;
        let b = &o.0;
        let mut r = [[0.0; 4]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
                if j == 3 {
                    *v += a[i][3];
                }
            }
        }
        Mat(r)
    }
    pub fn apply(&self, p: P3) -> P3 {
        let m = &self.0;
        [
            m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
            m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
            m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
        ]
    }
    pub fn vec(&self, v: P3) -> P3 {
        let m = &self.0;
        [
            m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
        ]
    }
    pub fn is_id(&self) -> bool {
        *self == Mat::ID
    }
    /// The most any length grows by (the longest column), for choosing chord tolerances.
    pub fn max_scale(&self) -> f64 {
        let m = &self.0;
        (0..3)
            .map(|j| (m[0][j] * m[0][j] + m[1][j] * m[1][j] + m[2][j] * m[2][j]).sqrt())
            .fold(0.0, f64::max)
    }
    /// A typical length factor (the cube root of the volume scale), for line type patterns.
    pub fn mean_scale(&self) -> f64 {
        let m = &self.0;
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        let s = dmath::cbrt(det.abs());
        if s > 1e-12 && s.is_finite() {
            s
        } else {
            self.max_scale()
        }
    }
}

/// Segments for a sweep at a radius so the chords stay within `chord` of the curve: at least
/// `min_full` per full turn (and 1), at most 1024 per turn.
pub fn arc_segments(r: f64, sweep: f64, chord: f64) -> usize {
    let turns = sweep.abs() / dmath::TAU;
    let step = if r > chord && chord > 0.0 {
        2.0 * dmath::acos((1.0 - chord / r).clamp(-1.0, 1.0))
    } else {
        dmath::TAU
    };
    let n = (sweep.abs() / step.max(1e-9)).ceil();
    let lo = (16.0 * turns).ceil().max(1.0);
    let hi = (1024.0 * turns).ceil().max(1.0);
    n.clamp(lo, hi) as usize
}

/// A circular arc in its plane: centre, radius, start angle and signed sweep (counter-clockwise
/// positive); the points from start to end.
pub fn arc_pts(c: P3, r: f64, a0: f64, sweep: f64, chord: f64, out: &mut Vec<P3>) {
    let n = arc_segments(r, sweep, chord);
    let mut sc = Vec::new();
    dmath::sin_cos_steps(a0, sweep / n as f64, n + 1, &mut sc);
    for (s, co) in sc {
        out.push([c[0] + r * co, c[1] + r * s, c[2]]);
    }
}

/// The counter-clockwise sweep from a0 to a1, in (0, 2pi] (a full turn when they meet).
pub fn ccw_sweep(a0: f64, a1: f64) -> f64 {
    let s = dmath::wrap_tau(a1 - a0);
    if s <= 1e-12 { dmath::TAU } else { s }
}

/// A polyline segment from p0 to p1 with a bulge: the points after p0, up to and including p1.
pub fn bulge_pts(p0: P3, p1: P3, bulge: f64, chord: f64, out: &mut Vec<P3>) {
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let d = (dx * dx + dy * dy).sqrt();
    if bulge.abs() < 1e-10 || d < 1e-300 || !bulge.is_finite() {
        out.push(p1);
        return;
    }
    // sagitta s, signed radius r; the arc's middle is right of p0->p1 for a positive bulge
    let s = bulge * d / 2.0;
    let r = (d * d / 4.0 + s * s) / (2.0 * s);
    let rn = [dy / d, -dx / d];
    let m = [(p0[0] + p1[0]) / 2.0, (p0[1] + p1[1]) / 2.0];
    let am = [m[0] + rn[0] * s, m[1] + rn[1] * s];
    let c = [am[0] - rn[0] * r, am[1] - rn[1] * r];
    let a0 = dmath::atan2(p0[1] - c[1], p0[0] - c[0]);
    let sweep = 4.0 * dmath::atan(bulge);
    let n = arc_segments(r.abs(), sweep, chord);
    let mut sc = Vec::new();
    dmath::sin_cos_steps(a0, sweep / n as f64, n + 1, &mut sc);
    let ra = r.abs();
    for (k, (sn, co)) in sc.into_iter().enumerate().skip(1) {
        if k == n {
            out.push(p1);
        } else {
            let z = p0[2] + (p1[2] - p0[2]) * k as f64 / n as f64;
            out.push([c[0] + ra * co, c[1] + ra * sn, z]);
        }
    }
}

/// A bulged polyline's points (vertices x, y with their bulges), closing back to the first when
/// `closed`; also each vertex's index in the output.
pub fn bulged(v: &[(P3, f64)], closed: bool, chord: f64) -> (Vec<P3>, Vec<usize>) {
    let mut out = Vec::with_capacity(v.len());
    let mut at = Vec::with_capacity(v.len() + 1);
    if v.is_empty() {
        return (out, at);
    }
    out.push(v[0].0);
    at.push(0);
    let n = v.len();
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs {
        let (p0, b) = v[i];
        let p1 = v[(i + 1) % n].0;
        bulge_pts(p0, p1, b, chord, &mut out);
        at.push(out.len() - 1);
    }
    if closed {
        // the last point is the first again
        out.pop();
        at.pop();
    }
    (out, at)
}

/// An ellipse's points from parameter t0 to t1 (counter-clockwise, t1 > t0): c + cos t * major
/// + sin t * minor.
pub fn ellipse_pts(c: P3, major: P3, minor: P3, t0: f64, t1: f64, chord: f64, out: &mut Vec<P3>) {
    let r = len(major).max(len(minor));
    let n = arc_segments(r, t1 - t0, chord);
    let mut sc = Vec::new();
    dmath::sin_cos_steps(t0, (t1 - t0) / n as f64, n + 1, &mut sc);
    for (s, co) in sc {
        out.push(add(c, add(mul(major, co), mul(minor, s))));
    }
}

/// A NURBS curve: degree, knots, control points and (optional) weights, cut into chords.
pub fn nurbs_pts(degree: usize, knots: &[f64], ctrl: &[P3], weights: &[f64], chord: f64) -> Vec<P3> {
    let n = ctrl.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![ctrl[0]];
    }
    let p = degree.clamp(1, n - 1);
    // a knot vector that fits, else a clamped uniform one
    let k: std::borrow::Cow<[f64]> = if knots.len() == n + p + 1 && knots.windows(2).all(|w| w[1] >= w[0]) {
        std::borrow::Cow::Borrowed(knots)
    } else {
        let inner = n - p;
        (0..n + p + 1)
            .map(|i| {
                if i <= p {
                    0.0
                } else if i >= n {
                    1.0
                } else {
                    (i - p) as f64 / inner as f64
                }
            })
            .collect::<Vec<f64>>()
            .into()
    };
    let rational = weights.len() == n && weights.iter().any(|&x| x != 1.0);
    let wt = |i: usize| {
        if rational && weights[i] > 0.0 {
            weights[i]
        } else {
            1.0
        }
    };
    let (lo, hi) = (k[p], k[n]);
    if hi <= lo {
        return ctrl.to_vec();
    }
    let mut out = Vec::with_capacity(n * 2);
    let mut tmp = vec![[0.0; 4]; p + 1];
    let mut inv_den: Vec<(f64, f64)> = Vec::with_capacity(p * (p + 1) / 2);
    for span in p..n {
        let (a, b) = (k[span], k[span + 1]);
        if b <= a {
            continue;
        }
        // chords per span from how far the span's control points stray from their chord
        let cp = &ctrl[span - p..=span];
        let (f, l) = (cp[0], cp[p]);
        let base = sub(l, f);
        let bl = len(base);
        let mut h: f64 = 0.0;
        for q in cp {
            let v = sub(*q, f);
            let off = if bl > 1e-300 {
                len(sub(v, mul(base, dot(v, base) / (bl * bl))))
            } else {
                len(v)
            };
            h = h.max(off);
        }
        // a span's curve strays from its chord by at most half (a quadratic) or three quarters
        // (higher degrees) of h, and by a quarter as much each time the span is halved
        let dev = if p == 2 { 0.5 * h } else { 0.75 * h };
        let segs = if p == 1 {
            1
        } else if chord > 0.0 {
            ((dev / chord).sqrt().ceil() as usize).clamp(1, 256)
        } else {
            8
        };
        // the span's de Boor steps: each blend's knot and reciprocal knot span
        inv_den.clear();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = span - p + j;
                let den = k[i + p + 1 - r] - k[i];
                inv_den.push((k[i], if den.abs() < 1e-300 { 0.0 } else { 1.0 / den }));
            }
        }
        let first = out.is_empty();
        for s in 0..=segs {
            if s == 0 && !first {
                continue;
            }
            let t = a + (b - a) * s as f64 / segs as f64;
            // de Boor in homogeneous coordinates
            let tmp = &mut tmp[..=p];
            for (j, slot) in tmp.iter_mut().enumerate() {
                let i = span - p + j;
                let c = ctrl[i];
                let w = wt(i);
                *slot = [c[0] * w, c[1] * w, c[2] * w, w];
            }
            let mut q = 0;
            for r in 1..=p {
                for j in (r..=p).rev() {
                    let (k0, inv) = inv_den[q];
                    q += 1;
                    let al = (t - k0) * inv;
                    let (lo, hi) = tmp.split_at_mut(j);
                    let (a, b) = (&lo[j - 1], &mut hi[0]);
                    for c in 0..4 {
                        b[c] = a[c] + al * (b[c] - a[c]);
                    }
                }
            }
            let h = tmp[p];
            if rational {
                let ww = if h[3].abs() < 1e-300 { 1.0 } else { h[3] };
                out.push([h[0] / ww, h[1] / ww, h[2] / ww]);
            } else {
                out.push([h[0], h[1], h[2]]);
            }
        }
    }
    out
}

/// The cubic through points, curvature continuous, as AutoCAD fits a spline to fit points: its
/// knots at the points' parameters spaced by `mode` (0 chord length, 1 square root of it, 2
/// uniform), the end tangents when given (natural ends otherwise); cut into chords.
pub fn interp_pts(pts: &[P3], t0: Option<P3>, t1: Option<P3>, mode: u8, chord: f64) -> Vec<P3> {
    // drop repeated points
    let mut p: Vec<P3> = Vec::with_capacity(pts.len());
    for q in pts {
        if p.last().is_none_or(|l| len(sub(*q, *l)) > 1e-12) {
            p.push(*q);
        }
    }
    let n = p.len();
    if n < 3 && t0.is_none() && t1.is_none() {
        return p;
    }
    if n < 2 {
        return p;
    }
    let h: Vec<f64> = (0..n - 1)
        .map(|i| {
            let l = len(sub(p[i + 1], p[i]));
            match mode {
                1 => l.sqrt(),
                2 => 1.0,
                _ => l,
            }
        })
        .collect();
    // a unit tangent as a slope in the parameter
    let rate = |i: usize| len(sub(p[i + 1], p[i])) / h[i];
    // tridiagonal system for the second derivatives M
    let mut a = vec![0.0; n];
    let mut b = vec![0.0; n];
    let mut c = vec![0.0; n];
    let mut d = vec![[0.0; 3]; n];
    let slope = |i: usize| mul(sub(p[i + 1], p[i]), 1.0 / h[i]);
    match t0.and_then(unit) {
        Some(t) => {
            let t = mul(t, rate(0));
            b[0] = 2.0 * h[0];
            c[0] = h[0];
            d[0] = mul(sub(slope(0), t), 6.0);
        }
        None => b[0] = 1.0,
    }
    for i in 1..n - 1 {
        a[i] = h[i - 1];
        b[i] = 2.0 * (h[i - 1] + h[i]);
        c[i] = h[i];
        d[i] = mul(sub(slope(i), slope(i - 1)), 6.0);
    }
    match t1.and_then(unit) {
        Some(t) => {
            let t = mul(t, rate(n - 2));
            a[n - 1] = h[n - 2];
            b[n - 1] = 2.0 * h[n - 2];
            d[n - 1] = mul(sub(t, slope(n - 2)), 6.0);
        }
        None => b[n - 1] = 1.0,
    }
    // Thomas
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m * c[i - 1];
        d[i] = sub(d[i], mul(d[i - 1], m));
    }
    let mut mm = vec![[0.0; 3]; n];
    mm[n - 1] = mul(d[n - 1], 1.0 / b[n - 1]);
    for i in (0..n - 1).rev() {
        mm[i] = mul(sub(d[i], mul(mm[i + 1], c[i])), 1.0 / b[i]);
    }
    let mut out = vec![p[0]];
    for i in 0..n - 1 {
        let hi = h[i];
        // how far the piece bows from its chord, roughly
        let bow = (len(mm[i]) + len(mm[i + 1])) * hi * hi / 16.0;
        let segs = if chord > 0.0 {
            ((bow / chord).sqrt().ceil() as usize).clamp(1, 128)
        } else {
            4
        };
        for s in 1..=segs {
            if s == segs {
                out.push(p[i + 1]);
                continue;
            }
            let u = hi * s as f64 / segs as f64;
            let v = hi - u;
            let mut q = [0.0; 3];
            for k in 0..3 {
                q[k] = mm[i][k] * v * v * v / (6.0 * hi)
                    + mm[i + 1][k] * u * u * u / (6.0 * hi)
                    + (p[i][k] / hi - mm[i][k] * hi / 6.0) * v
                    + (p[i + 1][k] / hi - mm[i + 1][k] * hi / 6.0) * u;
            }
            out.push(q);
        }
    }
    out
}

/// An axis-aligned rectangle (min, max).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub lo: P2,
    pub hi: P2,
}

impl Rect {
    pub fn contains(&self, p: P2) -> bool {
        p[0] >= self.lo[0] && p[0] <= self.hi[0] && p[1] >= self.lo[1] && p[1] <= self.hi[1]
    }
}

/// The parts of a polyline inside a rectangle (Liang-Barsky per segment, joined where they
/// meet).
pub fn clip_path(pts: &[P2], closed: bool, r: &Rect) -> Vec<Vec<P2>> {
    let mut out: Vec<Vec<P2>> = vec![];
    if pts.iter().all(|p| r.contains(*p)) {
        let mut v = pts.to_vec();
        if closed && v.len() > 2 {
            v.push(v[0]);
        }
        return vec![v];
    }
    let n = pts.len();
    if n == 1 {
        if r.contains(pts[0]) {
            out.push(pts.to_vec());
        }
        return out;
    }
    let segs = if closed { n } else { n - 1 };
    let mut cur: Vec<P2> = vec![];
    for i in 0..segs {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let Some((t0, t1)) = clip_seg(a, b, r) else {
            if cur.len() > 1 {
                out.push(std::mem::take(&mut cur));
            }
            cur.clear();
            continue;
        };
        let pa = [a[0] + (b[0] - a[0]) * t0, a[1] + (b[1] - a[1]) * t0];
        let pb = [a[0] + (b[0] - a[0]) * t1, a[1] + (b[1] - a[1]) * t1];
        if t0 > 0.0 || cur.is_empty() {
            if cur.len() > 1 {
                out.push(std::mem::take(&mut cur));
            }
            cur = vec![pa];
        }
        cur.push(pb);
        if t1 < 1.0 {
            out.push(std::mem::take(&mut cur));
        }
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

/// The parameter range of a segment inside a rectangle.
pub fn clip_seg(a: P2, b: P2, r: &Rect) -> Option<(f64, f64)> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for k in 0..2 {
        if d[k] == 0.0 {
            if a[k] < r.lo[k] || a[k] > r.hi[k] {
                return None;
            }
            continue;
        }
        let mut u = (r.lo[k] - a[k]) / d[k];
        let mut v = (r.hi[k] - a[k]) / d[k];
        if u > v {
            std::mem::swap(&mut u, &mut v);
        }
        t0 = t0.max(u);
        t1 = t1.min(v);
        if t0 > t1 {
            return None;
        }
    }
    Some((t0, t1))
}

/// A ring cut to a rectangle (Sutherland-Hodgman); None when nothing is left.
pub fn clip_ring(ring: &[P2], r: &Rect) -> Option<Vec<P2>> {
    if ring.iter().all(|p| r.contains(*p)) {
        return Some(ring.to_vec());
    }
    let mut cur = ring.to_vec();
    for (k, hi) in [(0, false), (0, true), (1, false), (1, true)] {
        if cur.is_empty() {
            return None;
        }
        let lim = if hi { r.hi[k] } else { r.lo[k] };
        let inside = |p: &P2| if hi { p[k] <= lim } else { p[k] >= lim };
        let mut next = Vec::with_capacity(cur.len());
        for i in 0..cur.len() {
            let a = cur[i];
            let b = cur[(i + 1) % cur.len()];
            let (ia, ib) = (inside(&a), inside(&b));
            if ia {
                next.push(a);
            }
            if ia != ib {
                let t = (lim - a[k]) / (b[k] - a[k]);
                next.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
        cur = next;
    }
    (cur.len() >= 3).then_some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: P3, b: P3, tol: f64) -> bool {
        len(sub(a, b)) < tol
    }

    #[test]
    fn bulge_semicircle() {
        let mut out = vec![[0.0, 0.0, 0.0]];
        bulge_pts([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 1.0, 1e-3, &mut out);
        assert!(near(*out.last().unwrap(), [2.0, 0.0, 0.0], 1e-12));
        // counter-clockwise from (0,0) to (2,0) about (1,0): through (1,-1)
        for p in &out {
            assert!((len(sub(*p, [1.0, 0.0, 0.0])) - 1.0).abs() < 1e-9);
            assert!(p[1] <= 1e-9);
        }
        let mid = out.iter().map(|p| p[1]).fold(0.0, f64::min);
        assert!((mid + 1.0).abs() < 1e-3);
        // a negative bulge bows the other way
        let mut out = vec![];
        bulge_pts([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], -1.0, 1e-3, &mut out);
        assert!(out.iter().all(|p| p[1] >= -1e-9));
        // a quarter circle: bulge tan(pi/8)
        let mut out = vec![];
        bulge_pts(
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            dmath::sin(dmath::PI / 8.0) / dmath::cos(dmath::PI / 8.0),
            1e-4,
            &mut out,
        );
        for p in &out {
            assert!((len(*p) - 1.0).abs() < 1e-9, "{p:?}");
        }
    }

    #[test]
    fn bulged_closed_polyline() {
        let v = [
            ([0.0, 0.0, 0.0], 0.0),
            ([1.0, 0.0, 0.0], 1.0),
            ([1.0, 1.0, 0.0], 0.0),
        ];
        let (pts, at) = bulged(&v, true, 1e-2);
        assert_eq!(at[0], 0);
        assert_eq!(pts[at[1]], [1.0, 0.0, 0.0]);
        assert_eq!(pts[at[2]], [1.0, 1.0, 0.0]);
        assert_eq!(at.len(), 3);
        assert!(pts.len() > 5);
    }

    #[test]
    fn arbitrary_axis() {
        // the usual plane
        let (x, y, z) = ocs_axes([0.0, 0.0, 1.0]);
        assert_eq!((x, y, z), ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]));
        // mirrored: x flips, y stays
        let (x, y, _) = ocs_axes([0.0, 0.0, -1.0]);
        assert!(near(x, [-1.0, 0.0, 0.0], 1e-15) && near(y, [0.0, 1.0, 0.0], 1e-15));
        // a wall facing +x: Wz x N
        let (x, y, _) = ocs_axes([1.0, 0.0, 0.0]);
        assert!(near(x, [0.0, 1.0, 0.0], 1e-15) && near(y, [0.0, 0.0, 1.0], 1e-15));
        let m = Mat::ocs([0.0, 0.0, -1.0]);
        assert!(near(m.apply([2.0, 3.0, 1.0]), [-2.0, 3.0, -1.0], 1e-15));
    }

    #[test]
    fn transforms_compose() {
        // insert at (10,0), rotated a quarter turn, scaled 2, block base (1,1)
        let m = Mat::translate([10.0, 0.0, 0.0])
            .then_from(&Mat::rotate_z(dmath::FRAC_PI_2))
            .then_from(&Mat::scale([2.0, 2.0, 2.0]))
            .then_from(&Mat::translate([-1.0, -1.0, 0.0]));
        assert!(near(m.apply([1.0, 1.0, 0.0]), [10.0, 0.0, 0.0], 1e-12));
        assert!(near(m.apply([2.0, 1.0, 0.0]), [10.0, 2.0, 0.0], 1e-12));
        assert!((m.max_scale() - 2.0).abs() < 1e-12);
        assert!((m.mean_scale() - 2.0).abs() < 1e-12);
        // nested: an outer insert of that
        let outer = Mat::translate([0.0, 5.0, 0.0]).then_from(&Mat::scale([0.5, 0.5, 0.5]));
        let both = outer.then_from(&m);
        assert!(near(
            both.apply([2.0, 1.0, 0.0]),
            outer.apply(m.apply([2.0, 1.0, 0.0])),
            1e-12
        ));
        assert!(near(both.apply([2.0, 1.0, 0.0]), [5.0, 6.0, 0.0], 1e-12));
    }

    #[test]
    fn nurbs_quarter_circle() {
        // the rational quadratic quarter circle
        let w = dmath::sin(dmath::PI / 4.0);
        let pts = nurbs_pts(
            2,
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            &[[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            &[1.0, w, 1.0],
            1e-4,
        );
        assert!(pts.len() > 4);
        assert!(near(pts[0], [1.0, 0.0, 0.0], 1e-12));
        assert!(near(*pts.last().unwrap(), [0.0, 1.0, 0.0], 1e-12));
        for p in &pts {
            assert!((len(*p) - 1.0).abs() < 1e-12, "{p:?}");
        }
        // a cubic Bezier's middle
        let pts = nurbs_pts(
            3,
            &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            &[[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            &[],
            1e-3,
        );
        let m = pts.len() / 2;
        assert!(pts.len() % 2 == 1 && near(pts[m], [0.5, 0.75, 0.0], 1e-12));
    }

    #[test]
    fn interpolation_passes_through() {
        let p = [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0], [3.0, 1.0, 0.0]];
        let out = interp_pts(&p, None, None, 0, 1e-3);
        for q in &p {
            assert!(out.iter().any(|o| near(*o, *q, 1e-12)));
        }
        assert!(out.len() > 8);
        // a straight run stays straight
        let out = interp_pts(&[[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]], None, None, 0, 1e-3);
        assert!(out.iter().all(|q| q[1].abs() < 1e-12));
    }

    #[test]
    fn clipping() {
        let r = Rect {
            lo: [0.0, 0.0],
            hi: [1.0, 1.0],
        };
        let parts = clip_path(&[[-1.0, 0.5], [2.0, 0.5]], false, &r);
        assert_eq!(parts, vec![vec![[0.0, 0.5], [1.0, 0.5]]]);
        // in, out, in again: two pieces
        let parts = clip_path(&[[0.5, 0.5], [0.5, 2.0], [0.6, 0.5]], false, &r);
        assert_eq!(parts.len(), 2);
        let ring = clip_ring(&[[-1.0, -1.0], [2.0, -1.0], [2.0, 2.0], [-1.0, 2.0]], &r).unwrap();
        assert_eq!(ring.len(), 4);
        assert!(ring.iter().all(|p| r.contains(*p)));
        assert!(clip_ring(&[[2.0, 2.0], [3.0, 2.0], [3.0, 3.0]], &r).is_none());
    }
}

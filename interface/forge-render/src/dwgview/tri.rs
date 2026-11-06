//! Even-odd filled rings as triangles, by a sweep in y: the plane is cut into slabs at every
//! vertex and every crossing of two edges, and in a slab the edges that span it, taken left to
//! right in pairs, bound the filled trapezoids. Holes, islands, rings that touch or cross
//! themselves and each other all come out as the even-odd rule has them; nothing is assumed of
//! the rings' direction or nesting.

/// An edge spanning `lo.y < hi.y`.
#[derive(Clone, Copy, Debug)]
struct Edge {
    lo: [f64; 2],
    hi: [f64; 2],
}

impl Edge {
    fn x_at(&self, y: f64) -> f64 {
        let t = (y - self.lo[1]) / (self.hi[1] - self.lo[1]);
        self.lo[0] + (self.hi[0] - self.lo[0]) * t
    }
}

/// The rings' even-odd area as triangles. Coordinates go through as they are; rings need no
/// closing point (one is ignored).
pub fn even_odd(rings: &[Vec<[f64; 2]>]) -> Vec<[[f64; 2]; 3]> {
    let mut edges = vec![];
    let mut ys = vec![];
    for ring in rings {
        let mut r: &[[f64; 2]] = ring;
        if r.len() > 1 && r[0] == r[r.len() - 1] {
            r = &r[..r.len() - 1];
        }
        if r.len() < 3 || r.iter().any(|p| !p[0].is_finite() || !p[1].is_finite()) {
            continue;
        }
        for k in 0..r.len() {
            let (a, b) = (r[k], r[(k + 1) % r.len()]);
            ys.push(a[1]);
            if a[1] != b[1] {
                let (lo, hi) = if a[1] < b[1] { (a, b) } else { (b, a) };
                edges.push(Edge { lo, hi });
            }
        }
    }
    let mut out = vec![];
    if edges.len() < 2 {
        return out;
    }
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    edges.sort_by(|a, b| a.lo[1].total_cmp(&b.lo[1]));
    let span = ys[ys.len() - 1] - ys[0];
    // crossings closer than this to a slab's edge are taken as at it
    let eps = span * 1e-12;
    let mut next = 0;
    let mut active: Vec<Edge> = vec![];
    for w in ys.windows(2) {
        let (y0, y1) = (w[0], w[1]);
        active.retain(|e| e.hi[1] > y0);
        while next < edges.len() && edges[next].lo[1] <= y0 {
            if edges[next].hi[1] > y0 {
                active.push(edges[next]);
            }
            next += 1;
        }
        slab(&active, y0, y1, eps, &mut out);
    }
    out
}

/// The filled trapezoids between y0 and y1, cut again at each crossing inside.
fn slab(active: &[Edge], mut y0: f64, y1: f64, eps: f64, out: &mut Vec<[[f64; 2]; 3]>) {
    let mut xs: Vec<(f64, f64)> = Vec::with_capacity(active.len());
    // a slab can't need more cuts than there are pairs to cross; past that, rounding is at work
    let mut cuts = active.len() * active.len() + 4;
    loop {
        xs.clear();
        xs.extend(active.iter().map(|e| (e.x_at(y0), e.x_at(y1))));
        xs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        // the first crossing above y0 is between two edges side by side at y0
        let mut yc = y1;
        for p in xs.windows(2) {
            let (a, b) = (p[0], p[1]);
            if a.1 > b.1 {
                // a - b goes from <= 0 at y0 to > 0 at y1
                let d0 = b.0 - a.0;
                let d1 = a.1 - b.1;
                let t = d0 / (d0 + d1);
                let y = y0 + (y1 - y0) * t;
                if y > y0 + eps && y < yc {
                    yc = y;
                }
            }
        }
        if yc < y1 && cuts > 0 {
            cuts -= 1;
            trapezoids(active, y0, yc, out);
            y0 = yc;
            continue;
        }
        trapezoids(active, y0, y1, out);
        return;
    }
}

fn trapezoids(active: &[Edge], y0: f64, y1: f64, out: &mut Vec<[[f64; 2]; 3]>) {
    let ym = 0.5 * (y0 + y1);
    let mut xs: Vec<(f64, f64, f64)> = active
        .iter()
        .map(|e| (e.x_at(ym), e.x_at(y0), e.x_at(y1)))
        .collect();
    xs.sort_by(|a, b| a.0.total_cmp(&b.0));
    for p in xs.as_chunks::<2>().0 {
        let (l, r) = (p[0], p[1]);
        let (a, b, c, d) = ([l.1, y0], [r.1, y0], [r.2, y1], [l.2, y1]);
        if r.1 > l.1 {
            out.push([a, b, c]);
        }
        if r.2 > l.2 {
            out.push([a, c, d]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(t: &[[[f64; 2]; 3]]) -> f64 {
        t.iter()
            .map(|[a, b, c]| ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() * 0.5)
            .sum()
    }

    fn square(x: f64, y: f64, s: f64) -> Vec<[f64; 2]> {
        vec![[x, y], [x + s, y], [x + s, y + s], [x, y + s]]
    }

    #[test]
    fn a_square_with_a_hole() {
        let t = even_odd(&[square(0.0, 0.0, 10.0), square(2.0, 2.0, 3.0)]);
        assert!((area(&t) - 91.0).abs() < 1e-9);
    }

    #[test]
    fn a_closing_point_and_either_direction() {
        let mut a = square(0.0, 0.0, 4.0);
        a.push(a[0]);
        let mut b = square(1.0, 1.0, 1.0);
        b.reverse();
        assert!((area(&even_odd(&[a, b])) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn a_bow_tie_crosses_itself() {
        // two triangles meeting at (1, 1)
        let t = even_odd(&[vec![[0.0, 0.0], [2.0, 2.0], [2.0, 0.0], [0.0, 2.0]]]);
        assert!((area(&t) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn overlapping_rings_cancel_where_they_overlap() {
        let t = even_odd(&[square(0.0, 0.0, 2.0), square(1.0, 1.0, 2.0)]);
        assert!((area(&t) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn a_circle() {
        let n = 64;
        let ring: Vec<[f64; 2]> = (0..n)
            .map(|k| {
                let a = k as f64 / n as f64 * std::f64::consts::TAU;
                let (s, c) = forge_math::dmath::sin_cos(a);
                [c * 5.0 + 100.0, s * 5.0 - 7.0]
            })
            .collect();
        let want = 0.5 * n as f64 * 25.0 * forge_math::dmath::sin_cos(std::f64::consts::TAU / n as f64).0;
        assert!((area(&even_odd(&[ring])) - want).abs() < 1e-9);
    }

    #[test]
    fn nothing_from_too_little() {
        assert!(even_odd(&[vec![[0.0, 0.0], [1.0, 1.0]]]).is_empty());
        assert!(even_odd(&[vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]]).is_empty());
        assert!(even_odd(&[vec![[0.0, 0.0], [f64::NAN, 1.0], [2.0, 0.0]]]).is_empty());
    }
}

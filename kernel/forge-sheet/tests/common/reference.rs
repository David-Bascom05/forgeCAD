//! The independent area of wall minus cuts, by exact scanlines of the input loops only: on each
//! row each loop gives its crossings in closed form (splines from a dense polyline) and the row's
//! length is the set difference of their intervals. Shares no code with either option.
use forge_math::{Point2, dmath};
use forge_ops::{ProfileLoop, Segment};

enum Mono {
    Line(Point2, Point2),
    /// A polyline monotone in y, read by binary search (a spline, finely).
    Run(Vec<Point2>),
    /// The half of the circle on one side of its centre: x = cx + sign sqrt(r^2 - dy^2), between y0 and y1.
    Half {
        c: Point2,
        r: f64,
        sign: f64,
        y0: f64,
        y1: f64,
    },
}

/// Rows run at this angle to the part: an edge level with the rows would make the row length jump
/// (the rule's error then first order in the spacing), and sketches are full of those.
const TURN: f64 = 0.37;

fn turned(p: Point2) -> Point2 {
    let (s, k) = dmath::sin_cos(TURN);
    Point2::new(k * p.x - s * p.y, s * p.x + k * p.y)
}

fn monos(l: &ProfileLoop, out: &mut Vec<Mono>) {
    let half = dmath::PI;
    for (s, _) in &l.segments {
        match s {
            Segment::Line { a, b } => out.push(Mono::Line(turned(*a), turned(*b))),
            Segment::Circle { center, radius, .. } => {
                let center = &turned(*center);
                for sign in [1.0, -1.0] {
                    out.push(Mono::Half {
                        c: *center,
                        r: *radius,
                        sign,
                        y0: center.y - radius,
                        y1: center.y + radius,
                    });
                }
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let center = &turned(*center);
                let (lo, hi) = (TURN + start.min(start + sweep), TURN + start.max(start + sweep));
                let mut a = lo;
                let mut k = ((lo - half / 2.0) / half).ceil();
                loop {
                    let b = (k * half + half / 2.0).min(hi);
                    if b > a {
                        let y = |t: f64| center.y + radius * dmath::sin(t);
                        let (ya, yb) = (y(a), y(b));
                        out.push(Mono::Half {
                            c: *center,
                            r: *radius,
                            sign: dmath::cos(0.5 * (a + b)).signum(),
                            y0: ya.min(yb),
                            y1: ya.max(yb),
                        });
                    }
                    if b >= hi {
                        break;
                    }
                    a = b;
                    k += 1.0;
                }
            }
            Segment::Spline(ps) => {
                let d = ps.c.domain();
                let n = 4000 * ps.c.pieces();
                let at = |i: usize| {
                    let q = ps.c.point(d.lo + d.width() * i as f64 / n as f64);
                    turned(Point2::new(q.x, q.y))
                };
                let mut run = vec![at(0)];
                for i in 1..=n {
                    let q = at(i);
                    let last = run[run.len() - 1];
                    if run.len() > 1 && (q.y > last.y) != (last.y > run[run.len() - 2].y) {
                        out.push(Mono::Run(std::mem::replace(&mut run, vec![last])));
                    }
                    run.push(q);
                }
                out.push(Mono::Run(run));
            }
        }
    }
}

struct Group {
    m: Vec<Mono>,
    y0: f64,
    y1: f64,
}

fn group(l: &[ProfileLoop]) -> Group {
    let mut m = Vec::new();
    for x in l {
        monos(x, &mut m);
    }
    let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
    for q in &m {
        let (a, b) = match *q {
            Mono::Line(a, b) => (a.y.min(b.y), a.y.max(b.y)),
            Mono::Half { y0, y1, .. } => (y0, y1),
            Mono::Run(ref r) => {
                let (a, b) = (r[0].y, r[r.len() - 1].y);
                (a.min(b), a.max(b))
            }
        };
        y0 = y0.min(a);
        y1 = y1.max(b);
    }
    Group { m, y0, y1 }
}

fn crossings(g: &Group, y: f64, xs: &mut Vec<f64>) {
    xs.clear();
    for q in &g.m {
        match *q {
            Mono::Run(ref r) => {
                let (a, b) = (r[0], r[r.len() - 1]);
                if (a.y > y) != (b.y > y) {
                    let up = b.y > a.y;
                    let i = r.partition_point(|q| (q.y > y) != up);
                    let (s, e) = (r[i - 1], r[i]);
                    xs.push(s.x + (e.x - s.x) * (y - s.y) / (e.y - s.y));
                }
            }
            Mono::Line(a, b) => {
                if (a.y > y) != (b.y > y) {
                    xs.push(a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y));
                }
            }
            Mono::Half { c, r, sign, y0, y1 } => {
                if y > y0 && y <= y1 {
                    let d = y - c.y;
                    xs.push(c.x + sign * (r * r - d * d).max(0.0).sqrt());
                }
            }
        }
    }
    xs.sort_by(f64::total_cmp);
}

/// The area and an estimate of its error, from rows of two spacings.
pub fn area(wall: &[ProfileLoop], cuts: &[ProfileLoop], rows: usize) -> (f64, f64) {
    let w = group(wall);
    let gs: Vec<Group> = cuts.iter().map(|c| group(std::slice::from_ref(c))).collect();
    let h = (w.y1 - w.y0) / rows as f64;
    // which cuts span each row
    let mut starts: Vec<(usize, usize)> = gs
        .iter()
        .enumerate()
        .map(|(i, g)| ((((g.y0 - w.y0) / h).max(0.0)) as usize, i))
        .collect();
    starts.sort();
    let mut next = 0;
    let mut active: Vec<usize> = Vec::new();
    let (mut sum, mut coarse) = (0.0, 0.0);
    let mut xs = Vec::new();
    let mut spans: Vec<(f64, f64)> = Vec::new();
    for r in 0..rows {
        let y = w.y0 + (r as f64 + 0.5) * h;
        while next < starts.len() && starts[next].0 <= r {
            active.push(starts[next].1);
            next += 1;
        }
        active.retain(|&i| gs[i].y1 >= y);
        crossings(&w, y, &mut xs);
        let walls: Vec<(f64, f64)> = xs.as_chunks::<2>().0.iter().map(|c| (c[0], c[1])).collect();
        spans.clear();
        for &i in &active {
            crossings(&gs[i], y, &mut xs);
            spans.extend(xs.as_chunks::<2>().0.iter().map(|c| (c[0], c[1])));
        }
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut len = 0.0;
        for &(a, b) in &walls {
            let mut x = a;
            for &(c, d) in &spans {
                if d <= x {
                    continue;
                }
                if c >= b {
                    break;
                }
                if c > x {
                    len += c - x;
                }
                x = x.max(d);
            }
            if x < b {
                len += b - x;
            }
        }
        sum += len;
        if r % 3 == 1 {
            coarse += len;
        }
    }
    let (fine, coarse) = (sum * h, coarse * 3.0 * h);
    (fine, (fine - coarse).abs() / 8.0)
}

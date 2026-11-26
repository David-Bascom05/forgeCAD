//! Integer nanometre geometry. The coordinate bound in the reader keeps the orientation
//! determinant well inside i128 even when two points lie at opposite limits.
pub type Point = [i64; 2];
pub type Segment = [Point; 2];

fn turn(a: Point, b: Point, c: Point) -> i128 {
    i128::from(b[0] - a[0]) * i128::from(c[1] - a[1]) - i128::from(b[1] - a[1]) * i128::from(c[0] - a[0])
}

fn on(a: Point, b: Point, p: Point) -> bool {
    turn(a, b, p) == 0 && (0..2).all(|k| (a[k].min(b[k])..=a[k].max(b[k])).contains(&p[k]))
}

fn intersects([a, b]: Segment, [c, d]: Segment) -> bool {
    on(a, b, c)
        || on(a, b, d)
        || on(c, d, a)
        || on(c, d, b)
        || (turn(a, b, c).signum() * turn(a, b, d).signum() < 0
            && turn(c, d, a).signum() * turn(c, d, b).signum() < 0)
}

pub fn sides(points: &[Point]) -> impl Iterator<Item = Segment> + Clone + '_ {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(&a, &b)| [a, b])
}

/// A simple closed straight-sided loop, including concave outlines. Adjacent collinear
/// sides can continue in the same direction, but must never double back over each other.
pub fn simple(points: &[Point]) -> bool {
    if points.len() < 3 {
        return false;
    }
    for (i, [a, b]) in sides(points).enumerate() {
        let c = points[(i + 2) % points.len()];
        if a == b || on(a, b, c) || on(b, c, a) {
            return false;
        }
        for (j, other) in sides(points).enumerate().skip(i + 2) {
            if !(i == 0 && j + 1 == points.len()) && intersects([a, b], other) {
                return false;
            }
        }
    }
    true
}

fn inside(points: &[Point], p: Point) -> bool {
    let mut inside = false;
    for [a, b] in sides(points) {
        if (a[1] > p[1]) != (b[1] > p[1]) && (turn(a, b, p) > 0) == (b[1] > a[1]) {
            inside = !inside;
        }
    }
    inside
}

/// Whether the distance from a point to a segment is strictly less than half the
/// diameter. The ±500 mm coordinate limit bounds 4 * determinant² within i128.
fn too_close(p: Point, [a, b]: Segment, diameter: i64) -> bool {
    let vec = |from: Point, to: Point| [i128::from(to[0] - from[0]), i128::from(to[1] - from[1])];
    let dot = |a: [i128; 2], b: [i128; 2]| a[0] * b[0] + a[1] * b[1];
    let ab = vec(a, b);
    let ap = vec(a, p);
    let along = dot(ap, ab);
    let len2 = dot(ab, ab);
    let d2 = i128::from(diameter).pow(2);
    if along <= 0 {
        return 4 * dot(ap, ap) < d2;
    }
    if along >= len2 {
        let bp = vec(b, p);
        return 4 * dot(bp, bp) < d2;
    }
    4 * turn(a, b, p).pow(2) < d2 * len2
}

pub(super) fn segments_too_close(first: Segment, second: Segment, diameter: i64) -> bool {
    intersects(first, second)
        || too_close(first[0], second, diameter)
        || too_close(first[1], second, diameter)
        || too_close(second[0], first, diameter)
        || too_close(second[1], first, diameter)
}

/// A round-ended straight trace or a circular via land ([p,p]) wholly on the substrate.
/// Tangency at the edge is allowed. Positive fabrication edge clearances are separate.
pub fn contains_copper(outline: &[Point], segment: Segment, diameter: i64) -> bool {
    inside(outline, segment[0]) && sides(outline).all(|edge| !segments_too_close(segment, edge, diameter))
}

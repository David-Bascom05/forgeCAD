use forge_geom::spline::{PlaneSpline, Spline};
use forge_math::predicates::{inside_polygon, orient2d};
use forge_math::{Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use std::sync::Arc;

/// One piece of a closed 2D profile, in sketch-plane coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    Line {
        a: Point2,
        b: Point2,
    },
    /// Counter-clockwise when `sweep` > 0. |sweep| < 2pi.
    Arc {
        center: Point2,
        radius: f64,
        start: f64,
        sweep: f64,
    },
    /// A whole circle, alone in its loop, starting and ending at angle 0.
    Circle {
        center: Point2,
        radius: f64,
        ccw: bool,
    },
    /// A B-spline, fit spline or elliptical arc, run with its parameter.
    Spline(Arc<PlaneSpline>),
}

/// A closed chain of segments. Each segment has an id, unique within the
/// profile, which names the faces and edges it produces.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileLoop {
    pub segments: Vec<(Segment, u64)>,
}

/// A planar region: the first loop is the outer boundary, the rest are holes.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub plane: Frame,
    pub loops: Vec<ProfileLoop>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProfileError {
    Empty,
    /// Segment `id` does not end where the next one starts.
    Open {
        id: u64,
        gap: f64,
    },
    Degenerate {
        id: u64,
    },
    DuplicateId {
        id: u64,
    },
    /// A circle must be the only segment of its loop.
    CircleNotAlone {
        id: u64,
    },
    /// A hole is not inside the outer loop.
    HoleOutside {
        loop_index: usize,
    },
    /// A hole touches the outer loop at `at` (world coordinates): the wall there has no thickness,
    /// so the solid would pinch along a line, which cannot be made.
    Pinched {
        loop_index: usize,
        at: Point3,
    },
    /// A spline or ellipse where this operation takes lines and arcs only.
    Curve {
        id: u64,
    },
    /// A loop crosses itself (a corner radius larger than half its side, say).
    SelfIntersecting {
        loop_index: usize,
    },
}

/// A piece of a loop as a line or an arc, for exact nearest points.
#[derive(Clone, Copy)]
enum Piece<'a> {
    Line(Point2, Point2),
    Arc(Point2, f64, f64, f64),
    Curve(&'a Spline),
}

fn p3(p: Point2) -> Point3 {
    Point3::new(p.x, p.y, 0.0)
}
fn p2(p: Point3) -> Point2 {
    Point2::new(p.x, p.y)
}

fn piece(s: &Segment) -> Piece<'_> {
    match *s {
        Segment::Spline(ref c) => Piece::Curve(&c.c),
        Segment::Line { a, b } => Piece::Line(a, b),
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => Piece::Arc(center, radius, start, sweep),
        Segment::Circle { center, radius, .. } => Piece::Arc(center, radius, 0.0, dmath::TAU),
    }
}

fn on_arc(angle: f64, start: f64, sweep: f64) -> bool {
    let (lo, w) = if sweep >= 0.0 {
        (start, sweep)
    } else {
        (start + sweep, -sweep)
    };
    (angle - lo).rem_euclid(dmath::TAU) <= w
}

impl Piece<'_> {
    fn ends(self) -> [Point2; 2] {
        match self {
            Piece::Curve(c) => {
                let (a, b) = c.ends();
                [p2(a), p2(b)]
            }
            Piece::Line(a, b) => [a, b],
            Piece::Arc(c, r, s, w) => [arc_point(c, r, s), arc_point(c, r, s + w)],
        }
    }
    /// The nearest point on this piece to p.
    fn nearest(self, p: Point2) -> Point2 {
        match self {
            Piece::Curve(c) => p2(c.point(c.closest(p3(p), c.domain()))),
            Piece::Line(a, b) => {
                let ab = b - a;
                a + ab * ((p - a).dot(ab) / ab.dot(ab).max(f64::MIN_POSITIVE)).clamp(0.0, 1.0)
            }
            Piece::Arc(c, r, s, w) => {
                let d = p - c;
                let at = dmath::atan2(d.y, d.x);
                if d.dot(d) > 0.0 && on_arc(at, s, w) {
                    arc_point(c, r, at)
                } else {
                    let [e0, e1] = self.ends();
                    if e0.distance(p) <= e1.distance(p) { e0 } else { e1 }
                }
            }
        }
    }
}

/// Where two pieces come nearest, among the places they can touch without crossing: their ends,
/// and along the line of centres (or the normal from a centre to a line): the point on `a`, and
/// the distance.
fn nearest_touch(a: Piece, b: Piece) -> (Point2, f64) {
    let mut best = (Point2::ORIGIN, f64::INFINITY);
    let mut try_on_a = |q: Point2| {
        let d = b.nearest(q).distance(q);
        if d < best.1 {
            best = (q, d);
        }
    };
    for e in a.ends() {
        try_on_a(e);
    }
    for e in b.ends() {
        try_on_a(a.nearest(e));
    }
    match (a, b) {
        (Piece::Arc(c, _, _, _), Piece::Line(..)) => {
            let foot = b.nearest(c);
            try_on_a(a.nearest(foot));
            // the far side of the circle from the line's foot is no touch from inside or out
        }
        (Piece::Line(..), Piece::Arc(c, _, _, _)) => try_on_a(a.nearest(c)),
        (Piece::Curve(c), _) | (_, Piece::Curve(c)) => {
            // from points along the curve, each to the other piece and back until it settles
            let on_curve = matches!(a, Piece::Curve(_));
            for q in c.samples(8 * c.pieces()) {
                let mut q = p2(q);
                for _ in 0..6 {
                    let far = if on_curve { b.nearest(q) } else { a.nearest(q) };
                    q = if on_curve { a.nearest(far) } else { b.nearest(far) };
                }
                try_on_a(if on_curve { q } else { a.nearest(q) });
            }
        }
        (Piece::Arc(c, r, s, w), Piece::Arc(d, _, _, _)) => {
            if let Some(u) = (d - c).normalize() {
                for k in [1.0, -1.0] {
                    let q = c + u * (k * r);
                    if on_arc(dmath::atan2(q.y - c.y, q.x - c.x), s, w) {
                        try_on_a(q);
                    }
                }
            }
        }
        _ => {}
    }
    best
}

/// Where two loops touch, within tolerance: each distinct point.
fn loops_touch(x: &ProfileLoop, y: &ProfileLoop) -> Vec<Point2> {
    let mut out: Vec<Point2> = Vec::new();
    for (s, _) in &x.segments {
        for (t, _) in &y.segments {
            // a round hole clear of a piece by its centre needs no more
            if let Segment::Circle { center, radius, .. } = *s
                && piece(t).nearest(center).distance(center) > radius + tol::LINEAR
            {
                continue;
            }
            let (q, d) = nearest_touch(piece(s), piece(t));
            if d <= tol::LINEAR && out.iter().all(|o| o.distance(q) > 10.0 * tol::LINEAR) {
                out.push(q);
            }
        }
    }
    out
}

/// Two loops of a profile touching at a point.
#[derive(Clone, Copy, Debug)]
pub struct Touch {
    pub hole: usize,
    pub other: usize,
    /// In the sketch's coordinates.
    pub at: Point2,
    /// The hole's direction there.
    pub tangent: Vec2,
}

fn loop_tangent(l: &ProfileLoop, at: Point2) -> Vec2 {
    let mut best = (f64::INFINITY, Vec2::X);
    for (s, _) in &l.segments {
        let (q, t) = match piece(s) {
            Piece::Curve(c) => {
                let d = c.deriv(c.closest(p3(at), c.domain()));
                (p2(d.p), Vec2::new(d.d1.x, d.d1.y).normalize().unwrap_or(Vec2::X))
            }
            Piece::Line(a, b) => (piece(s).nearest(at), (b - a).normalize().unwrap_or(Vec2::X)),
            Piece::Arc(c, ..) => {
                let q = piece(s).nearest(at);
                let d = (q - c).normalize().unwrap_or(Vec2::X);
                (q, Vec2::new(-d.y, d.x))
            }
        };
        if q.distance(at) < best.0 {
            best = (q.distance(at), t);
        }
    }
    best.1
}

/// Whether p lies inside a loop, its arcs exact: parity of crossings of the ray along +x.
fn loop_holds(l: &ProfileLoop, p: Point2) -> bool {
    let mut inside = false;
    for (s, _) in &l.segments {
        match piece(s) {
            Piece::Curve(c) => {
                // half-open like the lines': a point of the curve at the ray's height, by the
                // sign the curve's ends take
                for (t, touch) in c.plane_roots(Vec3::Y, -p.y, c.domain(), 0.0) {
                    if !touch && c.point(t).x > p.x {
                        inside = !inside;
                    }
                }
            }
            Piece::Line(a, b) => {
                if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                    inside = !inside;
                }
            }
            Piece::Arc(c, r, st, w) => {
                let dy = p.y - c.y;
                if dy.abs() >= r {
                    continue;
                }
                let dx = (r * r - dy * dy).sqrt();
                for x in [c.x - dx, c.x + dx] {
                    if x > p.x && on_arc(dmath::atan2(dy, x - c.x), st, w) {
                        inside = !inside;
                    }
                }
            }
        }
    }
    inside
}

impl Segment {
    pub fn start(&self) -> Point2 {
        match *self {
            Segment::Spline(ref c) => c.start(),
            Segment::Line { a, .. } => a,
            Segment::Arc {
                center,
                radius,
                start,
                ..
            } => arc_point(center, radius, start),
            Segment::Circle { center, radius, .. } => arc_point(center, radius, 0.0),
        }
    }
    pub fn end(&self) -> Point2 {
        match *self {
            Segment::Spline(ref c) => c.end(),
            Segment::Line { b, .. } => b,
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => arc_point(center, radius, start + sweep),
            Segment::Circle { center, radius, .. } => arc_point(center, radius, 0.0),
        }
    }
    pub fn reversed(&self) -> Segment {
        match *self {
            Segment::Spline(ref c) => Segment::Spline(Arc::new(c.reversed())),
            Segment::Line { a, b } => Segment::Line { a: b, b: a },
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => Segment::Arc {
                center,
                radius,
                start: start + sweep,
                sweep: -sweep,
            },
            Segment::Circle { center, radius, ccw } => Segment::Circle {
                center,
                radius,
                ccw: !ccw,
            },
        }
    }
    /// Mirror in the sketch x axis (y -> -y).
    pub fn mirrored(&self) -> Segment {
        let m = |p: Point2| Point2::new(p.x, -p.y);
        match *self {
            Segment::Spline(ref c) => Segment::Spline(Arc::new(c.mirrored())),
            Segment::Line { a, b } => Segment::Line { a: m(a), b: m(b) },
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => Segment::Arc {
                center: m(center),
                radius,
                start: -start,
                sweep: -sweep,
            },
            Segment::Circle { center, radius, ccw } => Segment::Circle {
                center: m(center),
                radius,
                ccw: !ccw,
            },
        }
    }
    /// This segment's share of the loop's signed area (the integral of x dy).
    pub fn area_term(&self) -> f64 {
        match *self {
            Segment::Spline(ref c) => c.c.area_term(),
            Segment::Line { a, b } => 0.5 * a.to_vec().cross(b.to_vec()),
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                // 1/2 integral of (p x dp) over the arc
                let (s0, c0) = dmath::sin_cos(start);
                let (s1, c1) = dmath::sin_cos(start + sweep);
                let chord = 0.5
                    * center
                        .to_vec()
                        .cross(Vec2::new(radius * (c1 - c0), radius * (s1 - s0)));
                0.5 * radius * radius * sweep + chord
            }
            Segment::Circle { radius, ccw, .. } => {
                let a = dmath::PI * radius * radius;
                if ccw { a } else { -a }
            }
        }
    }
    fn degenerate(&self) -> bool {
        match *self {
            Segment::Spline(ref c) => {
                let (b, r) = (c.c.bbox(c.c.domain()), tol::MIN_LENGTH);
                !(b.x.width() > r || b.y.width() > r) || !b.diagonal().is_finite()
            }
            // written to be true for NaN or infinite sizes too
            Segment::Line { a, b } => !(a.distance(b) > tol::MIN_LENGTH && a.distance(b) < f64::INFINITY),
            Segment::Arc { radius, sweep, .. } => {
                !(radius > tol::MIN_LENGTH
                    && radius < f64::INFINITY
                    && (radius * sweep).abs() > tol::MIN_LENGTH
                    && sweep.abs() < dmath::TAU - tol::ANGULAR)
            }
            Segment::Circle { radius, .. } => !(radius > tol::MIN_LENGTH && radius < f64::INFINITY),
        }
    }
    /// Points along the segment, for inside/outside tests.
    pub fn samples(&self, n: usize) -> Vec<Point2> {
        if let Segment::Spline(c) = self {
            return c.c.samples(n).into_iter().map(p2).collect();
        }
        (0..n)
            .map(|i| {
                let k = i as f64 / n as f64;
                match *self {
                    Segment::Spline(_) => unreachable!("taken above"),
                    Segment::Line { a, b } => a.lerp(b, k),
                    Segment::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    } => arc_point(center, radius, start + sweep * k),
                    Segment::Circle { center, radius, ccw } => {
                        let a = dmath::TAU * k;
                        arc_point(center, radius, if ccw { a } else { -a })
                    }
                }
            })
            .collect()
    }
}

pub fn arc_point(center: Point2, radius: f64, angle: f64) -> Point2 {
    let (s, c) = dmath::sin_cos(angle);
    Point2::new(center.x + radius * c, center.y + radius * s)
}

impl ProfileLoop {
    pub fn new(segments: Vec<(Segment, u64)>) -> Self {
        ProfileLoop { segments }
    }
    /// Curves with corners cut there, each arc a face of its own (as Inventor shows them), and one
    /// closing on itself cut in two. The first piece keeps the segment's id.
    /// None where a curve stops being one (a cusp, a piece too short).
    fn smooth_pieces(&self) -> Option<ProfileLoop> {
        let mut out = Vec::with_capacity(self.segments.len());
        let alone = self.segments.len() == 1;
        for (s, id) in &self.segments {
            let Segment::Spline(c) = s else {
                out.push((s.clone(), *id));
                continue;
            };
            let d = c.c.domain();
            let mut cuts = c.c.corners();
            if cuts.is_empty() && alone {
                // a joint where the pieces meet, else the middle
                let b = c.c.breaks();
                cuts.push(if b.len() > 2 { b[b.len() / 2] } else { d.mid() });
            }
            if cuts.is_empty() {
                out.push((s.clone(), *id));
                continue;
            }
            let mut lo = d.lo;
            for (k, &hi) in cuts.iter().chain([&d.hi]).enumerate() {
                let part = c.part(Interval::new(lo, hi))?;
                out.push((Segment::Spline(Arc::new(part)), *id ^ ((k as u64) << 48)));
                lo = hi;
            }
        }
        Some(ProfileLoop::new(out))
    }
    pub fn signed_area(&self) -> f64 {
        self.segments.iter().map(|(s, _)| s.area_term()).sum()
    }
    pub fn reversed(&self) -> ProfileLoop {
        ProfileLoop {
            segments: self
                .segments
                .iter()
                .rev()
                .map(|(s, id)| (s.reversed(), *id))
                .collect(),
        }
    }
    /// The nearest point of the loop to p.
    pub fn nearest(&self, p: Point2) -> Point2 {
        let mut best = (f64::INFINITY, p);
        for (s, _) in &self.segments {
            let q = piece(s).nearest(p);
            if q.distance(p) < best.0 {
                best = (q.distance(p), q);
            }
        }
        best.1
    }
    fn polygon(&self) -> Vec<Point2> {
        self.segments.iter().flat_map(|(s, _)| s.samples(16)).collect()
    }
    /// The polygon a crossing is looked for on: a line needs only its start.
    fn outline(&self) -> Vec<Point2> {
        self.segments
            .iter()
            .flat_map(|(s, _)| s.samples(if matches!(s, Segment::Line { .. }) { 1 } else { 16 }))
            .collect()
    }
}

/// Whether two edges of the closed polygon cross or touch (adjacent edges share a corner): an
/// edge through another's corner counts, as a crossing there has no proper one to show.
/// Edges are swept by their x extent, so only edges whose boxes overlap are tested, exactly.
fn crosses_itself(poly: &[Point2]) -> bool {
    let n = poly.len();
    let seg = |i: usize| (poly[i], poly[(i + 1) % n]);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_unstable_by(|&i, &j| {
        let (a, b) = (seg(i), seg(j));
        a.0.x.min(a.1.x).total_cmp(&b.0.x.min(b.1.x))
    });
    // strictly opposite sides (signs, not products: they could underflow)
    let apart = |x: f64, y: f64| (x > 0.0 && y < 0.0) || (x < 0.0 && y > 0.0);
    for (k, &i) in order.iter().enumerate() {
        let (a, b) = seg(i);
        let right = a.x.max(b.x);
        for &j in &order[k + 1..] {
            let (c, d) = seg(j);
            if c.x.min(d.x) > right {
                break;
            }
            if (i + 1) % n == j
                || (j + 1) % n == i
                || a.y.max(b.y) < c.y.min(d.y)
                || c.y.max(d.y) < a.y.min(b.y)
            {
                continue;
            }
            let (o1, o2, o3, o4) = (
                orient2d(a, b, c),
                orient2d(a, b, d),
                orient2d(c, d, a),
                orient2d(c, d, b),
            );
            if apart(o1, o2) && apart(o3, o4) {
                return true;
            }
            let on = |p: Point2, q: Point2, x: Point2| {
                x.x >= p.x.min(q.x) && x.x <= p.x.max(q.x) && x.y >= p.y.min(q.y) && x.y <= p.y.max(q.y)
            };
            if (o1 == 0.0 && on(a, b, c))
                || (o2 == 0.0 && on(a, b, d))
                || (o3 == 0.0 && on(c, d, a))
                || (o4 == 0.0 && on(c, d, b))
            {
                return true;
            }
        }
    }
    false
}

fn inside(p: Point2, poly: &[Point2]) -> bool {
    inside_polygon(p, poly)
}

/// Whether a segment's end direction turns back on itself into the next one's start. (Asked of every
/// extrude's every joint: an arc's direction comes from the joint, its neighbour's end, without trig
/// when it meets a line.)
fn doubles_back(a: &Segment, b: &Segment) -> bool {
    let at = match (a, b) {
        (Segment::Line { b: p, .. }, _) => *p,
        (_, Segment::Line { a: p, .. }) => *p,
        (Segment::Arc { .. }, Segment::Arc { .. }) => b.start(),
        _ => return false,
    };
    let dir = |s: &Segment| -> Vec2 {
        match *s {
            Segment::Line { a, b } => b - a,
            Segment::Arc { center, sweep, .. } => (at - center).perp() * sweep.signum(),
            Segment::Circle { .. } => Vec2::new(1.0, 0.0),
        }
    };
    let (u, v) = (dir(a), dir(b));
    let d = u.dot(v);
    d < 0.0 && d * d >= 0.9999 * u.length_sq() * v.length_sq()
}

/// A loop's joint where it doubles back on itself (a line leaving tangent along the arc it came on)
/// is a knife edge whose two sides run within tolerance of each other for a stretch: walls a boolean
/// cannot tell apart. They end instead where they part, the tip a shade shorter than drawn.
fn blunt_cusps(l: &ProfileLoop) -> ProfileLoop {
    let n = l.segments.len();
    let length = |s: &Segment| match *s {
        Segment::Line { a, b } => a.distance(b),
        Segment::Arc { radius, sweep, .. } => radius * sweep.abs(),
        Segment::Circle { .. } => 0.0,
    };
    // `s` along the segment back from its end, or on from its start
    let at = |seg: &Segment, end: bool, s: f64| -> Point2 {
        match *seg {
            Segment::Line { a, b } => {
                let t = s / a.distance(b);
                if end { b.lerp(a, t) } else { a.lerp(b, t) }
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let w = sweep.signum() * s / radius;
                arc_point(center, radius, if end { start + sweep - w } else { start + w })
            }
            Segment::Circle { center, .. } => center,
        }
    };
    let cut = |seg: &Segment, end: bool, s: f64| -> Segment {
        match *seg {
            Segment::Line { a, b } => {
                let q = at(seg, end, s);
                if end {
                    Segment::Line { a, b: q }
                } else {
                    Segment::Line { a: q, b }
                }
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let w = sweep.signum() * s / radius;
                Segment::Arc {
                    center,
                    radius,
                    start: if end { start } else { start + w },
                    sweep: sweep - w,
                }
            }
            c => c,
        }
    };
    let mut out = l.segments.clone();
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (out[i].0, out[j].0);
        if !doubles_back(&a, &b) {
            continue;
        }
        let part = 0.9 * tol::LINEAR;
        let apart = |s: f64| at(&a, true, s).distance(at(&b, false, s));
        let far = 0.5 * length(&a).min(length(&b));
        if apart(far) < part || apart(0.0) >= part {
            continue;
        }
        let (mut lo, mut hi) = (0.0, far);
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            if apart(mid) < part {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        out[i].0 = cut(&a, true, lo);
        out[j].0 = cut(&b, false, lo);
    }
    ProfileLoop::new(out)
}

impl Profile {
    /// Checks the profile and returns it with the outer loop counter-clockwise
    /// and every hole clockwise; a loop that crosses or touches itself is refused.
    pub fn normalized(&self) -> Result<Profile, ProfileError> {
        let p = self.normalized_touching()?;
        match p.touching().first() {
            Some(t) => Err(ProfileError::Pinched {
                loop_index: t.hole,
                at: self.plane.plane_point(t.at),
            }),
            None => Ok(p),
        }
    }

    /// Where the loops of a profile touch at a point: outer and hole, or two holes.
    pub fn touching(&self) -> Vec<Touch> {
        let mut out = Vec::new();
        if self.loops.len() < 2 {
            return out;
        }
        let spans: Vec<(f64, f64)> = self
            .loops
            .iter()
            .map(|l| {
                l.polygon()
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |m, p| {
                        (m.0.min(p.x), m.1.max(p.x))
                    })
            })
            .collect();
        for i in 1..self.loops.len() {
            for j in 0..i {
                // a sampled span is short of the true one by a chord's sag: the outer loop always asked
                let (a, b) = (spans[i], spans[j]);
                let slack = 0.2 * (a.1 - a.0).max(b.1 - b.0);
                if j > 0 && (a.0 > b.1 + slack || b.0 > a.1 + slack) {
                    continue;
                }
                for at in loops_touch(&self.loops[i], &self.loops[j]) {
                    out.push(Touch {
                        hole: i,
                        at,
                        tangent: loop_tangent(&self.loops[i], at),
                        other: j,
                    });
                }
            }
        }
        out
    }

    /// `normalized`, letting loops touch at a point: an extrude's solid pinches there, which is
    /// for its result to answer (`touching` says where).
    pub fn normalized_touching(&self) -> Result<Profile, ProfileError> {
        if self.loops.is_empty() || self.loops.iter().any(|l| l.segments.is_empty()) {
            return Err(ProfileError::Empty);
        }
        let mut ids: Vec<u64> = self
            .loops
            .iter()
            .flat_map(|l| l.segments.iter().map(|s| s.1))
            .collect();
        ids.sort();
        if let Some(w) = ids.windows(2).find(|w| w[0] == w[1]) {
            return Err(ProfileError::DuplicateId { id: w[0] });
        }
        for l in &self.loops {
            let n = l.segments.len();
            for (i, (s, id)) in l.segments.iter().enumerate() {
                let id = *id;
                if s.degenerate() {
                    return Err(ProfileError::Degenerate { id });
                }
                if matches!(s, Segment::Circle { .. }) && n != 1 {
                    return Err(ProfileError::CircleNotAlone { id });
                }
                let next = &l.segments[(i + 1) % n].0;
                let gap = s.end().distance(next.start());
                if gap > tol::LINEAR {
                    return Err(ProfileError::Open { id, gap });
                }
            }
        }
        for (i, l) in self.loops.iter().enumerate() {
            // a lone circle is drawn as a convex polygon: it cannot cross itself
            let circle = matches!(l.segments[..], [(Segment::Circle { .. }, _)]);
            // (a knife edge's sides, samples of them, cross within tolerance)
            if !circle && crosses_itself(&l.outline()) && crosses_itself(&blunt_cusps(l).outline()) {
                return Err(ProfileError::SelfIntersecting { loop_index: i });
            }
        }
        let mut loops = Vec::with_capacity(self.loops.len());
        for (i, l) in self.loops.iter().enumerate() {
            let ccw = l.signed_area() > 0.0;
            let want_ccw = i == 0;
            let l = if ccw == want_ccw { l.clone() } else { l.reversed() };
            loops.push(
                if l.segments.iter().any(|(s, _)| matches!(s, Segment::Spline(_))) {
                    l.smooth_pieces()
                        .ok_or(ProfileError::Degenerate { id: l.segments[0].1 })?
                } else {
                    l
                },
            );
        }
        if loops.len() > 1 {
            let outer = loops[0].polygon();
            for (i, l) in loops.iter().enumerate().skip(1) {
                // the sampled outline cuts its own curves' corners: a point it leaves out is asked
                // of the exact loop, so only a real stray makes the hole lie outside
                let out = |p: Point2| !inside(p, &outer) && !loop_holds(&loops[0], p);
                if l.polygon().into_iter().any(out) && loops_touch(l, &loops[0]).is_empty() {
                    return Err(ProfileError::HoleOutside { loop_index: i });
                }
            }
        }
        Ok(Profile {
            plane: self.plane,
            loops,
        })
    }

    /// This profile with its knife edges ended where their sides part, if it has any: for a boolean
    /// that cannot tell the sides apart (a tip that stays can meet other faces exactly, so it comes first).
    pub fn blunted(&self) -> Option<Profile> {
        let cusp = |l: &ProfileLoop| {
            let n = l.segments.len();
            (0..n).any(|i| doubles_back(&l.segments[i].0, &l.segments[(i + 1) % n].0))
        };
        if !self.loops.iter().any(cusp) {
            return None;
        }
        let loops: Vec<ProfileLoop> = self.loops.iter().map(blunt_cusps).collect();
        let same = |a: &ProfileLoop, b: &ProfileLoop| a.segments == b.segments;
        (!loops.iter().zip(&self.loops).all(|(a, b)| same(a, b))).then_some(Profile {
            plane: self.plane,
            loops,
        })
    }

    pub fn area(&self) -> f64 {
        self.loops.iter().map(|l| l.signed_area()).sum::<f64>().abs()
    }

    /// A w x h rectangle with its lower-left corner at `corner`; segment ids start at `first_id`.
    pub fn rectangle(corner: Point2, w: f64, h: f64, first_id: u64) -> ProfileLoop {
        let p = [
            corner,
            Point2::new(corner.x + w, corner.y),
            Point2::new(corner.x + w, corner.y + h),
            Point2::new(corner.x, corner.y + h),
        ];
        ProfileLoop::new(
            (0..4)
                .map(|i| {
                    (
                        Segment::Line {
                            a: p[i],
                            b: p[(i + 1) % 4],
                        },
                        first_id + i as u64,
                    )
                })
                .collect(),
        )
    }

    /// A rectangle with corners rounded to `r`.
    pub fn rounded_rectangle(corner: Point2, w: f64, h: f64, r: f64, first_id: u64) -> ProfileLoop {
        let (x0, y0, x1, y1) = (corner.x, corner.y, corner.x + w, corner.y + h);
        let q = dmath::FRAC_PI_2;
        let segs = vec![
            Segment::Line {
                a: Point2::new(x0 + r, y0),
                b: Point2::new(x1 - r, y0),
            },
            Segment::Arc {
                center: Point2::new(x1 - r, y0 + r),
                radius: r,
                start: -q,
                sweep: q,
            },
            Segment::Line {
                a: Point2::new(x1, y0 + r),
                b: Point2::new(x1, y1 - r),
            },
            Segment::Arc {
                center: Point2::new(x1 - r, y1 - r),
                radius: r,
                start: 0.0,
                sweep: q,
            },
            Segment::Line {
                a: Point2::new(x1 - r, y1),
                b: Point2::new(x0 + r, y1),
            },
            Segment::Arc {
                center: Point2::new(x0 + r, y1 - r),
                radius: r,
                start: q,
                sweep: q,
            },
            Segment::Line {
                a: Point2::new(x0, y1 - r),
                b: Point2::new(x0, y0 + r),
            },
            Segment::Arc {
                center: Point2::new(x0 + r, y0 + r),
                radius: r,
                start: 2.0 * q,
                sweep: q,
            },
        ];
        ProfileLoop::new(
            segs.into_iter()
                .enumerate()
                .map(|(i, s)| (s, first_id + i as u64))
                .collect(),
        )
    }

    pub fn circle(center: Point2, radius: f64, id: u64) -> ProfileLoop {
        ProfileLoop::new(vec![(
            Segment::Circle {
                center,
                radius,
                ccw: true,
            },
            id,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas() {
        let r = Profile::rectangle(Point2::new(1.0, 2.0), 3.0, 4.0, 0);
        assert!((r.signed_area() - 12.0).abs() < 1e-12);
        assert!((r.reversed().signed_area() + 12.0).abs() < 1e-12);
        let rr = Profile::rounded_rectangle(Point2::new(-5.0, 7.0), 10.0, 6.0, 1.5, 0);
        let want = 60.0 - (4.0 - dmath::PI) * 1.5 * 1.5;
        assert!((rr.signed_area() - want).abs() < 1e-12, "{}", rr.signed_area());
        let c = Profile::circle(Point2::new(3.0, 3.0), 2.0, 0);
        assert!((c.signed_area() - 4.0 * dmath::PI).abs() < 1e-12);
    }

    #[test]
    fn normalizes_orientation_and_rejects_bad_input() {
        let outer = Profile::rectangle(Point2::ORIGIN, 10.0, 10.0, 0).reversed();
        let hole = Profile::circle(Point2::new(5.0, 5.0), 2.0, 10);
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![outer.clone(), hole.clone()],
        }
        .normalized()
        .unwrap();
        assert!(p.loops[0].signed_area() > 0.0 && p.loops[1].signed_area() < 0.0);
        assert!((p.area() - (100.0 - 4.0 * dmath::PI)).abs() < 1e-9);

        let far = Profile::circle(Point2::new(50.0, 5.0), 2.0, 10);
        assert_eq!(
            Profile {
                plane: Frame::WORLD,
                loops: vec![outer.clone(), far]
            }
            .normalized(),
            Err(ProfileError::HoleOutside { loop_index: 1 })
        );
        // touching from inside: where, and not as if it lay outside
        let round = Profile::circle(Point2::ORIGIN, 40.0, 11);
        let Err(ProfileError::Pinched { loop_index: 1, at }) = (Profile {
            plane: Frame::WORLD,
            loops: vec![round, Profile::circle(Point2::new(0.0, -30.0), 10.0, 12)],
        })
        .normalized() else {
            panic!("a hole touching its outline");
        };
        assert!(at.distance(Point3::new(0.0, -40.0, 0.0)) < 0.5, "{at:?}");
        let mut open = outer.clone();
        open.segments.pop();
        assert!(matches!(
            Profile {
                plane: Frame::WORLD,
                loops: vec![open]
            }
            .normalized(),
            Err(ProfileError::Open { .. })
        ));
        let dup = Profile::circle(Point2::new(5.0, 5.0), 2.0, 0);
        assert_eq!(
            Profile {
                plane: Frame::WORLD,
                loops: vec![outer, dup]
            }
            .normalized(),
            Err(ProfileError::DuplicateId { id: 0 })
        );
    }
}

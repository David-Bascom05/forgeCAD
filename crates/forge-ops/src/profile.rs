use forge_math::{Frame, Point2, Vec2, dmath, tol};

/// One piece of a closed 2D profile, in sketch-plane coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
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
}

impl Segment {
    pub fn start(&self) -> Point2 {
        match *self {
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
            Segment::Line { a, b } => a.distance(b) <= tol::MIN_LENGTH,
            Segment::Arc { radius, sweep, .. } => {
                radius <= tol::MIN_LENGTH
                    || (radius * sweep).abs() <= tol::MIN_LENGTH
                    || sweep.abs() >= dmath::TAU - tol::ANGULAR
            }
            Segment::Circle { radius, .. } => radius <= tol::MIN_LENGTH,
        }
    }
    /// Points along the segment, for inside/outside tests.
    pub fn samples(&self, n: usize) -> Vec<Point2> {
        (0..n)
            .map(|i| {
                let k = i as f64 / n as f64;
                match *self {
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
    pub fn signed_area(&self) -> f64 {
        self.segments.iter().map(|(s, _)| s.area_term()).sum()
    }
    pub fn reversed(&self) -> ProfileLoop {
        ProfileLoop {
            segments: self
                .segments
                .iter()
                .rev()
                .map(|&(s, id)| (s.reversed(), id))
                .collect(),
        }
    }
    fn polygon(&self) -> Vec<Point2> {
        self.segments.iter().flat_map(|(s, _)| s.samples(16)).collect()
    }
}

fn inside(p: Point2, poly: &[Point2]) -> bool {
    let mut c = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

impl Profile {
    /// Checks the profile and returns it with the outer loop counter-clockwise
    /// and every hole clockwise. Self-intersection is not detected yet.
    pub fn normalized(&self) -> Result<Profile, ProfileError> {
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
            for (i, &(s, id)) in l.segments.iter().enumerate() {
                if s.degenerate() {
                    return Err(ProfileError::Degenerate { id });
                }
                if matches!(s, Segment::Circle { .. }) && n != 1 {
                    return Err(ProfileError::CircleNotAlone { id });
                }
                let next = l.segments[(i + 1) % n].0;
                let gap = s.end().distance(next.start());
                if gap > tol::LINEAR {
                    return Err(ProfileError::Open { id, gap });
                }
            }
        }
        let mut loops = Vec::with_capacity(self.loops.len());
        for (i, l) in self.loops.iter().enumerate() {
            let ccw = l.signed_area() > 0.0;
            let want_ccw = i == 0;
            loops.push(if ccw == want_ccw { l.clone() } else { l.reversed() });
        }
        let outer = loops[0].polygon();
        for (i, l) in loops.iter().enumerate().skip(1) {
            if !l.polygon().iter().all(|&p| inside(p, &outer)) {
                return Err(ProfileError::HoleOutside { loop_index: i });
            }
        }
        Ok(Profile {
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

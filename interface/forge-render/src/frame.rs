//! Framing a subject (docs/ux/STANDARD.md §2): the camera every environment uses when it is
//! entered, when its subject changes and on Fit. The subject's box fills a share of the part of
//! the view no docked panel covers, seen from a standard direction, with the eye outside every
//! obstacle (a machine's frame, a press's housings) when a direction allows it.

use crate::camera::{Camera, dot};

/// An axis-aligned box, mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Bounds {
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Bounds {
        Bounds { min, max }
    }

    /// The box around these points; none for no points.
    pub fn of(points: impl IntoIterator<Item = [f32; 3]>) -> Option<Bounds> {
        let mut it = points.into_iter();
        let p = it.next()?;
        Some(it.fold(Bounds::new(p, p), |b, q| b.grow(q)))
    }

    pub fn grow(self, p: [f32; 3]) -> Bounds {
        Bounds {
            min: [0, 1, 2].map(|k| self.min[k].min(p[k])),
            max: [0, 1, 2].map(|k| self.max[k].max(p[k])),
        }
    }

    pub fn union(self, o: Bounds) -> Bounds {
        self.grow(o.min).grow(o.max)
    }

    pub fn center(&self) -> [f32; 3] {
        [0, 1, 2].map(|k| (self.min[k] + self.max[k]) * 0.5)
    }

    pub fn size(&self) -> [f32; 3] {
        [0, 1, 2].map(|k| self.max[k] - self.min[k])
    }

    /// Half the diagonal.
    pub fn radius(&self) -> f32 {
        let s = self.size();
        (dot(s, s)).sqrt() * 0.5
    }

    pub fn corners(&self) -> [[f32; 3]; 8] {
        let (a, b) = (self.min, self.max);
        [0, 1, 2, 3, 4, 5, 6, 7].map(|i| {
            [
                if i & 1 == 0 { a[0] } else { b[0] },
                if i & 2 == 0 { a[1] } else { b[1] },
                if i & 4 == 0 { a[2] } else { b[2] },
            ]
        })
    }

    /// Strictly inside, by more than `margin`.
    pub fn contains(&self, p: [f32; 3], margin: f32) -> bool {
        (0..3).all(|k| p[k] > self.min[k] + margin && p[k] < self.max[k] - margin)
    }
}

/// Where the camera looks from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dir {
    /// The app's 3/4 view: front-right-top (nav's HOME_CORNER, z up).
    Iso,
    /// Front-right, lower: a machine's working zone, seen as an operator does.
    Operator,
    /// Yaw and pitch in degrees.
    Custom(f32, f32),
}

impl Dir {
    fn angles(self) -> (f32, f32) {
        match self {
            Dir::Iso => (-45.0, 35.26),
            Dir::Operator => (-70.0, 12.0),
            Dir::Custom(y, p) => (y, p),
        }
    }
}

/// The view, in pixels, and the part of it no docked panel or bar covers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewRect {
    pub width: f32,
    pub height: f32,
    /// Left, top, right, bottom insets covered by panels, px.
    pub covered: [f32; 4],
}

impl ViewRect {
    pub fn full(width: f32, height: f32) -> ViewRect {
        ViewRect {
            width,
            height,
            covered: [0.0; 4],
        }
    }

    /// The uncovered rect's centre and half size, in clip units (-1..1, y up).
    fn clip(&self) -> ([f32; 2], [f32; 2]) {
        let [l, t, r, b] = self.covered;
        let (w, h) = (self.width.max(1.0), self.height.max(1.0));
        let (x0, x1) = (l.min(w - 1.0), (w - r).max(l + 1.0));
        let (y0, y1) = (t.min(h - 1.0), (h - b).max(t + 1.0));
        let cx = (x0 + x1) / w - 1.0;
        let cy = 1.0 - (y0 + y1) / h;
        ([cx, cy], [(x1 - x0) / w, (y1 - y0) / h])
    }
}

/// How a subject is framed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub dir: Dir,
    /// The share of the uncovered rect's shorter side the subject spans, 0.6–0.75 by the standard.
    pub fill: f32,
}

impl Default for Framing {
    fn default() -> Framing {
        Framing {
            dir: Dir::Iso,
            fill: 0.7,
        }
    }
}

/// A framed camera, and whether its eye had to stay inside an obstacle (the caller then ghosts or
/// cuts away what is in front).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framed {
    pub camera: Camera,
    pub eye_blocked: bool,
}

/// Frame `subject` in `view` from `how.dir`; the eye kept out of `obstacles` by turning about the
/// subject in 15° steps (up to ±90° of yaw, then higher) when the standard direction would put it
/// inside one.
pub fn frame(subject: Bounds, view: ViewRect, how: Framing, obstacles: &[Bounds]) -> Framed {
    let (yaw0, pitch0) = how.dir.angles();
    let mut tries = vec![(yaw0, pitch0)];
    for step in 1..=6 {
        let d = step as f32 * 15.0;
        tries.push((yaw0 + d, pitch0));
        tries.push((yaw0 - d, pitch0));
    }
    for p in [40.0, 60.0] {
        tries.push((yaw0, p));
    }
    let mut first = None;
    for (y, p) in tries {
        let cam = frame_from(subject, view, how.fill, y, p);
        let eye = cam.eye();
        let blocked = obstacles.iter().any(|o| o.contains(eye, 0.0));
        if !blocked {
            return Framed {
                camera: cam,
                eye_blocked: false,
            };
        }
        first.get_or_insert(cam);
    }
    Framed {
        camera: first.unwrap_or_else(|| frame_from(subject, view, how.fill, yaw0, pitch0)),
        eye_blocked: true,
    }
}

/// The camera at this yaw/pitch (degrees) whose picture of the box spans `fill` of the uncovered
/// rect and is centred in it.
pub fn frame_from(subject: Bounds, view: ViewRect, fill: f32, yaw: f32, pitch: f32) -> Camera {
    let center = subject.center();
    let r = subject.radius().max(1e-3);
    let mut cam = Camera::fit(center, r);
    cam.yaw = yaw.to_radians();
    cam.pitch = pitch.to_radians();
    let (f, right, up) = cam.axes();
    let ty = (cam.fovy * 0.5).tan();
    let tx = ty * view.width.max(1.0) / view.height.max(1.0);
    let ([ox, oy], [wx, wy]) = view.clip();
    let fill = fill.clamp(0.05, 1.0);
    let corners = subject.corners().map(|p| [0, 1, 2].map(|k| p[k] - center[k]));
    // At distance d, panned so the centre lands on (ox, oy): a corner's clip x is
    // (x + ox d tx) / ((q.f + d) tx). It nears ox as d grows, so the least d that keeps every
    // corner within fill of the rect is found by bisection.
    let fits = |d: f32| {
        corners.iter().all(|q| {
            let z = dot(*q, f) + d;
            if z <= d * 0.05 {
                return false;
            }
            let x = (dot(*q, right) + ox * d * tx) / (z * tx);
            let y = (dot(*q, up) + oy * d * ty) / (z * ty);
            (x - ox).abs() <= fill * wx && (y - oy).abs() <= fill * wy
        })
    };
    let (mut lo, mut hi) = (r * 0.5, r * 4.0);
    while !fits(hi) && hi < r * 1e6 {
        hi *= 2.0;
    }
    for _ in 0..48 {
        let mid = 0.5 * (lo + hi);
        if fits(mid) { hi = mid } else { lo = mid }
    }
    let d = hi;
    // Pan: the target moves opposite the shift so the subject's centre sits at (ox, oy).
    let (px, py) = (-ox * d * tx, -oy * d * ty);
    cam.target = [0, 1, 2].map(|k| center[k] + right[k] * px + up[k] * py);
    cam.dist = d;
    cam
}

/// A grid step for a subject of this size: 1, 2 or 5 × 10ⁿ mm, about a tenth of its largest side.
pub fn grid_step(size: f32) -> f32 {
    let want = (size.abs() / 10.0).max(1e-3);
    let p = 10f32.powf(want.log10().floor());
    let m = want / p;
    let k = if m < 1.5 {
        1.0
    } else if m < 3.5 {
        2.0
    } else if m < 7.5 {
        5.0
    } else {
        10.0
    };
    k * p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip_of(cam: &Camera, p: [f32; 3], w: f32, h: f32) -> [f32; 2] {
        let (f, r, u) = cam.axes();
        let e = cam.eye();
        let q = [p[0] - e[0], p[1] - e[1], p[2] - e[2]];
        let ty = (cam.fovy * 0.5).tan();
        let tx = ty * w / h;
        let z = dot(q, f);
        [dot(q, r) / (z * tx), dot(q, u) / (z * ty)]
    }

    #[test]
    fn fills_and_centres_in_the_uncovered_rect() {
        let b = Bounds::new([0.0, 0.0, 0.0], [50.0, 10.0, 10.0]);
        let v = ViewRect {
            width: 1600.0,
            height: 1000.0,
            covered: [270.0, 0.0, 380.0, 0.0],
        };
        let cam = frame(b, v, Framing::default(), &[]).camera;
        let ([ox, oy], [wx, wy]) = v.clip();
        let pts = b.corners().map(|p| clip_of(&cam, p, v.width, v.height));
        let (mut x0, mut x1, mut y0, mut y1) = (9f32, -9f32, 9f32, -9f32);
        for [x, y] in pts {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        // inside the rect, and touching the fill bound on one axis
        assert!((x0 - ox) >= -0.7 * wx - 1e-3 && (x1 - ox) <= 0.7 * wx + 1e-3);
        assert!((y0 - oy) >= -0.7 * wy - 1e-3 && (y1 - oy) <= 0.7 * wy + 1e-3);
        let span = ((x1 - x0) / (2.0 * wx)).max((y1 - y0) / (2.0 * wy));
        assert!(span > 0.6, "span {span}");
        // the centre lands near the rect's centre
        let c = clip_of(&cam, b.center(), v.width, v.height);
        assert!(
            (c[0] - ox).abs() < 0.05 && (c[1] - oy).abs() < 0.05,
            "{c:?} vs {ox},{oy}"
        );
    }

    #[test]
    fn eye_leaves_an_obstacle() {
        let sheet = Bounds::new([-100.0, -50.0, 0.0], [100.0, 50.0, 2.0]);
        let cam0 = frame(sheet, ViewRect::full(1400.0, 860.0), Framing::default(), &[]).camera;
        // a housing wall around where the standard eye would be
        let e = cam0.eye();
        let wall = Bounds::new(
            [e[0] - 50.0, e[1] - 50.0, e[2] - 50.0],
            [e[0] + 50.0, e[1] + 50.0, e[2] + 50.0],
        );
        let f = frame(sheet, ViewRect::full(1400.0, 860.0), Framing::default(), &[wall]);
        assert!(!f.eye_blocked);
        assert!(!wall.contains(f.camera.eye(), 0.0));
    }

    #[test]
    fn grid_steps_are_round() {
        assert_eq!(grid_step(50.0), 5.0);
        assert_eq!(grid_step(2550.0), 200.0);
        assert_eq!(grid_step(150.0), 20.0);
        assert_eq!(grid_step(10.0), 1.0);
    }
}

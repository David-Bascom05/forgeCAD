//! Orbit camera: z up, looking at a target; an infinite reversed-z perspective (depth 1 at the
//! near plane, towards 0 far away), for even depth precision from millimetres to kilometres.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub fovy: f32,
    /// Turned about the view's axis, the picture clockwise.
    pub roll: f32,
    /// Parallel projection, as a very long lens: the eye far back, the field narrow (`set_parallel`).
    pub parallel: bool,
}

pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// a + b * s
pub fn along(a: [f32; 3], b: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

/// What a frame needs from the camera.
pub struct View {
    /// Column major.
    pub matrix: [f32; 16],
    pub eye: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
    /// Left, right, bottom, top, near: inside is a x + b y + c z + d >= 0, (a, b, c) unit.
    pub planes: [[f32; 4]; 5],
    pub near: f32,
    /// Pixels per mm at distance 1 from the eye.
    pub px_mm: f32,
    /// How this lens's field compares with the ordinary one, 1 at most.
    pub lens: f32,
}

/// The matrix and culling planes of clip x, y and w rows (a row of each maps a point to clip).
fn clip_rows(x: [f32; 4], y: [f32; 4], w: [f32; 4], near: f32) -> ([f32; 16], [[f32; 4]; 5]) {
    let matrix = [
        x[0], y[0], 0.0, w[0], x[1], y[1], 0.0, w[1], x[2], y[2], 0.0, w[2], x[3], y[3], near, w[3],
    ];
    let plane = |sign: f32, a: [f32; 4]| {
        let p = [0, 1, 2, 3].map(|k| w[k] + sign * a[k]);
        let l = dot([p[0], p[1], p[2]], [p[0], p[1], p[2]]).sqrt();
        p.map(|v| v / l)
    };
    let mut front = w;
    front[3] -= near;
    (
        matrix,
        [
            plane(1.0, x),
            plane(-1.0, x),
            plane(1.0, y),
            plane(-1.0, y),
            front,
        ],
    )
}

impl Camera {
    /// Looking at a sphere so it fills the view.
    pub fn fit(center: [f32; 3], radius: f32) -> Camera {
        let fovy = 45f32.to_radians();
        Camera {
            target: center,
            yaw: 35f32.to_radians(),
            pitch: 25f32.to_radians(),
            dist: radius / (fovy * 0.5).sin(),
            fovy,
            roll: 0.0,
            parallel: false,
        }
    }

    /// The distance at which a sphere just fills a window of this shape.
    pub fn fit_dist(&self, radius: f32, width: u32, height: u32) -> f32 {
        let half_y = self.fovy * 0.5;
        let half_x = (half_y.tan() * width as f32 / height as f32).atan();
        radius / half_y.min(half_x).sin()
    }

    /// Forward, right, up.
    pub fn axes(&self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let forward = [-cp * cy, -cp * sy, -sp];
        let flat = [-sy, cy, 0.0];
        let top = cross(flat, forward);
        if self.roll == 0.0 {
            return (forward, flat, top);
        }
        let (sr, cr) = self.roll.sin_cos();
        (
            forward,
            [0, 1, 2].map(|k| flat[k] * cr + top[k] * sr),
            [0, 1, 2].map(|k| top[k] * cr - flat[k] * sr),
        )
    }

    pub fn eye(&self) -> [f32; 3] {
        along(self.target, self.axes().0, -self.dist)
    }

    pub fn near(&self) -> f32 {
        (self.dist * 0.01).max(1e-3)
    }

    pub fn view(&self, width: u32, height: u32) -> View {
        self.view_window(width, height, (0.0, 0.0), (width as f32, height as f32))
    }

    /// The view of only `size` pixels of the window around `at`, stretched to fill the frame:
    /// what a pick draws. Everything else about it (the eye, the scale) is the whole view's.
    pub fn view_window(&self, width: u32, height: u32, at: (f32, f32), size: (f32, f32)) -> View {
        let (f, r, u) = self.axes();
        let eye = self.eye();
        let near = self.near();
        let fy = 1.0 / (self.fovy * 0.5).tan();
        let fx = fy * height as f32 / width as f32;
        let row = |a: [f32; 3], s: f32| [a[0] * s, a[1] * s, a[2] * s, -dot(a, eye) * s];
        let w = row(f, 1.0);
        let (sx, sy) = (width as f32 / size.0, height as f32 / size.1);
        // a whole view's centre is the window's own, so it moves nothing
        let (cx, cy) = if size == (width as f32, height as f32) {
            (0.0, 0.0)
        } else {
            (at.0 / width as f32 * 2.0 - 1.0, 1.0 - at.1 / height as f32 * 2.0)
        };
        let stretch = |a: [f32; 4], s: f32, c: f32| [0, 1, 2, 3].map(|k| (a[k] - w[k] * c) * s);
        let (x, y) = (stretch(row(r, fx), sx, cx), stretch(row(u, fy), sy, cy));
        let (matrix, planes) = clip_rows(x, y, w, near);
        View {
            matrix,
            eye,
            right: r,
            up: u,
            forward: f,
            planes,
            near,
            px_mm: 0.5 * height as f32 * fy,
            lens: ((self.fovy * 0.5).tan() / (PERSPECTIVE_FOVY * 0.5).tan()).min(1.0),
        }
    }

    /// A point on the ray through this pixel `depth` ahead of the eye (along the view, not along the ray).
    pub fn point_at(&self, px: (f32, f32), depth: f32, width: u32, height: u32) -> [f32; 3] {
        let (f, r, u) = self.axes();
        let tan = (self.fovy * 0.5).tan();
        let ny = (1.0 - px.1 / height as f32 * 2.0) * tan;
        let nx = (px.0 / width as f32 * 2.0 - 1.0) * tan * width as f32 / height as f32;
        let d = along(along(f, r, nx), u, ny);
        along(self.eye(), d, depth)
    }

    /// The pixel a point lands on, if ahead of the eye.
    pub fn pixel_of(&self, p: [f32; 3], width: u32, height: u32) -> Option<(f32, f32)> {
        let (f, r, u) = self.axes();
        let v = along(p, self.eye(), -1.0);
        let w = dot(v, f);
        if w <= 0.0 {
            return None;
        }
        let tan = (self.fovy * 0.5).tan();
        let nx = dot(v, r) / w / (tan * width as f32 / height as f32);
        let ny = dot(v, u) / w / tan;
        Some(((nx + 1.0) * 0.5 * width as f32, (1.0 - ny) * 0.5 * height as f32))
    }

    /// Turns the view by mouse pixels, swinging the eye round `pivot`.
    pub fn orbit_about(&mut self, pivot: [f32; 3], dx: f32, dy: f32) {
        let (f, r, u) = self.axes();
        let v = along(self.eye(), pivot, -1.0);
        let (a, b, c) = (dot(v, r), dot(v, u), dot(v, f));
        // a rolled view still follows the mouse on screen
        let (sr, cr) = self.roll.sin_cos();
        let (dx, dy) = (dx * cr + dy * sr, dy * cr - dx * sr);
        self.yaw -= dx * ORBIT;
        self.pitch = (self.pitch + dy * ORBIT).clamp(-PITCH_END, PITCH_END);
        let (f, r, u) = self.axes();
        let eye = along(along(along(pivot, r, a), u, b), f, c);
        self.target = along(eye, f, self.dist);
    }

    /// Moves the view by mouse pixels so a point `depth` ahead follows the cursor.
    pub fn pan(&mut self, dx: f32, dy: f32, depth: f32, height: u32) {
        let (_, r, u) = self.axes();
        let per_px = 2.0 * depth * (self.fovy * 0.5).tan() / height as f32;
        self.target = along(along(self.target, r, -dx * per_px), u, dy * per_px);
    }

    /// Moves towards `at` by wheel steps; `at` stays on the same pixel.
    pub fn zoom_at(&mut self, at: [f32; 3], steps: f32) {
        let s = (-steps * 0.12).exp();
        let (f, r, u) = self.axes();
        if self.parallel {
            // the eye stays back and the field narrows; the point keeps its pixel by a sideways move
            let v = along(at, self.eye(), -1.0);
            self.fovy = ((self.fovy * 0.5).tan() * s).atan() * 2.0;
            self.target = along(
                along(self.target, r, dot(v, r) * (1.0 - s)),
                u,
                dot(v, u) * (1.0 - s),
            );
            return;
        }
        let eye = along(at, along(self.eye(), at, -1.0), s);
        self.dist *= s;
        self.target = along(eye, f, self.dist);
    }

    /// The size of view that just fits a sphere in a window of this shape: by distance, or a
    /// parallel view by its field.
    pub fn fit_to(&mut self, radius: f32, width: u32, height: u32) {
        if self.parallel {
            self.dist = self.dist.max(radius * PARALLEL_BACK);
            let aspect = (width as f32 / height as f32).min(1.0);
            self.fovy = (radius / aspect / self.dist).atan() * 2.0;
        } else {
            self.dist = self.fit_dist(radius, width, height);
        }
    }

    /// The same picture at the target, by a very long lens or by this ordinary one.
    pub fn set_parallel(&mut self, on: bool, radius: f32) {
        if on == self.parallel {
            return;
        }
        let half = self.dist * (self.fovy * 0.5).tan();
        self.parallel = on;
        self.dist = if on {
            self.dist.max(radius * PARALLEL_BACK)
        } else {
            half / (PERSPECTIVE_FOVY * 0.5).tan()
        };
        self.fovy = (half / self.dist).atan() * 2.0;
    }

    /// Which way it looks, as a unit quaternion.
    fn turn(&self) -> [f32; 4] {
        let (f, r, u) = self.axes();
        let m = [r, u, f.map(|v| -v)];
        let t = m[0][0] + m[1][1] + m[2][2];
        let q = if t > 0.0 {
            let s = (1.0 + t).sqrt() * 2.0;
            [
                s / 4.0,
                (m[1][2] - m[2][1]) / s,
                (m[2][0] - m[0][2]) / s,
                (m[0][1] - m[1][0]) / s,
            ]
        } else if m[0][0] >= m[1][1] && m[0][0] >= m[2][2] {
            let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
            [
                (m[1][2] - m[2][1]) / s,
                s / 4.0,
                (m[1][0] + m[0][1]) / s,
                (m[2][0] + m[0][2]) / s,
            ]
        } else if m[1][1] >= m[2][2] {
            let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
            [
                (m[2][0] - m[0][2]) / s,
                (m[1][0] + m[0][1]) / s,
                s / 4.0,
                (m[2][1] + m[1][2]) / s,
            ]
        } else {
            let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
            [
                (m[0][1] - m[1][0]) / s,
                (m[2][0] + m[0][2]) / s,
                (m[2][1] + m[1][2]) / s,
                s / 4.0,
            ]
        };
        let l = q.iter().map(|v| v * v).sum::<f32>().sqrt();
        q.map(|v| v / l)
    }

    /// This camera, looking along `f` with `r` to its right (unit and square to each other).
    pub fn look(&mut self, f: [f32; 3], r: [f32; 3]) {
        self.pitch = (-f[2]).clamp(-1.0, 1.0).asin();
        if f[2].abs() > 0.99999 {
            // straight up or down: the right vector alone gives the turn
            self.yaw = (-r[0]).atan2(r[1]);
            self.roll = 0.0;
            self.pitch = if f[2] < 0.0 { PITCH_END } else { -PITCH_END };
        } else {
            self.yaw = (-f[1]).atan2(-f[0]);
            let (sy, cy) = self.yaw.sin_cos();
            let flat = [-sy, cy, 0.0];
            let top = cross(flat, f);
            self.roll = dot(r, top).atan2(dot(r, flat));
        }
    }

    /// This camera's angles set to look the way a quaternion says.
    fn turn_to(&mut self, q: [f32; 4]) {
        let [w, x, y, z] = q;
        let r = [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + w * z),
            2.0 * (x * z - w * y),
        ];
        let back = [
            2.0 * (x * z + w * y),
            2.0 * (y * z - w * x),
            1.0 - 2.0 * (x * x + y * y),
        ];
        self.look(back.map(|v| -v), r);
    }

    /// Between two views, t from 0 to 1: the shortest turn, the distance and field evenly by ratio.
    pub fn lerp(&self, to: &Camera, t: f32) -> Camera {
        if t <= 0.0 {
            return *self;
        }
        let (a, mut b) = (self.turn(), to.turn());
        let mut d: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        if d < 0.0 {
            b = b.map(|v| -v);
            d = -d;
        }
        let (ka, kb) = if d > 0.9995 {
            (1.0 - t, t)
        } else {
            let angle = d.acos();
            (
                ((1.0 - t) * angle).sin() / angle.sin(),
                (t * angle).sin() / angle.sin(),
            )
        };
        let mut q = [0, 1, 2, 3].map(|k| a[k] * ka + b[k] * kb);
        let l = q.iter().map(|v| v * v).sum::<f32>().sqrt();
        q = q.map(|v| v / l);
        let tan = (self.fovy * 0.5).tan() * ((to.fovy * 0.5).tan() / (self.fovy * 0.5).tan()).powf(t);
        let mut c = Camera {
            target: along(self.target, along(to.target, self.target, -1.0), t),
            dist: self.dist * (to.dist / self.dist).powf(t),
            fovy: tan.atan() * 2.0,
            parallel: if t < 1.0 { self.parallel } else { to.parallel },
            ..*self
        };
        if t >= 1.0 {
            c.yaw = to.yaw;
            c.pitch = to.pitch;
            c.roll = to.roll;
        } else {
            c.turn_to(q);
        }
        c
    }
}

/// How far back a parallel view's eye sits, in radii of what it shows.
pub const PARALLEL_BACK: f32 = 1000.0;
/// The ordinary lens.
pub const PERSPECTIVE_FOVY: f32 = 45.0 * core::f32::consts::PI / 180.0;

/// Radians per pixel of mouse.
const ORBIT: f32 = 0.006;
/// Straight up or down, and no further.
pub const PITCH_END: f32 = core::f32::consts::FRAC_PI_2;

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f32, f32), b: (f32, f32), tol: f32) -> bool {
        (a.0 - b.0).abs() < tol && (a.1 - b.1).abs() < tol
    }

    #[test]
    fn the_target_is_mid_screen_and_inside_the_frustum() {
        let c = Camera::fit([10.0, 20.0, 30.0], 5.0);
        let v = c.view(800, 600);
        let m = v.matrix;
        let p = [10.0f32, 20.0, 30.0, 1.0];
        let at = |r: usize| (0..4).map(|k| m[k * 4 + r] * p[k]).sum::<f32>();
        let w = at(3);
        assert!(w > 0.0);
        assert!((at(0) / w).abs() < 1e-3 && (at(1) / w).abs() < 1e-3);
        assert!((at(2) / w) > 0.0 && (at(2) / w) < 1.0);
        for pl in v.planes {
            assert!(pl[0] * p[0] + pl[1] * p[1] + pl[2] * p[2] + pl[3] > 0.0);
        }
    }

    #[test]
    fn a_window_of_the_view_puts_its_pixel_mid_frame() {
        let c = Camera::fit([0.0; 3], 50.0);
        let px = (612.0, 233.0);
        let p = c.point_at(px, c.dist, 800, 600);
        let m = c.view_window(800, 600, px, (11.0, 11.0)).matrix;
        let at = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
        assert!((at(0) / at(3)).abs() < 1e-2 && (at(1) / at(3)).abs() < 1e-2);
    }

    #[test]
    fn a_pixel_and_a_point_are_one_way_there_and_back() {
        let c = Camera::fit([3.0, -4.0, 5.0], 50.0);
        let px = (123.0, 456.0);
        let p = c.point_at(px, 200.0, 800, 600);
        assert!(close(c.pixel_of(p, 800, 600).unwrap(), px, 0.01));
    }

    #[test]
    fn orbiting_keeps_the_pivot_on_its_pixel_and_zooming_its_point() {
        let mut c = Camera::fit([0.0; 3], 50.0);
        let pivot = c.point_at((200.0, 150.0), 120.0, 800, 600);
        let before = c.pixel_of(pivot, 800, 600).unwrap();
        c.zoom_at(pivot, 3.0);
        assert!(close(c.pixel_of(pivot, 800, 600).unwrap(), before, 0.05));
        // an orbit turns the view, so the pivot holds its place in the view's own frame
        let v = along(pivot, c.eye(), -1.0);
        let (f, r, u) = c.axes();
        let was = [dot(v, f), dot(v, r), dot(v, u)];
        c.orbit_about(pivot, 40.0, -25.0);
        let v = along(pivot, c.eye(), -1.0);
        let (f, r, u) = c.axes();
        for (a, b) in was.iter().zip([dot(v, f), dot(v, r), dot(v, u)]) {
            assert!((a - b).abs() < 1e-3, "{a} {b}");
        }
    }

    #[test]
    fn panning_keeps_a_grabbed_point_under_the_cursor() {
        let mut c = Camera::fit([0.0; 3], 50.0);
        let px = (300.0, 200.0);
        let p = c.point_at(px, 150.0, 800, 600);
        c.pan(37.0, -12.0, 150.0, 600);
        assert!(close(c.pixel_of(p, 800, 600).unwrap(), (337.0, 188.0), 0.05));
    }

    #[test]
    fn a_rolled_view_turns_the_picture_and_lerp_takes_the_shortest_turn() {
        let mut c = Camera::fit([0.0; 3], 50.0);
        let (f0, r0, u0) = c.axes();
        c.roll = core::f32::consts::FRAC_PI_2;
        let (f, r, u) = c.axes();
        assert_eq!(f, f0);
        for k in 0..3 {
            assert!((r[k] - u0[k]).abs() < 1e-6 && (u[k] + r0[k]).abs() < 1e-6);
        }
        // the path's views each lie on the great circle between the ends: half way is half the angle
        let mut from = c;
        from.roll = 0.0;
        from.yaw = 0.2;
        from.pitch = 0.1;
        let mut to = from;
        to.pitch = PITCH_END;
        to.yaw = 1.5;
        let mid = from.lerp(&to, 0.5);
        let (a, b, m) = (from.axes().0, to.axes().0, mid.axes().0);
        let (ab, am) = (
            dot(a, b).clamp(-1.0, 1.0).acos(),
            dot(a, m).clamp(-1.0, 1.0).acos(),
        );
        assert!(am <= ab * 0.75, "{am} of {ab}");
        assert_eq!(from.lerp(&to, 1.0), to);
    }

    #[test]
    fn a_parallel_view_keeps_its_picture_and_zooms_in_place() {
        let mut c = Camera::fit([3.0, 2.0, 1.0], 50.0);
        let p = [20.0, 10.0, 5.0];
        let was = c.pixel_of(c.target, 800, 600).unwrap();
        let scale = c.dist * (c.fovy * 0.5).tan();
        c.set_parallel(true, 50.0);
        assert!(c.parallel && c.dist >= 50.0 * PARALLEL_BACK);
        assert!((c.dist * (c.fovy * 0.5).tan() - scale).abs() < scale * 1e-3);
        assert!(close(c.pixel_of(c.target, 800, 600).unwrap(), was, 0.05));
        // the far side and the near side of the part hardly differ in scale
        let (f, r, _) = c.axes();
        let side = [0, 1, 2].map(|k| c.target[k] + r[k] * 40.0);
        let (near_p, far_p) = (
            [0, 1, 2].map(|k| side[k] - f[k] * 40.0),
            [0, 1, 2].map(|k| side[k] + f[k] * 40.0),
        );
        let (a, b) = (
            c.pixel_of(near_p, 800, 600).unwrap(),
            c.pixel_of(far_p, 800, 600).unwrap(),
        );
        assert!((a.0 - b.0).abs() < 0.5, "{a:?} {b:?}");
        // a point stays on its pixel through a zoom, the eye doesn't come in
        let at = c.pixel_of(p, 800, 600).unwrap();
        let d = c.dist;
        c.zoom_at(p, 6.0);
        assert_eq!(c.dist, d);
        assert!(close(c.pixel_of(p, 800, 600).unwrap(), at, 0.05));
        c.set_parallel(false, 50.0);
        assert!(!c.parallel && (c.fovy - PERSPECTIVE_FOVY).abs() < 1e-3);
    }

    #[test]
    fn straight_down_is_reachable_and_the_short_way_round_is_taken() {
        let mut c = Camera::fit([0.0; 3], 50.0);
        c.pitch = PITCH_END;
        let (f, r, u) = c.axes();
        assert!((f[2] + 1.0).abs() < 1e-6 && dot(r, r) > 0.99 && dot(u, u) > 0.99);
        let mut to = c;
        c.yaw = 3.0;
        to.yaw = -3.0;
        let mid = c.lerp(&to, 0.5);
        assert!((mid.yaw.abs() - core::f32::consts::PI).abs() < 0.3);
    }
}

//! Orbit camera: z up, looking at a target; an infinite reversed-z perspective (depth 1 at the
//! near plane, towards 0 far away), for even depth precision from millimetres to kilometres.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub fovy: f32,
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
        let right = [-sy, cy, 0.0];
        (forward, right, cross(right, forward))
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
        let (f, _, _) = self.axes();
        let eye = along(at, along(self.eye(), at, -1.0), s);
        self.dist *= s;
        self.target = along(eye, f, self.dist);
    }

    /// Between two views, t from 0 to 1: the short way round, the distance evenly by ratio.
    pub fn lerp(&self, to: &Camera, t: f32) -> Camera {
        let tau = core::f32::consts::TAU;
        let turn = (to.yaw - self.yaw + core::f32::consts::PI).rem_euclid(tau) - core::f32::consts::PI;
        Camera {
            target: along(self.target, along(to.target, self.target, -1.0), t),
            yaw: self.yaw + turn * t,
            pitch: self.pitch + (to.pitch - self.pitch) * t,
            dist: self.dist * (to.dist / self.dist).powf(t),
            fovy: self.fovy,
        }
    }
}

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

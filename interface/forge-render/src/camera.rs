//! Orbit camera: z up, looking at a target; an infinite reversed-z perspective (depth 1 at the
//! near plane, towards 0 far away), for even depth precision from millimetres to kilometres.

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub fovy: f32,
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt();
    a.map(|v| v / l)
}

/// What a frame needs from the camera.
pub struct View {
    /// Column major.
    pub matrix: [f32; 16],
    pub eye: [f32; 3],
    /// Left, right, bottom, top, near: inside is a x + b y + c z + d >= 0, (a, b, c) unit.
    pub planes: [[f32; 4]; 5],
    pub near: f32,
    /// Pixels per mm at distance 1 from the eye.
    pub px_mm: f32,
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

    fn axes(&self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let forward = [-cp * cy, -cp * sy, -sp];
        let right = unit(cross(forward, [0.0, 0.0, 1.0]));
        (forward, right, cross(right, forward))
    }

    pub fn view(&self, width: u32, height: u32) -> View {
        let (f, r, u) = self.axes();
        let eye = [0, 1, 2].map(|k| self.target[k] - f[k] * self.dist);
        let near = (self.dist * 0.01).max(1e-3);
        let fy = 1.0 / (self.fovy * 0.5).tan();
        let fx = fy * height as f32 / width as f32;
        let row = |a: [f32; 3], s: f32| [a[0] * s, a[1] * s, a[2] * s, -dot(a, eye) * s];
        let (x, y, w) = (row(r, fx), row(u, fy), row(f, 1.0));
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
        View {
            matrix,
            eye,
            planes: [
                plane(1.0, x),
                plane(-1.0, x),
                plane(1.0, y),
                plane(-1.0, y),
                front,
            ],
            near,
            px_mm: 0.5 * height as f32 * fy,
        }
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.006;
        self.pitch = (self.pitch + dy * 0.006).clamp(-1.55, 1.55);
    }

    /// Moves the view by pixels on screen, at the target's depth.
    pub fn pan(&mut self, dx: f32, dy: f32, height: u32) {
        let (_, r, u) = self.axes();
        let per_px = 2.0 * self.dist * (self.fovy * 0.5).tan() / height as f32;
        for k in 0..3 {
            self.target[k] += (-dx * r[k] + dy * u[k]) * per_px;
        }
    }

    pub fn zoom(&mut self, steps: f32) {
        self.dist *= (-steps * 0.12).exp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

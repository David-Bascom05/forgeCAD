//! The fixed camera path and what `--bench` prints.

use crate::camera::Camera;

/// An orbit around a centre, breathing between two distances, so a run sees the whole scene from
/// outside and then parts close up.
#[derive(Clone, Copy, Debug)]
pub struct Path {
    pub center: [f32; 3],
    pub far: f32,
    pub near: f32,
    pub frames: usize,
}

impl Path {
    pub fn at(&self, frame: usize) -> Camera {
        let t = frame as f32 / self.frames as f32;
        let tau = core::f32::consts::TAU;
        let breath = 0.5 - 0.5 * (t * tau * 2.0).cos();
        Camera {
            target: self.center,
            yaw: t * tau,
            pitch: 0.35 * (t * tau * 3.0).sin() + 0.2,
            dist: self.far * (self.near / self.far).powf(breath),
            fovy: 45f32.to_radians(),
        }
    }
}

pub struct Summary {
    pub mean: f64,
    pub p99: f64,
    pub worst: f64,
}

pub fn summary(ms: &[f64]) -> Summary {
    let mut v = ms.to_vec();
    v.sort_by(f64::total_cmp);
    let at = |q: f64| v[((v.len() - 1) as f64 * q).ceil() as usize];
    Summary {
        mean: v.iter().sum::<f64>() / v.len() as f64,
        p99: at(0.99),
        worst: v[v.len() - 1],
    }
}

pub fn report(mode: &str, frame: &[f64], gpu: &[f64], adapter: &wgpu::AdapterInfo) -> String {
    let f = summary(frame);
    let g = summary(gpu);
    format!(
        "gpu: {} ({:?}, {})\n{mode}: {} frames  frame mean {:.2} p99 {:.2} worst {:.2} ms  gpu mean {:.2} p99 {:.2} worst {:.2} ms",
        adapter.name,
        adapter.device_type,
        adapter.backend,
        frame.len(),
        f.mean,
        f.p99,
        f.worst,
        g.mean,
        g.p99,
        g.worst
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_p99_of_a_hundred_frames_is_the_worst_one() {
        let ms: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let s = summary(&ms);
        assert_eq!((s.mean, s.p99, s.worst), (50.5, 100.0, 100.0));
        let s = summary(&[3.0]);
        assert_eq!((s.mean, s.p99, s.worst), (3.0, 3.0, 3.0));
    }
}

//! The Render Studio: photorealistic renders of parts and assemblies by a progressive path tracer
//! on the GPU (`trace.wgsl`, native and WebGPU), physically based appearances, image-based
//! lighting, a camera with depth of field, a denoiser, and renders to files. Design:
//! `research/render/studio.md`. The modules without a GPU (`bsdf`, `bvh`, `env`, `scene`, `cpu`)
//! are the tracer's maths in plain Rust, tested on any machine.

pub mod appearance;
pub mod bsdf;
pub mod bvh;
pub mod cpu;
pub mod env;
pub mod gpu;
pub mod scene;
#[cfg(not(target_arch = "wasm32"))]
pub mod shot;
pub mod v3;

#[cfg(test)]
mod bsdf_tests;

use crate::camera::Camera;

/// What a camera ray sees where it misses everything.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Backdrop {
    /// The lighting's own map.
    Environment,
    /// A flat colour, linear RGB.
    Colour([f32; 3]),
    /// Nothing: alpha 0, for compositing.
    Transparent,
}

/// The camera, as the tracer takes it: in mm, unit axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub eye: [f32; 3],
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    /// The vertical field, radians (perspective).
    pub fovy: f32,
    /// Half the height seen, mm, for a parallel view; 0 for perspective.
    pub parallel: f32,
    /// The lens's radius, mm; 0 is a pinhole (all in focus).
    pub aperture: f32,
    /// The distance in focus, mm.
    pub focus: f32,
}

/// The height of a 35 mm frame (36 x 24): focal lengths are given against it.
pub const FRAME_MM: f32 = 24.0;

impl View {
    /// The viewport's camera as it is.
    pub fn of(c: &Camera) -> View {
        let (forward, right, up) = c.axes();
        let parallel = if c.parallel {
            c.dist * (c.fovy * 0.5).tan()
        } else {
            0.0
        };
        View {
            eye: c.eye(),
            forward,
            right,
            up,
            fovy: c.fovy,
            parallel,
            aperture: 0.0,
            focus: c.dist,
        }
    }

    /// The focal length, mm, of a 35 mm camera seeing this field.
    pub fn focal_mm(&self) -> f32 {
        FRAME_MM * 0.5 / (self.fovy * 0.5).tan()
    }

    /// A lens of this focal length (mm, 35 mm frame) at this f-number, focused at `focus` mm:
    /// the depth of field a real camera would have at the part's real size.
    pub fn with_lens(mut self, focal_mm: f32, f_number: Option<f32>, focus: f32) -> View {
        self.fovy = 2.0 * (FRAME_MM * 0.5 / focal_mm).atan();
        self.aperture = f_number.map_or(0.0, |n| focal_mm / n * 0.5);
        self.focus = focus;
        self
    }
}

/// Everything about a render but the scene and the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub bounces: u32,
    pub seed: u32,
    /// Stops of exposure.
    pub exposure: f32,
    /// The map turned about z, radians, and its strength.
    pub env_turn: f32,
    pub env_strength: f32,
    /// The most one path may add to a pixel (fireflies); large is unbiased.
    pub clamp: f32,
    /// A floor: its height and its appearance's index.
    pub ground: Option<(f32, u32)>,
    pub backdrop: Backdrop,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            bounces: 8,
            seed: 0,
            exposure: 0.0,
            env_turn: 0.0,
            env_strength: 1.0,
            clamp: 1.0e4,
            ground: None,
            backdrop: Backdrop::Environment,
        }
    }
}

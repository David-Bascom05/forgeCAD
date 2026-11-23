//! The Render Studio: photorealistic renders of parts and assemblies by a progressive path tracer
//! on the GPU (`trace.wgsl`, native and WebGPU), physically based appearances, image-based
//! lighting, a camera with depth of field, a denoiser, and renders to files. Design:
//! `research/render/studio.md`. The modules without a GPU (`bsdf`, `v3`) are the tracer's
//! maths in plain Rust, tested on any machine.

pub mod bsdf;
pub mod v3;

#[cfg(test)]
mod bsdf_tests;

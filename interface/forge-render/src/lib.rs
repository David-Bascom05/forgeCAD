//! The ForgeCAD renderer, native on the GPU and as wasm on WebGPU. It draws what `mesh` holds;
//! natively `tess` builds that from the kernel, in the browser it arrives as a file.

pub mod bench;
pub mod camera;
pub mod mesh;
pub mod render;
pub mod scene;
pub mod viewer;

#[cfg(not(target_arch = "wasm32"))]
pub mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod load;
#[cfg(not(target_arch = "wasm32"))]
pub mod tess;
#[cfg(target_arch = "wasm32")]
mod web;

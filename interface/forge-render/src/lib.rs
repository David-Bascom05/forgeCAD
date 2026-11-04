//! The ForgeCAD renderer, native on the GPU and as wasm on WebGPU. It draws what `mesh` holds;
//! natively `tess` builds that from the kernel, in the browser it arrives as a file.

pub mod bench;
pub mod camera;
pub mod explode;
pub mod livegpu;
pub mod mesh;
pub mod nav;
pub mod origin;
pub mod overlay;
pub mod render;
pub mod scene;
pub mod shell;
pub mod viewcube;
pub mod viewer;

#[cfg(not(target_arch = "wasm32"))]
pub mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod asmshot;
#[cfg(not(target_arch = "wasm32"))]
pub mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod dragbench;
#[cfg(not(target_arch = "wasm32"))]
pub mod fg;
#[cfg(not(target_arch = "wasm32"))]
pub mod gui;
#[cfg(not(target_arch = "wasm32"))]
pub mod live;
#[cfg(not(target_arch = "wasm32"))]
pub mod load;
#[cfg(not(target_arch = "wasm32"))]
pub mod measure;
#[cfg(not(target_arch = "wasm32"))]
pub mod snaps;
#[cfg(not(target_arch = "wasm32"))]
pub mod tess;
#[cfg(target_arch = "wasm32")]
mod web;

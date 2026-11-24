//! The ForgeCAD renderer, native on the GPU and as wasm on WebGPU. It draws what `mesh` holds;
//! natively `tess` builds that from the kernel, in the browser it arrives as a file.

pub mod bench;
pub mod bindings;
pub mod camera;
pub mod camsim;
pub mod dwgview;
pub mod explode;
pub mod frame;
pub mod ghost;
pub mod library_mesh;
pub mod livegpu;
pub mod look;
pub mod machine_mesh;
pub mod mesh;
pub mod nav;
pub mod origin;
pub mod overlay;
pub mod palette;
pub mod parity;
pub mod plat;
pub mod render;
pub mod scene;
pub mod shell;
pub mod thumb;
pub mod viewcube;
pub mod viewer;

pub mod agent;
#[cfg(not(target_arch = "wasm32"))]
pub mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod asmshot;
pub mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod dragbench;
pub mod fg;
pub mod gui;
pub mod gui_camsim_panel;
pub mod live;
pub mod load;
pub mod measure;
pub mod snaps;
pub mod speech;
pub mod studio;
pub mod tess;
pub mod thermo_view;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod webapp;
#[cfg(target_arch = "wasm32")]
mod webdwg;
#[cfg(target_arch = "wasm32")]
mod webinput;

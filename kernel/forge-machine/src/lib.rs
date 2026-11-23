//! forge-machine: machine tools as data and as parts, and the simulation of posted G-code on
//! them (research/cam/design.md, COORDINATOR §4.7; docs/cam-machine.md).
//!
//! - `def`: the machine-definition format (kinematic tree, spindles, turrets, changer, mounts,
//!   collision bodies) and its structural validation.
//! - `library`: the built-in generic machines (VMC, trunnion 5-axis, lathe, mill-turn).
pub mod def;
pub mod library;

pub use def::{DefError, Kind, MachineDef};

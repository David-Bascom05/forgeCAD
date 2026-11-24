//! forge-machine: machine tools as data and as parts, and the simulation of posted G-code on
//! them (research/cam/design.md, COORDINATOR §4.7; docs/cam-machine.md).
//!
//! - `def`: the machine-definition format (kinematic tree, spindles, turrets, changer, mounts,
//!   collision bodies) and its structural validation.
//! - `library`: the built-in generic machines (VMC, trunnion 5-axis, lathe, mill-turn); each
//!   body's real part is a `.fg` file under parts/ (checked by interface/forge-machine-lang).
//! - `kin`: forward kinematics, in floats and in intervals.
//! - `gcode`: the interpreter (posted text to control events, exact integers); its Lean spec is
//!   verify/forge-machine.
//! - `sim`: events to joint motion and the timeline; `setup`: setup files.
//! - `collide`: every body through the motion; `removal`: the stock as cut; `multi`: several
//!   channels synchronised at waits, checked over every timing within the bounds.
pub mod collide;
pub mod comp;
pub mod def;
pub mod gcode;
pub mod homing;
pub mod kin;
pub mod library;
pub mod orient;
pub mod multi;
pub mod removal;
pub mod setup;
pub mod siemens;
pub mod sim;

pub use def::{DefError, Kind, MachineDef};

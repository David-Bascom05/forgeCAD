//! The machine builder: make and keep your own machine tools without writing TOML by hand
//! (research/cam/machine-builder.md).
//!
//! - `write`: a definition as canonical text (round-trips byte-stable).
//! - `catalog`: the project / user / built-in levels, with what each machine hides.
//! - `make`: templates and clones, saved to a level's folder.
//! - `models`: models the user attaches (their own STEP/STL), kept in their library only.
//! - `check`: validation, self-collision at home and through travel, provenance per value.
pub mod catalog;
pub mod check;
pub mod cli;
pub mod make;
pub mod models;
pub mod write;

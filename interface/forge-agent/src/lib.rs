//! ForgeCAD for agents: a plain-text part file, selectors that find entities without knowing
//! their names, measuring, a small PNG renderer, and a session that regenerates incrementally.

pub mod bench;
pub mod chat_bridge;
pub mod cmd;
pub mod facemesh;
pub mod features;
pub mod handles;
pub mod help;
pub mod history;
pub mod ipc;
pub mod json;
pub mod live;
pub mod measure;
pub mod model;
pub mod png;
pub mod render;
pub mod select;
pub mod tools;
pub use forge_doc::{expr, sketch, text};

/// A number for people: a few decimals, trailing zeros dropped.
pub fn short(x: f64) -> String {
    let p = if x.abs() >= 1000.0 {
        1
    } else if x.abs() >= 1.0 {
        3
    } else {
        4
    };
    let s = format!("{x:.p$}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

//! What the CLI, the app and the importers share about a built part: selectors that find
//! entities without knowing their names, measuring, face meshes, a small PNG renderer, JSON
//! replies, and the IPC and chat bridges.

pub mod chat_bridge;
pub mod facemesh;
pub mod features;
pub mod ipc;
pub mod json;
pub mod measure;
pub mod png;
pub mod render;
pub mod select;

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

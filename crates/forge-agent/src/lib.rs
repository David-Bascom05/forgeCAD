//! ForgeCAD for agents: a plain-text part file, selectors that find entities without knowing
//! their names, measuring, a small PNG renderer, and a session that regenerates incrementally.

pub mod cmd;
pub mod facemesh;
pub mod json;
pub mod measure;
pub mod model;
pub mod png;
pub mod render;
pub mod select;
pub mod text;

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

//! A body scaled uniformly about a point (forge_geom::similar): a similarity, so the same faces,
//! edges, vertices, loops and shells, with the same names and the same senses, each on the image of
//! its surface or curve. No reflection, so no loop is turned.
//!
//! Tolerances. A vertex's or an edge's tolerance is a length (how far apart its curves or faces may
//! be), so a gap of g becomes k g. The rule: tol' = max(LINEAR, k tol) where tol > LINEAR, and LINEAR
//! stays LINEAR. LINEAR is the kernel's floor, the tolerance of a vertex settled to the precision of
//! its arithmetic, whose gaps are rounding and do not grow with k; a tolerance above it was measured
//! (a boolean that could not settle the vertex, a file's deviation) and scales with what it
//! measured, never below the floor. An intersection curve's sag scales (forge_geom::similar).

use crate::{Body, Name};
use forge_geom::similar::{BadScale, check};
use forge_math::{Point3, tol};

/// Why a body cannot be scaled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScaleError {
    Bad(BadScale),
    /// The body was not compacted: a face or edge removed by an in-place edit is still held.
    Dead(Name),
}

/// A scaled tolerance, by the rule above.
pub fn scaled_tol(t: f64, k: f64) -> f64 {
    if t > tol::LINEAR {
        (t * k).max(tol::LINEAR)
    } else {
        t
    }
}

/// `body` (compacted) scaled by `k` (finite, > 0) about `c` (finite).
pub fn scaled(body: &Body, c: Point3, k: f64) -> Result<Body, ScaleError> {
    check(c, k).map_err(ScaleError::Bad)?;
    if let Some(f) = body.faces.iter().find(|f| !f.alive) {
        return Err(ScaleError::Dead(f.name));
    }
    if let Some(e) = body.edges.iter().find(|e| !e.alive) {
        return Err(ScaleError::Dead(e.name));
    }
    let mut out = body.clone();
    for f in &mut out.faces {
        f.surface = f.surface.scaled(c, k);
        f.bbox = Default::default();
        f.query = Default::default();
    }
    for e in &mut out.edges {
        (e.curve, e.range) = e.curve.scaled(e.range, c, k);
        e.tol = scaled_tol(e.tol, k);
        e.bbox = Default::default();
    }
    for v in &mut out.vertices {
        v.point = forge_geom::similar::point(v.point, c, k);
        v.tol = scaled_tol(v.tol, k);
    }
    out.hulls = Default::default();
    Ok(out)
}

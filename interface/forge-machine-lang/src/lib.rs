//! Machine tools as parts (COORDINATOR §4.7: "a machine library modelled as real parts").
//!
//! A machine definition's bodies (kernel/forge-machine) are collision envelopes: boxes and
//! cylinders the check reasons about exactly. Each body names the real part it stands for, a
//! `.fg` file. This builds those parts with forge-lang and checks every part lies inside its
//! envelope: every vertex of its tessellation (points on the exact surface) inside one of the
//! body's primitives, and the tessellation within `chord` of the surface, so the part lies in
//! the envelope grown by `chord` (0.01 mm by default; the collision check's clearance is 1 mm).
use forge_machine::def::{MachineDef, Prim};
use forge_mesh::{Quality, tessellate};
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct PartCheck {
    pub body: String,
    pub part: String,
    pub volume: f64,
    /// The envelope's volume (the sum of its primitives': an upper bound when they overlap).
    pub envelope: f64,
    pub vertices: usize,
    /// Vertices outside every primitive by more than the tolerance, with how far.
    pub outside: Vec<([f64; 3], f64)>,
}

/// How far a point lies outside a primitive (≤ 0 inside).
pub fn outside(p: &Prim, x: [f64; 3]) -> f64 {
    match p {
        Prim::Box { min, max } => {
            let mut d: f64 = f64::NEG_INFINITY;
            for k in 0..3 {
                d = d.max(min[k] - x[k]).max(x[k] - max[k]);
            }
            d
        }
        Prim::Cyl { base, axis, r, h } => {
            let w = [x[0] - base[0], x[1] - base[1], x[2] - base[2]];
            let z = w[0] * axis[0] + w[1] * axis[1] + w[2] * axis[2];
            let rad = [w[0] - z * axis[0], w[1] - z * axis[1], w[2] - z * axis[2]];
            let rho = (rad[0] * rad[0] + rad[1] * rad[1] + rad[2] * rad[2]).sqrt();
            (rho - r).max(-z).max(z - h)
        }
    }
}

fn prim_volume(p: &Prim) -> f64 {
    match p {
        Prim::Box { min, max } => (max[0] - min[0]) * (max[1] - min[1]) * (max[2] - min[2]),
        Prim::Cyl { r, h, .. } => forge_math::dmath::PI * r * r * h,
    }
}

/// Builds every body's part of a machine (paths relative to `dir`) and checks it against its
/// envelope with tolerance `tol` (≥ the tessellation's chord).
pub fn check_parts(def: &MachineDef, dir: &Path, tol: f64) -> Result<Vec<PartCheck>, String> {
    let q = Quality {
        chord: tol.min(Quality::default().chord),
        ..Quality::default()
    };
    let mut out = Vec::new();
    for b in &def.bodies {
        let Some(part) = &b.part else { continue };
        let path = dir.join(part);
        let src = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let opts = forge_lang::Options {
            dir: path.parent().map(Path::to_path_buf),
            ..forge_lang::Options::default()
        };
        let (run, _) = forge_lang::run(&src, opts, forge_lang::build::Cache::default());
        if !run.ok() {
            return Err(format!("{}: {:?}", path.display(), run.errors));
        }
        let body = run.body.ok_or_else(|| format!("{}: no body", path.display()))?;
        let mesh = tessellate(&body, &q).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let mut bad = Vec::new();
        for p in &mesh.positions {
            let x = [p.x, p.y, p.z];
            let d = b
                .prims
                .iter()
                .map(|pr| outside(pr, x))
                .fold(f64::INFINITY, f64::min);
            if d > tol {
                bad.push((x, d));
            }
        }
        out.push(PartCheck {
            body: b.name.clone(),
            part: part.clone(),
            volume: run.measure.as_ref().map(|m| m.volume).unwrap_or(f64::NAN),
            envelope: b.prims.iter().map(prim_volume).sum(),
            vertices: mesh.positions.len(),
            outside: bad,
        });
    }
    Ok(out)
}

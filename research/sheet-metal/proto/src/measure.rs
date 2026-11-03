//! The checks a built part must pass, against closed forms the construction doesn't use.
use forge_math::hash::BitHasher;
use forge_mesh::{Quality, tessellate};
use forge_step::{Options, Schema, export};
use forge_topo::mass::mass;
use forge_topo::{Body, check};

#[derive(Debug)]
pub struct Report {
    pub problems: usize,
    pub first_problem: Option<String>,
    pub faces: usize,
    pub edges: usize,
    pub open_edges: Option<usize>,
    pub mesh_volume: f64,
    pub mesh_area: f64,
    pub volume: f64,
    pub area: f64,
    pub err: [f64; 2],
    pub mass_failed: Option<String>,
    pub bits: u64,
}

pub fn bits(b: &Body) -> u64 {
    let mut h = BitHasher::new();
    for v in &b.vertices {
        h.p3(v.point);
    }
    for e in &b.edges {
        h.f64(e.range.lo);
        h.f64(e.range.hi);
    }
    h.u64(b.faces.len() as u64);
    h.finish()
}

pub fn report(b: &Body) -> Report {
    let probs = check(b);
    let mesh = tessellate(
        b,
        &Quality {
            chord: 0.002,
            ..Quality::default()
        },
    );
    let (open, mv, ma) = match &mesh {
        Ok(m) => (Some(m.open_edges().len()), m.volume(), m.area()),
        Err(_) => (None, f64::NAN, f64::NAN),
    };
    let (vol, area, err, failed) = match mass(b) {
        Ok(m) => (m.volume, m.area, [m.err[0], m.err[1]], None),
        Err(e) => (f64::NAN, f64::NAN, [f64::NAN; 2], Some(format!("{e}"))),
    };
    Report {
        problems: probs.len(),
        first_problem: probs.first().map(|p| format!("{p:?}")),
        faces: b.face_ids().count(),
        edges: b.edge_ids().count(),
        open_edges: open,
        mesh_volume: mv,
        mesh_area: ma,
        volume: vol,
        area,
        err,
        mass_failed: failed,
        bits: bits(b),
    }
}

/// The body as a STEP file, written where asked; the OCCT oracle reads it.
pub fn write_step(b: &Body, path: &str) -> Result<(usize, f64), String> {
    let e = export(
        b,
        &Options {
            schema: Schema::Ap214,
            ..Options::default()
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    std::fs::write(path, &e.text).map_err(|e| format!("{e}"))?;
    Ok((e.faces, e.uncertainty))
}

//! Shared helpers: a document's exact build hash and mesh hash.
use forge_math::hash::BitHasher;

pub struct Built {
    pub ok: bool,
    pub body_hash: u64,
    pub mesh_hash: u64,
    pub faces: usize,
    pub tris: usize,
    pub verts: usize,
    pub mesh_bytes: usize,
    pub errors: usize,
    pub build_ms: f64,
    pub mesh_ms: f64,
}

pub fn now_ms() -> f64 {
    forge_math::clock::system_now()
        .duration_since(forge_math::clock::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1e3)
}

pub fn opts(path: &std::path::Path) -> forge_lang::Options {
    forge_lang::Options {
        dir: path.parent().map(|p| p.to_path_buf()),
        ..forge_lang::Options::default()
    }
}

/// Hash of the body's exact bits: every vertex point, edge range, face count (as tools/speed's
/// `bits` example), plus the error kinds; and of its mesh at the default quality.
pub fn hash_run(run: &forge_lang::Run, build_ms: f64) -> Built {
    let mut h = BitHasher::new();
    let mut m = BitHasher::new();
    for e in &run.errors {
        for b in e.kind().bytes() {
            h.u64(b as u64);
        }
    }
    h.u64(run.errors.len() as u64);
    let (mut faces, mut tris, mut verts, mut mesh_bytes, mut mesh_ms) = (0, 0, 0, 0, 0.0);
    if let Some(b) = &run.body {
        for v in &b.vertices {
            h.p3(v.point);
        }
        for e in &b.edges {
            h.f64(e.range.lo);
            h.f64(e.range.hi);
        }
        faces = b.faces.len();
        h.u64(faces as u64);
        let t0 = now_ms();
        let q = forge_mesh::tessellate::Quality::default();
        let r = if b.from_file {
            forge_mesh::tessellate::tessellate_tolerant(b, &q)
        } else {
            forge_mesh::tessellate::tessellate(b, &q)
        };
        mesh_ms = now_ms() - t0;
        match r {
            Ok(mesh) => {
                for p in &mesh.positions {
                    m.p3(*p);
                }
                for t in &mesh.triangles {
                    m.u64(t[0] as u64 | (t[1] as u64) << 21 | (t[2] as u64) << 42);
                }
                tris = mesh.triangles.len();
                verts = mesh.positions.len();
                // what a viewer upload would carry: f32 xyz + f32 normal per vertex, u32 x3 per tri
                mesh_bytes = verts * 24 + tris * 12;
            }
            Err(_) => m.u64(u64::MAX),
        }
    } else {
        h.u64(u64::MAX);
    }
    Built {
        ok: run.body.is_some() && run.errors.is_empty(),
        body_hash: h.finish(),
        mesh_hash: m.finish(),
        faces,
        tris,
        verts,
        mesh_bytes,
        errors: run.errors.len(),
        build_ms,
        mesh_ms,
    }
}

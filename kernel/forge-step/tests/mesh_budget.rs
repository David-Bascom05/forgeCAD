//! A body read from a file is drawn within a budget of its own outside an import, and within the
//! import's inside one. HFL-1313-3 #69702 (taken out of the Haas machine models with
//! tools/kernel-stress/extract.py): 84 faces whose mesh spends about 150M evaluations, more than
//! a file's read is given (FILE, 33.5M) and less than a mesh's (MESH). Under FILE it was refused
//! "the boundary would take too long", with 101 more of the corpus's solids.
use forge_mesh::tessellate::{MESH, tessellate_tolerant};
use forge_mesh::{CdtError, Fault, Quality, TessError};
use forge_step::{Limits, import};
use forge_topo::Body;

fn heavy() -> Body {
    let p = format!(
        "{}/tests/data/haas/hfl1313_heavy_mesh.step",
        env!("CARGO_MANIFEST_DIR")
    );
    let imp = import(&std::fs::read(&p).unwrap(), &Limits::default()).unwrap();
    assert!(imp.refusals.is_empty(), "{:?}", imp.refusals);
    assert_eq!(imp.solids.len(), 1);
    imp.solids.into_iter().next().unwrap().body
}

/// What the mesh spends, measured under a budget it cannot run out of.
fn spend(b: &Body) -> u64 {
    let _b = forge_geom::budget::scoped(u64::MAX >> 1);
    let before = forge_geom::budget::left();
    tessellate_tolerant(b, &Quality::default()).unwrap();
    before - forge_geom::budget::left()
}

#[test]
fn a_heavy_file_body_meshes_outside_an_import() {
    let b = heavy();
    let s = spend(&b);
    assert!(
        s > forge_geom::budget::FILE && s < MESH,
        "spends {s}: the fixture no longer sits between FILE and MESH"
    );
    let t = tessellate_tolerant(&b, &Quality::default()).unwrap();
    assert!(t.open_edges().is_empty(), "{} open edges", t.open_edges().len());
    assert!(t.volume() > 0.0);
}

#[test]
fn inside_an_import_the_imports_smaller_budget_holds() {
    let b = heavy();
    // an import's budget is live (as `forge_step::import`'s own check meshes under it)
    let _import = forge_geom::budget::scoped(forge_geom::budget::FILE);
    let r = tessellate_tolerant(&b, &Quality::default());
    assert!(
        matches!(
            r,
            Err(TessError::Face {
                error: CdtError::Internal(Fault::BoundaryTooLong),
                ..
            })
        ),
        "{:?}",
        r.map(|m| m.triangles.len())
    );
}

//! The thermoform process sheet as the shop gets it: its PDF text in WinAnsi, its vent table.

#[test]
fn the_process_sheet_prints_its_signs_and_no_negative_zero() {
    let src = include_str!("../../../examples/thermoform/cover.fg");
    let opts = forge_lang::Options {
        dir: Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/thermoform").into()),
        ..Default::default()
    };
    let r = forge_lang::run(src, opts, Default::default()).0;
    let o = r.thermos.first().expect("the cover's thermoform");
    let (_, gfx) = forge_render::thermo_view::view(o, &r.solids, 3.0);
    let sheet = &gfx.sheet;
    assert!(
        sheet.iter().any(|l| l.starts_with("Blank") && l.contains('×')),
        "{sheet:?}"
    );
    assert!(!sheet.iter().any(|l| l.contains("-0.00")), "{sheet:?}");
    assert!(!sheet.last().unwrap().is_empty(), "{sheet:?}");
    let pdf = String::from_utf8_lossy(&forge_render::thermo_view::process_pdf(sheet)).to_string();
    assert!(pdf.contains("/Encoding /WinAnsiEncoding"));
    // × is WinAnsi 0xD7
    assert!(pdf.contains(" \\327 ") && !pdf.contains(" ? "), "{pdf}");
}

#[test]
fn the_view_hides_a_solid_by_its_eye_and_draws_the_simulated_part_unpicked() {
    use forge_render::thermo_view::{UNPICKED, compose, view};
    let src = include_str!("../../../examples/thermoform/cover.fg");
    let r = forge_lang::run(src, Default::default(), Default::default()).0;
    let o = r.thermos.first().expect("the cover's thermoform");
    let (row, gfx) = view(o, &r.solids, 3.0);
    let full = forge_render::tess::build_part(r.body.as_ref().unwrap()).unwrap();
    let mould = gfx.mould_span.clone().expect("the mould's faces");
    let part = gfx.part_span.clone().expect("the part's faces");
    // simulated, mould shown: the modelled part's faces go, the simulated part's come, unpicked
    let shown = compose(&full, &[&part], &[gfx.sim.as_ref().unwrap()]);
    for (a, b) in full.lods.iter().zip(&shown.lods) {
        let nv = (b.vertices.len() / forge_render::mesh::VERTEX) as u32;
        assert!(b.faces.iter().all(|f| !part.faces.contains(&f.face)));
        assert!(b.edges.iter().all(|e| !part.edges.contains(&e.edge)));
        let ids: Vec<u32> = (0..b.indices.len() / 4)
            .map(|k| u32::from_le_bytes(b.indices[k * 4..k * 4 + 4].try_into().unwrap()))
            .collect();
        assert!(ids.iter().all(|&i| i < nv));
        let kept = a.faces.iter().filter(|f| mould.faces.contains(&f.face)).count();
        assert_eq!(
            b.faces.iter().filter(|f| mould.faces.contains(&f.face)).count(),
            kept
        );
        assert!(b.faces.iter().any(|f| f.face >= UNPICKED));
        // each face's triangles where its range says
        let mut at = 0;
        for f in &b.faces {
            assert_eq!(f.first_tri, at);
            at += f.tris;
        }
        assert_eq!(at, b.tris());
    }
    assert!(row.simulated.as_ref().unwrap().volume < row.modelled.as_ref().unwrap().volume);
    assert!(!gfx.sim_heat.is_empty() && gfx.sim_heat.len() % 3 == 0);
}

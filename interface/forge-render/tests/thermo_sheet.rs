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
    let (row, mut gfx) = view(o, &r.solids, 3.0);
    let full = forge_render::tess::build_part(r.body.as_ref().unwrap()).unwrap();
    gfx.made_now(None);
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

/// The wall's colours are the legend's: the band at the model's thinnest point is the legend's MIN
/// (dark red), at its thickest MAX (deep blue), on the modelled part and on the simulated one,
/// and MIN and MAX stay where they are whatever the view does.
#[test]
fn the_colours_at_min_and_max_are_the_legends_in_both_states() {
    use forge_render::shell::thermo::BANDS;
    use forge_render::thermo_view::{unlit, view};
    let src = include_str!("../../../examples/thermoform/cover.fg");
    let r = forge_lang::run(src, Default::default(), Default::default()).0;
    let o = r.thermos.first().expect("the cover's thermoform");
    let (_, mut gfx) = view(o, &r.solids, 3.0);
    let full = forge_render::tess::build_part(r.body.as_ref().unwrap()).unwrap();
    gfx.made_now(Some(&full));
    let (lo, hi) = gfx.extremes.expect("MIN and MAX on the modelled part");
    let (slo, shi) = gfx.sim_extremes.expect("MIN and MAX on the simulated part");
    // the colour the map has nearest a point
    let nearest = |tris: &[forge_render::origin::WorldVert], p: [f32; 3]| {
        tris.chunks(3)
            .min_by(|a, b| {
                let d = |t: &[forge_render::origin::WorldVert]| {
                    let c: [f32; 3] =
                        std::array::from_fn(|k| (t[0].at[k] + t[1].at[k] + t[2].at[k]) / 3.0 - p[k]);
                    c[0] * c[0] + c[1] * c[1] + c[2] * c[2]
                };
                d(a).total_cmp(&d(b))
            })
            .unwrap()[0]
            .colour
    };
    assert_eq!(nearest(&gfx.heat, lo), unlit(BANDS[9]), "MIN, modelled");
    assert_eq!(nearest(&gfx.heat, hi), unlit(BANDS[0]), "MAX, modelled");
    assert_eq!(nearest(&gfx.sim_heat, slo), unlit(BANDS[9]), "MIN, simulated");
    assert_eq!(nearest(&gfx.sim_heat, shi), unlit(BANDS[0]), "MAX, simulated");
    // painted again (another mesh, another frame): the same places
    gfx.made_now(Some(&full));
    assert_eq!(gfx.extremes, Some((lo, hi)));
    assert_eq!(gfx.sim_extremes, Some((slo, shi)));
    let (_, mut again) = view(o, &r.solids, 3.0);
    again.made_now(Some(&full));
    assert_eq!(again.extremes, Some((lo, hi)));
    assert_eq!(again.sim_extremes, Some((slo, shi)));
    // every band is on the part, the extremes' too
    for c in BANDS {
        assert!(gfx.heat.iter().any(|v| v.colour == unlit(c)), "{c:?}");
    }
}

#[test]
fn the_paint_stands_off_every_face_at_a_hard_edge() {
    use forge_math::Point3;
    use forge_render::thermo_view::lifts;
    // a box's corner: a big face meeting two small ones, as a part's flat top meets its fillet-less
    // sides (and a concave step), each corner's lift checked against every face at it
    let p = |x: f64, y: f64, z: f64| {
        let q = Point3::new(x, y, z);
        (q, q)
    };
    let pts = vec![
        p(0.0, 0.0, 0.0),
        p(40.0, 0.0, 0.0),
        p(40.0, 40.0, 0.0),
        p(0.0, 40.0, 0.0),
        p(0.0, 0.0, -1.0),
        p(40.0, 0.0, -1.0),
        p(0.0, 40.0, -1.0),
        p(-5.0, 0.0, -1.0),
        p(-5.0, 40.0, -1.0),
    ];
    let mesh: Vec<[u32; 3]> = vec![
        // top, up
        [0, 1, 2],
        [0, 2, 3],
        // front wall, -y
        [0, 4, 5],
        [0, 5, 1],
        // left wall, -x (a step down)
        [0, 3, 6],
        [0, 6, 4],
        // the floor beyond the step, up
        [4, 6, 8],
        [4, 8, 7],
    ];
    let lift = lifts(&pts, &mesh);
    let lift_min = 0.079;
    for t in &mesh {
        let [a, b, c] = t.map(|i| pts[i as usize].0);
        let n = (b - a).cross(c - a).normalize().unwrap().get();
        for &i in t {
            let off = lift[i as usize].dot(n);
            assert!(off >= lift_min, "corner {i} sits {off} off face {t:?}");
        }
    }
}

/// The modelled part's map covers each face off the tool whole, and none on it: a deep tray's
/// walls (thinner than half a sheet, so its free side lies near the sheet's middle) and its flange's
/// cut edge are painted all over, its side against the mould not at all.
#[test]
fn every_face_off_the_tool_is_painted_whole() {
    use forge_math::Point3;
    use forge_render::thermo_view::view;
    let src = include_str!("../../../examples/thermoform/tray.fg");
    let r = forge_lang::run(src, Default::default(), Default::default()).0;
    let o = r.thermos.first().expect("the tray's thermoform");
    let (_, mut gfx) = view(o, &r.solids, 3.0);
    let full = forge_render::tess::build_part(r.body.as_ref().unwrap()).unwrap();
    gfx.made_now(Some(&full));
    let l = full.lods.iter().find(|l| l.chord >= 0.04).unwrap();
    let span = gfx.part_span.clone().unwrap();
    let tool = gfx.tool.clone().unwrap();
    let at = |i: usize| {
        let f = |k: usize| {
            let b = &l.vertices[i * forge_render::mesh::VERTEX + k..][..4];
            f32::from_le_bytes(b.try_into().unwrap()) as f64
        };
        Point3::new(f(0), f(4), f(8))
    };
    let idx = |k: usize| u32::from_le_bytes(l.indices[k * 4..k * 4 + 4].try_into().unwrap()) as usize;
    let heat: Vec<Point3> = gfx
        .heat
        .chunks(3)
        .map(|t| {
            let c = |k: usize| (t[0].at[k] + t[1].at[k] + t[2].at[k]) as f64 / 3.0;
            Point3::new(c(0), c(1), c(2))
        })
        .collect();
    let (mut painted, mut bare) = (0, 0);
    for f in l.faces.iter().filter(|f| span.faces.contains(&f.face)) {
        let tris: Vec<[Point3; 3]> = (f.first_tri..f.first_tri + f.tris)
            .map(|k| [0, 1, 2].map(|c| at(idx(k as usize * 3 + c))))
            .collect();
        let on = tris.iter().filter(|t| tool.has(**t)).count();
        // every triangle of the face has paint over it, or none has
        let covered = tris
            .iter()
            .filter(|t| {
                let g = Point3::new(
                    (t[0].x + t[1].x + t[2].x) / 3.0,
                    (t[0].y + t[1].y + t[2].y) / 3.0,
                    (t[0].z + t[1].z + t[2].z) / 3.0,
                );
                heat.iter().any(|h| (*h - g).length() < 1.5)
            })
            .count();
        if 2 * on > tris.len() {
            bare += 1;
            assert!(
                covered < tris.len(),
                "face {} is on the tool yet painted all over",
                f.face
            );
        } else {
            painted += 1;
            assert_eq!(
                covered,
                tris.len(),
                "face {} is off the tool, painted in part",
                f.face
            );
        }
    }
    assert!(painted > 10 && bare > 10, "{painted} painted, {bare} bare");
}

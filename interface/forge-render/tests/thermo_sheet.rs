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

/// The paint's triangles, binned by their middles in 2 mm cells, to ask whether paint lies just
/// off a part's triangle, on its own side.
struct Painted(std::collections::HashMap<(i64, i64, i64), Vec<[forge_math::Point3; 3]>>);

impl Painted {
    fn of(tris: &[forge_render::origin::WorldVert]) -> Painted {
        let mut m: std::collections::HashMap<_, Vec<_>> = Default::default();
        for t in tris.chunks(3) {
            let p = [0, 1, 2].map(|k| {
                let a = t[k].at;
                forge_math::Point3::new(a[0] as f64, a[1] as f64, a[2] as f64)
            });
            m.entry(Self::cell(middle(p))).or_default().push(p);
        }
        Painted(m)
    }

    fn cell(p: forge_math::Point3) -> (i64, i64, i64) {
        let k = |v: f64| (v / 2.0).floor() as i64;
        (k(p.x), k(p.y), k(p.z))
    }

    /// Whether paint lies over triangle `t`: its middle, lifted off it along its normal by the
    /// paint's lift, is within 0.1 mm of a painted triangle (a corner's lift leans along its faces'
    /// bisector). Paint on the wall's other side is a wall and two lifts away, 0.18 mm at least.
    fn over(&self, t: [forge_math::Point3; 3]) -> bool {
        let Some(n) = (t[1] - t[0]).cross(t[2] - t[0]).normalize() else {
            return true;
        };
        let q = middle(t) + n.get() * 0.08;
        let (x, y, z) = Self::cell(q);
        (-1..=1).any(|i| {
            (-1..=1).any(|j| {
                (-1..=1).any(|k| {
                    self.0
                        .get(&(x + i, y + j, z + k))
                        .is_some_and(|l| l.iter().any(|p| (closest(q, *p) - q).length() < 0.1))
                })
            })
        })
    }
}

/// The point of triangle `t` nearest `p`.
fn closest(p: forge_math::Point3, t: [forge_math::Point3; 3]) -> forge_math::Point3 {
    let [a, b, c] = t;
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let d = 1.0 / (va + vb + vc);
    a + ab * (vb * d) + ac * (vc * d)
}

fn middle(t: [forge_math::Point3; 3]) -> forge_math::Point3 {
    forge_math::Point3::new(
        (t[0].x + t[1].x + t[2].x) / 3.0,
        (t[0].y + t[1].y + t[2].y) / 3.0,
        (t[0].z + t[1].z + t[2].z) / 3.0,
    )
}

/// Each face of a formed part's mesh, and whether it lies on the tool, with how many of its
/// triangles have paint over them, of how many.
fn modelled_cover(file: &str) -> Vec<(u32, bool, usize, usize)> {
    use forge_math::Point3;
    use forge_render::thermo_view::view;
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/thermoform/");
    let src = std::fs::read_to_string(format!("{dir}{file}")).unwrap();
    let r = forge_lang::run(&src, Default::default(), Default::default()).0;
    let o = r.thermos.first().expect("the thermoform");
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
    let paint = Painted::of(&gfx.heat);
    l.faces
        .iter()
        .filter(|f| span.faces.contains(&f.face))
        .map(|f| {
            let tris: Vec<[Point3; 3]> = (f.first_tri..f.first_tri + f.tris)
                .map(|k| [0, 1, 2].map(|c| at(idx(k as usize * 3 + c))))
                .collect();
            let on = tris.iter().filter(|t| tool.has(**t)).count();
            let covered = tris.iter().filter(|t| paint.over(**t)).count();
            (f.face, 2 * on > tris.len(), covered, tris.len())
        })
        .collect()
}

/// The modelled part's map covers every face whole, on the tool and off it: a deep tray's walls
/// (thinner than half a sheet, so its free side lies near the sheet's middle), its flange's cut
/// edge and its side against the mould are all painted all over.
#[test]
fn every_face_of_the_part_is_painted_whole() {
    let faces = modelled_cover("tray.fg");
    let (mut on, mut off) = (0, 0);
    for (face, tool, covered, all) in faces {
        assert_eq!(
            covered, all,
            "face {face} (on the tool: {tool}) is painted in part"
        );
        if tool { on += 1 } else { off += 1 }
    }
    assert!(on > 10 && off > 10, "{on} on the tool, {off} off it");
}

/// A cup drawn into a female mould shows its wall's colours inside and out: the modelled part's
/// faces on the mould and its free ones, and the simulated part's sides on the tool and off it,
/// every triangle with paint over it.
#[test]
fn a_cup_is_coloured_on_both_its_sides() {
    use forge_render::thermo_view::view;
    use forge_thermo::simulated::TOOL_SIDE;
    let faces = modelled_cover("cup.fg");
    assert!(faces.iter().any(|f| f.1) && faces.iter().any(|f| !f.1));
    for (face, tool, covered, all) in &faces {
        assert_eq!(
            covered, all,
            "modelled face {face} (on the tool: {tool}) painted in part"
        );
    }
    let src = include_str!("../../../examples/thermoform/cup.fg");
    let r = forge_lang::run(src, Default::default(), Default::default()).0;
    let o = r.thermos.first().expect("the cup's thermoform");
    let (_, mut gfx) = view(o, &r.solids, 3.0);
    gfx.made_now(None);
    let m = gfx.sim_mesh.clone().expect("the simulated cup");
    let paint = Painted::of(&gfx.sim_heat);
    let (mut sides, mut bare) = ([0usize; 2], [0usize; 2]);
    for (t, f) in m.triangles.iter().zip(&m.face_of) {
        let k = usize::from(*f == TOOL_SIDE);
        sides[k] += 1;
        if !paint.over(t.map(|i| m.positions[i as usize])) {
            bare[k] += 1;
        }
    }
    assert!(sides[0] > 100 && sides[1] > 100, "{sides:?}");
    assert_eq!(
        bare,
        [0, 0],
        "simulated triangles bare, off the tool and on it, of {sides:?}"
    );
}

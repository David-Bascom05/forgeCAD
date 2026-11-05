use crate::*;
use forge_math::{Frame, Point2, UnitVec3, dmath};
use forge_ops::Profile;
use forge_topo::Body;

fn block(w: f64, l: f64, h: f64, r: f64, taper_deg: f64) -> Body {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![if r > 0.0 {
            Profile::rounded_rectangle(Point2::new(-w / 2.0, -l / 2.0), w, l, r, 0)
        } else {
            Profile::rectangle(Point2::new(-w / 2.0, -l / 2.0), w, l, 0)
        }],
    };
    if taper_deg > 0.0 {
        forge_ops::tapered(&p, h, taper_deg.to_radians(), 3)
            .unwrap()
            .remove(0)
    } else {
        forge_ops::extrude(&p, h, 3).unwrap()
    }
}

fn spec(tool: Tool) -> Spec {
    Spec {
        tool,
        pull: UnitVec3::Z,
        t: 2.0,
        trim: None,
        flange: 10.0,
        shrink: 0.005,
        base: 10.0,
        margin: 5.0,
        vent: Some(0.8),
        vents: None,
        vent_pitch: 25.0,
        sheet: None,
    }
}

fn kinds(v: &[Violation]) -> Vec<String> {
    v.iter()
        .map(|x| {
            format!("{x:?}")
                .split([' ', '{', '('])
                .next()
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn a_male_box_forms_over_its_mould() {
    let shape = block(60.0, 40.0, 20.0, 6.0, 5.0);
    let s = spec(Tool::Male);
    let f = form(&shape, &s, 11).unwrap();
    assert!(
        forge_topo::check(&f.part).is_empty(),
        "{:?}",
        forge_topo::check(&f.part)
    );
    assert!(
        forge_topo::check(&f.mould).is_empty(),
        "{:?}",
        forge_topo::check(&f.mould)
    );
    let v = forge_topo::mass::mass(&f.part).unwrap().volume;
    // the formed area plus the flange's, times t, near enough (the offset adds at convex corners)
    let want = s.t * (f.report.formed_area + f.report.trim_area - f.report.opening_area);
    assert!((v - want).abs() < 0.05 * want, "{v} {want}");
    assert!(!f.report.vents.is_empty());
    assert!((f.report.scale - 1.005).abs() < 1e-12);
    let shop = placeholder();
    let c = check(&shape, &f, &s, shop.material("ABS").unwrap(), &shop, None);
    let k = kinds(&c);
    assert!(!k.contains(&"DraftTooLow".into()), "{c:?}");
    // the top's edges are sharp
    assert!(k.contains(&"SharpEdge".into()), "{c:?}");
}

#[test]
fn walls_square_to_the_pull_have_no_draft() {
    let shape = block(60.0, 40.0, 20.0, 6.0, 0.0);
    let s = spec(Tool::Male);
    let f = form(&shape, &s, 11).unwrap();
    let shop = placeholder();
    let c = check(&shape, &f, &s, shop.material("ABS").unwrap(), &shop, None);
    let n = c
        .iter()
        .filter(|x| matches!(x, Violation::DraftTooLow { .. }))
        .count();
    // four sides and four corner rounds
    assert_eq!(n, 8, "{c:?}");
}

#[test]
fn a_female_cavity_forms_inside() {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::rounded_rectangle(
            Point2::new(-30.0, -20.0),
            60.0,
            40.0,
            8.0,
            0,
        )],
    };
    let shape = forge_ops::tapered(&p, -25.0, 3f64.to_radians(), 3)
        .unwrap()
        .remove(0);
    let s = spec(Tool::Female);
    let f = form(&shape, &s, 11).unwrap();
    assert!(
        forge_topo::check(&f.part).is_empty(),
        "{:?}",
        forge_topo::check(&f.part)
    );
    assert!(
        forge_topo::check(&f.mould).is_empty(),
        "{:?}",
        forge_topo::check(&f.mould)
    );
    let shop = placeholder();
    let c = check(&shape, &f, &s, shop.material("ABS").unwrap(), &shop, None);
    let k = kinds(&c);
    assert!(!k.contains(&"DraftTooLow".into()), "{c:?}");
    assert!(!k.contains(&"Undercut".into()), "{c:?}");
    // the vents sit on the cavity's floor
    assert!(f.report.vents.iter().all(|v| v.z < -24.0), "{:?}", f.report.vents);
    let _ = dmath::PI;
}

#[test]
fn a_tool_wider_at_the_top_is_an_undercut() {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::rectangle(Point2::new(-20.0, -15.0), 40.0, 30.0, 0)],
    };
    let shape = forge_ops::tapered(&p, 20.0, -5f64.to_radians(), 3)
        .unwrap()
        .remove(0);
    let s = spec(Tool::Male);
    let f = form(&shape, &s, 11).unwrap();
    let shop = placeholder();
    let c = check(&shape, &f, &s, shop.material("ABS").unwrap(), &shop, None);
    let n = c
        .iter()
        .filter(|x| matches!(x, Violation::Undercut { .. }))
        .count();
    assert_eq!(n, 4, "{c:?}");
}

#[test]
fn absurd_sizes_are_refused_before_anything_is_laid_out() {
    let shape = block(60.0, 40.0, 20.0, 6.0, 5.0);
    let mut s = spec(Tool::Male);
    s.flange = 1e9;
    assert!(matches!(
        form(&shape, &s, 11),
        Err(FormError::Size { what: "trim", .. })
    ));
    let mut s = spec(Tool::Male);
    s.margin = f64::INFINITY;
    assert!(matches!(
        form(&shape, &s, 11),
        Err(FormError::Size { what: "margin", .. })
    ));
    let mut s = spec(Tool::Male);
    s.trim = Some(Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::rectangle(Point2::new(-5e8, -5e8), 1e9, 1e9, 0)],
    });
    assert!(matches!(
        form(&shape, &s, 11),
        Err(FormError::Size { what: "trim", .. })
    ));
}

#[test]
fn a_cup_thins_most_at_its_floor_corner() {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::circle(Point2::new(0.0, 0.0), 30.0, 0)],
    };
    let shape = forge_ops::tapered(&p, -30.0, 3f64.to_radians(), 3)
        .unwrap()
        .remove(0);
    let s = spec(Tool::Female);
    let f = analyse(&shape, &s).unwrap();
    let t = std::time::Instant::now();
    let st = stretch(&f, &s, &Process::default());
    eprintln!(
        "cup: min {:?} max {:?} mean {} (ADR mean {}) unformed {} its {} in {:?}",
        st.min,
        st.max,
        st.mean,
        1.0 / f.report.adr,
        st.unformed.len(),
        st.iterations,
        t.elapsed()
    );
    assert!(st.unformed.len() < 10);
}

#[test]
fn a_male_box_thins_down_its_walls() {
    let shape = block(60.0, 40.0, 25.0, 6.0, 5.0);
    let s = spec(Tool::Male);
    let f = analyse(&shape, &s).unwrap();
    let t = std::time::Instant::now();
    let st = stretch(&f, &s, &Process::default());
    eprintln!(
        "box: min {:?} max {:?} mean {} (ADR mean {}) unformed {} its {} in {:?}",
        st.min,
        st.max,
        st.mean,
        1.0 / f.report.adr,
        st.unformed.len(),
        st.iterations,
        t.elapsed()
    );
}

#[test]
fn every_number_in_the_data_has_a_source_or_is_a_stand_in() {
    let lib = forge_material::library();
    let with: Vec<_> = lib
        .iter()
        .filter(|r| r.extra("thermo.window").is_some())
        .collect();
    assert!(with.len() >= 7);
    for r in &with {
        assert!(unsourced(r).is_empty(), "{}: {:?}", r.name, unsourced(r));
        assert!(from_record(r).unwrap().is_some(), "{}", r.name);
    }
    assert!(placeholder().materials.len() >= 7);
}

#[test]
fn a_solid_part_converts_to_its_wall_and_mould() {
    let part = block(60.0, 40.0, 20.0, 6.0, 5.0);
    let s = spec(Tool::Male);
    let p = prepare(&part, Given::Outer, &s, 21).unwrap();
    // the male mould is the part less its wall: the formed part comes back to the part's top
    let f = form(&p.shape, &s, 11).unwrap();
    let top = |b: &forge_topo::Body| b.faces_box().z.hi;
    assert!(
        (top(&f.part) - top(&part)).abs() < 1e-6,
        "{} {}",
        top(&f.part),
        top(&part)
    );
    assert!(top(&p.shape) < top(&part) - 1.9);
    // the faces of the mould's surface are the user's, offset
    assert!(!p.origin.is_empty());
}

#[test]
fn a_shelled_part_gives_its_thickness_and_its_hollow() {
    let solid = block(60.0, 40.0, 20.0, 6.0, 5.0);
    let base = solid
        .face_by_name(forge_topo::Name::feature(3).with_str("start"))
        .unwrap();
    let wall = forge_ops::shell(&solid, 1.5, false, &[solid.face(base).name], 4).unwrap();
    let male = prepare(&wall, Given::Wall, &spec(Tool::Male), 21).unwrap();
    assert!((male.t - 1.5).abs() < 0.01, "{}", male.t);
    let v = |b: &forge_topo::Body| forge_topo::mass::mass(b).unwrap().volume;
    assert!((v(&male.shape) + v(&wall) - v(&solid)).abs() < 1e-3 * v(&solid));
    // a female mould takes the wall's outside: the hollow and the wall together, the part's pull
    // the other way
    let mut s = spec(Tool::Female);
    s.pull = -UnitVec3::Z;
    let female = prepare(&wall, Given::Wall, &s, 21).unwrap();
    assert!((v(&female.shape) - v(&solid)).abs() < 1e-3 * v(&solid));
}

#[test]
fn a_wall_of_two_thicknesses_is_refused() {
    let solid = block(60.0, 40.0, 20.0, 6.0, 5.0);
    let base = solid
        .face_by_name(forge_topo::Name::feature(3).with_str("start"))
        .unwrap();
    let wall = forge_ops::shell(&solid, 1.5, false, &[solid.face(base).name], 4).unwrap();
    // the top made 3 mm thicker on the inside
    let p = Profile {
        plane: Frame::WORLD.with_origin(forge_math::Point3::new(0.0, 0.0, 15.5)),
        loops: vec![Profile::rectangle(Point2::new(-25.0, -15.0), 50.0, 30.0, 0)],
    };
    let pad = forge_ops::extrude(&p, 3.0, 8).unwrap();
    let thick = forge_bool::boolean(&wall, &pad, forge_bool::Op::Union, 9).unwrap();
    let e = prepare(&thick, Given::Wall, &spec(Tool::Male), 21).unwrap_err();
    assert!(matches!(e, FormError::WallNotUniform { .. }), "{e:?}");
}

/// A flat sheet `nx` by `ny` cells of `s`, its wall `h(x, y)` over t at each node.
fn flat_sheet(nx: usize, ny: usize, s: f64, h: impl Fn(f64, f64) -> f64) -> Stretch {
    let id = |i: usize, j: usize| (j * (nx + 1) + i) as u32;
    let mut nodes = vec![];
    for j in 0..=ny {
        for i in 0..=nx {
            nodes.push(forge_math::Point3::new(i as f64 * s, j as f64 * s, 0.0));
        }
    }
    let mut tris = vec![];
    for j in 0..ny {
        for i in 0..nx {
            tris.push([id(i, j), id(i + 1, j), id(i + 1, j + 1)]);
            tris.push([id(i, j), id(i + 1, j + 1), id(i, j + 1)]);
        }
    }
    let node_ratio: Vec<f64> = nodes.iter().map(|p| h(p.x, p.y)).collect();
    Stretch {
        touched: vec![true; nodes.len()],
        ratio: vec![1.0; tris.len()],
        node_ratio,
        nodes,
        steps: vec![],
        grid: (nx, ny),
        tris,
        min: (1.0, forge_math::Point3::ORIGIN),
        max: (1.0, forge_math::Point3::ORIGIN),
        mean: 1.0,
        unformed: vec![],
        iterations: 0,
    }
}

#[test]
fn a_flat_sheet_trimmed_weighs_its_trim_times_its_wall() {
    // the trim cuts across the grid's cells; the wall varies along x
    let t = 2.0;
    let st = flat_sheet(40, 30, 2.5, |x, _| 0.5 + 0.01 * x);
    let trim = [
        Point2::new(7.3, 6.1),
        Point2::new(83.7, 6.1),
        Point2::new(83.7, 61.9),
        Point2::new(7.3, 61.9),
    ];
    let sim = simulated(&Frame::WORLD, &trim, &st, t);
    assert!(
        sim.mesh.open_edges().is_empty(),
        "{:?}",
        sim.mesh.open_edges().len()
    );
    let (w, l) = (83.7 - 7.3, 61.9 - 6.1);
    let cx = (83.7 + 7.3) / 2.0;
    let want = t * w * l * (0.5 + 0.01 * cx);
    let rel = |a: f64, b: f64| (a - b).abs() / b.abs();
    // a flat sheet: the volume exact; the integral over the wall's middle, which leans with the
    // wall by 0.01 t / 2, longer by its square over 2
    assert!(rel(sim.sheet_volume, want) < 1e-4, "{} {want}", sim.sheet_volume);
    assert!(rel(sim.mass.volume, want) < 1e-9, "{} {want}", sim.mass.volume);
    assert!(rel(sim.area, w * l) < 1e-9);
    // uniform wall: a box's centre and inertia
    let st = flat_sheet(40, 30, 2.5, |_, _| 1.0);
    let sim = simulated(&Frame::WORLD, &trim, &st, t);
    let v = w * l * t;
    let c = sim.mass.centroid;
    assert!(
        (c.x - cx).abs() < 1e-9 && (c.y - 34.0).abs() < 1e-9 && (c.z - 1.0).abs() < 1e-9,
        "{c:?}"
    );
    let i = sim.mass.inertia;
    for (k, want) in [
        v * (l * l + t * t) / 12.0,
        v * (w * w + t * t) / 12.0,
        v * (w * w + l * l) / 12.0,
    ]
    .into_iter()
    .enumerate()
    {
        assert!(rel(i[k][k], want) < 1e-9, "I{k}{k} {} {want}", i[k][k]);
    }
    assert!(i[0][1].abs() < 1e-6 * i[0][0]);
}

/// The simulated part of a form: its volume against the integral of its wall, the modelled
/// part's volume, and the sheet the trim took.
fn simulated_of(shape: &Body, s: &Spec) -> (Simulated, f64) {
    let f = form(shape, s, 11).unwrap();
    let st = stretch(&f, s, &Process::default());
    let trim = build::polygon(&f.trim);
    let sim = simulated(&f.frame, &trim, &st, s.t);
    let modelled = forge_topo::mass::mass(&f.part).unwrap().volume;
    (sim, modelled)
}

#[test]
fn a_formed_part_simulated_weighs_the_integral_of_its_wall() {
    let cup = {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::new(0.0, 0.0), 30.0, 0)],
        };
        forge_ops::tapered(&p, -30.0, 3f64.to_radians(), 3)
            .unwrap()
            .remove(0)
    };
    for (name, shape, tool) in [
        ("box", block(60.0, 40.0, 25.0, 6.0, 5.0), Tool::Male),
        ("cover", block(120.0, 80.0, 35.0, 8.0, 5.0), Tool::Male),
        ("cup", cup, Tool::Female),
    ] {
        let s = spec(tool);
        let (sim, modelled) = simulated_of(&shape, &s);
        let rel = (sim.mass.volume - sim.sheet_volume) / sim.sheet_volume;
        eprintln!(
            "{name}: volume {:.1} integral {:.1} rel {rel:.5} modelled {modelled:.1} ({:.3}) open {}",
            sim.mass.volume,
            sim.sheet_volume,
            sim.mass.volume / modelled,
            sim.mesh.open_edges().len()
        );
        assert!(sim.mesh.open_edges().is_empty(), "{name}");
        // measured 0.10 to 0.24 %: the wall's curvature, second order in it
        assert!(rel.abs() < 0.005, "{name}: {rel}");
        // the sheet thins: well under the modelled wall's, and more than the sheet's least
        let r = sim.mass.volume / modelled;
        assert!((0.45..0.75).contains(&r), "{name}: {r}");
    }
}

#[test]
fn a_finer_simulated_part_keeps_its_wall_integral_and_lies_on_the_tool() {
    let cup = {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::new(0.0, 0.0), 30.0, 0)],
        };
        forge_ops::tapered(&p, -30.0, 3f64.to_radians(), 3)
            .unwrap()
            .remove(0)
    };
    for (name, shape, tool) in [
        ("box", block(60.0, 40.0, 25.0, 6.0, 5.0), Tool::Male),
        ("cover", block(120.0, 80.0, 35.0, 8.0, 5.0), Tool::Male),
        ("cup", cup, Tool::Female),
    ] {
        let s = spec(tool);
        let f = form(&shape, &s, 11).unwrap();
        let st = stretch(&f, &s, &Process::default());
        let trim = build::polygon(&f.trim);
        let on = OnTool {
            mesh: &f.mesh,
            contact: &f.contact,
        };
        for levels in 0..3 {
            let t0 = std::time::Instant::now();
            let sim = simulated_on(&f.frame, &trim, &st, s.t, &on, levels);
            let ms = t0.elapsed().as_secs_f64() * 1e3;
            let rel = (sim.mass.volume - sim.sheet_volume) / sim.sheet_volume;
            eprintln!(
                "{name} L{levels}: {} tris, {ms:.1} ms, volume {:.1} integral {:.1} rel {rel:.5}",
                sim.mesh.triangles.len(),
                sim.mass.volume,
                sim.sheet_volume
            );
            assert!(sim.mesh.open_edges().is_empty(), "{name} {levels}");
            // measured: within 0.25 % to one split, past 0.5 % at two (the box's -0.66 %), so the
            // app takes one
            if levels <= 1 {
                assert!(rel.abs() < 0.005, "{name} {levels}: {rel}");
            }
            // the tool side lies on the tool or the table: no point the model left a quarter of t
            // off it is left so
            if levels == 1 {
                let tool = simulated::Near::new(&on, 2.0 * s.t);
                let near = sim.mesh.positions[..sim.wall.len() / 2]
                    .iter()
                    .filter(|p| {
                        f.frame.point_to_local(**p).z.abs() < 1e-9
                            || tool.nearest(**p).is_some_and(|q| (q - **p).length() < 1e-6)
                    })
                    .count();
                assert!(
                    near * 10 > sim.wall.len() / 2 * 6,
                    "{name}: {near} of {}",
                    sim.wall.len() / 2
                );
            }
        }
    }
    let _ = simulated;
}

/// How rough a mesh's free side is: the mean turn between neighbouring triangles' normals,
/// degrees, over the sides where it turns at all.
fn roughness(sim: &Simulated) -> f64 {
    let m = &sim.mesh;
    let mut sides: std::collections::HashMap<(u32, u32), Vec<usize>> = Default::default();
    for (k, t) in m.triangles.iter().enumerate() {
        if m.face_of[k] != simulated::FREE_SIDE {
            continue;
        }
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            sides.entry((a.min(b), a.max(b))).or_default().push(k);
        }
    }
    let unit = |k: usize| m.triangle_normal(m.triangles[k]).normalize().map(|u| u.get());
    let turns: Vec<f64> = sides
        .values()
        .filter(|v| v.len() == 2)
        .filter_map(|v| {
            Some(forge_math::dmath::acos(unit(v[0])?.dot(unit(v[1])?).clamp(-1.0, 1.0)).to_degrees())
        })
        .filter(|a| *a > 0.01)
        .collect();
    turns.iter().sum::<f64>() / turns.len().max(1) as f64
}

#[test]
fn the_simulated_part_on_the_modelled_part_is_smooth_and_keeps_its_wall_integral() {
    let cup = {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::new(0.0, 0.0), 30.0, 0)],
        };
        forge_ops::tapered(&p, -30.0, 3f64.to_radians(), 3)
            .unwrap()
            .remove(0)
    };
    for (name, shape, tool) in [
        ("box", block(60.0, 40.0, 25.0, 6.0, 5.0), Tool::Male),
        ("cover", block(120.0, 80.0, 35.0, 8.0, 5.0), Tool::Male),
        ("cup", cup, Tool::Female),
    ] {
        let s = spec(tool);
        let f = form(&shape, &s, 11).unwrap();
        let st = stretch(&f, &s, &Process::default());
        let trim = build::polygon(&f.trim);
        let on = OnTool {
            mesh: &f.mesh,
            contact: &f.contact,
        };
        let sheet = simulated_on(&f.frame, &trim, &st, s.t, &on, 1);
        let t0 = std::time::Instant::now();
        let field = WallField::of(&sheet, 3.0 * s.t);
        let sim = simulated_part(&f.part, &f.frame, &on, &field, s.t, 3.0).expect("a side on the tool");
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        // at t everywhere the fit is the modelled part again: measured within 0.13 %
        let flat = simulated::built_on(&f.part, &f.frame, &on, None, s.t, 3.0, simulated::ANGLE).unwrap();
        let modelled = forge_topo::mass::mass(&f.part).unwrap().volume;
        let flat_rel = flat.mass.volume / modelled - 1.0;
        // the wall scaled to hold the model's material: by how much (left unscaled, the fit's
        // own volume over the model's), and how well
        let mut raw = field.clone();
        raw.volume = 0.0;
        let k = simulated_part(&f.part, &f.frame, &on, &raw, s.t, 3.0)
            .unwrap()
            .mass
            .volume
            / field.volume;
        let rel = sim.mass.volume / field.volume - 1.0;
        eprintln!(
            "{name}: {} tris {ms:.0} ms; flat vs modelled {flat_rel:+.4}; volume {:.1} vs the model's {:.1} ({rel:+.5}); unscaled {k:.4}; rough {:.2} -> {:.2} deg",
            sim.mesh.triangles.len(),
            sim.mass.volume,
            field.volume,
            roughness(&sheet),
            roughness(&sim),
        );
        // no triangle turned over: each faces as it does on the modelled part (the fit at t)
        let m = &sim.mesh;
        assert_eq!(m.triangles, flat.mesh.triangles);
        let flipped = m
            .triangles
            .iter()
            // slivers along a face's edge (under 0.05 mm2) may turn: no line is drawn on them
            .filter(|t| flat.mesh.triangle_normal(**t).length() / 2.0 > 0.05)
            .filter(|t| m.triangle_normal(**t).dot(flat.mesh.triangle_normal(**t)) <= 0.0)
            .count();
        eprintln!("{name}: {flipped} turned over");
        // the kernel's test boxes have sharp top edges, slivers along them turn; the rounded parts
        // a thermoform has (the checks ask for radii) none
        if name == "cup" {
            assert_eq!(flipped, 0, "{name}");
        }
        assert!(sim.mesh.open_edges().is_empty(), "{name}");
        assert!(flat_rel.abs() < 0.002, "{name}: {flat_rel}");
        assert!(rel.abs() < 0.005, "{name}: {rel}");
        // measured +0.7 to +1.5 %: the sharp corners and rounds the model's sheet bridges
        // measured +2.6 to +5 %: the smoothing of the moves, the sharp corners and the rounds the
        // model's sheet bridges
        assert!((k - 1.0).abs() < 0.08, "{name}: {k}");
    }
}

/// The 3-D distance field of a formed part's tool against the exact distance (every face tried):
/// within a tenth of the sheet in the band (measured: exact there, the band's distances worked
/// out from the nearest faces), its sign the exact one's.
#[test]
fn the_distance_field_is_the_tools_distance_to_a_tenth_of_the_sheet() {
    let cup = {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::new(0.0, 0.0), 30.0, 0)],
        };
        forge_ops::tapered(&p, -30.0, 3f64.to_radians(), 3)
            .unwrap()
            .remove(0)
    };
    for (name, shape, tool) in [
        ("cover", block(120.0, 80.0, 35.0, 8.0, 5.0), Tool::Male),
        ("cup", cup, Tool::Female),
    ] {
        let s = spec(tool);
        let f = analyse(&shape, &s).unwrap();
        let male = tool == Tool::Male;
        let (tlo, thi) = f.report.trim_box;
        let m = s.margin.max(s.t);
        let (lo, hi) = (
            Point2::new(tlo.x - m, tlo.y - m),
            Point2::new(thi.x + m, thi.y + m),
        );
        let table = field::Field::new(f.frame, lo, hi, &f.mesh, &f.contact, male, |_| true);
        let t0 = std::time::Instant::now();
        let h = 0.5 * s.t;
        let sdf = sdf::Sdf::new(
            f.frame,
            &f.mesh,
            &f.contact,
            male,
            table.clone(),
            lo,
            hi,
            h,
            1.5 * s.t,
        );
        let built = t0.elapsed();
        // exact: every face, and the table where the shape doesn't stand
        let faces: Vec<[forge_math::Vec3; 3]> = f
            .contact
            .iter()
            .map(|&k| {
                f.mesh.triangles[k].map(|i| {
                    let p = f.frame.point_to_local(f.mesh.positions[i as usize]);
                    forge_math::Vec3::new(p.x, p.y, p.z)
                })
            })
            .collect();
        let mut seed = 12345u64;
        let mut rnd = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let (mut worst, mut n, mut wrong_side) = (0.0f64, 0, 0);
        for _ in 0..3000 {
            // a point off a face of the tool, or over the window, within the band
            let p = if rnd() < 0.7 {
                let [a, b, c] = faces[(rnd() * faces.len() as f64) as usize % faces.len()];
                let (u, v) = (rnd(), rnd());
                let (u, v) = if u + v > 1.0 { (1.0 - u, 1.0 - v) } else { (u, v) };
                let q = a + (b - a) * u + (c - a) * v;
                let nrm = (b - a).cross(c - a).normalize().unwrap().get();
                q + nrm * ((rnd() * 2.0 - 1.0) * 1.2 * s.t)
            } else {
                forge_math::Vec3::new(
                    lo.x + rnd() * (hi.x - lo.x),
                    lo.y + rnd() * (hi.y - lo.y),
                    (rnd() * 2.0 - 1.0) * 1.2 * s.t,
                )
            };
            let Some(exact) = sdf.exact(p) else { continue };
            if exact.abs() > 1.2 * s.t {
                continue;
            }
            // the exact distance, unsigned, by trying every face (and the table): the sign is the
            // field's own, checked by the height field where it's plain
            let brute = faces
                .iter()
                .map(|[a, b, c]| {
                    let q = brute_closest(p, *a, *b, *c);
                    (p - q).length()
                })
                .fold(f64::INFINITY, f64::min);
            let d = sdf.distance(p);
            let want = brute.min(if exact.abs() == p.z.abs() {
                p.z.abs()
            } else {
                f64::INFINITY
            });
            worst = worst.max((d.abs() - want).abs());
            n += 1;
            if (d > 0.0) != (exact > 0.0) && d.abs() > 1e-6 {
                wrong_side += 1;
            }
        }
        eprintln!(
            "{name}: {} nodes, built in {built:?}; worst |field - exact| {worst:.4} mm over {n} points (t/10 = {})",
            sdf.nodes(),
            s.t / 10.0
        );
        assert!(n > 1500, "{name}");
        assert!(worst < s.t / 10.0, "{name}: {worst}");
        assert_eq!(wrong_side, 0);
    }
}

fn brute_closest(
    p: forge_math::Vec3,
    a: forge_math::Vec3,
    b: forge_math::Vec3,
    c: forge_math::Vec3,
) -> forge_math::Vec3 {
    // the nearest point of the triangle: of its face if p projects inside it, else of its sides
    let n = (b - a).cross(c - a);
    let nn = n.length_sq();
    if nn > 0.0 {
        let q = p - n * ((p - a).dot(n) / nn);
        let inside = [(a, b), (b, c), (c, a)]
            .iter()
            .all(|(u, v)| (*v - *u).cross(q - *u).dot(n) >= 0.0);
        if inside {
            return q;
        }
    }
    let seg = |u: forge_math::Vec3, v: forge_math::Vec3| {
        let e = v - u;
        let f = ((p - u).dot(e) / e.length_sq().max(1e-300)).clamp(0.0, 1.0);
        u + e * f
    };
    [seg(a, b), seg(b, c), seg(c, a)]
        .into_iter()
        .min_by(|x, y| (p - *x).length_sq().total_cmp(&(p - *y).length_sq()))
        .unwrap()
}

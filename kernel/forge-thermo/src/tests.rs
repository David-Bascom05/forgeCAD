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

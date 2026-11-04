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
    let shop = placeholder();
    assert!(shop.materials.len() >= 6);
    // a field with neither is refused, naming it
    let bad = material::DATA.replacen("\"source\": \"PLASTIGLAS", "\"note\": \"PLASTIGLAS", 1);
    let e = from_data(&bad).unwrap_err();
    assert_eq!(e.field, "forming_c");
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

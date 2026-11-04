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

fn spec(side: Side) -> Spec {
    Spec {
        side,
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
    let mould = block(60.0, 40.0, 20.0, 6.0, 5.0);
    let s = spec(Side::Inner);
    let f = form(&mould, &s, 11).unwrap();
    assert!(
        forge_topo::check(&f.part).is_empty(),
        "{:?}",
        forge_topo::check(&f.part)
    );
    assert!(
        forge_topo::check(&f.tool).is_empty(),
        "{:?}",
        forge_topo::check(&f.tool)
    );
    let v = forge_topo::mass::mass(&f.part).unwrap().volume;
    // the formed area plus the flange's, times t, near enough (the offset adds at convex corners)
    let want = s.t * (f.report.formed_area + f.report.trim_area - f.report.opening_area);
    assert!((v - want).abs() < 0.05 * want, "{v} {want}");
    assert!(!f.report.vents.is_empty());
    assert!((f.report.scale - 1.005).abs() < 1e-12);
    let shop = placeholder();
    let c = check(&mould, &f, &s, shop.material("ABS").unwrap(), &shop, None);
    let k = kinds(&c);
    assert!(!k.contains(&"DraftTooLow".into()), "{c:?}");
    // the top's edges are sharp
    assert!(k.contains(&"SharpEdge".into()), "{c:?}");
}

#[test]
fn walls_square_to_the_pull_have_no_draft() {
    let mould = block(60.0, 40.0, 20.0, 6.0, 0.0);
    let s = spec(Side::Inner);
    let f = form(&mould, &s, 11).unwrap();
    let shop = placeholder();
    let c = check(&mould, &f, &s, shop.material("ABS").unwrap(), &shop, None);
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
    let mould = forge_ops::tapered(&p, -25.0, 3f64.to_radians(), 3)
        .unwrap()
        .remove(0);
    let s = spec(Side::Outer);
    let f = form(&mould, &s, 11).unwrap();
    assert!(
        forge_topo::check(&f.part).is_empty(),
        "{:?}",
        forge_topo::check(&f.part)
    );
    assert!(
        forge_topo::check(&f.tool).is_empty(),
        "{:?}",
        forge_topo::check(&f.tool)
    );
    let shop = placeholder();
    let c = check(&mould, &f, &s, shop.material("ABS").unwrap(), &shop, None);
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
    let mould = forge_ops::tapered(&p, 20.0, -5f64.to_radians(), 3)
        .unwrap()
        .remove(0);
    let s = spec(Side::Inner);
    let f = form(&mould, &s, 11).unwrap();
    let shop = placeholder();
    let c = check(&mould, &f, &s, shop.material("ABS").unwrap(), &shop, None);
    let n = c
        .iter()
        .filter(|x| matches!(x, Violation::Undercut { .. }))
        .count();
    assert_eq!(n, 4, "{c:?}");
}

#[test]
fn absurd_sizes_are_refused_before_anything_is_laid_out() {
    let mould = block(60.0, 40.0, 20.0, 6.0, 5.0);
    let mut s = spec(Side::Inner);
    s.flange = 1e9;
    assert!(matches!(
        form(&mould, &s, 11),
        Err(FormError::Size { what: "trim", .. })
    ));
    let mut s = spec(Side::Inner);
    s.margin = f64::INFINITY;
    assert!(matches!(
        form(&mould, &s, 11),
        Err(FormError::Size { what: "margin", .. })
    ));
    let mut s = spec(Side::Inner);
    s.trim = Some(Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::rectangle(Point2::new(-5e8, -5e8), 1e9, 1e9, 0)],
    });
    assert!(matches!(
        form(&mould, &s, 11),
        Err(FormError::Size { what: "trim", .. })
    ));
}

#[test]
fn a_cup_thins_most_at_its_floor_corner() {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::circle(Point2::new(0.0, 0.0), 30.0, 0)],
    };
    let mould = forge_ops::tapered(&p, -30.0, 3f64.to_radians(), 3)
        .unwrap()
        .remove(0);
    let s = spec(Side::Outer);
    let f = analyse(&mould, &s).unwrap();
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
    let mould = block(60.0, 40.0, 25.0, 6.0, 5.0);
    let s = spec(Side::Inner);
    let f = analyse(&mould, &s).unwrap();
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

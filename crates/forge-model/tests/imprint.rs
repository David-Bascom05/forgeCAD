//! The local-imprint boolean against the general one: every boolean it takes on these parts is
//! done both ways and must come out the same, entity for entity.
use forge_bool::imprint::{Mode, set_mode, taken};
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_model::{Combine, Extent, Feature, Part, suite, torture};
use forge_ops::{Profile, ProfileLoop};

fn extrude(at: Point3, lp: ProfileLoop, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(at),
            loops: vec![lp],
        },
        extent: Extent::Distance(d),
        op,
    }
}

/// A plate with a tunnel through it, then tools of each kind this path takes, apart.
fn tools() -> Part {
    let p = |x, y, z| Point3::new(x, y, z);
    let o = Point2::ORIGIN;
    let tunnel = Feature::Extrude {
        profile: Profile {
            plane: Frame::from_zx(p(0.0, -1.0, 0.0), Vec3::Y.normalize().unwrap(), Vec3::X).unwrap(),
            loops: vec![Profile::rectangle(Point2::new(60.0, -6.0), 30.0, 2.0, 0)],
        },
        extent: Extent::Distance(82.0),
        op: Combine::Cut,
    };
    let features = vec![
        extrude(
            p(0.0, 0.0, 0.0),
            Profile::rectangle(o, 100.0, 80.0, 0),
            10.0,
            Combine::New,
        ),
        tunnel,
        extrude(
            p(10.0, 10.0, 6.0),
            Profile::rectangle(o, 12.0, 7.0, 0),
            5.0,
            Combine::Cut,
        ),
        extrude(
            p(30.0, 12.0, 8.0),
            Profile::rectangle(o, 9.0, 6.0, 0),
            7.0,
            Combine::Add,
        ),
        extrude(
            p(11.0, 30.0, 5.0),
            Profile::rounded_rectangle(o, 14.0, 6.0, 2.0, 0),
            6.0,
            Combine::Cut,
        ),
        extrude(
            p(31.0, 32.0, 9.0),
            Profile::rounded_rectangle(o, 10.0, 8.0, 2.0, 0),
            4.0,
            Combine::Add,
        ),
        extrude(
            p(15.0, 60.0, -1.0),
            Profile::circle(o, 3.0, 0),
            12.0,
            Combine::Cut,
        ),
        // its sides on the lines of the first pocket's
        extrude(
            p(10.0, 45.0, 7.0),
            Profile::rectangle(o, 12.0, 5.0, 0),
            4.0,
            Combine::Cut,
        ),
        // through both slabs either side of the tunnel
        extrude(
            p(70.0, 40.0, -1.0),
            Profile::circle(o, 2.5, 0),
            12.0,
            Combine::Cut,
        ),
        extrude(
            p(80.0, 20.0, -1.0),
            Profile::rectangle(o, 4.0, 3.0, 0),
            12.0,
            Combine::Cut,
        ),
    ];
    Part { features }
}

#[test]
#[ignore = "imprint names its new edges as the general boolean did before the naming batch; it is off"]
fn imprint_matches_the_general_boolean() {
    set_mode(Mode::Check);
    let before = taken();
    tools().regenerate().expect("tools part regenerates");
    let on_tools = taken() - before;
    torture::part(221).regenerate().expect("torture regenerates");
    let on_torture = taken() - before - on_tools;
    for name in suite::NAMES {
        let _ = suite::part(name).map(|p| p.regenerate());
    }
    // where the general boolean cuts at a box an arc in a turned frame makes wide: a cap's over an
    // edge of a face clear of the tool (11), faces only the tool's arcs' boxes reach (14)
    for seed in [11, 14] {
        let _ = suite::fuzz(seed, 80).regenerate();
    }
    set_mode(Mode::Off);
    assert_eq!(on_tools, 9, "the tools part's booleans not all taken");
    assert!(
        on_torture >= 150,
        "only {on_torture} booleans took the imprint path"
    );
}

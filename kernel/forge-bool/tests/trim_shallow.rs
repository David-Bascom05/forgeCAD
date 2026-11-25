use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3};
use forge_ops::{Profile, extrude};
use forge_topo::{check, mass};

/// Two blind holes of radius 5, floors at one depth, their centres `d` apart (9 to 9.9999 mm): the
/// second's floor meets the first's wall in the first's floor circle, a run along that edge from
/// 4.36 mm down to 0.045 mm long. Trim's 0.3 mm shallow-pass bound collapses the runs under 0.3 mm
/// to one touch and keeps the others' two ends (research/decide-once/flat-g11.md, boolean.rs:3337,
/// U); either way the result is valid and its volume the exact one, the plate less the two discs'
/// union (the lens counted once) times their depth. This pins today's answers on both sides of the
/// bound.
#[test]
fn two_blind_holes_a_hair_apart_have_the_exact_volume() {
    let plate = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(-20.0, -20.0), 50.0, 40.0, 0)],
        },
        20.0,
        1,
    )
    .unwrap();
    let hole = |x: f64, id: u64| {
        extrude(
            &Profile {
                plane: Frame::WORLD.with_origin(Point3::new(x, 0.0, 15.0)),
                loops: vec![Profile::circle(Point2::ORIGIN, 5.0, id)],
            },
            10.0,
            id + 100,
        )
        .unwrap()
    };
    let r: f64 = 5.0;
    for d in [9.0, 9.9, 9.99, 9.995, 9.998, 9.999, 9.9995, 9.9999] {
        let one = boolean(&plate, &hole(0.0, 2), Op::Subtract, 10).unwrap();
        let both = boolean(&one, &hole(d, 3), Op::Subtract, 11).unwrap();
        assert!(check(&both).is_empty(), "d {d}: {:?}", check(&both));
        let lens =
            2.0 * r * r * forge_math::dmath::acos(d / (2.0 * r)) - 0.5 * d * (4.0 * r * r - d * d).sqrt();
        let want = 50.0 * 40.0 * 20.0 - 5.0 * (2.0 * forge_math::dmath::PI * r * r - lens);
        let got = mass::mass_rel(&both, 1e-9).unwrap().volume;
        assert!((got - want).abs() <= 1e-9 * want, "d {d}: {got} vs {want}");
    }
}

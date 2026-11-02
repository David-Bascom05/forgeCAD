//! A lofted blade, its long edges rounded, joined to a hub revolved from a spline: every curve where
//! they meet ends on one of the round's springs, and a spring is not in a plane.
use forge_agent::model::Model;
use forge_bool::{Index, Op, Where, boolean};
use forge_math::{Point3, Vec3, dmath};
use forge_topo::Body;

fn body(src: &str) -> (Body, f64) {
    let mut m = Model::new(src);
    assert!(m.error.is_none(), "{src}: {:?}", m.error.map(|e| e.msg));
    let v = m.mass().expect("mass").volume;
    (m.body().expect("a body").clone(), v)
}

fn hub() -> String {
    let mut pts = String::new();
    for i in 1..=8 {
        let a = (90.0 * (1.0 - i as f64 / 8.0)).to_radians();
        pts += &format!("{} {} ", 55.0 - 40.0 * dmath::cos(a), 42.0 - 32.0 * dmath::sin(a));
    }
    format!(
        "param1 = 1\nparam2 = 1\nrevolve xz axis y loop spline 3 55 10 {pts}line 15 42 15 50 line 15 50 0 50 line 0 50 0 0 line 0 0 55 0 line 55 0 55 10\n"
    )
}

/// Two rectangles turned apart on a stack leaning out from the axis, the four long walls' edges rounded.
fn blade(az: f64) -> (String, Point3) {
    let (s, c) = dmath::sin_cos(az.to_radians());
    let (st, ct) = dmath::sin_cos(36f64.to_radians());
    let origin = Vec3::new(37.0 * c, 37.0 * s, 30.0);
    let z = Vec3::new(c * st, s * st, ct);
    let x0 = Vec3::new(c * ct, s * ct, -st);
    let y0 = z.cross(x0);
    let section = |d: f64, turn: f64| {
        let (sn, cs) = dmath::sin_cos(turn.to_radians());
        (origin + z * d, x0 * cs + y0 * sn, y0 * cs - x0 * sn)
    };
    let ends = [section(-15.0, 0.0), section(21.0, 48.0)];
    let mut src = String::from("loft smooth");
    for (o, x, _) in ends {
        src += &format!(
            " profile z {} {} {} x {} {} {} at {} {} {} rect -22 -2.5 44 5",
            z.x, z.y, z.z, x.x, x.y, x.z, o.x, o.y, o.z
        );
    }
    src += "\nfillet 1.5";
    for (a, b) in [(22.0, 2.5), (-22.0, 2.5), (-22.0, -2.5), (22.0, -2.5)] {
        let at = |(o, x, y): (Vec3, Vec3, Vec3)| o + x * a + y * b;
        let m = (at(ends[0]) + at(ends[1])) * 0.5;
        src += &format!(" (edges near {} {} {})", m.x, m.y, m.z);
    }
    src += "\n";
    let p = origin + z * 15.0;
    (src, Point3::new(p.x, p.y, p.z))
}

#[test]
fn a_rounded_blade_stands_on_a_spline_hub() {
    let (h, vh) = body(&hub());
    let (src, p) = blade(292.0);
    let (b, vb) = body(&src);
    let u = boolean(&h, &b, Op::Union, 11).expect("the union");
    let problems = forge_topo::check(&u);
    assert!(problems.is_empty(), "{problems:?}");
    let vu = forge_topo::mass::mass(&u).expect("mass").volume;
    assert!(
        vu > vh + 0.2 * vb && vu < vh + vb,
        "the blade has gone: {vu}, hub {vh}, blade {vb}"
    );
    // a point of the blade clear of the hub is in the union
    assert_eq!(Index::new(&b).classify(p), Where::Inside);
    assert_eq!(Index::new(&u).classify(p), Where::Inside);
}

//! A hole all but tangent to a rim whose wall goes vertical at the top: the curve where they meet climbs
//! to a tip far narrower than the marching step. Judged against slices, each a disk less a lens.
use forge_agent::model::Model;
use forge_math::dmath;

/// How much of a disk radius `a` a circle radius `r` at `c` from its centre covers.
fn lens(a: f64, r: f64, c: f64) -> f64 {
    if c >= a + r {
        return 0.0;
    }
    if c + r <= a {
        return dmath::PI * r * r;
    }
    let u = dmath::acos((c * c + a * a - r * r) / (2.0 * c * a));
    let v = dmath::acos((c * c + r * r - a * a) / (2.0 * c * r));
    let k = (-c + a + r) * (c + a - r) * (c - a + r) * (c + a + r);
    a * a * u + r * r * v - 0.5 * k.max(0.0).sqrt()
}

/// The volume by slices, spaced closely towards the top where the wall is steep.
fn sliced(rim: f64, big: f64, r: f64, c: f64) -> f64 {
    let n = 400_000;
    let (mut v, h) = (0.0, 24.0);
    for k in 0..n {
        // z = 40 - h t^3, t running from 1 to 0
        let t = (k as f64 + 0.5) / n as f64;
        let d = h * t * t * t;
        let dz = h * 3.0 * t * t / n as f64;
        let a = rim - big + (big * big - d * d).sqrt();
        v += (dmath::PI * a * a - lens(a, r, c)) * dz;
    }
    v
}

#[test]
fn a_hole_all_but_tangent_to_a_vertical_rim_has_the_volume_of_its_slices() {
    let (rim, big, r) = (42.0, 30.0, 2.5);
    let foot = rim - big + (big * big - 24.0f64 * 24.0).sqrt();
    let arc = dmath::atan2(24.0, 18.0).to_degrees();
    for gap in [1e-3, 1e-4, 3e-5, 1e-5] {
        for turn in [0.0, 37.0, 100.0, 200.0, 300.0] {
            let c = rim - gap - r;
            let (s, co) = dmath::sin_cos(f64::to_radians(turn));
            let src = format!(
                "revolve xz axis y loop line 0 16 {foot} 16 arc {} 40 {big} {} {arc} line {rim} 40 0 40 line 0 40 0 16\n\
                 extrude cut 30 xy at 0 0 14 circle {} {} {r}\n",
                rim - big,
                -arc,
                c * co,
                c * s
            );
            let mut m = Model::new(&src);
            assert!(m.error.is_none(), "{gap} {turn}: {:?}", m.error.map(|e| e.msg));
            let problems = forge_topo::check(m.body().expect("a body"));
            assert!(problems.is_empty(), "{gap} {turn}: {problems:?}");
            let p = m.mass().expect("mass");
            let want = sliced(rim, big, r, c);
            assert!(
                (p.volume - want).abs() < 1e-2 && (p.volume - want).abs() <= p.err[0] + 1e-3,
                "{gap} {turn}: {} against {want}, reported {}",
                p.volume,
                p.err[0]
            );
        }
    }
}

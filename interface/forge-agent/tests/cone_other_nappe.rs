//! A smooth loft through a cone's apex: the cone's equation holds its other nappe too, and the wall's
//! curve on that nappe is not on the cone.
use forge_agent::model::Model;
use forge_math::{Point3, dmath};

const BASE: &str =
    "base: revolve xz axis y loop line 0 -20 60 -20 line 60 -20 60 2 line 60 2 0 14 line 0 14 0 -20\n";

fn frame(z: f64, deg: f64) -> String {
    let (s, c) = dmath::sin_cos(deg.to_radians());
    format!("z 0 0 1 x {c} {s} 0 at 0 0 {z}")
}

fn blade(section: &str, twist: f64) -> String {
    let (a, b, c) = (frame(0.0, 0.0), frame(25.0, 0.5 * twist), frame(50.0, twist));
    format!("blade: loft smooth profile {a} {section} profile {b} {section} profile {c} {section}\n")
}

/// Where an edge of `m`'s body running up from z = 0 is at height z, for the four that do.
fn corners(m: &mut Model, z: f64) -> Vec<Point3> {
    let b = m.body().expect("a body");
    let mut out = Vec::new();
    for e in b.edge_ids() {
        let pts = b.edge_points(e, 400);
        let (zs, ze) = (pts[0].z, pts[pts.len() - 1].z);
        if zs.min(ze) > z || zs.max(ze) < z || (zs - ze).abs() < 1.0 {
            continue;
        }
        let i = pts
            .windows(2)
            .position(|w| w[0].z.min(w[1].z) <= z && w[0].z.max(w[1].z) >= z);
        let w = &pts[i.expect("it crosses z")..];
        out.push(w[0].lerp(w[1], (z - w[0].z) / (w[1].z - w[0].z)));
    }
    // round the section from the first
    let f = out[0];
    let turn = |p: &Point3| {
        dmath::atan2(f.x * p.y - f.y * p.x, f.x * p.x + f.y * p.y).rem_euclid(std::f64::consts::TAU)
    };
    out.sort_by(|p, q| turn(p).total_cmp(&turn(q)));
    out
}

/// The blade's volume inside the cone, by slices: its walls are straight across each height.
fn inside_cone(m: &mut Model) -> f64 {
    let (nz, nu, nv) = (60, 300, 300);
    let dz = 15.0 / nz as f64;
    let mut v = 0.0;
    for k in 0..nz {
        let z = (k as f64 + 0.5) * dz;
        let c = corners(m, z);
        assert_eq!(c.len(), 4);
        let at = |s: f64, t: f64| {
            let (p, q) = (c[0].lerp(c[1], s), c[3].lerp(c[2], s));
            p.lerp(q, t)
        };
        for i in 0..nu {
            for j in 0..nv {
                let (s, t) = ((i as f64 + 0.5) / nu as f64, (j as f64 + 0.5) / nv as f64);
                let p = at(s, t);
                let h = 1e-4;
                let (a, b) = (at(s + h, t) - at(s - h, t), at(s, t + h) - at(s, t - h));
                let area = (a.x * b.y - a.y * b.x).abs() / (4.0 * h * h);
                if z < 14.0 - 0.2 * (p.x * p.x + p.y * p.y).sqrt() {
                    v += area * dz / (nu * nv) as f64;
                }
            }
        }
    }
    v
}

#[test]
fn a_twisted_wall_cuts_only_the_cones_own_nappe() {
    let base = dmath::PI * 3600.0 * (22.0 + 4.0);
    for twist in [15.0, 30.0, 60.0] {
        let loft = blade("rect -25 -2 50 4", twist);
        let mut alone = Model::new(&loft);
        let blade_volume = alone.mass().expect("the blade's mass").volume;
        let cut = inside_cone(&mut alone);
        let mut m = Model::new(&format!("{BASE}{loft}"));
        assert!(m.error.is_none(), "{twist}: {:?}", m.error.map(|e| e.msg));
        let problems = forge_topo::check(m.body().expect("a body"));
        assert!(problems.is_empty(), "{twist}: {problems:?}");
        let volume = m.mass().expect("the union's mass").volume;
        let want = base + blade_volume - cut;
        assert!((volume - want).abs() < 2.0, "{twist}: {volume} against {want}");
    }
}

#[test]
fn rounded_sections_too() {
    for twist in [15.0, 30.0, 60.0] {
        let mut m = Model::new(&format!("{BASE}{}", blade("rrect -25 -2 50 4 1.5", twist)));
        let problems = forge_topo::check(m.body().expect("a body"));
        assert!(problems.is_empty(), "{twist}: {problems:?}");
        assert!(m.mass().expect("the union's mass").volume > 3e5, "{twist}");
    }
}

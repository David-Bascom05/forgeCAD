//! Reproducers shrunk from the Haas machine-model sweep (tools/kernel-stress, research/kernel-stress/
//! haas-report.md). Each stands for a class of failure the sweep found on real files, made from
//! nothing but the kernel's own parts so no model's geometry is in the repository.
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_ops::{Profile, extrude};
use forge_step::{Limits, Options, export, import};
use forge_topo::{Body, check};

/// A round bar: its one LINE is the seam, an edge with the one face on both sides.
fn bar(r: f64, h: f64) -> Body {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
    };
    extrude(&p, h, 1).unwrap()
}

/// The file's text with the first LINE's point moved by `d`: an edge that misses its faces, as
/// SolidWorks writes some (HA5C2: 20 edges up to 2.06e-4 mm off).
fn edge_off(text: &str, d: Vec3) -> String {
    let at = text.find("=LINE(").expect("a line");
    let refs: Vec<&str> = text[at..]
        .split(['#', ',', ')'])
        .filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty())
        .collect();
    let pt = format!("#{}=", refs[0]);
    let p0 = text.find(&pt).expect("the line's point");
    let open = p0 + text[p0..].find("',(").unwrap() + 3;
    let close = open + text[open..].find(')').unwrap();
    let xyz: Vec<f64> = text[open..close]
        .split(',')
        .map(|v| v.trim().parse().unwrap())
        .collect();
    let moved = format!("{:e},{:e},{:e}", xyz[0] + d.x, xyz[1] + d.y, xyz[2] + d.z);
    format!("{}{}{}", &text[..open], moved, &text[close..])
}

/// `b` moved rigidly by `d`, everything else as it was.
fn moved(b: &Body, d: Vec3) -> Body {
    use forge_geom::{CurveGeom, SurfaceGeom};
    let f = Frame::WORLD.with_origin(Point3::ORIGIN + d);
    let mut c = b.clone();
    for v in &mut c.vertices {
        v.point = f.point_to_world(v.point);
    }
    for e in &mut c.edges {
        e.curve = e.curve.transformed(&f);
        e.bbox = Default::default();
    }
    for x in &mut c.faces {
        x.surface = x.surface.transformed(&f);
        x.bbox = Default::default();
        x.query = Default::default();
    }
    c.hulls = Default::default();
    c
}

/// An edge a file leaves off its faces is read with a tolerance: it must hold with room, so the
/// body still checks once moved. It was set to the gap measured to the last bit, and the copy
/// beside it, or any boolean of it, failed `check` with EdgeOffFace at exactly that gap
/// (union_beside, union_over, int_over on HA5C2 and others).
#[test]
fn a_tolerant_edge_still_checks_once_moved() {
    let text = export(&bar(5.0, 10.0), &Options::default()).unwrap().text;
    assert_eq!(text.matches("=LINE(").count(), 1);
    // the seam, out of the cylinder by 0.2 um: nothing heals a seam, so it is read as tolerant
    let bad = edge_off(&text, Vec3::new(1.4e-4, 1.4e-4, 0.0));
    let imp = import(bad.as_bytes(), &Limits::default()).unwrap();
    let b = &imp.solids[0].body;
    assert!(imp.stats.tolerant_edges > 0, "the edge is read as tolerant");
    assert!(check(b).is_empty(), "{:?}", check(b));
    for d in [
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(123.456, -78.9, 0.001),
        Vec3::new(-1e3, 1e3, 1e3),
    ] {
        let p = check(&moved(b, d));
        assert!(p.is_empty(), "moved by {d:?}: {p:?}");
    }
}

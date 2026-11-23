//! The curved decide-once builder on kernel solids (feature `decide_once_curved`).
#![cfg(feature = "decide_once_curved")]

use forge_bool::decide_once_curved::{Kind, carrier_classes, near_pairs, read_faces, read_trims};
use forge_math::{Frame, Point2, Point3};
use forge_ops::{Profile, extrude};
use forge_topo::Body;

fn block(origin: Point3, w: f64, h: f64, d: f64, feature: u64) -> Body {
    let plane = Frame::WORLD.with_origin(origin);
    extrude(
        &Profile {
            plane,
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)],
        },
        d,
        feature,
    )
    .unwrap()
}

fn rod(centre: Point3, r: f64, len: f64, feature: u64) -> Body {
    let plane = Frame::WORLD.with_origin(centre);
    extrude(
        &Profile {
            plane,
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        len,
        feature,
    )
    .unwrap()
}

/// Stage (a) on a plate and a drill: faces, carrier classes, near pairs.
#[test]
fn stage_a_plate_and_drill() {
    let plate = block(Point3::ORIGIN, 30.0, 20.0, 6.0, 1);
    let drill = rod(Point3::new(15.0, 10.0, -1.0), 4.0, 8.0, 2);
    let fs = read_faces(&[&plate, &drill]).unwrap();
    let ncyl = fs.iter().filter(|f| f.q.kind == Kind::Cyl).count();
    assert!(ncyl >= 1, "the drill's wall is read");
    assert!(fs.iter().all(|f| f.outward), "both solids' faces face out");
    let (cls, keys) = carrier_classes(&fs);
    assert_eq!(cls.len(), fs.len());
    // the plate's 6 planes, the drill's 2 caps (on other planes) and its one cylinder
    assert_eq!(keys.len(), 9);
    let ps = near_pairs(&fs, &cls);
    // the cylinder wall meets the plate's top and bottom (and their boxes overlap)
    let wall: Vec<usize> = (0..fs.len()).filter(|&i| fs[i].q.kind == Kind::Cyl).collect();
    assert!(ps.iter().any(|&(i, j)| wall.contains(&i) || wall.contains(&j)));
    // a face's own body's neighbours are near too, but never two faces of one carrier
    assert!(ps.iter().all(|&(i, j)| cls[i] != cls[j]));
}

/// A drilled plate's hole wall is read with the solid outside it.
#[test]
fn stage_a_hole_wall_inward() {
    let plate = block(Point3::ORIGIN, 30.0, 20.0, 6.0, 1);
    let drill = rod(Point3::new(15.0, 10.0, -1.0), 4.0, 8.0, 2);
    let holed = forge_bool::boolean(&plate, &drill, forge_bool::Op::Subtract, 3).unwrap();
    let fs = read_faces(&[&holed]).unwrap();
    let walls: Vec<_> = fs.iter().filter(|f| f.q.kind == Kind::Cyl).collect();
    assert!(!walls.is_empty());
    assert!(walls.iter().all(|f| !f.outward));
}

/// Stage (c) part 1: a drilled plate's trims. The hole's rims are closed circles between the
/// wall and the plate's top and bottom; every other edge is a line between two planes.
#[test]
fn stage_c_trims_of_a_drilled_plate() {
    let plate = block(Point3::ORIGIN, 30.0, 20.0, 6.0, 1);
    let drill = rod(Point3::new(15.0, 10.0, -1.0), 4.0, 8.0, 2);
    let holed = forge_bool::boolean(&plate, &drill, forge_bool::Op::Subtract, 3).unwrap();
    let fs = read_faces(&[&holed]).unwrap();
    let (cls, _) = carrier_classes(&fs);
    let ts = read_trims(&[&holed], &fs, &cls).unwrap();
    let wall: Vec<usize> = (0..fs.len()).filter(|&i| fs[i].q.kind == Kind::Cyl).collect();
    let rims: Vec<_> = ts
        .iter()
        .filter(|t| wall.contains(&t.faces.0) != wall.contains(&t.faces.1))
        .collect();
    assert!(!rims.is_empty());
    // each rim lies between the wall and a plane of a different class
    assert!(rims.iter().all(|t| t.classes.0 != t.classes.1));
    // the box's 12 edges are between two planes
    assert!(ts.len() >= 12 + rims.len());
}

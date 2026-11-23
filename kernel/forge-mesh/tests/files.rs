//! Mesh files as a slicer or viewer gets them: each format written and read back by our own
//! readers, every object closed, manifold and facing out, its volume within what the chord
//! tolerance allows of the B-rep's exact volume, and the bytes the same on every target (CI runs
//! this natively and as wasm32). tools/meshcheck checks the same files with other readers.

use forge_math::hash::hash128;
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_mesh::scene::{IDENTITY, Item, Material, Object, Scene, Solids, body_objects};
use forge_mesh::{ObjOptions, Quality, read_3mf, read_obj, read_stl};
use forge_ops::{Axis2, Profile, extrude, revolve};
use forge_topo::Body;

/// A plate: rounded outline, a slot, two holes.
fn plate() -> Body {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rounded_rectangle(Point2::new(-40.0, -25.0), 80.0, 50.0, 8.0, 0),
            Profile::rounded_rectangle(Point2::new(-15.0, -4.0), 30.0, 8.0, 3.999, 100),
            Profile::circle(Point2::new(-30.0, 15.0), 3.25, 200),
            Profile::circle(Point2::new(30.0, 15.0), 3.25, 201),
        ],
    };
    extrude(&p, 6.0, 1).unwrap()
}

/// A ring with a hole through its section, revolved 270 degrees: tori, cones and planar caps.
fn ring() -> Body {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rounded_rectangle(Point2::new(8.0, 0.0), 14.0, 10.0, 3.0, 0),
            Profile::circle(Point2::new(15.0, 5.0), 2.5, 20),
        ],
    };
    let y = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };
    revolve(&p, y, 1.5 * dmath::PI, 1).unwrap()
}

fn bodies() -> Vec<(&'static str, Body)> {
    vec![("plate", plate()), ("ring", ring())]
}

fn objects(name: &str, b: &Body, q: &Quality, normals: bool) -> Vec<Object> {
    let by = Solids { name, solids: &[] };
    body_objects(b, q, &by, normals).unwrap()
}

fn scene() -> Scene {
    let q = Quality::default();
    let mut objs = Vec::new();
    for (n, b) in bodies() {
        objs.extend(objects(n, &b, &q, true));
    }
    objs[0].material = Some(0);
    let mut s = Scene::of_objects(
        "files test",
        objs,
        vec![Material {
            name: "PLA".into(),
            rgb: [10, 200, 30],
        }],
    );
    // a second ring, turned a quarter about z and moved: an assembly's second occurrence
    s.items.push(Item {
        object: 1,
        transform: [0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 100.0, 0.0, 0.0],
        name: "ring 2".into(),
    });
    s
}

#[test]
fn every_object_is_closed_manifold_and_faces_out() {
    for (n, b) in bodies() {
        for q in [
            Quality::default(),
            Quality {
                chord: 0.2,
                max_angle: 0.5,
            },
        ] {
            let objs = objects(n, &b, &q, true);
            assert_eq!(objs.len(), 1, "{n}: one piece");
            let c = objs[0].check();
            assert!(c.watertight() && c.manifold(), "{n}: {c:?}");
            assert_eq!(c.degenerate, 0, "{n}");
            // normals: one per corner, each facing the way its triangle does
            let o = &objs[0];
            assert_eq!(o.corner_normals.len(), o.triangles.len());
            for (t, cn) in o.triangles.iter().zip(&o.corner_normals) {
                let flat = o.trimesh().triangle_normal(*t);
                for &k in cn {
                    assert!(o.normals[k as usize].dot(flat) > 0.0, "{n}");
                }
            }
        }
    }
}

#[test]
fn volume_is_within_the_tolerance_of_the_exact() {
    for (n, b) in bodies() {
        let exact = forge_topo::mass::mass(&b).unwrap().volume;
        let mut last = f64::INFINITY;
        for chord in [0.1, 0.01, 0.001] {
            let q = Quality {
                chord,
                max_angle: Quality::default().max_angle,
            };
            let c = objects(n, &b, &q, false)[0].check();
            // every point of the mesh lies within `chord` of the surface: the volume between them
            // is at most the area times that
            let err = (c.volume - exact).abs();
            assert!(
                err <= c.area * chord,
                "{n} chord {chord}: {} vs {exact}",
                c.volume
            );
            assert!(err < last || err < 1e-9 * exact, "{n}: finer is closer");
            last = err;
        }
    }
}

#[test]
fn three_mf_round_trips() {
    let s = scene();
    let bytes = forge_mesh::to_3mf(&s).unwrap();
    let back = read_3mf(&bytes).unwrap();
    assert_eq!(back.title, s.title);
    assert_eq!(back.materials, s.materials);
    assert_eq!(back.items.len(), 3);
    for (a, b) in s.items.iter().zip(&back.items) {
        assert_eq!(a.object, b.object);
        assert_eq!(a.name, b.name);
        for (x, y) in a.transform.iter().zip(&b.transform) {
            assert!((x - y).abs() < 1e-12);
        }
    }
    for (a, b) in s.objects.iter().zip(&back.objects) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.material, b.material);
        assert_eq!(a.triangles, b.triangles, "shared vertices, the same triangles");
        for (p, q) in a.positions.iter().zip(&b.positions) {
            assert!((*p - *q).length() < 1e-6, "{p:?} {q:?}");
        }
        assert!(b.check().watertight());
    }
    for (a, b) in s.checks().iter().zip(back.checks()) {
        assert!((a.volume - b.volume).abs() < 1e-6 * a.volume);
    }
}

#[test]
fn obj_round_trips() {
    let s = scene();
    let text = forge_mesh::to_obj(
        &s,
        &ObjOptions {
            normals: true,
            face_groups: true,
        },
    );
    let back = read_obj(&text).unwrap();
    let flat = s.flatten();
    assert_eq!(back.len(), flat.objects.len());
    for (a, b) in flat.objects.iter().zip(&back) {
        assert_eq!(a.name.replace(' ', "_"), b.name);
        assert_eq!(a.triangles, b.triangles);
        assert_eq!(a.corner_normals, b.corner_normals);
        assert_eq!(a.normals.len(), b.normals.len());
        assert!(b.check().watertight());
        assert!((a.check().volume - b.check().volume).abs() < 1e-6 * a.check().volume);
    }
    let plain = read_obj(&forge_mesh::to_obj(&s, &ObjOptions::default())).unwrap();
    assert!(
        plain
            .iter()
            .all(|o| o.normals.is_empty() && o.check().watertight())
    );
}

#[test]
fn stl_round_trips_binary_and_ascii() {
    let s = scene();
    let flat = s.flatten();
    let want: usize = flat.objects.iter().map(|o| o.triangles.len()).sum();
    let bin = forge_mesh::scene_binary_stl(&s);
    assert_eq!(bin.len(), 84 + 50 * want);
    let (objs, wrong) = read_stl(&bin).unwrap();
    assert_eq!(wrong, 0, "facet normals agree with their corners");
    assert_eq!(objs.len(), 1);
    // welded at equal points the triangles close up again: the objects don't touch
    let c = objs[0].check();
    assert!(c.watertight(), "{c:?}");
    assert_eq!(
        c.vertices,
        flat.objects.iter().map(|o| o.positions.len()).sum::<usize>()
    );
    let ascii = forge_mesh::scene_ascii_stl(&s);
    let (objs, wrong) = read_stl(ascii.as_bytes()).unwrap();
    assert_eq!(wrong, 0);
    assert_eq!(objs.len(), flat.objects.len());
    for (a, b) in flat.objects.iter().zip(&objs) {
        assert_eq!(a.name.replace(' ', "_"), b.name);
        assert_eq!(a.triangles.len(), b.triangles.len());
        let (ca, cb) = (a.check(), b.check());
        assert!(cb.watertight(), "{cb:?}");
        assert!((ca.volume - cb.volume).abs() < 1e-4 * ca.volume);
    }
}

#[test]
fn a_mirroring_placement_still_faces_out() {
    let s = scene();
    let m = [-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
    let o = s.objects[0].transformed(&m);
    let (a, b) = (s.objects[0].check(), o.check());
    assert!(b.watertight());
    assert!((a.volume - b.volume).abs() < 1e-9 * a.volume);
    assert_eq!(IDENTITY[0], 1.0);
}

/// Every file's bytes, hashed: equal on every target.
const EXPECTED: u128 = 0x27900457fa17f6b90ab862a43b01d9bb;

#[test]
fn same_bytes_on_every_target() {
    let s = scene();
    let mut all = Vec::new();
    all.extend(forge_mesh::to_3mf(&s).unwrap());
    all.extend(forge_mesh::scene_binary_stl(&s));
    all.extend(forge_mesh::scene_ascii_stl(&s).into_bytes());
    all.extend(
        forge_mesh::to_obj(
            &s,
            &ObjOptions {
                normals: true,
                face_groups: true,
            },
        )
        .into_bytes(),
    );
    // and again: one process, the same bytes
    assert_eq!(
        forge_mesh::to_3mf(&s).unwrap(),
        forge_mesh::to_3mf(&scene()).unwrap()
    );
    let got = hash128(&all);
    assert_eq!(got, EXPECTED, "mesh files changed: {got:#034x}");
}

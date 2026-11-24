//! 3+2 orientation (orient::solve): table-table on the spinner-u5-620 kinematics and
//! generic-trunnion-bc, head-table and head-head variants of the trunnion, refusals.
use forge_machine::def::MachineDef;
use forge_machine::kin::{self, M3, V3};
use forge_machine::library;
use forge_machine::orient::{self, OrientError};

const U5_620: &str = include_str!("data/spinner-u5/u5-620-kinematics.toml");

fn u5() -> MachineDef {
    MachineDef::parse_all(U5_620).unwrap().remove(0)
}
fn trunnion() -> MachineDef {
    library::builtin_named("generic-trunnion-bc").unwrap()
}
fn trunnion_src() -> String {
    let d = include_str!("../data/machines/generic.toml");
    let i = d.find("[[machine]]\nname = \"generic-trunnion-bc\"").unwrap();
    let rest = &d[i + 1..];
    let j = rest.find("\n[[machine]]").map(|j| i + 1 + j).unwrap_or(d.len());
    d[i..j].to_string()
}
fn variant(edit: impl Fn(String) -> String) -> MachineDef {
    let s = edit(trunnion_src());
    MachineDef::parse_all(&s)
        .map(|mut v| v.remove(0))
        .map_err(|e| format!("{e:?}\n{s}"))
        .unwrap()
}

/// A right-handed plane with normal n (unit) and x along the given direction's part off n.
fn plane(n: V3, x: V3) -> M3 {
    let x = kin::sub(x, kin::scale(n, kin::dot(n, x)));
    let x = kin::scale(x, 1.0 / kin::norm(x));
    let y = kin::cross(n, x);
    [[x[0], y[0], n[0]], [x[1], y[1], n[1]], [x[2], y[2], n[2]]]
}

fn joints(o: &orient::Pose) -> Vec<(String, f64)> {
    o.joints
        .iter()
        .map(|(n, v)| (n.clone(), (v * 1e9).round() / 1e9))
        .collect()
}

/// The tool, in the part's frame at the chosen pose, lies along the plane's normal.
fn check_along(def: &MachineDef, o: &orient::Pose, n: V3) {
    let mut q = vec![0.0; def.joints.len()];
    for (name, v) in &o.joints {
        q[def.joint_index(name).unwrap()] = *v;
    }
    let (_, ax) = kin::tip_in_part(def, &q, "spindle", "table", 0.0).unwrap();
    assert!(kin::norm(kin::sub(ax, n)) < 1e-9, "{ax:?} vs {n:?}");
}

#[test]
fn u5_side_face_is_b90_c0() {
    // Achskreuz's X_ACHSE: the part's +x face up at B90 C0 (AROT checks, tests/traori.rs)
    let m = u5();
    let n = [1.0, 0.0, 0.0];
    let o = orient::solve(&m, "table", "spindle", &plane(n, [0.0, 1.0, 0.0])).unwrap();
    assert_eq!(joints(&o.chosen), vec![("B".into(), 90.0), ("C".into(), 0.0)]);
    assert_eq!(o.chosen.tilt, 90.0);
    check_along(&m, &o.chosen, n);
    // B -90 with C ±180 is the other solution (B prefers +); C ±360 the same pose a turn round
    let alts: Vec<_> = o.alternatives.iter().map(joints).collect();
    assert!(
        alts.contains(&vec![("B".into(), -90.0), ("C".into(), 180.0)]),
        "{alts:?}"
    );
    assert!(
        alts.contains(&vec![("B".into(), 90.0), ("C".into(), 360.0)]),
        "{alts:?}"
    );
    for a in &o.alternatives {
        check_along(&m, a, n);
    }
}

#[test]
fn top_face_needs_no_tilt() {
    for m in [u5(), trunnion()] {
        let o = orient::solve(&m, "table", "spindle", &plane([0.0, 0.0, 1.0], [1.0, 0.0, 0.0])).unwrap();
        assert_eq!(
            joints(&o.chosen),
            vec![("B".into(), 0.0), ("C".into(), 0.0)],
            "{}",
            m.name
        );
        assert_eq!(o.chosen.spin, 0.0);
    }
}

#[test]
fn oblique_faces_reach_along_the_normal() {
    for m in [u5(), trunnion()] {
        for n in [
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.6, 0.0, 0.8],
            [0.48, -0.36, 0.8],
        ] {
            let o = orient::solve(&m, "table", "spindle", &plane(n, [1.0, 0.3, 0.1])).unwrap();
            check_along(&m, &o.chosen, n);
            for a in &o.alternatives {
                check_along(&m, a, n);
                assert!(a.tilt >= o.chosen.tilt - 1e-9 || m.joints.iter().any(|j| j.preference != 0));
            }
        }
    }
}

#[test]
fn spin_is_the_plane_x_seen_from_the_tool() {
    let m = u5();
    // the plane's x along part +y on the +x face: at B90 C0 part +y stays base +y
    let o = orient::solve(&m, "table", "spindle", &plane([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])).unwrap();
    assert_eq!(o.chosen.spin, 90.0);
}

#[test]
fn out_of_range_and_unreachable_refuse() {
    // the U5-620's B stops at -91 / 110: a face pointing down (-z) needs B 180
    let m = u5();
    match orient::solve(&m, "table", "spindle", &plane([0.0, 0.0, -1.0], [1.0, 0.0, 0.0])) {
        Err(OrientError::OutOfRange(c)) => assert!(c.iter().all(|(_, j)| j == "B"), "{c:?}"),
        r => panic!("{r:?}"),
    }
    // the trunnion's B stops at ±120: a normal 130 deg from +z is out
    let t = trunnion();
    let (s, c) = kin::sin_cos_deg(130.0);
    let n = [s, 0.0, c];
    assert!(matches!(
        orient::solve(&t, "table", "spindle", &plane(n, [0.0, 1.0, 0.0])),
        Err(OrientError::OutOfRange(_))
    ));
    // a plane that is not a rotation
    let bad = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
    assert!(matches!(
        orient::solve(&m, "table", "spindle", &bad),
        Err(OrientError::Plane(_))
    ));
}

#[test]
fn head_table_and_head_head() {
    // head-table: B swivels the head (on Z), C turns the table
    let ht = variant(|s| {
        s.replace(
            "name = \"B\"\nparent = \"base\"\nside = \"table\"",
            "name = \"B\"\nparent = \"Z\"\nside = \"head\"",
        )
        .replace("name = \"C\"\nparent = \"B\"", "name = \"C\"\nparent = \"base\"")
        .replace("kind = \"tool\"\nlink = \"Z\"", "kind = \"tool\"\nlink = \"B\"")
        .replace("kind = \"table-table\"", "kind = \"head-table\"")
    });
    // head-head: C on Z, B on C, the spindle on B; the part on the base
    let hh = variant(|s| {
        s.replace(
            "name = \"B\"\nparent = \"base\"\nside = \"table\"",
            "name = \"B\"\nparent = \"C\"\nside = \"head\"",
        )
        .replace(
            "name = \"C\"\nparent = \"B\"\nside = \"table\"",
            "name = \"C\"\nparent = \"Z\"\nside = \"head\"",
        )
        .replace("kind = \"tool\"\nlink = \"Z\"", "kind = \"tool\"\nlink = \"B\"")
        .replace(
            "kind = \"part\"\nlink = \"C\"",
            "kind = \"part\"\nlink = \"base\"",
        )
        .replace("kind = \"table-table\"", "kind = \"head-head\"")
    });
    for m in [ht, hh] {
        for n in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.6, 0.0, 0.8],
            [0.48, -0.36, 0.8],
        ] {
            let o = orient::solve(&m, "table", "spindle", &plane(n, [0.0, 0.0, 1.0]))
                .unwrap_or_else(|e| panic!("{}: {e}", m.name));
            check_along(&m, &o.chosen, n);
            for a in &o.alternatives {
                check_along(&m, a, n);
            }
        }
    }
}

#[test]
fn plane_frame_maps_the_plane_origin() {
    let m = u5();
    let pl = plane([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let o = orient::solve(&m, "table", "spindle", &pl).unwrap();
    let mut q = vec![0.0; m.joints.len()];
    for (n, v) in &o.chosen.joints {
        q[m.joint_index(n).unwrap()] = *v;
    }
    let origin = [40.0, 5.0, 12.0];
    let f = orient::plane_frame(&m, &q, "table", &pl, origin).unwrap();
    // the plane's z is base +z; its origin is where the part point lands
    assert!(kin::norm(kin::sub(f.dir([0.0, 0.0, 1.0]), [0.0, 0.0, 1.0])) < 1e-9);
    let pf = kin::part_frame(&m, &kin::pose(&m, &q), "table").unwrap();
    assert!(kin::norm(kin::sub(f.apply([0.0; 3]), pf.apply(origin))) < 1e-9);
}

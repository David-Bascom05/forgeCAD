//! Robot documents (research/export/urdf-editor.md): a URDF linked to the arm in
//! examples/robot with overrides on top. Each override applied, each refusal typed, conflicts when
//! the source moves under an override and none once it is reset, convex-hull collisions that hold
//! every vertex, re-rooting and freezing checked by reading the URDF back and running its forward
//! kinematics against the assembly's own solve, and the bytes the same every time and on wasm32.

mod urdf_util;

use forge_lang::robot::{self, RobotError};
use forge_lang::urdf::{self, Exported, UrdfError};
use forge_lang::{Options, Run};
use std::path::{Path, PathBuf};
use urdf_util::*;

fn arm_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/robot")
}

fn tmp(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("robot-{name}"));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// The arm's assembly, solved, with its source text changed by `edit` (a param, say).
fn arm_run(edit: &dyn Fn(String) -> String) -> (Run, PathBuf) {
    let d = arm_dir();
    let src = edit(std::fs::read_to_string(d.join("arm2.fg")).expect("the example"));
    let opts = Options {
        dir: Some(d.clone()),
        render: true,
        kin: true,
        overlap: false,
        ..Options::default()
    };
    let run = forge_lang::run(&src, opts, Default::default()).0;
    assert!(
        run.errors.is_empty(),
        "{:?}",
        run.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    (run, d)
}

/// A robot document over the arm: its header, then `body`.
fn doc(body: &str) -> String {
    let src = arm_dir().join("arm2.fg");
    format!(
        "robot\nsrc = source(\"{}\")\nactuation(effort: 5, velocity: 1)\n{body}",
        src.display()
    )
}

fn export_with(text: &str, edit: &dyn Fn(String) -> String) -> (Run, Result<Exported, UrdfError>) {
    let r = robot::read(text, Some(&tmp("docs"))).unwrap_or_else(|e| panic!("{e:?}"));
    let (run, d) = arm_run(edit);
    let e = urdf::export_robot(&run, "robot", Some(&d), &urdf::Options::default(), Some(&r));
    (run, e)
}

fn export(body: &str) -> (Run, Result<Exported, UrdfError>) {
    export_with(&doc(body), &|s| s)
}

fn ok(name: &str, e: Result<Exported, UrdfError>) -> (Exported, El) {
    let e = e.unwrap_or_else(|err| panic!("{name}: {err}"));
    let robot = parse(&e.urdf);
    check_urdf(name, &e);
    (e, robot)
}

/// The largest gap (m) between where the URDF draws each visual of an occurrence and where
/// `run`'s solve put that occurrence.
fn gap(run: &Run, robot: &El, e: &Exported) -> f64 {
    let world = forward(robot, e);
    let asm = run.asm.as_ref().expect("an assembly");
    let mut worst = 0.0f64;
    for l in robot.all("link") {
        for v in l.all("visual") {
            let Some((_, _, pose)) = asm.occurrences.iter().find(|o| o.0 == v.attr("name")) else {
                continue;
            };
            let got = mul(&world[l.attr("name")], &origin_of(v.kid("origin")));
            let want = frame_m4(pose);
            for i in 0..3 {
                for j in 0..4 {
                    worst = worst.max((got[i][j] - want[i][j]).abs());
                }
            }
        }
    }
    worst
}

fn link<'a>(robot: &'a El, name: &str) -> &'a El {
    robot
        .all("link")
        .find(|l| l.attr("name") == name)
        .unwrap_or_else(|| panic!("link {name}"))
}

fn joint<'a>(robot: &'a El, name: &str) -> &'a El {
    robot
        .all("joint")
        .find(|l| l.attr("name") == name)
        .unwrap_or_else(|| panic!("joint {name}"))
}

#[test]
fn a_document_is_read_and_its_bad_lines_are_named() {
    assert!(robot::is_robot("robot\nsrc = source(\"a.fg\")\n"));
    assert!(!robot::is_robot("shared x = 1\n"));
    let e = robot::read(
        "robot\nsrc = source(\"a.fg\")\nwiggle(src.a)\njoint(src.j, colour: 1)\n",
        None,
    );
    let errs: Vec<RobotError> = e.expect_err("two bad lines");
    assert_eq!(errs.iter().map(|e| e.line).collect::<Vec<_>>(), vec![3, 4]);
    assert!(errs[0].message.contains("wiggle"));
    assert!(errs[1].message.contains("colour"));
    let r = robot::read(
        "robot\nshared lo = -90deg\nsrc = source(none)\njoint(src.e, limit: (lo, 90deg), effort: 2 * 3)\n",
        None,
    )
    .expect("reads");
    let j = &r.joints[0];
    assert_eq!(j.limit, Some((-90f64.to_radians(), 90f64.to_radians())));
    assert_eq!(j.effort, Some(6.0));
    assert!(r.source.is_none());
}

#[test]
fn a_robot_with_no_overrides_is_the_export_with_hull_collisions() {
    let (run, e) = export("");
    let (e, robot) = ok("plain", e);
    assert!(gap(&run, &robot, &e) < 1e-9);
    let names: Vec<&str> = robot.all("joint").map(|j| j.attr("name")).collect();
    assert_eq!(names, vec!["shoulder", "elbow", "grip"]);
    // every link's collision is its hull, which holds every vertex of its visuals
    for l in robot.all("link") {
        let c = l.kid("collision").expect("a collision");
        let file = c
            .kid("geometry")
            .and_then(|g| g.kid("mesh"))
            .expect("a mesh")
            .attr("filename")
            .to_string();
        assert!(file.ends_with("_hull.stl"), "{file}");
        let hull = &e.meshes.iter().find(|m| m.0 == file).expect("the hull's file").1;
        let (objects, _) = forge_mesh::read_stl(hull).expect("an STL");
        let h = &objects[0];
        let check = forge_mesh::Scene::of_objects("h", objects.clone(), vec![]).checks()[0];
        assert!(
            check.watertight() && check.manifold() && check.volume > 0.0,
            "{file}"
        );
        for v in l.all("visual") {
            let vf = v
                .kid("geometry")
                .and_then(|g| g.kid("mesh"))
                .expect("mesh")
                .attr("filename");
            let (vo, _) =
                forge_mesh::read_stl(&e.meshes.iter().find(|m| m.0 == vf).expect("mesh").1).expect("stl");
            let at = origin_of(v.kid("origin"));
            for p in vo.iter().flat_map(|o| o.positions.iter()) {
                // the visual's point in the link frame, mm
                let q = [
                    at[0][0] * p.x + at[0][1] * p.y + at[0][2] * p.z + at[0][3] * 1000.0,
                    at[1][0] * p.x + at[1][1] * p.y + at[1][2] * p.z + at[1][3] * 1000.0,
                    at[2][0] * p.x + at[2][1] * p.y + at[2][2] * p.z + at[2][3] * 1000.0,
                ];
                for t in &h.triangles {
                    let [a, b, c] = t.map(|i| h.positions[i as usize]);
                    let n = (b - a).cross(c - a);
                    let d = n.dot(forge_math::Point3::new(q[0], q[1], q[2]) - a) / n.length();
                    // STL is f32: the hull's corners are within ~1e-5 mm of the points
                    assert!(d < 1e-4, "{file}: a vertex {d} mm outside");
                }
            }
        }
    }
}

#[test]
fn names_frames_primitives_dynamics_and_transmissions() {
    let (run, e) = export(
        "forearm = link(src.fore)
collision(src.base, box(size: (80, 80, 20), at: pose(at: (0, 0, 10))))
collision(src.upper, capsule(r: 12, len: 120, at: pose(at: (60, 0, 5), z: (1, 0, 0))))
collisions(\"mesh\")
tool0 = frame(on: forearm, at: pose(at: (144, 0, 5)))
joint(src.elbow, damping: 0.1, friction: 0.02)
transmission(src.shoulder, actuator: \"m1\", reduction: 50, interface: \"hardware_interface/EffortJointInterface\")
",
    );
    let (e, robot) = ok("names", e);
    assert!(gap(&run, &robot, &e) < 1e-9);
    assert_eq!(
        joint(&robot, "elbow").kid("child").expect("child").attr("link"),
        "forearm"
    );
    let b = link(&robot, "base")
        .kid("collision")
        .and_then(|c| c.kid("geometry"))
        .and_then(|g| g.kid("box"));
    assert_eq!(b.expect("a box").attr("size"), "0.08 0.08 0.02");
    let up: Vec<&str> = link(&robot, "upper")
        .all("collision")
        .map(|c| c.kid("geometry").expect("g").kids[0].name.as_str())
        .collect();
    assert_eq!(up, vec!["cylinder", "sphere", "sphere"]);
    // collisions("mesh"): the others keep their visual mesh
    let fc = link(&robot, "forearm")
        .kid("collision")
        .and_then(|c| c.kid("geometry"))
        .and_then(|g| g.kid("mesh"));
    assert_eq!(fc.expect("mesh").attr("filename"), "meshes/link.stl");
    let t = joint(&robot, "tool0_joint");
    assert_eq!(
        (t.attr("type"), t.kid("parent").expect("p").attr("link")),
        ("fixed", "forearm")
    );
    let dy = joint(&robot, "elbow").kid("dynamics").expect("dynamics");
    assert_eq!((dy.attr("damping"), dy.attr("friction")), ("0.1", "0.02"));
    assert!(e.urdf.contains("<mechanicalReduction>50</mechanicalReduction>"));
    assert!(
        e.urdf
            .contains("<hardwareInterface>hardware_interface/EffortJointInterface</hardwareInterface>")
    );
    let fields: Vec<(&str, &str)> = e.overrides.iter().map(|o| (o.1.as_str(), o.2.as_str())).collect();
    assert_eq!(
        fields,
        vec![("joint(elbow)", "damping"), ("joint(elbow)", "friction")]
    );
}

#[test]
fn a_merge_across_a_moving_joint_needs_a_freeze() {
    let (_, e) = export("merge(src.fore, src.finger)\n");
    match e {
        Err(UrdfError::Robot(m)) => assert!(m.contains("freeze(src.grip"), "{m}"),
        other => panic!("{other:?}"),
    }
    let (run, e) = export("freeze(src.grip, at: 0)\nmerge(src.fore, src.finger)\n");
    let (e, robot) = ok("merge", e);
    assert!(
        gap(&run, &robot, &e) < 1e-9,
        "frozen at its solved value: nothing moves"
    );
    let fore = link(&robot, "fore");
    let vs: Vec<&str> = fore.all("visual").map(|v| v.attr("name")).collect();
    assert_eq!(vs, vec!["fore", "finger"]);
    assert!(robot.all("joint").all(|j| j.attr("name") != "grip"));
    let m = |n: &str| {
        e.links
            .iter()
            .find(|l| l.name == n)
            .and_then(|l| l.mass)
            .expect("mass")
    };
    // the merged link's mass is both parts' (same as the plain export's two links)
    let (_, plain) = export("");
    let plain = plain.expect("plain");
    let pm = |n: &str| {
        plain
            .links
            .iter()
            .find(|l| l.name == n)
            .and_then(|l| l.mass)
            .expect("mass")
    };
    assert!((m("fore") - (pm("fore") + pm("finger"))).abs() < 1e-15);
}

#[test]
fn freezing_at_another_value_moves_the_subtree_as_the_assembly_would() {
    // the elbow frozen at 20deg: fore and finger where the assembly puts them with elbow.turn = 20deg
    let (_, e) = export("freeze(src.elbow, at: 20deg)\n");
    let (e, robot) = ok("freeze", e);
    let (moved, _) = arm_run(&|s| s.replace("param elbow.turn = -60deg", "param elbow.turn = 20deg"));
    assert!(gap(&moved, &robot, &e) < 1e-9, "{}", gap(&moved, &robot, &e));
    assert_eq!(joint(&robot, "elbow").attr("type"), "fixed");
    let (_, e) = export("freeze(src.elbow, at: (1, 2))\n");
    assert!(matches!(e, Err(UrdfError::Robot(m)) if m.contains("takes 1 value")));
}

#[test]
fn a_robot_can_be_rooted_anywhere_and_puts_every_part_in_place() {
    let (run, e) = export("root(src.fore)\n");
    let (e, robot) = ok("reroot", e);
    assert!(gap(&run, &robot, &e) < 1e-9);
    let kids: Vec<&str> = robot
        .all("joint")
        .map(|j| j.kid("child").expect("c").attr("link"))
        .collect();
    assert!(!kids.contains(&"fore"), "fore is the root: {kids:?}");
    // the elbow is now crossed from its A side: its axis turned round
    assert_eq!(
        joint(&robot, "elbow").kid("axis").expect("axis").attr("xyz"),
        "0 0 -1"
    );
}

#[test]
fn drops_take_leaves_and_refuse_links_others_hang_from() {
    let (_, e) = export("drop(src.finger)\n");
    let (_, robot) = ok("drop", e);
    assert!(robot.all("link").all(|l| l.attr("name") != "finger"));
    assert!(robot.all("joint").all(|j| j.attr("name") != "grip"));
    let (_, e) = export("drop(src.upper)\n");
    assert!(matches!(e, Err(UrdfError::Robot(m)) if m.contains("hang from it")));
    let (_, e) = export("drop(src.base)\n");
    assert!(matches!(e, Err(UrdfError::Robot(m)) if m.contains("root")));
}

#[test]
fn an_inertia_override_is_checked_physical_and_listed() {
    let (_, plain) = export("");
    let m0 = plain.expect("plain").links[1].mass.expect("upper's mass");
    let (_, e) = export(&format!(
        "inertial(src.upper, mass: 0.2, mass_was: {m0}, com: (60, 0, 5), inertia: (10, 0, 0, 400, 0, 400))\n"
    ));
    let (e, robot) = ok("inertial", e);
    let i = link(&robot, "upper").kid("inertial").expect("inertial");
    assert_eq!(i.kid("mass").expect("m").attr("value"), "0.2");
    assert_eq!(i.kid("inertia").expect("i").attr("iyy"), "0.0004");
    let up = e.links.iter().find(|l| l.name == "upper").expect("upper");
    assert!(up.err.is_none(), "an override's bounds are not known");
    assert!(e.overrides.iter().any(|o| o.2 == "mass" && o.4 == "0.2"));
    // a principal moment above the other two's sum: no body has it
    let (_, e) = export("inertial(src.upper, inertia: (1, 0, 0, 1, 0, 5))\n");
    assert!(matches!(e, Err(UrdfError::Robot(m)) if m.contains("not one a body can have")));
    let (_, e) = export("inertial(src.upper, mass: -1)\n");
    assert!(matches!(e, Err(UrdfError::Robot(m)) if m.contains("not positive")));
}

#[test]
fn an_override_against_a_moved_source_is_a_conflict_until_kept_or_reset() {
    let body = "joint(src.elbow, limit: (-90deg, 90deg), limit_was: (-135deg, 135deg))\n";
    // the source as the override was made: applied, listed with the linked value
    let (_, e) = export(body);
    let (e, robot) = ok("limit", e);
    let lim = joint(&robot, "elbow").kid("limit").expect("limit");
    assert_eq!(lim.attr("upper"), "1.5707963267948966");
    assert_eq!(e.overrides[0].3, "(-135deg, 135deg)");
    // the source's limit changes under it: refused, with keep and reset
    let moved = |s: String| s.replace("param elbow.hi = 135deg", "param elbow.hi = 120deg");
    let (_, e) = export_with(&doc(body), &moved);
    match &e {
        Err(err @ UrdfError::Conflicts(cs)) => {
            assert_eq!(cs.len(), 1);
            assert_eq!(
                (cs[0].was.as_str(), cs[0].now.as_str()),
                ("(-135deg, 135deg)", "(-135deg, 120deg)")
            );
            assert_eq!(cs[0].line, 4);
            let f = err.fixes();
            assert!(f[0].contains("keep (limit_was: (-135deg, 120deg))"), "{f:?}");
            assert!(f[1].contains("reset"));
        }
        other => panic!("{other:?}"),
    }
    // kept: the was rewritten to the source's; reset: the statement gone, the source's limit back
    let kept = body.replace("limit_was: (-135deg, 135deg)", "limit_was: (-135deg, 120deg)");
    assert!(export_with(&doc(&kept), &moved).1.is_ok());
    let (_, e) = export_with(&doc(""), &moved);
    let (_, robot) = ok("reset", e);
    let up = joint(&robot, "elbow")
        .kid("limit")
        .expect("limit")
        .attr("upper")
        .to_string();
    assert_eq!(floats(&up)[0], 120f64.to_radians());
}

#[test]
fn a_robot_export_is_the_same_bytes_every_time_and_on_wasm() {
    let body = "forearm = link(src.fore)\nfreeze(src.grip, at: 0)\nmerge(forearm, src.finger)\ntool0 = frame(on: forearm, at: pose(at: (144, 0, 5)))\n";
    // the source path differs between checkouts: the document is read with it, the bytes not
    let (_, a) = export(body);
    let (_, b) = export(body);
    let (a, b) = (a.expect("a"), b.expect("b"));
    assert_eq!(a.zip, b.zip);
    let h = forge_math::hash::hash128(&a.zip);
    assert_eq!(
        format!("{h:032x}"),
        "0aca847da2db7dddc877ee68e13182db",
        "the robot's ZIP"
    );
}

#[test]
fn the_example_robot_document_exports() {
    let d = arm_dir();
    let text = std::fs::read_to_string(d.join("arm2_robot.fg")).expect("the example");
    let r = robot::read(&text, Some(&d)).expect("reads");
    let (run, _) = arm_run(&|s| s);
    let e = urdf::export_robot(&run, "arm2_robot", Some(&d), &urdf::Options::default(), Some(&r));
    let (e, robot) = ok("example", e);
    assert!(gap(&run, &robot, &e) < 1e-9);
    let _ = Path::new("");
}

/// The largest gap (mm, and axis × 1000) between the view's placement of each occurrence and
/// `run`'s solve.
fn view_gap(run: &Run, placed: &[(String, forge_math::Frame)]) -> f64 {
    let asm = run.asm.as_ref().expect("an assembly");
    let mut worst = 0.0f64;
    let mut seen = 0;
    for (path, f) in placed {
        let Some((_, _, pose)) = asm.occurrences.iter().find(|o| o.0 == *path) else {
            continue;
        };
        seen += 1;
        worst = worst
            .max((f.origin - pose.origin).length())
            .max((f.x().get() - pose.x().get()).length() * 1000.0)
            .max((f.z().get() - pose.z().get()).length() * 1000.0);
    }
    assert!(seen > 0, "no occurrence placed");
    worst
}

#[test]
fn the_view_drives_the_robot_as_the_assembly_moves() {
    let (run, e) = export("");
    let v = e.expect("plain").view;
    assert_eq!(v.root, "base");
    // at the export pose: where the solve put every part
    let at = v.place(&std::collections::BTreeMap::new());
    assert!(view_gap(&run, &v.occurrences(&at)) < 1e-9);
    // the elbow at 20deg and the finger slid 7 mm: as the assembly solves those values
    let q = std::collections::BTreeMap::from([
        ("elbow".to_string(), 20f64.to_radians()),
        ("grip".to_string(), 0.007),
    ]);
    let (moved, _) = arm_run(&|s| {
        s.replace("param elbow.turn = -60deg", "param elbow.turn = 20deg")
            .replace("param grip.along = 0", "param grip.along = 7")
    });
    let placed = v.occurrences(&v.place(&q));
    assert!(view_gap(&moved, &placed) < 1e-9, "{}", view_gap(&moved, &placed));
    // held within limits
    let elbow = v.joints.iter().find(|j| j.name == "elbow").expect("elbow");
    assert_eq!(elbow.clamp(10.0), 135f64.to_radians());
    // a centre of mass in mm, a mass in kg
    let up = v.links.iter().find(|l| l.name == "upper").expect("upper");
    assert!((up.com[0] - 60.0).abs() < 1e-9 && up.mass.is_some());
}

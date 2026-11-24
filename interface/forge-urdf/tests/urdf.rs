//! URDF export (research/export/urdf.md): a part's mass and inertia against the closed forms of
//! a box and a cylinder; assemblies (a two-link arm both ways round, a slider, cylindrical and
//! ball chains, a gear's mimic, a floating body) checked by reading the URDF back with a small
//! XML reader of its own and running its forward kinematics at the export pose, which must put
//! every part where the solve put it; a closed loop refused with its joints named, and cut on
//! request; byte-identical output twice over; and urdfdom's `check_urdf` on every file when it
//! is installed (`apt install liburdfdom-tools`).

use forge_lang::{Options, Run};
use forge_math::dmath;
use forge_urdf::{self as urdf, Exported, UrdfError};
use std::path::{Path, PathBuf};

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("urdf-{name}"));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn write(d: &Path, file: &str, text: &str) {
    std::fs::write(d.join(file), text).expect("write a fixture");
}

const MAT: &str = "ASTM A36";

/// A bar `len` long with anchors `a` (start) and `b` (end), z up, of steel.
fn bar(len: f64) -> String {
    format!(
        "shared ln = {len}
shared w = 8
shared t = 4
properties(material: \"{MAT}\")
sketch plan(on: xy) {{
  outline = rect(corner: (-w / 2, -w / 2), size: (ln + w, w))
  p0 = pt(0, 0)
  p1 = pt(ln, 0)
}}
bar = extrude(plan, t)
a = anchor(plane(at: plan.p0, normal: plan.plane.normal, x: plan.outline[0].dir))
b = anchor(plane(at: plan.p1, normal: plan.plane.normal, x: plan.outline[0].dir))
"
    )
}

/// A ground plate: pivots `o2` at the origin and `o4` at (d, 0), a track along x.
fn base(d: f64) -> String {
    format!(
        "shared d = {d}
shared w = 20
shared t = 3
properties(material: \"{MAT}\")
sketch plan(on: xy) {{
  outline = rect(corner: (-w, -w - 300), size: (d + w + w, w + w))
  g0 = pt(0, 0)
  g1 = pt(d, 0)
}}
plate = extrude(plan, t)
o2 = anchor(plane(at: plan.g0, normal: plan.plane.normal, x: plan.outline[0].dir))
o4 = anchor(plane(at: plan.g1, normal: plan.plane.normal, x: plan.outline[0].dir))
track = anchor(plane(at: plan.g0, normal: plan.outline[0].dir, x: plan.plane.normal))
"
    )
}

const BLOCK: &str = "shared s = 6
properties(material: \"ASTM A36\")
sketch plan(on: xy) {
  outline = rect(corner: (-s, -s), size: (s + s, s + s))
  o = pt(0, 0)
}
body = extrude(plan, s)
pin = anchor(plane(at: plan.o, normal: plan.plane.normal, x: plan.outline[0].dir))
glide = anchor(plane(at: plan.o, normal: plan.outline[0].dir, x: plan.plane.normal))
";

fn project(name: &str) -> PathBuf {
    let d = dir(name);
    write(&d, "base.fg", &base(100.0));
    write(&d, "bar.fg", &bar(60.0));
    write(&d, "crank.fg", &bar(40.0));
    write(&d, "coupler.fg", &bar(120.0));
    write(&d, "rocker.fg", &bar(80.0));
    write(&d, "block.fg", BLOCK);
    d
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        render: true,
        kin: true,
        overlap: false,
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn export(d: &Path, src: &str, words: &[&str]) -> (Run, Result<Exported, UrdfError>) {
    let run = run_in(d, src);
    let errs: Vec<String> = run
        .errors
        .iter()
        .map(|e| format!("{} {}", e.kind(), e.text()))
        .collect();
    assert!(errs.is_empty(), "{errs:?}");
    // effort and velocity given unless the test is about them (they are refused when missing)
    let mut words: Vec<String> = words.iter().map(|w| w.to_string()).collect();
    if !words
        .iter()
        .any(|w| w == "effort" || w == "velocity" || w == "placeholder-limits" || w == "none")
    {
        words.extend(["effort", "1", "velocity", "1"].map(String::from));
    }
    words.retain(|w| w != "none");
    let o = urdf::Options::parse(&words).expect("options");
    let e = urdf::export(&run, "robot", Some(d), &o);
    (run, e)
}

mod urdf_util;
use urdf_util::*;

/// The largest gap (m, and axis) between where the URDF puts each part and where the solve did.
fn placement_gap(run: &Run, robot: &El, e: &Exported) -> f64 {
    let world = forward(robot, e);
    let asm = run.asm.as_ref().expect("an assembly");
    let mut worst = 0.0f64;
    let mut seen = 0;
    for l in robot.all("link") {
        for v in l.all("visual") {
            let (path, _, pose) = asm
                .occurrences
                .iter()
                .find(|o| v.attr("name") == o.0.replace(['.', '[', ']'], "_"))
                .unwrap_or_else(|| panic!("visual {} is an occurrence", v.attr("name")));
            let _ = path;
            let got = mul(&world[l.attr("name")], &origin_of(v.kid("origin")));
            let want = frame_m4(pose);
            for i in 0..3 {
                for j in 0..4 {
                    worst = worst.max((got[i][j] - want[i][j]).abs());
                }
            }
            seen += 1;
        }
    }
    assert_eq!(seen, asm.occurrences.len(), "every occurrence drawn once");
    worst
}

fn ok(name: &str, run: &Run, e: Result<Exported, UrdfError>) -> (Exported, El) {
    let e = e.unwrap_or_else(|err| panic!("{name}: {err}"));
    let robot = parse(&e.urdf);
    assert_eq!(robot.name, "robot");
    check_urdf(name, &e);
    if run.asm.is_some() {
        let gap = placement_gap(run, &robot, &e);
        assert!(
            gap < 1e-9,
            "{name}: the URDF puts a part {gap} off its solved place"
        );
        assert!(e.residual < 1e-6, "{name}: residual {}", e.residual);
    }
    (e, robot)
}

fn density() -> f64 {
    forge_material::find(MAT).expect("in the library").density
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-30)
}

// ---- parts ------------------------------------------------------------------------------------

#[test]
fn a_box_part_is_one_link_with_its_closed_form_inertia() {
    let d = dir("box");
    let src = format!(
        "shared L = 100
shared W = 50
shared H = 20
properties(material: \"{MAT}\")
sketch plan(on: xy) {{
  o = rect(corner: (0, 0), size: (L, W))
}}
b = extrude(plan, H)
"
    );
    let (run, e) = export(&d, &src, &["urdf"]);
    let (e, robot) = ok("box", &run, e);
    assert_eq!(robot.all("link").count(), 1);
    assert_eq!(robot.all("joint").count(), 0);
    let (a, b, c) = (0.1, 0.05, 0.02);
    let m = density() * a * b * c;
    let l = &e.links[0];
    assert!(close(l.mass.expect("mass"), m, 1e-9), "{:?} vs {m}", l.mass);
    let com = [a / 2.0, b / 2.0, c / 2.0];
    for (got, want) in l.com.iter().zip(com) {
        assert!((got - want).abs() < 1e-12, "com {:?}", l.com);
    }
    let i = l.inertia.expect("inertia");
    let want = [
        m * (b * b + c * c) / 12.0,
        m * (a * a + c * c) / 12.0,
        m * (a * a + b * b) / 12.0,
    ];
    for k in 0..3 {
        assert!(
            close(i[k][k], want[k], 1e-9),
            "I{k}{k} {} vs {}",
            i[k][k],
            want[k]
        );
        for (j, x) in i[k].iter().enumerate() {
            if j != k {
                assert!(x.abs() < 1e-12 * want[0], "I{k}{j} {x}");
            }
        }
    }
    // the bounds are reported, and the values sit within them
    let err = l.err.expect("bounds known for one material");
    assert!(err.iter().all(|x| x.is_finite() && *x >= 0.0));
    assert!((l.mass.expect("mass") - m).abs() <= err[0] + 1e-15);
    // the same numbers in the file
    let inertial = robot
        .kid("link")
        .and_then(|l| l.kid("inertial"))
        .expect("an inertial");
    let mv: f64 = inertial
        .kid("mass")
        .expect("mass")
        .attr("value")
        .parse()
        .expect("a number");
    assert_eq!(mv, l.mass.expect("mass"));
    let ix: f64 = inertial
        .kid("inertia")
        .expect("inertia")
        .attr("ixx")
        .parse()
        .expect("a number");
    assert_eq!(ix, i[0][0]);
    assert_eq!(e.meshes.len(), 1);
    assert!(e.watertight);
}

#[test]
fn a_cylinder_part_has_the_closed_form_moments() {
    let d = dir("cyl");
    let src = format!(
        "shared r = 10
shared h = 40
properties(material: \"{MAT}\")
sketch s(on: xy) {{
  c = circle((0, 0), r)
}}
cyl = extrude(s, h)
"
    );
    let (run, e) = export(&d, &src, &["urdf"]);
    let (e, _) = ok("cyl", &run, e);
    let (r, h) = (0.01, 0.04);
    let m = density() * std::f64::consts::PI * r * r * h;
    let l = &e.links[0];
    assert!(close(l.mass.expect("mass"), m, 1e-9));
    let i = l.inertia.expect("inertia");
    let ixx = m * (3.0 * r * r + h * h) / 12.0;
    assert!(close(i[0][0], ixx, 1e-9) && close(i[1][1], ixx, 1e-9), "{i:?}");
    assert!(close(i[2][2], m * r * r / 2.0, 1e-9), "{i:?}");
    assert!((l.com[2] - h / 2.0).abs() < 1e-12);
}

#[test]
fn a_part_without_a_material_is_refused_not_given_a_density() {
    let d = dir("nomat");
    let src = "shared L = 10
sketch plan(on: xy) {
  o = rect(corner: (0, 0), size: (L, L))
}
b = extrude(plan, L)
";
    let run = run_in(&d, src);
    let e = urdf::export(&run, "robot", Some(&d), &urdf::Options::default());
    assert!(matches!(e, Err(UrdfError::NoMaterial(_))), "{e:?}");
}

// ---- assemblies -------------------------------------------------------------------------------

const ARM: &str = "assembly

param j1.turn = 30deg
param j2.turn = -45deg
param j2.lo = -90deg
param j2.hi = 90deg

base = part(\"base.fg\", ground: true)
upper = part(\"bar.fg\")
fore = part(\"crank.fg\")
j1 = revolute(upper.a, base.o2, turn: turn)
j2 = revolute(fore.a, upper.b, turn: turn, limit: (lo, hi))
";

#[test]
fn a_two_link_arm_is_a_chain_of_revolute_joints_at_its_values() {
    let d = project("arm");
    let (run, e) = export(&d, ARM, &["urdf", "effort", "10", "velocity", "2"]);
    let (e, robot) = ok("arm", &run, e);
    let kinds: Vec<(&str, &str)> = robot
        .all("joint")
        .map(|j| (j.attr("name"), j.attr("type")))
        .collect();
    assert_eq!(kinds, vec![("j1", "continuous"), ("j2", "revolute")]);
    let j2 = robot.all("joint").nth(1).expect("j2");
    let lim = j2.kid("limit").expect("a limit");
    assert_eq!(floats(lim.attr("lower"))[0], -90f64.to_radians());
    assert_eq!(floats(lim.attr("upper"))[0], 90f64.to_radians());
    assert_eq!(lim.attr("effort"), "10");
    assert!(e.warnings.is_empty(), "{:?}", e.warnings);
    // q is forge's value
    let p = |n: &str| e.joints.iter().find(|j| j.name == n).expect("joint").position[0];
    assert!((p("j1") - 30f64.to_radians()).abs() < 1e-12);
    assert!((p("j2") + 45f64.to_radians()).abs() < 1e-12);
    // each link's mass is its part's
    let m = |len: f64| density() * ((len + 8.0) * 8.0 * 4.0) * 1e-9;
    let upper = e.links.iter().find(|l| l.name == "upper").expect("upper");
    assert!(close(upper.mass.expect("mass"), m(60.0), 1e-9));
}

#[test]
fn a_joint_written_from_the_ground_side_is_turned_round() {
    let d = project("arm-rev");
    let src = ARM.replace("j1 = revolute(upper.a, base.o2", "j1 = revolute(base.o2, upper.a");
    let (run, e) = export(&d, &src, &["urdf"]);
    let (e, robot) = ok("arm-rev", &run, e);
    let j1 = robot.all("joint").next().expect("j1");
    assert_eq!(j1.kid("axis").expect("axis").attr("xyz"), "0 0 -1");
    assert!(e.warnings.is_empty(), "{:?}", e.warnings);
}

#[test]
fn a_slider_needs_a_limit_or_a_travel() {
    let d = project("slider");
    let src = "assembly

param s.along = 12
base = part(\"base.fg\", ground: true)
block = part(\"block.fg\")
s = slider(block.glide, base.track, along: along)
";
    let (_, e) = export(&d, src, &["urdf"]);
    match e {
        Err(UrdfError::UnlimitedTranslation { joint, .. }) => assert_eq!(joint, "s"),
        other => panic!("{other:?}"),
    }
    let (run, e) = export(&d, src, &["urdf", "travel", "50"]);
    let (e, robot) = ok("slider-travel", &run, e);
    let j = robot.kid("joint").expect("s");
    assert_eq!(j.attr("type"), "prismatic");
    assert_eq!(j.kid("limit").expect("limit").attr("upper"), "0.05");
    assert!((e.joints[0].position[0] - 0.012).abs() < 1e-15);
    assert!(e.notes.iter().any(|n| n.contains("travel")));
    let limited = src
        .replace("along: along)", "along: along, limit: (lo, hi))")
        .replace(
            "param s.along = 12",
            "param s.along = 12\nparam s.lo = 0\nparam s.hi = 80",
        );
    let (run, e) = export(&d, &limited, &["urdf"]);
    let (_, robot) = ok("slider", &run, e);
    let lim = robot.kid("joint").and_then(|j| j.kid("limit")).expect("limit");
    assert_eq!((lim.attr("lower"), lim.attr("upper")), ("0", "0.08"));
}

#[test]
fn a_cylindrical_joint_is_a_prismatic_and_a_revolute_through_a_massless_link() {
    let d = project("cyl-joint");
    let src = "assembly

param j.along = 7
param j.turn = 25deg
param j.lo = 0
param j.hi = 30
base = part(\"base.fg\", ground: true)
arm = part(\"bar.fg\")
j = cylindrical(arm.a, base.o4, along: along, turn: turn, along_limit: (lo, hi))
";
    let (run, e) = export(&d, src, &["urdf"]);
    let (e, robot) = ok("cylindrical", &run, e);
    let kinds: Vec<&str> = robot.all("joint").map(|j| j.attr("type")).collect();
    assert_eq!(kinds, vec!["prismatic", "continuous"]);
    let dummy = e.links.iter().filter(|l| l.dummy).count();
    assert_eq!(dummy, 1);
    assert!(
        robot
            .all("link")
            .any(|l| l.attr("name") == "j__1" && l.kid("inertial").is_none())
    );
}

#[test]
fn a_ball_joint_is_three_turns_and_puts_the_part_where_it_was() {
    let d = project("ball");
    let src = "assembly

base = part(\"base.fg\", ground: true)
arm = part(\"bar.fg\", at: pose(z: (0.3, -0.2, 0.9), x: (0.9, 0.1, -0.3)))
j = ball(arm.a, base.o2)
";
    let (run, e) = export(&d, src, &["urdf"]);
    let (e, robot) = ok("ball", &run, e);
    let kinds: Vec<&str> = robot.all("joint").map(|j| j.attr("type")).collect();
    assert_eq!(kinds, vec!["continuous"; 3]);
    assert_eq!(e.links.iter().filter(|l| l.dummy).count(), 2);
    // a ball's angles are not all zero here: the placement check above saw a real turn
    assert!(e.joints.iter().any(|j| j.position[0].abs() > 0.1));
}

#[test]
fn a_pin_slot_and_a_limited_planar_are_chains_and_an_open_planar_is_native() {
    let d = project("planar");
    let src = "assembly

param p.x = 10
param p.y = -5
param p.turn = 20deg
base = part(\"base.fg\", ground: true)
block = part(\"block.fg\")
p = planar(block.pin, base.o2, x: x, y: y, turn: turn)
";
    let (run, e) = export(&d, src, &["urdf"]);
    let (_, robot) = ok("planar-native", &run, e);
    assert_eq!(robot.kid("joint").expect("p").attr("type"), "planar");
    let limited = src
        .replace("turn: turn)", "turn: turn, x_limit: (lo, hi), y_limit: (lo, hi))")
        .replace(
            "param p.x = 10",
            "param p.x = 10\nparam p.lo = -50\nparam p.hi = 50",
        );
    let (run, e) = export(&d, &limited, &["urdf"]);
    let (_, robot) = ok("planar-chain", &run, e);
    let kinds: Vec<&str> = robot.all("joint").map(|j| j.attr("type")).collect();
    assert_eq!(kinds, vec!["prismatic", "prismatic", "continuous"]);
    let pin = "assembly

param p.along = 4
param p.turn = -30deg
param p.lo = -20
param p.hi = 20
base = part(\"base.fg\", ground: true)
arm = part(\"bar.fg\")
p = pin_slot(arm.a, base.o4, along: along, turn: turn, along_limit: (lo, hi))
";
    let (run, e) = export(&d, pin, &["urdf"]);
    let (_, robot) = ok("pin-slot", &run, e);
    let kinds: Vec<&str> = robot.all("joint").map(|j| j.attr("type")).collect();
    assert_eq!(kinds, vec!["prismatic", "continuous"]);
}

#[test]
fn a_gear_is_a_mimic_with_forges_ratio_and_offset() {
    let d = project("gear");
    let src = "assembly

param g.turn = 40deg
base = part(\"base.fg\", ground: true)
w1 = part(\"bar.fg\")
w2 = part(\"crank.fg\")
g = revolute(w1.a, base.o2, turn: turn)
h = revolute(w2.a, base.o4)
m = gear(g, h, ratio: 0.5)
";
    let (run, e) = export(&d, src, &["urdf"]);
    let (e, robot) = ok("gear", &run, e);
    let h = robot.all("joint").find(|j| j.attr("name") == "h").expect("h");
    let mm = h.kid("mimic").expect("a mimic");
    assert_eq!(mm.attr("joint"), "g");
    assert_eq!(floats(mm.attr("multiplier"))[0], -0.5);
    let pos = |n: &str| e.joints.iter().find(|j| j.name == n).expect("joint").position[0];
    let off = floats(mm.attr("offset"))[0];
    assert!((pos("h") - (-0.5 * pos("g") + off)).abs() < 1e-12);
}

#[test]
fn a_closed_loop_is_refused_with_its_joints_and_cut_on_request() {
    let d = project("fourbar");
    let t2 = 30f64.to_radians();
    let (a, b, c, dd) = (40.0, 120.0, 80.0, 100.0);
    let (px, py) = (a * dmath::cos(t2), a * dmath::sin(t2));
    let (ex, ey) = (dd - px, -py);
    let l = (ex * ex + ey * ey).sqrt();
    let x = (b * b - c * c + l * l) / (2.0 * l);
    let h = (b * b - x * x).max(0.0).sqrt();
    let (ux, uy) = (ex / l, ey / l);
    let (qx, qy) = (px + ux * x - uy * h, py + uy * x + ux * h);
    let (t3, t4) = (dmath::atan2(qy - py, qx - px), dmath::atan2(qy, qx - dd));
    let src = format!(
        "assembly

param j1.turn = 30deg
base = part(\"base.fg\", ground: true)
crank = part(\"crank.fg\", at: pose(rz: 30deg))
coupler = part(\"coupler.fg\", at: pose(at: ({px}, {py}, 0), rz: {}deg))
rocker = part(\"rocker.fg\", at: pose(at: ({dd}, 0, 0), rz: {}deg))
j1 = revolute(crank.a, base.o2, turn: turn)
j2 = revolute(coupler.a, crank.b)
j3 = revolute(rocker.b, coupler.b)
j4 = revolute(rocker.a, base.o4)
",
        t3.to_degrees(),
        t4.to_degrees()
    );
    let (_, e) = export(&d, &src, &["urdf"]);
    match &e {
        Err(err @ UrdfError::ClosedLoop { joints }) => {
            assert_eq!(joints, &["j1", "j2", "j3", "j4"]);
            assert_eq!(err.fixes(), vec!["cut j1", "cut j2", "cut j3", "cut j4"]);
        }
        other => panic!("{other:?}"),
    }
    let (run, e) = export(&d, &src, &["urdf", "cut", "j3"]);
    let (e, robot) = ok("fourbar-cut", &run, e);
    assert_eq!(e.cut, vec!["j3"]);
    assert!(e.urdf.contains("CUT: joint j3"));
    assert_eq!(robot.all("joint").count(), 3);
    let (_, e) = export(&d, &src, &["urdf", "cut", "j9"]);
    assert!(matches!(e, Err(UrdfError::NoSuchJoint(_))), "{e:?}");
}

#[test]
fn a_body_joined_to_nothing_floats_and_fastened_parts_share_a_link() {
    let d = project("float");
    let src = "assembly

param j.turn = 10deg
base = part(\"base.fg\", ground: true)
arm = part(\"bar.fg\")
tip = part(\"crank.fg\")
loose = part(\"block.fg\", at: pose(at: (0, 0, 50)))
j = revolute(arm.a, base.o2, turn: turn)
f = fastened(tip.a, arm.b)
";
    let (run, e) = export(&d, src, &["urdf"]);
    let (e, robot) = ok("float", &run, e);
    let arm = robot.all("link").find(|l| l.attr("name") == "arm").expect("arm");
    assert_eq!(arm.all("visual").count(), 2, "arm and tip are one link");
    let kinds: Vec<&str> = robot.all("joint").map(|j| j.attr("type")).collect();
    assert_eq!(kinds, vec!["continuous", "floating"]);
    assert!(e.notes.iter().any(|n| n.contains("floating")));
    // the group's mass is both parts'
    let m = |len: f64| density() * ((len + 8.0) * 8.0 * 4.0) * 1e-9;
    let l = e.links.iter().find(|l| l.name == "arm").expect("arm");
    assert!(close(l.mass.expect("mass"), m(60.0) + m(40.0), 1e-9));
}

#[test]
fn the_export_is_the_same_bytes_every_time() {
    let d = project("det");
    let (_, a) = export(&d, ARM, &["urdf"]);
    let (_, b) = export(&d, ARM, &["urdf"]);
    let (a, b) = (a.expect("a"), b.expect("b"));
    assert_eq!(a.urdf, b.urdf);
    assert_eq!(a.zip, b.zip);
    // the ZIP holds the URDF and the meshes, as a reader sees them
    let members = forge_mesh::zip::read(&a.zip, 1 << 30).expect("a zip");
    let names: Vec<&str> = members.iter().map(|m| m.0.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "robot.urdf",
            "meshes/base.stl",
            "meshes/bar.stl",
            "meshes/crank.stl"
        ]
    );
}

#[test]
fn the_examples_export() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for (file, words) in [
        (
            "unified/hinged_box.fg",
            vec!["urdf", "effort", "2", "velocity", "1"],
        ),
        ("robot/arm2.fg", vec!["urdf", "effort", "5", "velocity", "1"]),
    ] {
        let p = root.join(file);
        let src = std::fs::read_to_string(&p).expect("the example");
        let d = p.parent().expect("a directory");
        let (run, e) = export(d, &src, &words);
        let (e, _) = ok(file, &run, e);
        assert!(e.links.iter().all(|l| l.dummy || l.mass.is_some()), "{file}");
    }
}

/// The ZIP's hash, the same on every platform: wasm32-wasip1 runs this test too (tools/test-wasm.sh).
#[test]
fn the_export_is_the_same_bytes_native_and_wasm() {
    let d = project("bits");
    let (_, e) = export(&d, ARM, &["urdf"]);
    let e = e.expect("the arm");
    let h = forge_math::hash::hash128(&e.zip);
    assert_eq!(
        format!("{h:032x}"),
        "fb941a5a7ae14ad964368332141fbc7c",
        "the arm's URDF ZIP"
    );
}

#[test]
fn effort_and_velocity_are_never_zero_unless_asked() {
    let d = project("actuation");
    // none given: refused, the joints named, with their fixes
    let (_, e) = export(&d, ARM, &["urdf", "none"]);
    match &e {
        Err(err @ UrdfError::MissingLimits { joints }) => {
            assert_eq!(joints, &["j1", "j2"]);
            let f = err.fixes();
            assert_eq!(f[0], "effort N velocity V");
            assert!(f.contains(&"effort j1 N velocity j1 V".to_string()));
            assert_eq!(f.last().map(String::as_str), Some("placeholder-limits"));
        }
        other => panic!("{other:?}"),
    }
    // one joint's own, the other still missing: refused for that one only
    let (_, e) = export(&d, ARM, &["urdf", "effort", "j1", "3", "velocity", "j1", "0.5"]);
    assert!(
        matches!(&e, Err(UrdfError::MissingLimits { joints }) if joints == &["j2"]),
        "{e:?}"
    );
    // per joint over the export's
    let (run, e) = export(
        &d,
        ARM,
        &[
            "urdf", "effort", "4", "velocity", "1", "effort", "j1", "3", "velocity", "j1", "0.5",
        ],
    );
    let (_, robot) = ok("actuation-per-joint", &run, e);
    let lim = |n: &str| {
        let j = robot.all("joint").find(|j| j.attr("name") == n).expect("joint");
        let l = j.kid("limit").expect("limit");
        (l.attr("effort").to_string(), l.attr("velocity").to_string())
    };
    assert_eq!(lim("j1"), ("3".to_string(), "0.5".to_string()));
    assert_eq!(lim("j2"), ("4".to_string(), "1".to_string()));
    // the opt-in: 0, each joint marked, and said
    let (run, e) = export(&d, ARM, &["urdf", "placeholder-limits"]);
    let (e, _) = ok("actuation-placeholder", &run, e);
    assert_eq!(
        e.urdf.matches("PLACEHOLDER: effort and velocity unknown").count(),
        2
    );
    assert!(e.urdf.contains("effort=\"0\" velocity=\"0\""));
    assert!(
        e.warnings.iter().any(|w| w.contains("placeholder")),
        "{:?}",
        e.warnings
    );
}

#[test]
fn the_view_moves_a_mimic_with_its_leader() {
    let d = project("view-gear");
    let src = "assembly

param g.turn = 40deg
base = part(\"base.fg\", ground: true)
w1 = part(\"bar.fg\")
w2 = part(\"crank.fg\")
g = revolute(w1.a, base.o2, turn: turn)
h = revolute(w2.a, base.o4)
m = gear(g, h, ratio: 0.5)
";
    let (_, e) = export(&d, src, &["urdf"]);
    let e = e.expect("the gear");
    let v = &e.view;
    let h = v.joints.iter().find(|j| j.name == "h").expect("h");
    let (_, m, off) = h.mimic.clone().expect("a mimic");
    let lead = std::collections::BTreeMap::from([("g".to_string(), 1.0)]);
    let both = std::collections::BTreeMap::from([("g".to_string(), 1.0), ("h".to_string(), m + off)]);
    let (a, b) = (v.place(&lead), v.place(&both));
    let (fa, fb) = (a["w2"], b["w2"]);
    assert!((fa.origin - fb.origin).length() < 1e-12);
    assert!((fa.x().get() - fb.x().get()).length() < 1e-12);
    // and it did move from the export pose
    let rest = v.place(&std::collections::BTreeMap::new());
    assert!((rest["w2"].x().get() - fa.x().get()).length() > 1e-3);
}

//! `forgecad import ROBOT.urdf`: a URDF read into an unlinked robot document and written back.
//! The XML reader on what URDF files hold (comments, entities, CDATA, single quotes); the arm's
//! export imported and exported again with the same tree, kinematics (forward kinematics at the
//! same positions, every link to 1e-9 m), inertials and geometry; a hand-written URDF with what
//! the arm lacks (an inertial turned by rpy, primitives, a prismatic mimicking a revolute,
//! package:// meshes, a gazebo tag) the same; and an unlinked robot's refusals.

mod urdf_util;

use forge_lang::Options;
use forge_urdf::import::{import, parse_xml};
use forge_urdf::robot;
use forge_urdf::{self as urdf, Exported};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use urdf_util::*;

fn tmp(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("import-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("a directory");
    d
}

/// The export's files written under `d`.
fn unpack(e: &Exported, d: &Path) -> PathBuf {
    std::fs::create_dir_all(d).expect("a directory");
    let f = d.join(format!("{}.urdf", e.name));
    std::fs::write(&f, &e.urdf).expect("write");
    for (n, b) in &e.meshes {
        let p = d.join(n);
        std::fs::create_dir_all(p.parent().expect("a dir")).expect("dir");
        std::fs::write(p, b).expect("write");
    }
    f
}

/// A URDF file imported into `doc_dir`, read back and exported (unlinked).
fn round_trip(urdf_file: &Path, doc_dir: &Path, name: &str) -> (String, Exported) {
    let xml = std::fs::read_to_string(urdf_file).expect("the urdf");
    let i = import(&xml, urdf_file.parent().expect("dir"), doc_dir).expect("imports");
    let r = robot::read(&i.text, Some(doc_dir)).unwrap_or_else(|e| panic!("{e:?}\n{}", i.text));
    assert!(r.source.is_none());
    let e = urdf::export_unlinked(name, &urdf::Options::default(), &r)
        .unwrap_or_else(|e| panic!("{e}\n{}", i.text));
    (i.text, e)
}

fn close(a: &M4, b: &M4, tol: f64) -> bool {
    (0..3).all(|i| (0..4).all(|j| (a[i][j] - b[i][j]).abs() <= tol))
}

#[test]
fn the_xml_reader_takes_what_urdf_files_hold() {
    let x = parse_xml(
        "<?xml version='1.0'?>\n<!-- a <comment> -->\n<!DOCTYPE robot>\n<robot name='a&amp;b'>\n<link name=\"l &#x41;\"><![CDATA[x<y]]></link>\n</robot>\n",
    )
    .expect("parses");
    assert_eq!(x.attr("name"), Some("a&b"));
    let l = x.kid("link").expect("link");
    assert_eq!(l.attr("name"), Some("l A"));
    assert_eq!(l.text, "x<y");
    assert!(parse_xml("<robot><link></robot>").is_err());
    assert!(parse_xml("<robot/><robot/>").is_err());
    assert!(parse_xml("<robot a=b/>").is_err());
}

#[test]
fn the_arm_comes_back_the_same() {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/robot");
    let src = std::fs::read_to_string(d.join("arm2.fg")).expect("the arm");
    let opts = Options {
        dir: Some(d.clone()),
        render: true,
        kin: true,
        overlap: false,
        ..Options::default()
    };
    let run = forge_lang::run(&src, opts, Default::default()).0;
    let o =
        urdf::Options::parse(&["effort".into(), "5".into(), "velocity".into(), "1".into()]).expect("options");
    let a = urdf::export(&run, "arm2", Some(&d), &o).expect("exports");
    let dir = tmp("arm");
    let file = unpack(&a, &dir.join("from"));
    let doc_dir = dir.join("doc");
    std::fs::create_dir_all(&doc_dir).expect("dir");
    let (text, b) = round_trip(&file, &doc_dir, "arm2");
    assert!(text.contains("mesh(\"../from/meshes/link.stl\""), "{text}");
    check_urdf("import-arm", &b);
    let (ra, rb) = (parse(&a.urdf), parse(&b.urdf));
    let names = |r: &El, t: &str| r.all(t).map(|x| x.attr("name").to_string()).collect::<Vec<_>>();
    assert_eq!(names(&ra, "link"), names(&rb, "link"));
    assert_eq!(names(&ra, "joint"), names(&rb, "joint"));
    // the same placements at the arm's own positions
    let pos: BTreeMap<String, Vec<f64>> = a
        .joints
        .iter()
        .map(|j| (j.name.clone(), j.position.clone()))
        .collect();
    let (wa, wb) = (forward_at(&ra, &pos), forward_at(&rb, &pos));
    for (k, m) in &wa {
        assert!(close(m, &wb[k], 1e-12), "{k}");
    }
    for (la, lb) in a.links.iter().zip(&b.links) {
        assert_eq!(la.name, lb.name);
        let (ma, mb) = (la.mass.expect("mass"), lb.mass.expect("mass"));
        assert!((ma - mb).abs() <= 1e-15 * ma, "{}: {ma} {mb}", la.name);
        for k in 0..3 {
            assert!((la.com[k] - lb.com[k]).abs() <= 1e-15, "{}", la.name);
            for j in 0..3 {
                let (x, y) = (la.inertia.expect("i")[k][j], lb.inertia.expect("i")[k][j]);
                assert!(
                    (x - y).abs() <= 1e-15 * la.inertia.expect("i")[2][2].abs(),
                    "{}: I{k}{j}",
                    la.name
                );
            }
        }
    }
    // the limits and the actuation as they were
    for (ja, jb) in ra.all("joint").zip(rb.all("joint")) {
        assert_eq!(ja.attr("type"), jb.attr("type"));
        if let (Some(x), Some(y)) = (ja.kid("limit"), jb.kid("limit")) {
            for k in ["lower", "upper", "effort", "velocity"] {
                assert_eq!(x.attrs.get(k), y.attrs.get(k), "{} {k}", ja.attr("name"));
            }
        }
    }
    // each mesh once in the package, though two links draw link.stl
    let stl: Vec<&str> = b.meshes.iter().map(|m| m.0.as_str()).collect();
    assert_eq!(
        stl,
        vec!["meshes/base.stl", "meshes/link.stl", "meshes/finger.stl"]
    );
}

const HAND: &str = r#"<?xml version="1.0"?>
<robot name="hand">
  <material name="blue"><color rgba="0 0 0.8 1"/></material>
  <link name="palm">
    <inertial>
      <origin xyz="0.01 0 0.02" rpy="0.3 -0.2 0.5"/>
      <mass value="0.5"/>
      <inertia ixx="0.002" ixy="0" ixz="0" iyy="0.003" iyz="0" izz="0.004"/>
    </inertial>
    <visual>
      <origin xyz="0 0 0.01" rpy="0 0 0"/>
      <geometry><box size="0.08 0.06 0.02"/></geometry>
      <material name="blue"/>
    </visual>
    <collision><geometry><mesh filename="package://hand_description/meshes/palm.dae" scale="1 1 1"/></geometry></collision>
  </link>
  <link name="finger">
    <inertial><mass value="0.05"/><inertia ixx="1e-5" ixy="0" ixz="0" iyy="1e-5" iyz="0" izz="2e-6"/></inertial>
    <visual><geometry><cylinder radius="0.008" length="0.05"/></geometry></visual>
  </link>
  <link name="slide">
    <visual><origin xyz="0 0 0" rpy="1.5707963267948966 0 0"/><geometry><sphere radius="0.01"/></geometry></visual>
  </link>
  <joint name="knuckle" type="revolute">
    <parent link="palm"/><child link="finger"/>
    <origin xyz="0.04 0 0.02" rpy="0 1.5707963267948966 0"/>
    <axis xyz="0 1 0"/>
    <limit lower="-0.5" upper="1.2" effort="2" velocity="3"/>
    <dynamics damping="0.05"/>
    <safety_controller k_velocity="10"/>
  </joint>
  <joint name="slider" type="prismatic">
    <parent link="palm"/><child link="slide"/>
    <origin xyz="0 0.03 0" rpy="0 0 0"/>
    <axis xyz="1 0 0"/>
    <limit lower="0" upper="0.02" effort="10" velocity="0.1"/>
    <mimic joint="knuckle" multiplier="0.01" offset="0.001"/>
  </joint>
  <gazebo reference="palm"><material>Gazebo/Blue</material></gazebo>
</robot>
"#;

#[test]
fn a_hand_written_urdf_comes_back_with_what_the_document_can_carry() {
    let dir = tmp("hand");
    let file = dir.join("hand.urdf");
    std::fs::write(&file, HAND).expect("write");
    let (text, e) = round_trip(&file, &dir, "hand");
    assert!(
        text.contains("# not imported: joint knuckle: its safety_controller"),
        "{text}"
    );
    assert!(text.contains("# not imported: <gazebo>"), "{text}");
    assert!(
        text.contains("\"package://hand_description/meshes/palm.dae\""),
        "{text}"
    );
    assert!(
        e.warnings.iter().any(|w| w.contains("palm.dae is referenced")),
        "{:?}",
        e.warnings
    );
    check_urdf("import-hand", &e);
    let r = parse(&e.urdf);
    // the referenced mesh as it was written, scale and all
    let m = r
        .kid("link")
        .and_then(|l| l.kid("collision"))
        .and_then(|c| c.kid("geometry"))
        .and_then(|g| g.kid("mesh"));
    let m = m.expect("the mesh");
    assert_eq!(
        (m.attr("filename"), m.attr("scale")),
        ("package://hand_description/meshes/palm.dae", "1 1 1")
    );
    // the inertial turned into the link's axes: R diag(2, 3, 4) Rᵀ (g m²), com where it was
    let palm = &e.links[0];
    let rpy = [0.3f64, -0.2, 0.5];
    let rz = rot_axis([0.0, 0.0, 1.0], rpy[2]);
    let ry = rot_axis([0.0, 1.0, 0.0], rpy[1]);
    let rx = rot_axis([1.0, 0.0, 0.0], rpy[0]);
    let rot = mul(&mul(&rz, &ry), &rx);
    let d = [0.002, 0.003, 0.004];
    for a in 0..3 {
        for b in 0..3 {
            let want: f64 = (0..3).map(|p| rot[a][p] * d[p] * rot[b][p]).sum();
            let got = palm.inertia.expect("inertia")[a][b];
            assert!((got - want).abs() < 1e-15, "I{a}{b} {got} {want}");
        }
    }
    assert!((palm.com[0] - 0.01).abs() < 1e-15 && (palm.com[2] - 0.02).abs() < 1e-15);
    // the slider mimics the knuckle with the same multiplier and offset (m per rad, m)
    let s = r
        .all("joint")
        .find(|j| j.attr("name") == "slider")
        .expect("slider");
    let mm = s.kid("mimic").expect("mimic");
    assert_eq!(floats(mm.attr("multiplier"))[0], 0.01);
    assert!((floats(mm.attr("offset"))[0] - 0.001).abs() < 1e-18);
    let lim = s.kid("limit").expect("limit");
    assert_eq!((lim.attr("lower"), lim.attr("upper")), ("0", "0.02"));
    // primitives in metres again, colours from the named material
    let v = r
        .kid("link")
        .and_then(|l| l.kid("visual"))
        .expect("palm's visual");
    let bx = v
        .kid("geometry")
        .and_then(|g| g.kid("box"))
        .expect("box")
        .attr("size");
    assert_eq!(floats(bx), vec![0.08, 0.06, 0.02]);
    assert_eq!(
        v.kid("material")
            .and_then(|m| m.kid("color"))
            .expect("colour")
            .attr("rgba"),
        "0 0 0.8 1"
    );
    // the same kinematics at a pose
    let pos: BTreeMap<String, Vec<f64>> = [
        ("knuckle".to_string(), vec![0.7]),
        ("slider".to_string(), vec![0.012]),
    ]
    .into();
    let src = parse(HAND);
    let (wa, wb) = (forward_at(&src, &pos), forward_at(&r, &pos));
    for (k, m) in &wa {
        assert!(close(m, &wb[k], 1e-12), "{k}");
    }
}

#[test]
fn an_unlinked_robot_must_be_a_tree() {
    let e = |body: &str| {
        let r = robot::read(&format!("robot\nsrc = source(none)\n{body}"), None).expect("reads");
        urdf::export_unlinked("t", &urdf::Options::default(), &r)
    };
    let two_roots = e("a = link()\nb = link()\n");
    assert!(matches!(two_roots, Err(urdf::UrdfError::Robot(m)) if m.contains("one root link, not 2")));
    let twice = e(
        "a = link()\nb = link()\nc = link()\nj = joint(parent: a, child: c, type: \"fixed\")\nk = joint(parent: b, child: c, type: \"fixed\")\n",
    );
    assert!(matches!(twice, Err(urdf::UrdfError::Robot(m)) if m.contains("child of two joints")));
    let lim = e("a = link()\nb = link()\nj = joint(parent: a, child: b, type: \"revolute\")\n");
    assert!(matches!(lim, Err(urdf::UrdfError::Robot(m)) if m.contains("needs a limit")));
    let ok = e(
        "a = link(mass: 1, inertia: (1, 0, 0, 1, 0, 1))\nb = link()\nj = joint(parent: a, child: b, type: \"continuous\", axis: (0, 0, 2), effort: 1, velocity: 1)\n",
    );
    let ok = ok.expect("a tree");
    assert!(ok.urdf.contains("<axis xyz=\"0 0 1\"/>"));
    let linked = robot::read("robot\nsrc = source(\"x.fg\")\na = link()\n", None);
    assert!(linked.is_err(), "a linked robot's links are its source's");
}

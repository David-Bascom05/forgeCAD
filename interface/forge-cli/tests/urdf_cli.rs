//! `forgecad export F urdf` and `forgecad build ROBOT.fg` as a user runs them: the folder written
//! (the robot and its meshes), the ZIP as it is with `-o X.zip`, a robot document's build listing
//! its overrides, and refusals as a failing exit with the typed error.
//! Native only: they start `forgecad` as a process.
#![cfg(not(target_os = "wasi"))]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)
}

fn scratch(name: &str) -> PathBuf {
    let d = forge_dirs::temp_dir().join(format!("forgecad-urdf-{name}-{}", forge_dirs::process_id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn forgecad(args: &[&str]) -> (bool, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args(args)
        .output()
        .unwrap();
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr),
    )
}

#[test]
fn an_assembly_exports_as_a_folder_or_a_zip_and_refuses_without_actuation() {
    let arm = repo("examples/robot/arm2.fg");
    let arm = arm.to_str().unwrap();
    let d = scratch("arm");
    let out = d.join("arm");
    let (ok, text) = forgecad(&[
        "export",
        arm,
        "urdf",
        "effort",
        "5",
        "velocity",
        "1",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{text}");
    assert!(out.join("arm2.urdf").is_file());
    for m in ["base", "link", "finger"] {
        assert!(out.join(format!("meshes/{m}.stl")).is_file(), "{m}");
    }
    let zip = d.join("arm.zip");
    let (ok, text) = forgecad(&[
        "export",
        arm,
        "urdf",
        "--placeholder-limits",
        "-o",
        zip.to_str().unwrap(),
    ]);
    assert!(ok, "{text}");
    let members = forge_mesh::zip::read(&std::fs::read(&zip).unwrap(), 1 << 30).unwrap();
    let urdf = String::from_utf8(members[0].1.clone()).unwrap();
    assert_eq!(urdf.matches("PLACEHOLDER").count(), 3);
    let (ok, text) = forgecad(&[
        "export",
        arm,
        "urdf",
        "--json",
        "-o",
        d.join("x").to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(text.contains("\"kind\":\"MissingLimits\""), "{text}");
}

#[test]
fn a_robot_document_builds_and_exports() {
    let r = repo("examples/robot/arm2_robot.fg");
    let r = r.to_str().unwrap();
    let (ok, text) = forgecad(&["build", r, "--json"]);
    assert!(ok, "{text}");
    assert!(text.contains("\"field\":\"limit\""), "{text}");
    assert!(text.contains("\"linked\":\"(-135deg, 135deg)\""), "{text}");
    let out = scratch("robot").join("r");
    let (ok, text) = forgecad(&["export", r, "urdf", "-o", out.to_str().unwrap()]);
    assert!(ok, "{text}");
    let urdf = std::fs::read_to_string(out.join("arm2_robot.urdf")).unwrap();
    assert!(urdf.contains("<link name=\"forearm\">"));
    assert!(urdf.contains("<link name=\"tool0\">"));
    assert!(out.join("meshes/upper_hull.stl").is_file());
}

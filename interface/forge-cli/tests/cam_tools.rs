//! A tool made in the builder reaches `forgecad cam`: `forgecad tool new .. --project` writes
//! it at the project's `cam/tools/`, and a machining document beside it uses it (the library the
//! levels make, forge-tooling-builder's `library`). Native only: it starts `forgecad`.
#![cfg(not(target_os = "wasi"))]

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch() -> PathBuf {
    let d = forge_dirs::temp_dir().join(format!("forgecad-cam-tools-{}", forge_dirs::process_id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(d).unwrap()
}

fn forgecad(dir: &Path, args: &[&str]) -> (bool, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .current_dir(dir)
        .args(args)
        // no user level from the machine running the test
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("HOME", dir)
        .output()
        .unwrap();
    (
        o.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
    )
}

#[test]
fn a_project_tool_from_the_builder_is_used_by_cam() {
    let d = scratch();
    let flange = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined/flange.fg");
    std::fs::copy(flange, d.join("flange.fg")).unwrap();
    std::fs::write(
        d.join("d_cam.fg"),
        "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt9 = tool(\"shop-flat-8\", t: 9, stickout: 30, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01)\nface1 = facing(op1, tool: t9, to: 18, step: 1)\nprog = program(op1, post: \"fanuc\", number: 1)\n",
    )
    .unwrap();
    // not built in: refused
    let (_, out) = forgecad(&d, &["cam", "d_cam.fg", "-o", "o.nc"]);
    assert!(out.contains("no tool `shop-flat-8`"), "{out}");
    // made in the builder at the project level, then used
    let (ok, out) = forgecad(
        &d,
        &[
            "tool",
            "new",
            "shop-flat-8",
            "--from",
            "flat-10",
            "dc=8",
            "dcon=8",
            "--project",
        ],
    );
    assert!(ok, "{out}");
    assert!(d.join("cam/tools/shop-flat-8.toml").exists(), "{out}");
    // (the exit status says the program is not fully certified: no `at:`, G5 unchecked)
    let (_, out) = forgecad(&d, &["cam", "d_cam.fg", "-o", "o.nc"]);
    assert!(out.contains("face1 facing built") && out.contains("certified"), "{out}");
    let nc = std::fs::read_to_string(d.join("o.nc")).unwrap();
    assert!(nc.contains("T9 M06"), "{nc}");
    let _ = std::fs::remove_dir_all(&d);
}

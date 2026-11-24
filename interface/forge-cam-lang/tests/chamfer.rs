//! Chamfer / deburr (Fusion's 2D Chamfer): `edge_break` with any chamfer mill or spot drill of 30°
//! to 150° (the chamfer at half the tool's angle from the wall), `width:` (or `size:`) and
//! `tip_offset:`; Δ declared and certified as for the 90° tool.
mod common;
use common::dir;

fn src(tool: &str, args: &str) -> String {
    std::fs::read_to_string(dir().join("plate_break_cam.fg"))
        .unwrap()
        .replace("\"chamfer-10-90\"", &format!("\"{tool}\""))
        .replace("tool: t7, size: 0.3)", &format!("tool: t7{args})"))
}

fn lib() -> forge_tooling::Library {
    let mut lib = forge_tooling::Library::builtin();
    lib.add(
        "project/cam/tools/shop.toml",
        "[[tool]]\nid = \"shop-chamfer-12-60\"\ntype = \"chamfer\"\ndc = 12.0\ntip_d = 1.0\nangle = 60.0\nnof = 4\nlcf = 10.0\ndcon = 12.0\noal = 63.0\nmaterial = \"carbide\"\nsource = \"shop\"\ntaken = \"2026-11-24\"\n",
    )
    .unwrap();
    lib
}

fn brk(cam: &forge_cam_lang::Cam) -> &forge_cam_lang::OpOut {
    let b = cam.ops.iter().find(|o| o.name == "brk").unwrap();
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    assert_eq!(b.status, "built", "{:?} {:?}", b.refusal, b.fix);
    b
}

#[test]
fn chamfer_with_other_tools_widths_and_tip_offsets() {
    // the 90° tool: width as size, a deeper tip
    let cam = forge_cam_lang::run(&src("chamfer-10-90", ", width: 0.3, tip_offset: 0.5"), &dir());
    let b = brk(&cam);
    assert!(
        b.removals[0].starts_with("edge_break 0.3 x 45°"),
        "{}",
        b.removals[0]
    );
    // the tip is 0.5 under the chamfer's bottom (z top − 0.3 − 0.5)
    let zmin = b.paths[0]
        .moves
        .iter()
        .filter_map(|m| match m {
            forge_cam::ops::PMove::Line { to, .. } => Some(to[2]),
            _ => None,
        })
        .fold(f64::INFINITY, f64::min);
    let top = 8.0;
    assert!((zmin - (top - 0.8)).abs() < 1e-9, "{zmin}");
    // a spot drill of 90°
    let cam = forge_cam_lang::run(&src("spot-10-90", ", width: 0.3"), &dir());
    brk(&cam);
    // a 60° chamfer mill: a 30° chamfer, deeper than wide
    let cam = forge_cam_lang::run_with_library(&src("shop-chamfer-12-60", ", width: 0.3"), &dir(), lib());
    let b = brk(&cam);
    assert!(
        b.removals[0].starts_with("edge_break 0.3 x 30°"),
        "{}",
        b.removals[0]
    );
    // both names refused
    let cam = forge_cam_lang::run(&src("chamfer-10-90", ", width: 0.3, size: 0.3"), &dir());
    let b = cam.ops.iter().find(|o| o.name == "brk").unwrap();
    assert!(
        cam.errors
            .iter()
            .chain(b.refusal.iter())
            .any(|e| e.contains("not both")),
        "{:?} {:?}",
        cam.errors,
        b.refusal
    );
}

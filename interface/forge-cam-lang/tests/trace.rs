//! Trace with compensation (Fusion's Trace: left/right offset, axial offset): `offset:` moves the
//! tool off each curve to its left (> 0) or right, `axial_offset:` raises the tip; the moved
//! path is certified against the part as any engrave is.
mod common;
use common::dir;
use forge_cam::ops::PMove;

fn run(args: &str) -> forge_cam_lang::Cam {
    let src = std::fs::read_to_string(dir().join("plate_mark_cam.fg"))
        .unwrap()
        .replace(
            "depth: 0.3, step: 0.15)",
            &format!("depth: 0.3, step: 0.15{args})"),
        );
    forge_cam_lang::run(&src, &dir())
}

fn path(cam: &forge_cam_lang::Cam) -> (Vec<PMove>, &'static str) {
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    let o = cam.ops.iter().find(|o| o.name == "mark").unwrap();
    (
        o.paths.first().map(|p| p.moves.clone()).unwrap_or_default(),
        o.status,
    )
}

#[test]
fn offsets_move_the_path_and_are_certified() {
    let (base, st) = path(&run(""));
    assert_eq!(st, "built");
    let (same, _) = path(&run(", offset: 0, axial_offset: 0"));
    assert_eq!(same, base);
    let zmin = |m: &[PMove]| {
        m.iter()
            .filter_map(|x| match x {
                PMove::Line { to, .. } => Some(to[2]),
                _ => None,
            })
            .fold(f64::INFINITY, f64::min)
    };
    let (up, st) = path(&run(", axial_offset: 0.1"));
    assert_eq!(st, "built");
    assert!((zmin(&up) - (zmin(&base) + 0.1)).abs() < 1e-9);
    // moved sideways the tool leaves the modelled groove: the part is cut, so it is refused
    let cam = run(", offset: 0.5");
    let (moved, st) = path(&cam);
    assert_ne!(moved, base);
    assert_eq!(st, "refused");
    let cam = run(", axial_offset: 1");
    assert!(
        cam.errors
            .iter()
            .chain(cam.ops.iter().filter_map(|o| o.refusal.as_ref()))
            .any(|e| e.contains("not under the top")),
        "{:?}",
        cam.errors
    );
}

//! A foreign program (verify/cam-parts/rptu-10, Autodesk CAM) read and measured: the reader keeps
//! reading it, its ops and numbers stay put (FORGE_BLESS does not apply: change the numbers here
//! by hand, with the reason).
use forge_cam_quality::foreign::{Foreign, measure_text};

fn run(n: u32) -> forge_cam_quality::JobQ {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cam-parts/rptu-10");
    let text = std::fs::read_to_string(root.join(format!("{n}.ngc"))).unwrap();
    let f = Foreign {
        stock: ([0.0, 0.0, -50.0], [80.0, 80.0, 0.0]),
        flutes: None,
        dialect: "linuxcnc".into(),
        machine: "generic-vmc3".into(),
        cell: 0.1,
    };
    measure_text(&text, format!("{n}.ngc"), &f)
}

#[test]
fn rptu_part_3_reads_and_measures() {
    let j = run(3);
    assert!(j.ok && j.errors.is_empty(), "{:?}", j.errors);
    let names: Vec<&str> = j.ops.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["PLANEN2", "2D ADAPTIVE2", "2D-TASCHE1"]);
    let p = &j.progs[0];
    assert!(p.error.is_none(), "{:?}", p.error);
    assert!((p.t_stop - 474.4).abs() < 0.1, "t_stop {}", p.t_stop);
    // the facing pass: one-way, 41 passes, each lifted out
    assert_eq!(j.ops[0].retracts, 41);
    // Autodesk's adaptive: climb only, engagement at most 80 degrees, almost no air
    let a = &j.ops[1];
    assert!(a.phi_max <= 81.0 && a.climb > 0.99, "{} {}", a.phi_max, a.climb);
    assert!(a.t_air / (a.t_cut + a.t_air) < 0.05);
    // the offset pocket after it starts in a slot
    assert!(j.ops[2].phi_max > 170.0);
}

#[test]
fn rptu_part_2_machined_by_forge_cam_from_its_step() {
    // the part imports its STEP model relative to its own file (forge-cam-lang's part())
    let f = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("parts/rptu/rptu2_none_cam.fg");
    let j = forge_cam_quality::measure(&f, &forge_cam_quality::Config::default());
    assert!(j.ok, "{:?}", j.errors);
    assert_eq!(j.ops.len(), 2);
    assert!(
        j.ops.iter().all(|o| o.status == "built"),
        "{:?}",
        j.ops.iter().map(|o| &o.status).collect::<Vec<_>>()
    );
    assert!(j.progs[0].error.is_none());
}

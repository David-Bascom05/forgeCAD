//! Real files the readers were wrong on, read from their public repositories at pinned commits (not
//! committed here).
//!
//! Slicers' 3MF packages: each is ZIP64, as Bambu Studio, OrcaSlicer and Cura write even small packages, and was
//! refused before zip.rs read ZIP64. The volumes are an independent reader's (Python's zipfile and
//! ElementTree, each part's unit and every transform applied).
//!
//! FreeCAD 1.1's PartDesignExample.FCStd: its pad is symmetric by SideType, Midplane left false;
//! read one-sided it was 36% low. FreeCAD's own stored Body shape is 531.7328 mm3 (OCCT 8.0.1).
//!
//!   cargo test --release -p forge-import --test public_files -- --ignored

use std::path::PathBuf;
use std::process::Command;

/// The file at `path` on raw.githubusercontent.com, fetched once, checked against its sha256.
fn fetch(path: &str, sha: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join("forge-public-files");
    std::fs::create_dir_all(&dir).unwrap();
    let f: PathBuf = dir.join(path.rsplit('/').next().unwrap());
    if !f.exists() {
        let ok = Command::new("curl")
            .args(["-sSfL", "-o"])
            .arg(&f)
            .arg(format!("https://raw.githubusercontent.com/{path}"))
            .status()
            .unwrap()
            .success();
        assert!(ok, "fetch {path}");
    }
    let got = Command::new("sha256sum").arg(&f).output().unwrap();
    assert!(
        String::from_utf8_lossy(&got.stdout).starts_with(sha),
        "{path}: not the pinned file"
    );
    std::fs::read(&f).unwrap()
}

const FILES: &[(&str, &str, usize, f64)] = &[
    (
        "bambulab/BambuStudio/8e524a39d4c4cd0541801c1c46d2f77debb0bf1d/resources/calib/filament_flow/flowrate-test-pass1.3mf",
        "f71b839130000caab52f99c258a575dabb0f46b1ddc74591c9e438720740e03c",
        6920,
        10772.473126,
    ),
    (
        "bambulab/BambuStudio/8e524a39d4c4cd0541801c1c46d2f77debb0bf1d/resources/calib/filament_flow/flowrate-test-pass2.3mf",
        "b67d9c294b3579873ba99855c2f3faf6fa83c472cebeceb1bd387de46aa5940f",
        5596,
        11978.411974,
    ),
    (
        "SoftFever/OrcaSlicer/fcbfbaaf9196b6d88c9c5b65a2bf33efc8752385/resources/calib/filament_flow/Orca-LinearFlow.3mf",
        "c998359cef8c247c5c9c50ea0d07e778590be7cdaa1d21beaf84abccea7840ca",
        15160,
        14002.023494,
    ),
    (
        "Ultimaker/Cura/1a221ec29a86960d16a412940becc1f63534c41f/resources/meshes/makergear_m2_platform.3mf",
        "96bf1cde33c690e2c6ccb55d0e603e6b97f11fca836536a795e9a8b17193730a",
        15438,
        256013.046491,
    ),
];

#[test]
#[ignore = "fetches four public slicer packages: run with --ignored"]
fn slicers_zip64_packages_read() {
    for (path, sha, triangles, volume) in FILES {
        let data = fetch(path, sha);
        assert!(data.windows(4).any(|w| w == [0x50, 0x4b, 6, 6]), "{path}: ZIP64");
        let m = forge_import::threemf::read(&data).unwrap_or_else(|e| panic!("{path}: {e}"));
        let s = forge_mesh::read::summary(&m);
        assert_eq!(m.triangles.len(), *triangles, "{path}");
        assert!(s.closed, "{path}");
        let v = s.volume.unwrap();
        assert!(
            (v - volume).abs() <= 1e-6 * volume,
            "{path}: {v} against {volume}"
        );
    }
}

const SHA_PDE: &str = "0aa09dac21c19ba07f8abe32ebd7957b8539a430f6832c66ce2e15609d762ac7";

#[test]
#[ignore = "fetches FreeCAD's PartDesignExample.FCStd: run with --ignored"]
fn freecad_1_1_part_design_example_matches_its_stored_shape() {
    let data = fetch(
        "FreeCAD/FreeCAD/908196946fc947024acdc99e7f3e837a9a91f9a3/data/examples/PartDesignExample.FCStd",
        SHA_PDE,
    );
    let bodies = forge_import::fcstd::translate(&data).unwrap();
    assert_eq!(bodies.len(), 1);
    let src = bodies[0]
        .fg
        .clone()
        .unwrap_or_else(|| panic!("{:?}", bodies[0].fail));
    let (r, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    assert!(r.errors.is_empty(), "{src}");
    let v = r.measure.expect("measured").volume;
    assert!(
        (v - 531.7328).abs() <= 1e-4 * 531.7328,
        "{v} against FreeCAD's 531.7328"
    );
}

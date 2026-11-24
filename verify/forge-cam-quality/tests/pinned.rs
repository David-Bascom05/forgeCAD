//! The quality numbers of a CI-cheap subset, pinned: a change that makes the paths slower, rubs or
//! overloads the tool, plunges, or cuts more air shows here as speed regressions do in bits.txt.
//! After a judged improvement, re-pin with FORGE_BLESS=1 (and put the before/after in the commit).
use forge_cam_quality::{Config, JobQ, measure};
use std::fmt::Write as _;
use std::path::PathBuf;

/// One line per op: the numbers that judge a path, rounded so that only a real change moves them.
fn summary(jobs: &[JobQ]) -> String {
    let mut o = String::new();
    for j in jobs {
        let stem = PathBuf::from(&j.file)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        for q in &j.ops {
            let _ = writeln!(
                o,
                "{stem}/{} {} t_stop={:.1} t_min={:.1} cut={:.1} air={:.1} rapid={:.0} retracts={} plunges={} ramps={} c90={} \
phi_max={:.0} phi_mean={:.0} climb={:.2} slot={:.2} hex_max={:.4} hex_p99={:.4} hex_mean={:.4} thin={:.2} thick={:.2} removed={:.0} fc={:.0}",
                q.name,
                q.kind,
                q.t_stop,
                q.t_min,
                q.t_cut,
                q.t_air,
                q.len_rapid,
                q.retracts,
                q.plunges,
                q.ramps,
                q.corners90,
                q.phi_max,
                q.phi_mean,
                q.climb,
                q.slotting,
                q.hex_max,
                q.hex_p99,
                q.hex_mean,
                q.thin,
                q.thick,
                q.removed,
                q.fc_max
            );
        }
        for p in &j.progs {
            let _ = writeln!(
                o,
                "{stem} program {} t_stop={:.1} t_min={:.1} changes={} error={:?}",
                p.name, p.t_stop, p.t_min, p.tool_changes, p.error
            );
        }
        for e in &j.errors {
            let _ = writeln!(o, "{stem} error {e}");
        }
    }
    o
}

#[test]
fn pinned_subset() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let files = [
        root.join("../../examples/machined/well_island_cam.fg"),
        root.join("parts/deep_slot_cam.fg"),
    ];
    let cfg = Config {
        cell: Some(0.1),
        machine: None,
    };
    let jobs: Vec<JobQ> = files.iter().map(|f| measure(f, &cfg)).collect();
    let got = summary(&jobs);
    let pin = root.join("tests/pinned.txt");
    if std::env::var_os("FORGE_BLESS").is_some() {
        std::fs::write(&pin, &got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&pin).unwrap_or_default();
    if got != want {
        for (g, w) in got.lines().zip(want.lines()) {
            if g != w {
                eprintln!("want {w}\n got {g}");
            }
        }
        panic!(
            "toolpath quality moved (re-pin with FORGE_BLESS=1 only for a judged change, numbers in the commit):\n{got}"
        );
    }
}

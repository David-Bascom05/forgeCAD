//! The built-in models (data/models.toml and the compiled faces beside them):
//! - every spec reads, every group is in its model's compiled faces, and each group's box is the
//!   hull of its faces' boxes (so a stale compiled file is caught);
//! - the record that names a model agrees with it: its jaw height and height are the model's
//!   own, its jaw plate fits the model's fixed jaw, and its opening covers the model's;
//! - with the model files downloaded (Git LFS; CI and ordinary clones have the pointers), the
//!   compiled text is re-derived from the STEP and compared byte for byte, and the STEP's bytes
//!   are checked against the sha256 the record cites. Without them the pointer's own oid is
//!   checked against that sha256, so a model swapped under a citation is caught either way.
use forge_workholding::{Library, model};
use std::path::{Path, PathBuf};

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn is_pointer(b: &[u8]) -> bool {
    b.starts_with(b"version https://git-lfs.github.com/spec/")
}

/// The sha256 an LFS pointer names.
fn pointer_oid(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("oid sha256:"))
        .map(str::to_string)
}

#[test]
fn every_model_reads_and_its_groups_match_its_compiled_faces() {
    let ms = model::builtin();
    assert!(!ms.is_empty());
    for m in ms {
        assert!(m.opening > 0.0 && m.chord > 0.0, "{}", m.name);
        assert!(!m.how.is_empty(), "{}: say where the model came from", m.name);
        assert_eq!(m.sha256.len(), 64, "{}: the STEP's sha256", m.name);
        let groups = forge_cam::fixture_model::read(m.check).unwrap_or_else(|e| panic!("{}: {e}", m.name));
        assert_eq!(
            groups.len(),
            m.groups.len(),
            "{}: {} groups in models.toml, {} compiled",
            m.name,
            m.groups.len(),
            groups.len()
        );
        for g in &m.groups {
            let fs = &groups
                .iter()
                .find(|x| x.0 == g.name)
                .unwrap_or_else(|| panic!("{}: `{}` is not compiled", m.name, g.name))
                .1;
            assert!(!fs.is_empty(), "{}/{}: no faces", m.name, g.name);
            assert!(!g.solids.is_empty(), "{}/{}: no solids", m.name, g.name);
            let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
            for f in fs {
                for k in 0..3 {
                    lo[k] = lo[k].min(f.bbox[k].lo);
                    hi[k] = hi[k].max(f.bbox[k].hi);
                }
            }
            assert_eq!(
                (lo, hi),
                (g.min, g.max),
                "{}/{}: the box is not its faces'",
                m.name,
                g.name
            );
            // the checker's enclosure holds the body itself, which the group's own box holds
            // to within the chord it was grown by
            for k in 0..3 {
                assert!(
                    g.min[k] <= g.tight_min[k] + m.chord && g.max[k] >= g.tight_max[k] - m.chord,
                    "{}/{}: its faces do not enclose the body on axis {k}",
                    m.name,
                    g.name
                );
            }
        }
        // the jaws: the fixed one's face at x = 0, the moving one's at the opening
        if let (Some(f), Some(v)) = (
            m.groups
                .iter()
                .find(|g| g.role == forge_workholding::Role::FixedJaw),
            m.groups
                .iter()
                .find(|g| g.role == forge_workholding::Role::MovingJaw),
        ) {
            assert!(
                f.tight_max[0].abs() <= m.chord + 1e-9,
                "{}: the fixed jaw's face at x = 0 ({})",
                m.name,
                f.tight_max[0]
            );
            assert!(
                (v.tight_min[0] - m.opening).abs() <= m.chord + 1e-9,
                "{}: the moving jaw's face at the opening ({} vs {})",
                m.name,
                v.tight_min[0],
                m.opening
            );
            // the bed at z = 0: the jaws stand on it
            for g in [f, v] {
                assert!(
                    g.tight_min[2] <= 0.0 && g.tight_max[2] > 0.0,
                    "{}/{}: across the bed",
                    m.name,
                    g.name
                );
            }
        }
    }
}

#[test]
fn a_record_that_names_a_model_agrees_with_it() {
    let l = Library::builtin();
    let mut seen = 0;
    for (r, _) in &l.records {
        let Some(name) = r.words.get("model") else {
            continue;
        };
        seen += 1;
        let m = model::get(name).unwrap_or_else(|| panic!("{}: no model `{name}`", r.name));
        let fixed = m
            .groups
            .iter()
            .find(|g| g.role == forge_workholding::Role::FixedJaw)
            .unwrap_or_else(|| panic!("{}: the model has no fixed jaw", m.name));
        // the record's published sizes against the model's own box, which is its mesh grown by
        // the chord: the chord each way, and the published value's own rounding
        let close = |a: f64, b: f64| (a - b).abs() <= 2.0 * m.chord + 0.01;
        assert!(
            close(r.get("jaw_height").unwrap(), fixed.tight_max[2]),
            "{}: jaw_height {} vs the model's {}",
            r.name,
            r.get("jaw_height").unwrap(),
            fixed.max[2]
        );
        // the jaw plate is part of the fixed jaw group (with the jaw body behind it), so the
        // record's plate thickness is at most the group's own depth
        let t = r.get("jaw_thickness").unwrap();
        assert!(
            t > 0.0 && t <= fixed.tight_max[0] - fixed.tight_min[0] + 2.0 * m.chord,
            "{}: jaw_thickness {t} is not within the model's fixed jaw ({})",
            r.name,
            fixed.tight_max[0] - fixed.tight_min[0]
        );
        let top = m
            .groups
            .iter()
            .map(|g| g.tight_max[2])
            .fold(f64::NEG_INFINITY, f64::max);
        let bottom = m
            .groups
            .iter()
            .map(|g| g.tight_min[2])
            .fold(f64::INFINITY, f64::min);
        assert!(
            close(r.get("height").unwrap(), top - bottom),
            "{}: height {} vs the model's {}",
            r.name,
            r.get("height").unwrap(),
            top - bottom
        );
        assert!(
            r.get("max_opening").unwrap() >= m.opening - 0.01,
            "{}: max_opening {} is less than the model's own {}",
            r.name,
            r.get("max_opening").unwrap(),
            m.opening
        );
    }
    assert!(seen >= 1, "no record names a model");
}

#[test]
fn the_compiled_faces_are_the_step_the_record_cites() {
    for m in model::builtin() {
        let path = data().join("models").join(&m.step);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if is_pointer(&bytes[..bytes.len().min(200)]) {
            let oid = pointer_oid(&String::from_utf8_lossy(&bytes)).expect("an LFS pointer's oid");
            assert_eq!(
                oid, m.sha256,
                "{}: the pointer's object is not the one cited",
                m.name
            );
            continue;
        }
        let mut h = forge_git::hash::Sha256::new();
        h.update(&bytes);
        let hex: String = h.finish().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex, m.sha256,
            "{}: the STEP's bytes are not the ones cited",
            m.name
        );
        let spec = forge_workholding_builder::compile::Spec::of(m);
        let c = forge_workholding_builder::compile::compile(&bytes, &spec)
            .unwrap_or_else(|e| panic!("{}: {e}", m.name));
        assert_eq!(
            c.text, m.check,
            "{}: the compiled faces are stale: re-run `cargo run --release -p forge-workholding-builder --example wh_compile`",
            m.name
        );
    }
}

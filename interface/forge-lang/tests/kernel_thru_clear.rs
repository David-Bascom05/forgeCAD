//! L6 for through holes (research/ops-spec/design.md §2, H1): on every repo part, every through
//! hole, at every place it is drilled, leaves no material on its axis ahead of its mouth. The
//! spec's thru is the half-infinite wall (`Ops/Hole.lean`), so a point of the drill's axis past the
//! mouth inside the part after the hole is a hole cut short: the box it was sized by did not hold
//! the part. Each axis is sampled across the part's whole extent along it and classified exactly
//! against the part as built (the language's own stages).
//!
//! Parts of one solid only (a hole picking bodies may leave the others standing in its way).
//! What was checked is held to `thru_clear.digest` (FG_REGEN=1 rewrites it): a part, hole or
//! place no longer checked shows in the diff.
mod common;

use forge_bool::{Index, Where};
use forge_math::{Point3, Vec3};
use forge_model::{At, End, Feature};

const DIGEST: &str = "tests/thru_clear.digest";

fn parts() -> Vec<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = Vec::new();
    let mut stack = vec![root.join("examples"), root.join("interface/forge-lang/parts")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "fg") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Each place a hole is drilled at: its mouth.
fn mouths(h: &forge_model::Hole) -> Vec<Point3> {
    match &h.places {
        Some(At::Sketch { at, .. }) if !at.is_empty() => {
            at.iter().map(|&p| h.at.origin + (p - at[0])).collect()
        }
        _ => vec![h.at.origin],
    }
}

#[test]
fn every_thru_hole_clears_its_axis() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut lines = Vec::new();
    let mut bad = Vec::new();
    for path in parts() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !src.contains("thru") {
            continue;
        }
        let (run, cache) = common::run(&src);
        if !run.errors.is_empty() {
            continue;
        }
        let name = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
        let (features, _) = cache.history();
        for (i, stage) in cache.stages().enumerate() {
            let Some(Feature::Hole(h)) = features.get(i) else {
                continue;
            };
            if !matches!(h.end, End::Through) || stage.len() != 1 {
                continue;
            }
            let body = &stage[0].body;
            let index = Index::new(body);
            let bx = body.faces_box();
            let dir: Vec3 = h.at.z().get();
            for (k, m) in mouths(h).into_iter().enumerate() {
                // the part's extent along the axis, from the mouth
                let corners = [
                    (bx.x.lo, bx.y.lo, bx.z.lo),
                    (bx.x.hi, bx.y.lo, bx.z.lo),
                    (bx.x.lo, bx.y.hi, bx.z.lo),
                    (bx.x.hi, bx.y.hi, bx.z.lo),
                    (bx.x.lo, bx.y.lo, bx.z.hi),
                    (bx.x.hi, bx.y.lo, bx.z.hi),
                    (bx.x.lo, bx.y.hi, bx.z.hi),
                    (bx.x.hi, bx.y.hi, bx.z.hi),
                ];
                let far = corners
                    .iter()
                    .map(|&(x, y, z)| dir.dot(Point3::new(x, y, z) - m))
                    .fold(0.0f64, f64::max);
                let n = 256;
                let left = (1..=n).find_map(|j| {
                    let t = far * j as f64 / n as f64;
                    (index.classify(m + dir * t) == Where::Inside).then_some(t)
                });
                lines.push(format!("{name} feature {i} place {k}"));
                if let Some(t) = left {
                    bad.push(format!(
                        "{name}: hole (feature {i}) place {k} leaves material {t:.3} past its mouth"
                    ));
                }
            }
        }
    }
    assert!(bad.is_empty(), "through holes cut short:\n{}", bad.join("\n"));
    let got = lines.join("\n") + "\n";
    if std::env::var_os("FG_REGEN").is_some() {
        std::fs::write(DIGEST, &got).expect("write the digest");
        return;
    }
    let want = std::fs::read_to_string(DIGEST).unwrap_or_default();
    assert!(
        want == got,
        "the holes checked changed (FG_REGEN=1 rewrites {DIGEST}):\nwant\n{want}\ngot\n{got}"
    );
}

//! L2 for holes (research/ops-spec/design.md §2): every repo part's every hole's profile, as the
//! kernel revolves it (`Hole::revolve`), is the spec's exact profile (`Ops/Hole.lean`'s `profile`,
//! truncated at the kernel's through depth by H1), frustum for frustum, decided over ℚ by the
//! reference `forge-hole-ref` (verify/forge-exact, `lake build forge-hole-ref`), and well formed
//! as the spec's size checks state (H3).
//!
//! The reference runs if built: FORGE_HOLE_REF names it (CI sets it, and then it must be there),
//! else verify/forge-exact/.lake/build/bin/forge-hole-ref if present, else the test says so and
//! passes. What was checked is held to `hole_ref.digest` (FG_REGEN=1 rewrites it).
mod common;

use forge_model::{End, Feature, Head};
use forge_ops::Segment;
use std::io::Write;

const DIGEST: &str = "tests/hole_ref.digest";

/// An f64 as `[m, e]`, `m · 2^e`, exactly.
fn dy(x: f64) -> String {
    if x == 0.0 {
        return "[0,0]".into();
    }
    let b = x.to_bits();
    let neg = b >> 63 == 1;
    let exp = ((b >> 52) & 0x7ff) as i64;
    let frac = (b & ((1u64 << 52) - 1)) as i64;
    let (m, e) = if exp == 0 {
        (frac, -1074)
    } else {
        (frac | (1i64 << 52), exp - 1075)
    };
    format!("[{},{e}]", if neg { -m } else { m })
}

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

/// The hole's line for the reference, from the kernel's revolve of it.
fn line(h: &forge_model::Hole) -> Option<String> {
    // a hole up to a face or the next face is resolved to a depth before it is revolved
    if !matches!(h.end, End::Depth(_) | End::Through) {
        return None;
    }
    let reach = forge_math::Box3::from_point(h.at.origin).grow(100.0);
    let Ok(Feature::Revolve { profile, .. }) = h.revolve(&reach) else {
        return None;
    };
    // the kernel's frusta: the rim's segments neither square to the axis nor on it
    let mut fr = Vec::new();
    let mut pts = Vec::new();
    for (s, _) in &profile.loops[0].segments {
        let Segment::Line { a, b } = s else { return None };
        pts.push(*b);
        if a.y != b.y && !(a.x == 0.0 && b.x == 0.0) {
            fr.push(format!("[{},{},{},{}]", dy(a.x), dy(b.x), dy(a.y), dy(b.y)));
        }
    }
    let r = h.diameter / 2.0;
    // the rim's points: mouth, the head's, the wall's foot, the bottom, the axis
    let wall = pts.iter().rev().find(|p| p.x == r && p.y > 0.0)?;
    let bottom = pts.iter().find(|p| p.x == 0.0 && p.y > 0.0)?;
    let head = match h.head {
        Head::Plain => r#"{"kind":"plain"}"#.to_string(),
        Head::Counterbore { diameter, depth } => {
            format!(
                r#"{{"kind":"cbore","Rc":{},"dc":{}}}"#,
                dy(diameter / 2.0),
                dy(depth)
            )
        }
        Head::Countersink { diameter, .. } => {
            // its depth as the kernel worked it out from the angle (the one rounding)
            let sink = pts.iter().find(|p| p.x == r && p.y > 0.0)?;
            format!(
                r#"{{"kind":"csink","Rc":{},"dk":{}}}"#,
                dy(diameter / 2.0),
                dy(sink.y)
            )
        }
    };
    let end = match h.end {
        End::Depth(d) => {
            let p = if bottom.y > d {
                dy(bottom.y - d)
            } else {
                "null".into()
            };
            format!(r#"{{"kind":"blind","D":{},"p":{p}}}"#, dy(d))
        }
        End::Through => format!(r#"{{"kind":"thru","L":{}}}"#, dy(wall.y)),
        _ => return None,
    };
    Some(format!(
        r#"{{"r":{},"head":{head},"end":{end},"kernel":[{}]}}"#,
        dy(r),
        fr.join(",")
    ))
}

#[test]
fn every_hole_is_the_specs_profile() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let exe = match std::env::var_os("FORGE_HOLE_REF") {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let p = root.join("verify/forge-exact/.lake/build/bin/forge-hole-ref");
            if !p.exists() {
                eprintln!("forge-hole-ref not built (lake build forge-hole-ref): not checked");
                return;
            }
            p
        }
    };
    let mut names = Vec::new();
    let mut input = String::new();
    for path in parts() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !src.contains("hole(") {
            continue;
        }
        let (run, cache) = common::run(&src);
        if !run.errors.is_empty() {
            continue;
        }
        let name = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
        let (features, _) = cache.history();
        for (i, f) in features.iter().enumerate() {
            let Feature::Hole(h) = f else { continue };
            if let Some(l) = line(h) {
                names.push(format!("{name} feature {i}"));
                input.push_str(&l);
                input.push('\n');
            }
        }
    }
    let mut child = std::process::Command::new(&exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("forge-hole-ref runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("writes");
    let out = child.wait_with_output().expect("answers");
    let answers: Vec<String> = String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(answers.len(), names.len(), "one answer per hole");
    let mut bad = Vec::new();
    let mut lines = Vec::new();
    for (n, a) in names.iter().zip(&answers) {
        let ok = a.contains(r#""same":true"#) && a.contains(r#""wf":true"#);
        if !ok {
            bad.push(format!("{n}: {a}"));
        }
        lines.push(format!("{n} {}", if ok { "same" } else { "DIFFERS" }));
    }
    assert!(
        bad.is_empty(),
        "holes not the spec's profile:\n{}",
        bad.join("\n")
    );
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

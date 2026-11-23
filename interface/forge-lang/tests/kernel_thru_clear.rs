//! L6 for through tools (research/ops-spec/design.md §2, H1): on every repo part, every through
//! hole at every place it is drilled, and every through extrude cut at points across its profile,
//! leaves no material of the bodies it cuts on its path ahead of its sketch. The spec's thru is
//! the half-infinite wall (`Ops/Hole.lean`; a through cut is the half-infinite prism), so a point
//! of the path inside a cut body after the cut is a tool cut short: the box it was sized by did not
//! hold the part. Each path is sampled across the cut bodies' whole extent along it and classified
//! exactly against the part as built (the language's own stages).
//!
//! The bodies a feature cuts are the ones it picks (`bodies:`), else the last made, as the
//! regen's `targets` takes them. What was checked is held to `thru_clear.digest` (FG_REGEN=1
//! rewrites it): a part, tool or place no longer checked shows in the diff.
mod common;

use forge_bool::{Index, Where};
use forge_math::{Point3, Vec3};
use forge_model::{At, BodyFeature, Combine, End, Extent, Feature, Pick, Solid, Toward};

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

/// The bodies feature `f` cuts, of the stage after it: its pick, else the last made.
fn cut<'a>(stage: &'a [Solid], pick: Option<&Pick>, made: &[usize]) -> Vec<&'a Solid> {
    match pick {
        Some(Pick::All) => stage.iter().collect(),
        Some(Pick::Some(ids)) => stage.iter().filter(|s| ids.contains(&s.id)).collect(),
        // the last body made before the feature, if it still stands
        None => made
            .last()
            .and_then(|&id| stage.iter().find(|s| s.id == id))
            .into_iter()
            .collect(),
    }
}

/// The first point of `path(h)`, h in (0, far], inside any of `bodies`.
fn left(bodies: &[(&Solid, Index)], far: f64, path: impl Fn(f64) -> Point3) -> Option<f64> {
    let n = 256;
    (1..=n).find_map(|j| {
        let h = far * j as f64 / n as f64;
        let p = path(h);
        bodies
            .iter()
            .any(|(_, ix)| ix.classify(p) == Where::Inside)
            .then_some(h)
    })
}

/// How far the bodies reach along `dir` from `from`.
fn reach_along(bodies: &[(&Solid, Index)], from: Point3, dir: Vec3) -> f64 {
    let mut far = 0.0f64;
    for (s, _) in bodies {
        let bx = s.body.faces_box();
        for x in [bx.x.lo, bx.x.hi] {
            for y in [bx.y.lo, bx.y.hi] {
                for z in [bx.z.lo, bx.z.hi] {
                    far = far.max(dir.dot(Point3::new(x, y, z) - from));
                }
            }
        }
    }
    far
}

/// Points well inside a profile's region: a 9 by 9 grid over its box, each kept if it is inside
/// the profile's own prism (a millimetre tall), classified exactly.
fn inside_points(profile: &forge_ops::Profile) -> Vec<forge_math::Point2> {
    use forge_ops::Segment;
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut add = |x: f64, y: f64| {
        lo = (lo.0.min(x), lo.1.min(y));
        hi = (hi.0.max(x), hi.1.max(y));
    };
    for l in &profile.loops {
        for (s, _) in &l.segments {
            match s {
                Segment::Line { a, b } => {
                    add(a.x, a.y);
                    add(b.x, b.y);
                }
                Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. } => {
                    add(center.x - radius, center.y - radius);
                    add(center.x + radius, center.y + radius);
                }
                Segment::Spline(c) => {
                    let b = c.c.bbox(c.c.domain());
                    add(b.x.lo, b.y.lo);
                    add(b.x.hi, b.y.hi);
                }
            }
        }
    }
    let Ok(prism) = forge_ops::extrude(profile, 1.0, 1) else {
        return vec![];
    };
    let ix = Index::new(&prism);
    let mut out = Vec::new();
    for i in 1..10 {
        for j in 1..10 {
            let q = forge_math::Point2::new(
                lo.0 + (hi.0 - lo.0) * i as f64 / 10.0,
                lo.1 + (hi.1 - lo.1) * j as f64 / 10.0,
            );
            let p = profile.plane.plane_point(q) + profile.plane.z().get() * 0.5;
            if ix.classify(p) == Where::Inside {
                out.push(q);
            }
        }
    }
    out
}

#[test]
fn every_thru_tool_clears_its_path() {
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
        // the bodies made so far, in order (each solid's id is the feature that made it)
        let mut made: Vec<usize> = Vec::new();
        for (i, stage) in cache.stages().enumerate() {
            let Some(f) = features.get(i) else { continue };
            let (pick, inner) = match f {
                Feature::Body(b) => match &**b {
                    BodyFeature::In { bodies, feature } => (Some(bodies), feature),
                    _ => (None, f),
                },
                _ => (None, f),
            };
            let bodies: Vec<(&Solid, Index)> = cut(stage, pick, &made)
                .into_iter()
                .map(|s| (s, Index::new(&s.body)))
                .collect();
            for s in stage {
                if !made.contains(&s.id) {
                    made.push(s.id);
                }
            }
            match inner {
                Feature::Hole(h) if matches!(h.end, End::Through) => {
                    let dir: Vec3 = h.at.z().get();
                    for (k, m) in mouths(h).into_iter().enumerate() {
                        let far = reach_along(&bodies, m, dir);
                        lines.push(format!("{name} feature {i} hole place {k}"));
                        if let Some(t) = left(&bodies, far, |t| m + dir * t) {
                            bad.push(format!(
                                "{name}: hole (feature {i}) place {k} leaves material {t:.3} past its mouth"
                            ));
                        }
                    }
                }
                Feature::Extrude {
                    profile,
                    extent: extent @ (Extent::Through(_) | Extent::ThroughAlong { .. }),
                    op: Combine::Cut,
                } => {
                    let z: Vec3 = profile.plane.z().get();
                    // the path a point of the profile runs: along the normal, or slanted
                    let (toward, run) = match extent {
                        Extent::Through(t) => (*t, z),
                        Extent::ThroughAlong { toward, dir } => {
                            let Some(u) = dir.normalize() else { continue };
                            let rise = u.dot(z);
                            if rise.abs() < 1e-9 {
                                continue;
                            }
                            // a height h along the normal is u h / rise along the slant
                            (*toward, u.get() * (1.0 / rise))
                        }
                        _ => unreachable!(),
                    };
                    let sides: &[f64] = match toward {
                        Toward::Forward => &[1.0],
                        Toward::Back => &[-1.0],
                        Toward::Both => &[1.0, -1.0],
                    };
                    for (k, q) in inside_points(profile).into_iter().enumerate() {
                        let p0 = profile.plane.plane_point(q);
                        for &sg in sides {
                            let far = reach_along(&bodies, p0, z * sg);
                            lines.push(format!("{name} feature {i} cut point {k} side {sg}"));
                            if let Some(h) = left(&bodies, far, |h| p0 + run * (h * sg)) {
                                bad.push(format!("{name}: through cut (feature {i}) point {k} side {sg} leaves material {h:.3} past its sketch"));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    assert!(bad.is_empty(), "through tools cut short:\n{}", bad.join("\n"));
    let got = lines.join("\n") + "\n";
    if std::env::var_os("FG_REGEN").is_some() {
        std::fs::write(DIGEST, &got).expect("write the digest");
        return;
    }
    let want = std::fs::read_to_string(DIGEST).unwrap_or_default();
    assert!(
        want == got,
        "the tools checked changed (FG_REGEN=1 rewrites {DIGEST}):\nwant\n{want}\ngot\n{got}"
    );
}

//! `forge_topo::check` against its Lean model, and what check takes as given.
//!
//! - `model_agrees` (ignored; `verify/forge-topo-check/diff.sh` runs it): the kernel's
//!   combinatorial problems against `lean/`'s `topo-check`, on the benchmark and fuzz parts and
//!   on mutants of them.
//! - `kernel_bodies_have_back_links`: the parts' bodies pass check and have the back-links (`WF`)
//!   the proof assumes.
//! - `*_rejected`, `seam_corners_one_to_one`: the holes the first model left (a shell in pieces,
//!   broken back-links, stray vertices, corner tables not one to one), each now rejected.

use forge_math::rng::Rng;
use forge_math::{Frame, Point2};
use forge_model::suite;
use forge_ops::{Profile, extrude};
use forge_topo::body::Body;
use forge_topo::check::{Problem, check, genus};
use forge_topo_check::{as_checked, back_links, combinatorial, corners, encode, mutants};
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn bodies() -> Vec<(String, Body)> {
    let mut parts: Vec<(String, forge_model::Part)> = suite::parts()
        .into_iter()
        .filter(|(n, _)| !matches!(*n, "torture-2" | "text" | "thread"))
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    parts.extend((0..12u64).map(|s| (format!("fuzz {s}"), suite::fuzz(s, 4))));
    suite::each(&parts, |(n, p)| (n.clone(), p.regenerate().ok()))
        .into_iter()
        .filter_map(|(n, b)| b.map(|b| (n, as_checked(&b))))
        .collect()
}

#[test]
fn kernel_bodies_have_back_links() {
    let bodies = bodies();
    assert!(bodies.len() >= 20, "{} bodies", bodies.len());
    for (n, b) in &bodies {
        assert_eq!(back_links(b), Ok(()), "{n}");
        assert_eq!(check(b), vec![], "{n}");
    }
}

#[test]
#[ignore = "needs lean/'s topo-check built: run verify/forge-topo-check/diff.sh"]
fn model_agrees() {
    let exe = std::env::var("TOPO_CHECK_EXE").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/lean/.lake/build/bin/topo-check").to_string()
    });
    let mut cases: Vec<(String, Body)> = Vec::new();
    let mut r = Rng::new(0x7070);
    for (n, b) in bodies() {
        for _ in 0..4 {
            for (what, m) in mutants(&b, &mut r) {
                cases.push((format!("{n}: {what}"), m));
            }
        }
        cases.push((n, b));
    }
    // the kernel's answer and the model's input; a panic is no answer
    std::panic::set_hook(Box::new(|_| {}));
    let kernel: Vec<Option<(Vec<String>, String)>> = suite::each(&cases, |(_, b)| {
        catch_unwind(AssertUnwindSafe(|| {
            let mut text = String::new();
            encode(b, &mut text);
            (combinatorial(&check(b)), text)
        }))
        .ok()
    });
    let _ = std::panic::take_hook();
    let mut input = String::new();
    for k in &kernel {
        input.push_str(k.as_ref().map_or("body 0\nend\n", |k| &k.1));
    }
    let mut child = std::process::Command::new(&exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("{exe}: {e} (build it with `lake build` in lean/)"));
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "topo-check failed");
    let text = String::from_utf8(out.stdout).unwrap();
    let model: Vec<Vec<String>> = text
        .split("end\n")
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.lines().skip(1).map(str::to_string).collect())
        .collect();
    assert_eq!(model.len(), cases.len());
    let (mut same, mut flagged, mut panics) = (0, 0, Vec::new());
    let mut rules = std::collections::BTreeMap::<String, usize>::new();
    let mut differ = Vec::new();
    for (((n, _), k), m) in cases.iter().zip(&kernel).zip(&model) {
        match k {
            None => panics.push(n.clone()),
            Some((k, _)) if k == m => {
                same += 1;
                flagged += usize::from(!k.is_empty());
                for p in k {
                    *rules.entry(p.split(' ').next().unwrap().to_string()).or_default() += 1;
                }
            }
            Some((k, _)) => differ.push(format!("{n}\n  kernel {k:?}\n  model  {m:?}")),
        }
    }
    eprintln!(
        "{} cases: {same} the same ({flagged} with problems), {} differ, {} panic in check",
        cases.len(),
        differ.len(),
        panics.len()
    );
    eprintln!("problems agreed on, by rule: {rules:?}");
    for p in &panics {
        eprintln!("check panics: {p}");
    }
    assert!(differ.is_empty(), "{}", differ.join("\n"));
    // the corner route: where a face's corner table pairs two leaving coedges with one arriving,
    // the walk is not a permutation and one fan there proves nothing (Proof.lean, one_cycle_by)
    std::panic::set_hook(Box::new(|_| {}));
    let lax: Vec<Option<Vec<String>>> = suite::each(&cases, |(n, b)| {
        catch_unwind(AssertUnwindSafe(|| {
            let mut out = Vec::new();
            for f in b.face_ids() {
                let mut vs: Vec<_> = b
                    .face(f)
                    .loops
                    .iter()
                    .flat_map(|&l| b.lp(l).coedges())
                    .map(|c| b.coedge_start(c))
                    .collect();
                vs.sort();
                vs.dedup();
                for v in vs {
                    let mut into: Vec<_> = corners(b, f, v).iter().map(|x| x.1).collect();
                    let k = into.len();
                    into.sort();
                    into.dedup();
                    if into.len() < k {
                        out.push(format!("{n}: face {} at vertex {}", f.0, v.0));
                    }
                }
            }
            out
        }))
        .ok()
    });
    let _ = std::panic::take_hook();
    let lax: Vec<String> = lax.into_iter().flatten().flatten().collect();
    eprintln!(
        "{} corner tables pair two leaving coedges with one arriving",
        lax.len()
    );
    for l in lax.iter().take(10) {
        eprintln!("  {l}");
    }
    assert!(flagged > cases.len() / 2, "the mutants should mostly be caught");
    for rule in [
        "edge-uses",
        "edge-same-way",
        "loop-broken",
        "shell-euler",
        "vertex-fans",
    ] {
        assert!(rules.contains_key(rule), "no case reached {rule}");
    }
}

fn plate(feature: u64, at: Point2, w: f64, hole: bool) -> Body {
    let mut loops = vec![Profile::rectangle(at, w, w, 0)];
    if hole {
        loops.push(Profile::circle(
            Point2::new(at.x + w / 2.0, at.y + w / 2.0),
            w / 6.0,
            10,
        ));
    }
    let p = Profile {
        plane: Frame::WORLD,
        loops,
    };
    extrude(&p, 10.0, feature).expect("a plate")
}

/// Two bodies as one shell, as a hand-built or edited body can have them.
fn one_shell(a: &Body, b: &Body) -> Body {
    let mut m = a.clone();
    m.append(b);
    let moved: Vec<_> = m.shells.drain(1..).flat_map(|s| s.faces).collect();
    for &f in &moved {
        m.faces[f.index()].shell = forge_topo::body::ShellId(0);
    }
    m.shells[0].faces.extend(moved);
    m
}

/// A shell in two pieces: a plate with a hole (V − E + 2F − L = 0) and an overlapping plate (2) as
/// one shell. Its Euler value (2) passes, and the overlap test runs only between shells; the
/// connectivity test finds it.
#[test]
fn disconnected_shell_rejected() {
    let holed = plate(1, Point2::new(0.0, 0.0), 40.0, true);
    let other = plate(2, Point2::new(20.0, 20.0), 40.0, false);
    assert_eq!(check(&holed), vec![]);
    assert_eq!(genus(&holed), 1);
    let one = one_shell(&holed, &other);
    assert_eq!(one.shells.len(), 1);
    assert_eq!(
        check(&one),
        vec![Problem::ShellDisconnected {
            shell: forge_topo::body::ShellId(0)
        }]
    );
    // a face its shell does not list, and one listed twice
    let mut lost = holed.clone();
    lost.shells[0].faces.pop();
    assert!(
        check(&lost)
            .iter()
            .any(|p| matches!(p, Problem::ShellFaces { .. }))
    );
    let mut twice = holed.clone();
    let f = twice.shells[0].faces[0];
    twice.shells[0].faces.push(f);
    assert!(
        check(&twice)
            .iter()
            .any(|p| matches!(p, Problem::ShellFaces { .. }))
    );
}

/// Broken back-links: an edge's use swapped for a copy of it outside every listed loop (the old
/// `check` accepted this), and two coedges trading their loops (the old `check` panicked).
#[test]
fn broken_back_links_rejected() {
    let mut b = plate(1, Point2::new(0.0, 0.0), 40.0, false);
    let f = forge_topo::body::FaceId(0);
    let l0 = b.face(f).loops[0];
    let uses: Vec<_> = b
        .lp(l0)
        .coedges()
        .map(|c| (b.coedge(c).edge, b.coedge(c).forward))
        .collect();
    let first = b.coedges.len() as u32;
    // two copies, left over as `remove_loop` leaves loops, so V − E + 2F − L stays even
    for _ in 0..2 {
        let lp = b.add_loop(f, &uses);
        b.faces[0].loops.retain(|&l| l != lp);
    }
    for e in b.edges.iter_mut() {
        e.coedges.retain(|c| c.0 < first);
    }
    assert_eq!(check(&b), vec![], "left-over loops are no part of the body");
    let c = b.lp(l0).coedge(0);
    let e = b.coedge(c).edge;
    for x in b.edges[e.index()].coedges.iter_mut() {
        if *x == c {
            *x = forge_topo::body::CoedgeId(first);
        }
    }
    assert!(back_links(&b).is_err());
    let p = check(&b);
    assert!(
        !p.is_empty() && p.iter().all(|p| matches!(p, Problem::BackLink { .. })),
        "{p:?}"
    );

    let mut b = plate(1, Point2::new(0.0, 0.0), 40.0, false);
    let (la, lb) = (
        b.face(forge_topo::body::FaceId(0)).loops[0],
        b.face(forge_topo::body::FaceId(1)).loops[0],
    );
    let (ca, cb) = (b.lp(la).coedge(0), b.lp(lb).coedge(0));
    b.coedges[ca.index()].lp = lb;
    b.coedges[cb.index()].lp = la;
    let p = check(&b);
    assert!(
        !p.is_empty() && p.iter().all(|p| matches!(p, Problem::BackLink { .. })),
        "{p:?}"
    );
}

/// Vertices on no edge: two added to a plate with a hole, where the global V − E + 2F − L goes
/// from 0 to 2 and so passed.
#[test]
fn stray_vertices_rejected() {
    let mut b = plate(1, Point2::new(0.0, 0.0), 40.0, true);
    let n = b.vertices.len() as u32;
    for _ in 0..2 {
        b.add_vertex(
            forge_math::Point3::new(500.0, 500.0, 500.0),
            forge_topo::Name::NONE,
        );
    }
    assert_eq!(
        check(&b),
        vec![
            Problem::StrayVertex {
                vertex: forge_topo::body::VertexId(n)
            },
            Problem::StrayVertex {
                vertex: forge_topo::body::VertexId(n + 1)
            }
        ]
    );
}

fn pin(feature: u64, frame: Frame) -> Body {
    let p = Profile {
        plane: frame,
        loops: vec![Profile::circle(Point2::new(0.0, 0.0), 5.0, 0)],
    };
    extrude(&p, 10.0, feature).expect("a pin")
}

/// `Body::corners` at a cylinder's seam vertex, where the closed circle edge both leaves and
/// arrives: the seam's other use leaves the same way as the seam, and is taken last, so the two
/// leaving coedges are closed by two different arriving ones. (`fans` trusts a corner table only
/// where it is one to one in any case.)
#[test]
fn seam_corners_one_to_one() {
    let b = pin(1, Frame::WORLD);
    assert_eq!(check(&b), vec![]);
    let side = b
        .face_ids()
        .find(|&f| matches!(b.face(f).surface, forge_geom::Surface::Cylinder(_)))
        .expect("a side");
    let seam_vertex = b.edge(b.coedge(b.lp(b.face(side).loops[0]).coedge(0)).edge).start;
    let k = corners(&b, side, seam_vertex);
    assert_eq!(k.len(), 2, "{k:?}");
    assert_ne!(k[0].1, k[1].1, "{k:?}");
    // and as loop order closes them
    for (c, into) in k {
        let lp = b.lp(b.coedge(c).lp);
        let n = lp.coedges().len();
        let i = (c.0 - lp.coedge(0).0) as usize;
        assert_eq!(into, lp.coedge((i + n - 1) % n));
    }
}

/// Two tangent pins sharing their seam vertices: pinched at both, which check finds (through the
/// corner route, with more fans than loop order gives).
#[test]
fn pinched_pins_rejected() {
    let a = pin(1, Frame::WORLD);
    let turned = Frame::from_zx(
        forge_math::Point3::new(10.0, 0.0, 0.0),
        forge_math::UnitVec3::Z,
        forge_math::Vec3::new(-1.0, 0.0, 0.0),
    )
    .expect("a frame");
    let b = pin(2, turned);
    let mut m = a.clone();
    m.append(&b);
    let n = a.vertices.len();
    let mut shared = 0;
    for (i, v) in b.vertices.iter().enumerate() {
        if let Some(j) = a.vertices.iter().position(|w| w.point.distance(v.point) < 1e-9) {
            shared += 1;
            for e in m.edges.iter_mut() {
                for x in [&mut e.start, &mut e.end] {
                    if x.index() == n + i {
                        *x = forge_topo::body::VertexId(j as u32);
                    }
                }
            }
        }
    }
    assert_eq!(shared, 2);
    let p = check(&m);
    assert_eq!(
        p.iter()
            .filter(|p| matches!(p, Problem::VertexFans { .. }))
            .count(),
        2,
        "{p:?}"
    );
}

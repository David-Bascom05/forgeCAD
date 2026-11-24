//! `check_against` answers as `check` does. On real parts, every checked operation runs both ways; on
//! one fault put into any one entity of a good body, the comparison must find it, so a rule left out
//! or a comparison grown too loose fails here (research/check-local). The parts are Forge parts.
mod common;
mod kernel_util;

use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::{Frame, Vec3};
use forge_topo::{Body, Problem, check, check_against};
use kernel_util::built;
use std::collections::BTreeSet;

const BODIES: &str = r#"sketch s(on: xy) {
  r = rect(corner: (-30, -20), size: (60, 40))
}
base = extrude(s, 12)
round = fillet(edges(of: base, along: Z), 3)
rim = chamfer(edges(of: base.end), 0.7)
sketch b(on: base.end) {
  c = circle((-12, 0), 7)
}
boss = extrude(b, 6)
bossrim = chamfer(edges(of: boss.end), 0.5)
sketch h(on: xy) {
  p = pt(10, 5)
}
drill = hole(at: h.p, d: 8, depth: thru, flip: true)
"#;

fn kind(p: &Problem) -> String {
    format!("{p:?}")
        .split([' ', '('])
        .next()
        .unwrap_or("")
        .to_string()
}

/// `b` with nothing it worked out kept: a box from before the fault would hide it.
fn afresh(b: &Body) -> Body {
    let mut b = b.clone();
    b.faces.iter_mut().for_each(|f| f.bbox = Default::default());
    b.edges.iter_mut().for_each(|e| e.bbox = Default::default());
    b
}

fn same_answer(bad: &Body, good: &Body, what: &str, kinds: &mut BTreeSet<String>) {
    let bad = &afresh(bad);
    let (full, local) = (check(bad), check_against(bad, good));
    assert_eq!(local, full, "{what}");
    kinds.extend(full.iter().map(kind));
}

/// The stages of a part (its first body after each kernel feature that leaves one), compacted as
/// a check compacts them.
fn stages_of(src: &str) -> Vec<Body> {
    built(src)
        .stages()
        .iter()
        .filter_map(|s| s.first().map(|s| s.body.compact()))
        .collect()
}

#[test]
fn a_fault_in_any_one_entity_is_found() {
    let stages = stages_of(BODIES);
    let good = stages[stages.len() - 1].clone();
    assert!(check(&good).is_empty());
    let shift = Frame::WORLD.with_origin(Frame::WORLD.origin + Vec3::new(0.01, 0.02, 0.03));
    let mut kinds = BTreeSet::new();
    // against itself, and against the stage before the hole, whose top face has one loop less
    for old in [&good, &stages[stages.len() - 2]] {
        assert!(check(old).is_empty());
        let mut run = |what: String, change: &dyn Fn(&mut Body)| {
            let mut bad = good.clone();
            change(&mut bad);
            same_answer(&bad, old, &what, &mut kinds);
        };
        run("nothing".to_string(), &|_| {});
        run("one vertex too many".to_string(), &|b| {
            b.vertices.push(b.vertices[0].clone())
        });
        for i in 0..good.faces.len() {
            // its loops taken off as `remove_loop` does (emptying the list alone leaves its edges
            // naming coedges of loops no face lists: a broken link)
            run(format!("face {i} no loops"), &|b| {
                b.faces[i].loops.retain(|_| false)
            });
            run(format!("face {i} loops removed"), &|b| {
                let f = forge_topo::FaceId(i as u32);
                let loops: Vec<_> = b.faces[i].loops.iter().copied().collect();
                for l in loops {
                    b.remove_loop(f, l);
                }
            });
            // taken off in place, links kept: the shell is open at its edges
            run(format!("face {i} killed"), &|b| {
                b.kill_face(forge_topo::FaceId(i as u32))
            });
            run(format!("face {i} sense"), &|b| b.faces[i].same_sense ^= true);
            run(format!("face {i} surface"), &|b| {
                b.faces[i].surface = b.faces[i].surface.transformed(&shift);
            });
            // the same plane turned over: its edges still lie on it, only the side test can tell
            if let forge_geom::Surface::Plane(p) = &good.faces[i].surface {
                let over = forge_geom::Plane::new(Frame::from_z(p.frame.origin, -p.frame.z()));
                run(format!("face {i} plane over"), &|b| {
                    b.faces[i].surface = forge_geom::Surface::Plane(over)
                });
            }
            run(format!("face {i} name"), &|b| {
                b.faces[i].name = b.faces[(i + 1) % b.faces.len()].name
            });
        }
        for i in 0..good.edges.len() {
            run(format!("edge {i} curve"), &|b| {
                b.edges[i].curve = b.edges[i].curve.transformed(&shift)
            });
            run(format!("edge {i} range"), &|b| {
                let r = b.edges[i].range;
                b.edges[i].range = forge_math::Interval::new(r.lo + 0.25 * r.width(), r.hi);
            });
            // walked the other way by both faces: a closed edge's loops still close
            run(format!("edge {i} both ways"), &|b| {
                for c in b.edges[i].coedges.clone().iter() {
                    b.coedges[c.index()].forward ^= true;
                }
            });
            run(format!("edge {i} ends"), &|b| {
                let e = &mut b.edges[i];
                std::mem::swap(&mut e.start, &mut e.end);
            });
            run(format!("edge {i} one use"), &|b| {
                let mut first = true;
                b.edges[i].coedges.retain(|_| std::mem::take(&mut first));
            });
        }
        // two corners made one: two fans of faces round it
        run("corners joined".to_string(), &|b| {
            let (keep, other) = (b.edges[0].start, b.edges[b.edges.len() - 1].end);
            for e in &mut b.edges {
                if e.start == other {
                    e.start = keep;
                }
                if e.end == other {
                    e.end = keep;
                }
            }
        });
        for i in 0..good.vertices.len() {
            run(format!("vertex {i} point"), &|b| b.vertices[i].point.x += 0.01);
            run(format!("vertex {i} tolerance"), &|b| b.grow_vertex_tol(forge_topo::VertexId(i as u32), 1e3));
        }
        for i in 0..good.coedges.len() {
            run(format!("coedge {i} way"), &|b| b.coedges[i].forward ^= true);
            run(format!("coedge {i} edge"), &|b| {
                b.coedges[i].edge = b.coedges[(i + 1) % b.coedges.len()].edge;
            });
        }
    }
    for k in [
        "EdgeUses",
        "EdgeUsesSameWay",
        "EdgeOffFace",
        "VertexOffEdge",
        "VertexTolerance",
        "FaceWrongSide",
        "LoopBroken",
        "DuplicateName",
        "EdgeDegenerate",
        "OuterNotFirst",
        "VertexFans",
        "Euler",
        "FaceNoLoops",
        "BackLink",
        "StrayVertex",
    ] {
        assert!(kinds.contains(k), "no fault tried shows {k}: {kinds:?}");
    }
}

/// `good` with these faces, their edges and vertices moved.
fn moved(good: &Body, faces: &[forge_topo::FaceId], by: Vec3) -> Body {
    let shift = Frame::WORLD.with_origin(Frame::WORLD.origin + by);
    let mut bad = good.clone();
    let (mut es, mut vs) = (BTreeSet::new(), BTreeSet::new());
    for &f in faces {
        bad.faces[f.index()].surface = good.face(f).surface.transformed(&shift);
        for &l in &good.face(f).loops {
            for c in good.lp(l).coedges() {
                let e = good.coedge(c).edge;
                es.insert(e);
                vs.insert(good.edge(e).start);
                vs.insert(good.edge(e).end);
            }
        }
    }
    for e in es {
        bad.edges[e.index()].curve = good.edge(e).curve.transformed(&shift);
    }
    for v in vs {
        bad.vertices[v.index()].point = good.vertex(v).point + by;
    }
    bad
}

/// A hole's wall moved over the edge of its face: the face's loops cross.
#[test]
fn loops_that_cross_are_found() {
    let stages = stages_of(BODIES);
    let good = stages[stages.len() - 1].clone();
    let wall: Vec<_> = good
        .face_ids()
        .filter(|&f| matches!(&good.face(f).surface, forge_geom::Surface::Cylinder(c) if (c.radius - 4.0).abs() < 1e-9))
        .collect();
    assert!(!wall.is_empty());
    let mut kinds = BTreeSet::new();
    for old in [&good, &stages[stages.len() - 2]] {
        same_answer(
            &moved(&good, &wall, Vec3::new(18.0, 0.0, 0.0)),
            old,
            "over the rim",
            &mut kinds,
        );
    }
    assert!(kinds.contains("LoopsCross"), "{kinds:?}");
}

/// A new hole round a kept one on a plane face: the kept loop's coedges face into the new hole, though neither
/// loop crosses the other. What is near a change must be tested.
#[test]
fn a_kept_loop_inside_a_new_one_is_found() {
    let stages = stages_of(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-60, -60), size: (120, 120))
}
base = extrude(s1, 12)
sketch s2(on: xy) {
  p = pt(0, 0)
}
hole1 = hole(at: s2.p, d: 3, depth: thru, flip: true)
sketch d3(on: offset(xy, 0)) {
  p = pt(30, 0)
}
sketch s4(on: plane(at: d3.p, normal: Z, x: X)) {
  p = pt(0, 0)
}
hole2 = hole(at: s4.p, d: 12, depth: thru, flip: true)
"#,
    );
    let (old, good) = (&stages[stages.len() - 2], &stages[stages.len() - 1]);
    let wall: Vec<_> = good
        .face_ids()
        .filter(|&f| matches!(&good.face(f).surface, forge_geom::Surface::Cylinder(c) if (c.radius - 6.0).abs() < 1e-9))
        .collect();
    assert!(!wall.is_empty());
    let mut kinds = BTreeSet::new();
    same_answer(
        &moved(good, &wall, Vec3::new(-30.0, 0.0, 0.0)),
        old,
        "round the small hole",
        &mut kinds,
    );
    // against the body with both: the moved loop has the old one's names, and must not pass for it
    let before = kinds.len();
    let mut again = BTreeSet::new();
    same_answer(
        &moved(good, &wall, Vec3::new(-30.0, 0.0, 0.0)),
        good,
        "named as it was",
        &mut again,
    );
    assert!(again.contains("FaceWrongSide") && before > 0, "{again:?}");
    assert!(kinds.contains("FaceWrongSide"), "{kinds:?}");
}

/// Faces unchanged, their material changed: a void moved round another, and a void taken away from round an
/// island. Only an unchanged face's probe tells, so where material changed must be asked about.
#[test]
fn unchanged_faces_in_changed_material_are_found() {
    let stages = stages_of(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-60, -60), size: (120, 120))
}
base = extrude(s1, 60)
sketch d2(on: offset(xy, 20)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (-40, -40), size: (30, 30))
}
big = cut(s3, -20)
sketch d4(on: offset(xy, 28)) {
  p = pt(0, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  r1 = rect(corner: (25, 25), size: (4, 4))
}
small = cut(s5, -4)
"#,
    );
    let good = stages[stages.len() - 1].clone();
    assert_eq!(good.shells.len(), 3, "{:?}", good.shells.len());
    let mut kinds = BTreeSet::new();
    let of = |x: f64| {
        (0..good.shells.len()).find(|&k| {
            good.shells[k].faces.iter().any(|&f| {
                let b = good.face_box(f);
                (b.x.hi - b.x.lo - x).abs() < 1e-3
            })
        })
    };
    let big = of(30.0).expect("the big void");
    let by = Vec3::new(40.0, 40.0, 0.0);
    same_answer(
        &moved(&good, &good.shells[big].faces, by),
        &good,
        "big round small",
        &mut kinds,
    );
    assert!(kinds.contains("ShellsOverlap"), "{kinds:?}");

    let island = stages_of(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-60, -60), size: (120, 120))
}
base = extrude(s1, 60)
sketch d2(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (-40, -40), size: (80, 80))
}
big = cut(s3, -40)
sketch d4(on: offset(xy, 25)) {
  p = pt(0, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  r1 = rect(corner: (-5, -5), size: (10, 10))
}
isle = extrude(s5, 10)
"#,
    );
    let good = island[island.len() - 1].clone();
    assert_eq!(good.shells.len(), 3, "{:?}", good.shells.len());
    assert!(check(&good).is_empty());
    let void = (0..good.shells.len())
        .find(|&k| {
            good.shells[k].faces.iter().any(|&f| {
                let b = good.face_box(f);
                (b.x.hi - b.x.lo - 80.0).abs() < 1e-3
            })
        })
        .expect("the void");
    let mut bad = good.clone();
    for &f in &good.shells[void].faces {
        bad.kill_face(f);
    }
    let bad = bad.compact();
    let mut kinds = BTreeSet::new();
    same_answer(&bad, &good, "void gone from round the island", &mut kinds);
    assert!(kinds.contains("ShellsOverlap"), "{kinds:?}");
}

/// Only a plane face is compared loop by loop, and only where its new loops' edges are long enough to leave the step alone.
#[test]
fn which_faces_are_compared_by_loop() {
    let of = |src: &str| {
        let s = stages_of(src);
        forge_topo::check::faces_by_loop(&s[s.len() - 1], &s[s.len() - 2])
    };
    assert_eq!(
        of(r#"sketch s1(on: xy) {
  r1 = rect(corner: (-60, -60), size: (120, 120))
}
base = extrude(s1, 40)
sketch d2(on: offset(xy, 0)) {
  p = pt(-30, -30)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  p = pt(0, 0)
}
hole1 = hole(at: s3.p, d: 6, depth: thru, flip: true)
sketch d4(on: offset(xy, 0)) {
  p = pt(30, 30)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  p = pt(0, 0)
}
hole2 = hole(at: s5.p, d: 6, depth: thru, flip: true)
"#),
        2,
        "top and bottom"
    );
    // edges shorter than 2
    assert_eq!(
        of(r#"sketch s1(on: xy) {
  r1 = rect(corner: (-60, -60), size: (120, 120))
}
base = extrude(s1, 40)
sketch d2(on: offset(xy, 0)) {
  p = pt(-30, -30)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  p = pt(0, 0)
}
hole1 = hole(at: s3.p, d: 6, depth: thru, flip: true)
sketch d4(on: offset(xy, 0)) {
  p = pt(30, 30)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  p = pt(0, 0)
}
hole2 = hole(at: s5.p, d: 0.5, depth: thru, flip: true)
"#),
        0
    );
    // through a round wall: not a plane
    assert_eq!(
        of(r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 20)
}
base = extrude(s1, 80)
sketch d2(on: offset(xy, 20)) {
  p = pt(-40, 0)
}
sketch s3(on: plane(at: d2.p, normal: X, x: Y)) {
  c1 = circle((0, 0), 4)
}
c = cut(s3, -80)
sketch d4(on: offset(xy, 60)) {
  p = pt(-40, 0)
}
sketch s5(on: plane(at: d4.p, normal: X, x: Y)) {
  c1 = circle((0, 0), 4)
}
d = cut(s5, -80)
"#),
        0
    );
}

/// A body with a void: moved out of the material, or into another void, the void's shell is at fault.
#[test]
fn a_shell_moved_is_found() {
    let stages = stages_of(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-40, -40), size: (80, 80))
}
base = extrude(s1, 30)
sketch d2(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (-10, -10), size: (20, 20))
}
void1 = cut(s3, -10)
sketch s4(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (20, 20), size: (10, 10))
}
void2 = cut(s4, -10)
sketch d5(on: offset(xy, 0)) {
  p = pt(-30, -30)
}
sketch s6(on: plane(at: d5.p, normal: Z, x: X)) {
  p = pt(0, 0)
}
hole1 = hole(at: s6.p, d: 4, depth: thru, flip: true)
"#,
    );
    let good = stages[stages.len() - 1].clone();
    assert!(good.shells.len() > 1, "{}", good.shells.len());
    assert!(check(&good).is_empty());
    let mut kinds = BTreeSet::new();
    for old in [&good, &stages[stages.len() - 2]] {
        for shell in 1..good.shells.len() {
            for by in [
                Vec3::new(0.0, 0.0, 100.0),
                Vec3::new(30.0, 30.0, 0.0),
                Vec3::new(0.0, 0.0, 3.0),
            ] {
                let bad = moved(&good, &good.shells[shell].faces, by);
                // a void out in the open is in no material: a fault, though its box is clear of every shell
                if by.z == 100.0 {
                    assert!(
                        check(&afresh(&bad))
                            .iter()
                            .any(|p| matches!(p, Problem::ShellsOverlap { .. })),
                        "shell {shell}: {} faces of {}, found {:?} shells {:?}",
                        good.shells[shell].faces.len(),
                        good.faces.len(),
                        check(&afresh(&bad)),
                        good.shells.iter().map(|s| s.faces.len()).collect::<Vec<_>>()
                    );
                }
                same_answer(&bad, old, &format!("shell {shell} by {by:?}"), &mut kinds);
            }
        }
    }
    assert!(kinds.contains("ShellsOverlap"), "{kinds:?}");
}

/// The bench parts, built from their Forge files: the history the kernel regenerates.
const BENCH: [(&str, &str); 5] = [
    ("bracket", include_str!("../parts/bench/bracket.fg")),
    ("clamp", include_str!("../parts/bench/clamp.fg")),
    ("hub", include_str!("../parts/bench/hub.fg")),
    ("insert", include_str!("../parts/bench/insert.fg")),
    ("heatsink", include_str!("../parts/bench/heatsink.fg")),
];

/// The worker's check of each stage on the bench: none faults, and going on from a stage finds what going
/// from the start does. A fault put into a stage is found at that stage.
#[test]
fn stages_of_the_bench_pass_and_a_fault_is_named() {
    use forge_model::verify::{Verdict, faults};
    let stop = std::sync::atomic::AtomicBool::new(false);
    for (name, src) in BENCH {
        let mut stages = built(src).stages();
        // the stages from the first solid on, as the worker holds them
        stages.retain(|s| !s.is_empty());
        let of = |s: &[Vec<forge_model::Solid>], from| faults(s.iter().map(|x| &x[..]), from, &stop);
        assert_eq!(of(&stages, 0), Verdict::Done(vec![]), "{name}");
        let at = stages.len() - 1;
        let mut bad = stages.clone();
        let f = bad[at][0].body.faces.len() - 1;
        bad[at][0].body.faces[f].same_sense ^= true;
        for from in [0, at] {
            let Verdict::Done(found) = of(&bad, from) else {
                panic!("{name}: stopped");
            };
            assert_eq!(found.len(), 1, "{name}: {found:?}");
            let i = &found[0].issues[0];
            assert_eq!(found[0].feature, at, "{name}");
            assert!(
                !i.reason.is_empty() && i.rule == "face-wrong-side" && i.entity.is_some(),
                "{name}: {i:?}"
            );
        }
    }
}

#[test]
fn an_edge_held_to_less_than_it_was_is_measured_again() {
    let mut loose = stages_of(BODIES).pop().expect("stages");
    let v = loose.edges[0].start;
    loose.vertices[v.index()].point = loose.vertices[v.index()].point + Vec3::new(1e-3, 0.0, 0.0);
    loose.edges.iter_mut().for_each(|e| e.tol = 1e-2);
    assert_eq!(check(&afresh(&loose)), vec![], "within what it may stray");
    let mut tight = loose.clone();
    tight
        .edges
        .iter_mut()
        .for_each(|e| e.tol = forge_math::tol::LINEAR);
    assert!(!check(&afresh(&tight)).is_empty(), "past what it may stray");
    same_answer(&tight, &loose, "tolerance tightened", &mut BTreeSet::new());
}

#[test]
fn both_ways_on_real_parts() {
    forge_topo::check::check_against_both_ways(true);
    for (_, src) in &BENCH[..4] {
        built(src).seq().expect("builds");
    }
    // the fuzz corpus: the seeds that build
    for seed in include_str!("kernel_parts/check_local_corpus.txt")
        .split("# seed")
        .skip(1)
    {
        let src = seed.split_once('\n').map_or("", |x| x.1);
        let (r, cache) = common::run(src);
        if r.errors.is_empty() {
            let (f, ids) = cache.history();
            let _ = forge_model::Part { features: f.to_vec() }.regenerate_solids_ids(Some(ids));
        }
    }
    forge_topo::check::check_against_both_ways(false);
}

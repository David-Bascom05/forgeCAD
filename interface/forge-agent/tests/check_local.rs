//! `check_against` answers as `check` does. On real parts, every checked operation runs both ways; on
//! one fault put into any one entity of a good body, the comparison must find it, so a rule left out
//! or a comparison grown too loose fails here (research/check-local).
use forge_agent::bench;
use forge_geom::{CurveGeom, SurfaceGeom};
use forge_math::{Frame, Vec3};
use forge_model::Part;
use forge_topo::{Body, Problem, check, check_against};
use std::collections::BTreeSet;

fn resolved(src: &str) -> Option<Part> {
    let m = bench::model(src);
    m.error.is_none().then(|| Part {
        features: m.features.clone(),
    })
}

const BODIES: &str = "base: extrude 12 rect -30 -20 60 40
fillet 3 (edges of base along +z)
chamfer 0.7 (edges of base top)
boss: extrude add 6 at 0 0 12 circle -12 0 7
chamfer 0.5 (edges of boss top)
hole 8 at 10 5 0 thru
";

fn kind(p: &Problem) -> String {
    format!("{p:?}")
        .split([' ', '('])
        .next()
        .unwrap_or("")
        .to_string()
}

fn same_answer(bad: &Body, good: &Body, what: &str, kinds: &mut BTreeSet<String>) {
    let (full, local) = (check(bad), check_against(bad, good));
    assert_eq!(local, full, "{what}");
    kinds.extend(full.iter().map(kind));
}

/// The stages of a part, compacted as a check compacts them.
fn stages_of(src: &str) -> Vec<Body> {
    let part = resolved(src).expect("resolves");
    let h = forge_model::History::new(&part.features, None);
    let (mut solids, mut out) = (Vec::new(), Vec::new());
    for i in 0..part.features.len() {
        solids = forge_model::step_solids(h, i, solids).expect("builds");
        out.extend(solids.first().map(|s: &forge_model::Solid| s.body.compact()));
    }
    out
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
            run(format!("face {i} no loops"), &|b| {
                b.faces[i].loops.retain(|_| false)
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
        for i in 0..good.vertices.len() {
            run(format!("vertex {i} point"), &|b| b.vertices[i].point.x += 0.01);
            run(format!("vertex {i} tolerance"), &|b| b.vertices[i].tol = 1e3);
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
    ] {
        assert!(kinds.contains(k), "no fault tried shows {k}: {kinds:?}");
    }
}

/// The worker's check of each stage on the bench: none faults, and going on from a stage finds what going
/// from the start does. A fault put into a stage is found at that stage.
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
        "base: extrude 12 rect -60 -60 120 120
hole 3 at 0 0 0 thru
hole 12 at 30 0 0 thru
",
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
    assert!(kinds.contains("FaceWrongSide"), "{kinds:?}");
}

/// Faces unchanged, their material changed: a void moved round another, and a void taken away from round an
/// island. Only an unchanged face's probe tells, so where material changed must be asked about.
#[test]
fn unchanged_faces_in_changed_material_are_found() {
    let stages = stages_of(
        "base: extrude 60 rect -60 -60 120 120
big: extrude cut 20 at 0 0 20 rect -40 -40 30 30
small: extrude cut 4 at 0 0 28 rect 25 25 4 4
",
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
        "base: extrude 60 rect -60 -60 120 120
big: extrude cut 40 at 0 0 10 rect -40 -40 80 80
isle: extrude add 10 at 0 0 25 rect -5 -5 10 10
",
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

/// A body with a void: moved out of the material, or into another void, the void's shell is at fault.
#[test]
fn a_shell_moved_is_found() {
    let stages = stages_of(
        "base: extrude 30 rect -40 -40 80 80
void: extrude cut 10 at 0 0 10 rect -10 -10 20 20
void2: extrude cut 10 at 0 0 10 rect 20 20 10 10
hole 4 at -30 -30 0 thru
",
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
                same_answer(&bad, old, &format!("shell {shell} by {by:?}"), &mut kinds);
            }
        }
    }
    assert!(kinds.contains("ShellsOverlap"), "{kinds:?}");
}

#[test]
fn stages_of_the_bench_pass_and_a_fault_is_named() {
    use forge_model::verify::{Verdict, faults};
    let stop = std::sync::atomic::AtomicBool::new(false);
    for name in ["bracket", "clamp", "hub", "insert", "heatsink"] {
        let part = bench::part(name);
        let h = forge_model::History::new(&part.features, None);
        let mut stages: Vec<Vec<forge_model::Solid>> = Vec::new();
        for i in 0..part.features.len() {
            let before = stages.last().cloned().unwrap_or_default();
            stages.push(forge_model::step_solids(h, i, before).expect("builds"));
        }
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
fn a_worker_can_hold_the_stages() {
    fn across_threads<T: Send + Sync>() {}
    across_threads::<std::sync::Arc<forge_agent::model::Stage>>();
}

#[test]
fn both_ways_on_real_parts() {
    forge_topo::check::check_against_both_ways(true);
    for name in ["bracket", "clamp", "hub", "insert"] {
        bench::part(name).regenerate_solids().expect("builds");
    }
    for src in include_str!("../../../research/check-local/fuzz/corpus.txt").split("# seed") {
        if let Some(part) = resolved(src.split_once('\n').map_or("", |x| x.1)) {
            let _ = part.regenerate_solids();
        }
    }
    forge_topo::check::check_against_both_ways(false);
}

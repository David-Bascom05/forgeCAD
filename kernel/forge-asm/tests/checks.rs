//! Interference, clearance, motion sweeps and the BOM on real kernel solids, and the
//! interference oracle (prior-art §11.10): the conservative mesh phase never misses an exact
//! overlap deeper than its tolerance.

mod common;
use common::*;
use forge_asm::bom::{BomItem, bom};
use forge_asm::check::*;
use forge_asm::*;
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{Profile, ProfileLoop, extrude};
use std::sync::Arc;

const CHORD: f64 = 0.01;

fn solid(loop_: ProfileLoop, h: f64) -> forge_topo::Body {
    extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![loop_],
        },
        h,
        1,
    )
    .unwrap()
}

fn block(w: f64, d: f64, h: f64) -> forge_topo::Body {
    solid(Profile::rectangle(Point2::new(0.0, 0.0), w, d, 1), h)
}

fn rod(r: f64, h: f64) -> forge_topo::Body {
    solid(Profile::circle(Point2::new(0.0, 0.0), r, 1), h)
}

fn add(cat: &mut Catalog, ex: &mut BrepExact, name: &str, b: forge_topo::Body) {
    let q = forge_mesh::Quality {
        chord: CHORD,
        max_angle: dmath::PI / 12.0,
    };
    let m = forge_mesh::tessellate(&b, &q).unwrap();
    let info = PartInfo {
        mesh: Some(Arc::new(PartMesh::from_trimesh(&m, CHORD))),
        ..PartInfo::default()
    };
    cat.parts.insert((DocId(name.into()), 0), info);
    ex.bodies.insert((DocId(name.into()), 0), b);
}

fn shop() -> (Catalog, BrepExact) {
    let mut c = Catalog::default();
    let mut e = BrepExact::default();
    add(&mut c, &mut e, "block", block(10.0, 10.0, 10.0));
    add(&mut c, &mut e, "big", block(40.0, 40.0, 40.0));
    add(&mut c, &mut e, "rod", rod(2.0, 20.0));
    add(&mut c, &mut e, "arm", block(30.0, 4.0, 4.0));
    (c, e)
}

fn of(id: u64, name: &str, pose: Frame) -> Instance {
    Instance::part(id, PartRef::new(name, 0), pose)
}

fn placed(insts: Vec<Instance>) -> Assembly {
    let mut insts = insts;
    insts[0].ground = true;
    // every instance held where written
    let joints = insts
        .iter()
        .skip(1)
        .map(|i| {
            Joint::new(
                i.id.0,
                JointKind::Fastened,
                End::local(&[i.id.0], Frame::WORLD),
                End::local(&[insts[0].id.0], insts[0].pose.inverse().compose(&i.pose)),
            )
        })
        .collect();
    Assembly {
        instances: insts,
        joints,
        ..Assembly::default()
    }
}

#[test]
fn overlaps_touches_containment_and_fits() {
    let (c, e) = shop();
    let asm = placed(vec![
        of(1, "block", Frame::WORLD),
        of(2, "block", at(8.0, 0.0, 0.0)),  // 2 deep: 200 mm³
        of(3, "block", at(0.0, 10.0, 0.0)), // face to face
        of(4, "big", at(100.0, 0.0, 0.0)),
        of(5, "block", at(110.0, 10.0, 10.0)), // wholly inside 4
        of(6, "rod", at(5.0, 5.0, -5.0)),      // through 1, declared a fit
    ]);
    let mut asm = asm;
    asm.fits.push(Fit {
        a: vec![InstanceId(6)],
        b: vec![InstanceId(1)],
    });
    let sol = solve(&asm, &c, &Options::default());
    let mut ck = Checker::new(&c, &e);
    let rep = ck.interference(&sol, &asm.fits, &[]);
    let pair = |a: u64, b: u64| (vec![InstanceId(a)], vec![InstanceId(b)]);
    let find = |a: u64, b: u64| {
        rep.interferences
            .iter()
            .find(|h| (h.a.clone(), h.b.clone()) == pair(a, b))
    };
    let h12 = find(1, 2).expect("an overlap");
    assert!(h12.confirmed);
    assert!((h12.volume.unwrap() - 200.0).abs() < 1e-6, "{:?}", h12.volume);
    assert!(find(1, 3).is_none());
    let h45 = find(4, 5).expect("containment");
    assert!((h45.volume.unwrap() - 1000.0).abs() < 1e-6);
    // the rod is a declared fit: intended contact, reported, not interference
    assert!(find(1, 6).is_none());
    assert_eq!(
        rep.intended,
        vec![IntendedContact {
            a: vec![InstanceId(1)],
            b: vec![InstanceId(6)],
            by: Intent::Fit { index: 0 },
        }]
    );
    assert_eq!(rep.interferences.len(), 2);
    assert!(rep.interferences.iter().all(|h| h.tolerance < 0.03));
    // as diagnostics: warnings and information, never errors
    let d = rep.diagnostics();
    assert_eq!(d.len(), 3);
    assert!(d.iter().all(|x| x.severity() != Severity::Error));
    assert_eq!(d.iter().filter(|x| x.severity() == Severity::Warning).count(), 2);
}

/// A plate with M4 tapped holes at (5, 5) and (5, 15), and rods that are M4 bolts.
fn tapped() -> (Catalog, BrepExact) {
    let (mut c, e) = shop();
    let axis = |x: f64, y: f64| Ok(Reference::axis(p(x, y, 0.0), forge_math::UnitVec3::Z));
    let plate = c.parts.get_mut(&(DocId("big".into()), 0)).unwrap();
    plate.refs.insert("tap1".into(), axis(5.0, 5.0));
    plate.refs.insert("tap2".into(), axis(5.0, 15.0));
    let m4_hole = Feature {
        kind: FeatureKind::InternalThread,
        size: 4.0,
        depth: Some(10.0),
    };
    plate.features = vec![("tap1".into(), m4_hole.clone()), ("tap2".into(), m4_hole)];
    let rod = c.parts.get_mut(&(DocId("rod".into()), 0)).unwrap();
    rod.refs.insert("shank".into(), axis(0.0, 0.0));
    rod.features = vec![(
        "shank".into(),
        Feature {
            kind: FeatureKind::ExternalThread,
            size: 4.0,
            depth: Some(8.0),
        },
    )];
    (c, e)
}

#[test]
fn fit_rules_cover_bolts_in_matching_holes_and_say_which() {
    let (c, e) = tapped();
    let mut asm = placed(vec![
        of(1, "big", Frame::WORLD),
        of(2, "rod", at(5.0, 5.0, -5.0)),   // in tap1
        of(3, "rod", at(5.0, 15.0, 2.0)),   // in tap2, 8 engaged
        of(4, "rod", at(20.0, 20.0, -5.0)), // not in a hole: still interference
    ]);
    let rule = FitRule {
        id: 7,
        min_engagement: None,
        a: FitMatch::Feature {
            kind: FeatureKind::ExternalThread,
            size: Some(4.0),
        },
        b: FitMatch::Feature {
            kind: FeatureKind::InternalThread,
            size: Some(4.0),
        },
    };
    asm.fit_rules.push(rule.clone());
    let sol = solve(&asm, &c, &Options::default());
    let mut ck = Checker::new(&c, &e);
    let rep = ck.interference(&sol, &[], &asm.fit_rules);
    let p = |i: u64| vec![InstanceId(i)];
    assert_eq!(
        rep.rules,
        vec![RuleUse {
            rule: 7,
            pairs: vec![(p(1), p(2)), (p(1), p(3))],
        }]
    );
    assert_eq!(rep.interferences.len(), 1);
    // the rod in tap1 starts 5 below the plate: 3 of its 8 thread engage, under the shop's one
    // diameter (4); the one in tap2 engages all 8
    assert_eq!(
        rep.shallow,
        vec![ShallowEngagement {
            a: p(1),
            b: p(2),
            rule: 7,
            engaged: 3.0,
            min: 4.0,
        }]
    );
    assert!(rep.diagnostics().iter().any(|d| d.name() == "ShallowEngagement"));
    assert_eq!(
        (rep.interferences[0].a.clone(), rep.interferences[0].b.clone()),
        (p(1), p(4))
    );
    // a size that does not match covers nothing
    let m5 = FitRule {
        id: 8,
        min_engagement: None,
        a: FitMatch::Feature {
            kind: FeatureKind::ExternalThread,
            size: Some(5.0),
        },
        ..rule.clone()
    };
    let rep = Checker::new(&c, &e).interference(&sol, &[], &[m5]);
    assert_eq!(rep.interferences.len(), 3);
    assert!(rep.rules.is_empty());
    // a part rule covers every rod against the plate, wherever it stands
    let parts = FitRule {
        id: 9,
        min_engagement: None,
        a: FitMatch::Part {
            doc: DocId("rod".into()),
            body: None,
        },
        b: FitMatch::Part {
            doc: DocId("big".into()),
            body: None,
        },
    };
    let rep = Checker::new(&c, &e).interference(&sol, &[], &[parts]);
    assert!(rep.interferences.is_empty());
    assert_eq!(rep.rules[0].pairs.len(), 3);
    assert_eq!(rep.exact_runs, 0, "intended contact needs no exact confirmation");
}

#[test]
fn clearance_between_blocks() {
    let (c, e) = shop();
    let asm = placed(vec![
        of(1, "block", Frame::WORLD),
        of(2, "block", at(13.0, 2.0, 0.0)),
    ]);
    let sol = solve(&asm, &c, &Options::default());
    let ck = Checker::new(&c, &e);
    let cl = ck
        .clearance(&sol, &vec![InstanceId(1)], &vec![InstanceId(2)], 100.0)
        .unwrap();
    assert!((cl.distance - 3.0).abs() <= cl.tolerance + 1e-9, "{cl:?}");
    assert_eq!(ck.clearances(&sol, 2.0).len(), 0);
    assert_eq!(ck.clearances(&sol, 4.0).len(), 1);
}

#[test]
fn a_drag_retests_only_what_moved() {
    let (c, e) = shop();
    let mut insts = vec![of(1, "big", at(0.0, 0.0, -40.0))];
    for i in 0..30 {
        insts.push(of(
            2 + i,
            "block",
            at(12.0 * (i % 6) as f64, 12.0 * (i / 6) as f64, 0.0),
        ));
    }
    insts.push(of(40, "arm", at(0.0, 0.0, 30.0)));
    let mut asm = placed(insts);
    // the arm swings on a revolute about a post at the origin, 30 up
    asm.joints.retain(|j| j.id != JointId(40));
    asm.joints.push(
        Joint::new(
            40,
            JointKind::Revolute,
            End::local(&[40], Frame::WORLD),
            End::local(&[1], at(0.0, 0.0, 70.0)),
        )
        .driven(Freedom::Rz, 0.0),
    );
    let mut s = Solver::new(asm, &c, Options::default());
    let mut ck = Checker::new(&c, &e);
    let cold = ck.interference(s.solution(), &[], &[]);
    assert!(cold.candidates > 10, "{cold:?}");
    let sol = s.set_drive(JointId(40), Freedom::Rz, 0.3).clone();
    let warm = ck.interference(&sol, &[], &[]);
    assert!(
        warm.candidates <= 2,
        "only the arm moved: {} candidates",
        warm.candidates
    );
    assert_eq!(warm.interferences, cold.interferences);
    assert_eq!(warm.touching, cold.touching);
}

#[test]
fn a_motion_sweep_finds_where_the_arm_hits() {
    let (c, e) = shop();
    // an arm 30 long on a revolute at the origin; a block 20 out along +y in its path
    let mut asm = placed(vec![
        of(1, "big", at(-50.0, -50.0, -40.0)),
        of(2, "arm", at(0.0, -2.0, 0.0)),
        of(3, "block", at(15.0, 15.0, -3.0)),
    ]);
    asm.joints.retain(|j| j.id != JointId(2));
    asm.joints.push(
        Joint::new(
            2,
            JointKind::Revolute,
            End::local(&[2], at(0.0, 2.0, 0.0)),
            End::local(&[1], at(50.0, 50.0, 40.0)),
        )
        .driven(Freedom::Rz, 0.0),
    );
    let mut s = Solver::new(asm, &c, Options::default());
    let mut ck = Checker::new(&c, &e);
    let watch = vec![(vec![InstanceId(2)], vec![InstanceId(3)])];
    let steps = sweep(
        &mut s,
        &mut ck,
        JointId(2),
        Freedom::Rz,
        (0.0, dmath::PI / 2.0),
        18,
        &watch,
        50.0,
    );
    assert_eq!(steps.len(), 19);
    let hit: Vec<bool> = steps.iter().map(|x| !x.overlaps.is_empty()).collect();
    assert!(!hit[0] && !hit[18] && hit[9], "{hit:?}");
    // clearance shrinks to zero going in
    assert!(steps[0].clearances[0].unwrap() > steps[3].clearances[0].unwrap());
    assert_eq!(steps[9].clearances[0], Some(0.0));
}

#[test]
fn the_mesh_phase_never_misses_a_real_overlap() {
    // random pairs; an exact overlap deeper than the tolerance must be a mesh hit
    let (c, e) = shop();
    let mut r = Rng::new(0x0e7);
    let names = ["block", "rod", "arm"];
    let (mut overlaps, mut hits) = (0, 0);
    for case in 0..300 {
        let (na, nb) = (names[r.below(3)], names[r.below(3)]);
        // half the cases square to the axes on a 1 mm grid: faces and edges that coincide
        let pose = if case % 2 == 0 {
            let g = |r: &mut Rng| (r.range(-12.0, 12.0)).round();
            let turn = [0.0, dmath::FRAC_PI_2, dmath::PI][r.below(3)];
            rz(p(g(&mut r), g(&mut r), g(&mut r)), turn)
        } else {
            let z = v(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
            Frame::from_zx(
                p(r.range(-12.0, 12.0), r.range(-12.0, 12.0), r.range(-12.0, 12.0)),
                z.normalize().unwrap(),
                v(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0)),
            )
            .unwrap()
        };
        let asm = placed(vec![of(1, na, Frame::WORLD), of(2, nb, pose)]);
        let sol = solve(&asm, &c, &Options { threads: 1 });
        let mut ck = Checker::new(&c, &e);
        let rep = ck.interference(&sol, &[], &[]);
        let exact = e.overlap_volume(&PartRef::new(na, 0), &PartRef::new(nb, 0), &pose);
        let Some(vol) = exact else { continue };
        if vol > 0.01 {
            overlaps += 1;
            assert!(
                !rep.interferences.is_empty(),
                "case {case}: {na} vs {nb} share {vol} mm³, mesh missed it"
            );
        }
        if let Some(h) = rep.interferences.first() {
            hits += 1;
            assert!(
                (h.volume.unwrap() - vol).abs() <= 1e-9 * vol.max(1.0),
                "case {case}"
            );
        }
    }
    assert!(overlaps > 40, "{overlaps} overlaps of 300");
    assert!(hits >= overlaps);
}

#[test]
fn bom_rolls_up_through_subassemblies() {
    let mut c = catalog();
    let sub = Assembly {
        instances: vec![
            of(1, "plate", Frame::WORLD),
            of(2, "bolt", Frame::WORLD),
            of(3, "bolt", Frame::WORLD),
            Instance {
                reference: true,
                ..of(4, "envelope", Frame::WORLD)
            },
        ],
        ..Assembly::default()
    };
    c.assemblies.insert(DocId("corner".into()), sub);
    let mut top = vec![of(1, "frame", Frame::WORLD).grounded()];
    for i in 0..4 {
        top.push(Instance {
            of: Component::Assembly(DocId("corner".into())),
            ..of(10 + i, "x", at(i as f64 * 100.0, 0.0, 0.0))
        });
    }
    top.push(Instance {
        suppressed: true,
        ..of(20, "bolt", Frame::WORLD)
    });
    let asm = Assembly {
        instances: top,
        ..Assembly::default()
    };
    let b = bom(&asm, &c);
    let qty = |lines: &[forge_asm::bom::BomLine], n: &str| {
        lines
            .iter()
            .find(|l| matches!(&l.item, BomItem::Part { doc, .. } if doc.0 == n))
            .map_or(0, |l| l.qty)
    };
    assert_eq!(qty(&b.flat, "bolt"), 8);
    assert_eq!(qty(&b.flat, "plate"), 4);
    assert_eq!(qty(&b.flat, "frame"), 1);
    assert_eq!(qty(&b.flat, "envelope"), 0);
    assert_eq!(b.top.len(), 2);
    assert_eq!(qty(&b.subs[&DocId("corner".into())], "bolt"), 2);
}

#[test]
fn subassemblies_place_their_insides_and_joints_reach_into_them() {
    let mut c = catalog();
    let mut info = PartInfo::default();
    info.refs
        .insert("tip".into(), Ok(Reference::frame(at(10.0, 0.0, 0.0))));
    c.parts.insert((DocId("link".into()), 0), info);
    c.parts.insert((DocId("hinge".into()), 0), PartInfo::default());
    // a rigid pair: a link fastened 5 up on a base
    let sub = Assembly {
        instances: vec![
            of(1, "p", Frame::WORLD).grounded(),
            of(2, "link", at(0.0, 0.0, 9.0)),
        ],
        joints: vec![joint(
            1,
            JointKind::Fastened,
            (2, Frame::WORLD),
            (1, at(0.0, 0.0, 5.0)),
        )],
        ..Assembly::default()
    };
    c.assemblies.insert(DocId("pair".into()), sub);
    let top = Assembly {
        instances: vec![
            of(1, "p", Frame::WORLD).grounded(),
            Instance {
                of: Component::Assembly(DocId("pair".into())),
                ..of(2, "x", at(50.0, 0.0, 0.0))
            },
            of(3, "hinge", Frame::WORLD),
        ],
        joints: vec![
            joint(1, JointKind::Fastened, (2, Frame::WORLD), (1, at(50.0, 0.0, 0.0))),
            Joint::new(
                2,
                JointKind::Revolute,
                End::local(&[3], Frame::WORLD),
                End::named(&[2, 2], "tip"),
            )
            .driven(Freedom::Rz, 0.5),
        ],
        ..Assembly::default()
    };
    let sol = solve(&top, &c, &Options::default());
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    let link = sol
        .occurrences
        .iter()
        .find(|o| o.path == vec![InstanceId(2), InstanceId(2)])
        .unwrap();
    assert!(link.pose.origin.distance(p(50.0, 0.0, 5.0)) < 1e-12);
    let hinge = sol.pose(InstanceId(3)).unwrap();
    assert!(hinge.origin.distance(p(60.0, 0.0, 5.0)) < 1e-12);
    let (sn, cs) = dmath::sin_cos(0.5);
    assert!((hinge.x().get() - v(cs, sn, 0.0)).length() < 1e-12);
    // the hashes used: every part, once
    assert_eq!(sol.hashes.len(), 3);
}

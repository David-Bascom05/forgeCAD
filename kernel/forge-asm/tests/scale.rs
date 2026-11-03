//! The 5,000-instance gate (prior-art §11.11, decisions 2026-11-03): plates bolted to a base by
//! named anchors, plus four-bars and slider-cranks. Targets, best of three runs:
//! anchor check < 10 ms; overconstraint re-check after one joint edit < 10 ms; overlap check cold
//! < 1 s; overlap after a drag < 50 ms. A drag re-solves only its partition. Native only: the
//! targets are for the native build.
#![cfg(not(target_family = "wasm"))]

mod common;
use common::*;
use forge_asm::check::*;
use forge_asm::*;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{Profile, extrude};
use std::sync::Arc;
use std::time::{Duration, Instant};

const PLATES: u64 = 230;
const BOLTS: u64 = 20;
const MECHS: u64 = 50;

fn body(lp: forge_ops::ProfileLoop, z: f64, h: f64) -> forge_topo::Body {
    extrude(
        &Profile {
            plane: at(0.0, 0.0, z),
            loops: vec![lp],
        },
        h,
        1,
    )
    .unwrap()
}

fn add(c: &mut Catalog, e: &mut BrepExact, name: &str, b: forge_topo::Body, refs: Vec<(String, Reference)>) {
    let q = forge_mesh::Quality {
        chord: 0.01,
        max_angle: dmath::PI / 12.0,
    };
    let m = forge_mesh::tessellate(&b, &q).unwrap();
    let mut info = PartInfo {
        mesh: Some(Arc::new(PartMesh::from_trimesh(&m, 0.01))),
        ..PartInfo::default()
    };
    for (n, r) in refs {
        info.refs.insert(n, Ok(r));
    }
    c.parts.insert((DocId(name.into()), 0), info);
    e.bodies.insert((DocId(name.into()), 0), b);
}

fn seat(k: u64) -> Point2 {
    Point2::new(10.0 + 20.0 * (k % 5) as f64, 10.0 + 25.0 * (k / 5) as f64)
}

fn project() -> (Catalog, BrepExact) {
    let mut c = Catalog::default();
    let mut e = BrepExact::default();
    add(
        &mut c,
        &mut e,
        "base",
        body(
            Profile::rectangle(Point2::new(0.0, 0.0), 10.0, 10.0, 1),
            -10.0,
            10.0,
        ),
        vec![],
    );
    let seats = (0..BOLTS)
        .map(|k| {
            (
                format!("seat{k}"),
                Reference::frame(at(seat(k).x, seat(k).y, 5.0)),
            )
        })
        .chain([("corner".to_string(), Reference::frame(Frame::WORLD))])
        .collect();
    add(
        &mut c,
        &mut e,
        "plate",
        body(
            Profile::rectangle(Point2::new(0.0, 0.0), 100.0, 100.0, 1),
            0.0,
            5.0,
        ),
        seats,
    );
    add(
        &mut c,
        &mut e,
        "bolt",
        // an M6 bolt: its shank goes 5 down into the plate's tapped seat
        body(Profile::circle(Point2::new(0.0, 0.0), 3.0, 1), -5.0, 17.0),
        vec![("base".into(), Reference::frame(Frame::WORLD))],
    );
    let m6 = |kind| Feature::new(kind, 6.0);
    c.parts.get_mut(&(DocId("bolt".into()), 0)).unwrap().features =
        vec![("base".into(), m6(FeatureKind::ExternalThread))];
    c.parts.get_mut(&(DocId("plate".into()), 0)).unwrap().features = (0..BOLTS)
        .map(|k| (format!("seat{k}"), m6(FeatureKind::InternalThread)))
        .collect();
    for (i, z) in [0.0, 5.0, 10.0].into_iter().enumerate() {
        add(
            &mut c,
            &mut e,
            &format!("link{i}"),
            body(Profile::rectangle(Point2::new(-2.0, -2.0), 4.0, 4.0, 1), z, 4.0),
            vec![],
        );
    }
    (c, e)
}

/// The assembly, and the joint ids of each four-bar's crank.
fn assembly() -> (Assembly, Vec<JointId>) {
    let mut asm = Assembly::default();
    // one rule instead of 4,600 fit declarations
    asm.fit_rules.push(FitRule {
        within: vec![],
        id: 1,
        min_engagement: None,
        a: FitMatch::Feature {
            kind: FeatureKind::ExternalThread,
            size: Some(6.0),
        },
        b: FitMatch::Feature {
            kind: FeatureKind::InternalThread,
            size: Some(6.0),
        },
    });
    asm.instances
        .push(Instance::part(1, PartRef::new("base", 0), at(-50.0, -50.0, 0.0)).grounded());
    let mut next = 2u64;
    let mut jid = 1u64;
    for pl in 0..PLATES {
        let corner = at(110.0 * (pl % 15) as f64, 110.0 * (pl / 15) as f64, 0.0);
        let plate = next;
        next += 1;
        asm.instances
            .push(Instance::part(plate, PartRef::new("plate", 0), corner));
        if pl == 0 {
            // a sliding table: the first plate and its bolts move along x
            let along = frame(p(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0));
            asm.joints.push(
                Joint::new(
                    jid,
                    JointKind::Slider,
                    End::local(&[plate], along),
                    End::local(&[1], at(50.0, 50.0, 0.0).compose(&corner).compose(&along)),
                )
                .driven(Freedom::Tz, 0.0),
            );
        } else {
            asm.joints.push(Joint::new(
                jid,
                JointKind::Fastened,
                End::named(&[plate], "corner"),
                End::local(&[1], at(50.0, 50.0, 0.0).compose(&corner)),
            ));
        }
        jid += 1;
        for k in 0..BOLTS {
            let b = next;
            next += 1;
            let s = seat(k);
            asm.instances.push(Instance::part(
                b,
                PartRef::new("bolt", 0),
                corner.compose(&at(s.x, s.y, 5.0)),
            ));
            asm.joints.push(Joint::new(
                jid,
                JointKind::Fastened,
                End::named(&[b], "base"),
                End::named(&[plate], &format!("seat{k}")),
            ));
            jid += 1;
        }
    }
    let mut cranks = Vec::new();
    for m in 0..MECHS {
        let o = at(-400.0 - 300.0 * (m % 5) as f64, 300.0 * (m / 5) as f64, 0.0);
        // a four-bar: crank, coupler, rocker
        let fb = four_bar(40.0, 120.0, 80.0, 100.0, 0.6);
        let map = |i: u64| if i == 1 { 1 } else { next + i - 2 };
        for (i, inst) in fb.instances.iter().enumerate().skip(1) {
            asm.instances.push(Instance::part(
                map(i as u64 + 1),
                PartRef::new(&format!("link{}", i - 1), 0),
                o.compose(&inst.pose),
            ));
        }
        for j in &fb.joints {
            let fix = |e: &End| {
                let AnchorRef::Local(r) = &e.anchor else {
                    unreachable!()
                };
                let id = e.path[0].0;
                End::local(
                    &[map(id)],
                    if id == 1 {
                        at(50.0, 50.0, 0.0).compose(&o).compose(&r.frame)
                    } else {
                        r.frame
                    },
                )
            };
            let mut nj = Joint::new(jid, j.kind, fix(&j.a), fix(&j.b));
            if j.id == JointId(1) {
                nj = nj.driven(Freedom::Rz, 0.6);
                cranks.push(JointId(jid));
            }
            asm.joints.push(nj);
            jid += 1;
        }
        next += 3;
        // a slider-crank beside it
        let sc = slider_crank(30.0, 100.0, 0.4);
        let o2 = o.compose(&at(0.0, 150.0, 0.0));
        let map = |i: u64| if i == 1 { 1 } else { next + i - 2 };
        for (i, inst) in sc.instances.iter().enumerate().skip(1) {
            asm.instances.push(Instance::part(
                map(i as u64 + 1),
                PartRef::new(&format!("link{}", i - 1), 0),
                o2.compose(&inst.pose),
            ));
        }
        for j in &sc.joints {
            let fix = |e: &End| {
                let AnchorRef::Local(r) = &e.anchor else {
                    unreachable!()
                };
                let id = e.path[0].0;
                End::local(
                    &[map(id)],
                    if id == 1 {
                        at(50.0, 50.0, 0.0).compose(&o2).compose(&r.frame)
                    } else {
                        r.frame
                    },
                )
            };
            asm.joints.push(Joint::new(jid, j.kind, fix(&j.a), fix(&j.b)));
            jid += 1;
        }
        next += 3;
    }
    (asm, cranks)
}

fn best<T>(n: usize, mut f: impl FnMut() -> T) -> (Duration, T) {
    let mut out = None;
    let mut t = Duration::MAX;
    for _ in 0..n {
        let s = Instant::now();
        let r = f();
        t = t.min(s.elapsed());
        out = Some(r);
    }
    (t, out.unwrap())
}

#[test]
fn five_thousand_instances_meet_the_targets() {
    let (c, e) = project();
    let (asm, cranks) = assembly();
    assert!(asm.instances.len() >= 5000, "{} instances", asm.instances.len());

    let (t_anchor, broken) = best(3, || check_anchors(&asm, &c));
    assert!(broken.is_empty(), "{broken:?}");

    let opts = Options::default();
    let (t_cold, mut s) = best(3, || Solver::new(asm.clone(), &c, opts));
    let sol = s.solution().clone();
    assert_eq!(
        sol.errors().count(),
        0,
        "{:?}",
        sol.errors().take(3).collect::<Vec<_>>()
    );
    // each four-bar and slider-crank: 3 redundant rows; the crank drive leaves the four-bars still
    assert_eq!(redundant_rows(&sol), 6 * MECHS as usize);
    assert_eq!(sol.mobility, MECHS as usize);
    assert_eq!(s.assembly().joints[0].kind, JointKind::Slider);

    // one joint edit: the crank's drive, then its limits
    let mut v = 0.6;
    let (t_edit, edit) = best(3, || {
        v += 0.05;
        s.set_drive(cranks[7], Freedom::Rz, v).clone()
    });
    assert_eq!(edit.stats.partitions_solved, 1, "{:?}", edit.stats);
    assert_eq!(edit.stats.blocks_solved, 1);
    assert_eq!(redundant_rows(&edit), 6 * MECHS as usize);
    let mut j = s
        .assembly()
        .joints
        .iter()
        .find(|j| j.id == cranks[7])
        .unwrap()
        .clone();
    j.limits = vec![(Freedom::Rz, -3.0, 3.0)];
    let (t_limit, _) = best(1, || s.edit_joint(j.clone()).stats);

    let mut ck = Checker::new(&c, &e);
    let (t_overlap, rep) = best(3, || {
        let mut fresh = Checker::new(&c, &e);
        fresh.interference(s.solution(), &[], &asm.fit_rules)
    });
    assert_eq!(
        rep.interferences.len(),
        0,
        "{:?}",
        rep.interferences.iter().take(3).collect::<Vec<_>>()
    );
    ck.interference(s.solution(), &[], &asm.fit_rules);
    // drag the table (21 instances among neighbours), solve and re-check overlap
    let mut x = 0.0;
    let (t_drag, drag) = best(3, || {
        x += 1.0;
        let sol = s.set_drive(JointId(1), Freedom::Tz, x).clone();
        assert_eq!(sol.stats.partitions_solved, 1);
        ck.interference(&sol, &[], &asm.fit_rules)
    });
    assert!(
        drag.candidates >= 20 && drag.candidates < 40,
        "{} candidates after a drag",
        drag.candidates
    );
    assert!(drag.interferences.is_empty());
    // every bolt is intended contact, by the one rule, and none needed an exact boolean
    assert_eq!(rep.rules.len(), 1);
    assert_eq!(rep.rules[0].pairs.len(), (PLATES * BOLTS) as usize);
    assert_eq!(rep.exact_runs, 0);
    assert_eq!(drag.rules[0].pairs.len(), (PLATES * BOLTS) as usize);

    println!(
        "instances {} joints {}: anchors {:?}, cold solve {:?}, edit re-check {:?}, limit edit {:?}, overlap cold {:?} ({} candidates, {} exact), overlap after drag {:?} ({} candidates, {} cached)",
        asm.instances.len(),
        asm.joints.len(),
        t_anchor,
        t_cold,
        t_edit,
        t_limit,
        t_overlap,
        rep.candidates,
        rep.exact_runs,
        t_drag,
        drag.candidates,
        drag.cache_hits
    );
    assert!(t_anchor < Duration::from_millis(10), "anchor check {t_anchor:?}");
    assert!(t_edit < Duration::from_millis(10), "edit re-check {t_edit:?}");
    assert!(t_overlap < Duration::from_secs(1), "overlap cold {t_overlap:?}");
    assert!(
        t_drag < Duration::from_millis(50),
        "overlap after drag {t_drag:?}"
    );
}

//! STEP export of the assembly structure, read back (prior-art §11.9): one product per part and
//! per assembly document, one next_assembly_usage_occurrence per use, and the kernel's own reader
//! placing every use where the solve put it.

mod common;
use common::*;
use forge_asm::*;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{Profile, extrude};
use std::collections::BTreeMap;

fn solid(lp: forge_ops::ProfileLoop, h: f64) -> forge_topo::Body {
    extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![lp],
        },
        h,
        1,
    )
    .unwrap()
}

fn of(id: u32, name: &str, pose: Frame) -> Instance {
    Instance::part(id, PartRef::new(name, 0), pose)
}

fn sub(id: u32, doc: &str, pose: Frame) -> Instance {
    Instance {
        of: Component::Assembly(DocId(doc.into())),
        ..of(id, "x", pose)
    }
}

fn fasten(id: u32, child: u32, to: u32, at: Frame) -> Joint {
    joint(id, JointKind::Fastened, (child, Frame::WORLD), (to, at))
}

fn project() -> (Catalog, BTreeMap<(DocId, u64), forge_topo::Body>, Assembly) {
    let mut c = Catalog::default();
    let mut bodies = BTreeMap::new();
    for (n, b) in [
        (
            "block",
            solid(Profile::rectangle(Point2::new(0.0, 0.0), 10.0, 20.0, 1), 5.0),
        ),
        ("rod", solid(Profile::circle(Point2::new(0.0, 0.0), 2.0, 1), 15.0)),
    ] {
        c.parts.insert((DocId(n.into()), 0), PartInfo::default());
        bodies.insert((DocId(n.into()), 0), b);
    }
    // a subassembly: a block with a rod standing on it
    c.assemblies.insert(
        DocId("post".into()),
        Assembly {
            instances: vec![
                of(1, "block", Frame::WORLD).grounded(),
                of(2, "rod", Frame::WORLD),
            ],
            joints: vec![fasten(1, 2, 1, at(5.0, 10.0, 5.0))],
            ..Assembly::default()
        },
    );
    let quarter = rz(p(30.0, 0.0, 0.0), dmath::FRAC_PI_2);
    let asm = Assembly {
        instances: vec![
            of(1, "block", Frame::WORLD).grounded(),
            of(2, "block", Frame::WORLD),
            sub(3, "post", Frame::WORLD),
            sub(4, "post", Frame::WORLD),
            Instance {
                reference: true,
                ..of(5, "block", at(0.0, 0.0, -100.0))
            },
            Instance {
                suppressed: true,
                ..of(6, "rod", Frame::WORLD)
            },
        ],
        joints: vec![
            fasten(1, 2, 1, quarter),
            fasten(2, 3, 1, at(0.0, 50.0, 0.0)),
            fasten(3, 4, 1, rz(p(60.0, 50.0, 5.0), 0.3)),
        ],
        ..Assembly::default()
    };
    (c, bodies, asm)
}

fn count(text: &str, entity: &str) -> usize {
    text.lines()
        .filter(|l| l.contains(&format!("={entity}(")))
        .count()
}

/// Each solid's lowest corner and volume, sorted.
fn corners(bodies: impl Iterator<Item = forge_topo::Body>) -> Vec<[f64; 4]> {
    let mut out: Vec<[f64; 4]> = bodies
        .map(|b| {
            let lo = |k: usize| {
                b.vertices
                    .iter()
                    .map(|v| [v.point.x, v.point.y, v.point.z][k])
                    .fold(f64::INFINITY, f64::min)
            };
            let vol = forge_topo::mass::mass(&b).unwrap().volume;
            [lo(0), lo(1), lo(2), vol].map(|x| (x * 1e6).round() / 1e6)
        })
        .collect();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out
}

#[test]
fn the_structure_reads_back_where_the_solve_put_it() {
    let (c, bodies, asm) = project();
    let sol = solve(&asm, &c, &Options::default());
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    let text = step::export(&asm, "top", &c, &bodies).unwrap();
    assert!(forge_step::part21::parse(text.as_bytes(), &forge_step::Limits::default()).is_ok());
    // 2 parts and 2 assembly documents; 4 uses at the top (not the reference or the suppressed
    // one) and 2 in the subassembly, written once however often it is used
    assert_eq!(count(&text, "PRODUCT"), 4);
    assert_eq!(count(&text, "ADVANCED_BREP_SHAPE_REPRESENTATION"), 2);
    assert_eq!(count(&text, "NEXT_ASSEMBLY_USAGE_OCCURRENCE"), 6);
    assert_eq!(count(&text, "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"), 6);

    let got = forge_step::import(text.as_bytes(), &forge_step::Limits::default()).expect("reads");
    if got.stats.assembly == 0 {
        // a reader that places assemblies (step-asm): every use of a part where the solve put it
        let want = corners(
            sol.occurrences
                .iter()
                .filter(|o| !o.reference)
                .map(|o| forge_asm::check::rigid_copy(&bodies[&(o.part.doc.clone(), o.part.body)], &o.pose)),
        );
        assert_eq!(want.len(), 6);
        assert_eq!(corners(got.solids.into_iter().map(|s| s.body)), want);
    } else {
        // a reader that leaves assemblies alone says so, and still gives each part
        assert_eq!(got.solids.len(), 2);
    }
}

#[test]
fn a_part_with_no_body_is_named() {
    let (c, mut bodies, asm) = project();
    bodies.remove(&(DocId("rod".into()), 0));
    assert_eq!(
        step::export(&asm, "top", &c, &bodies),
        Err(step::StepError::NoBody {
            doc: DocId("rod".into()),
            body: 0
        })
    );
}

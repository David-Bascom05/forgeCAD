//! The quick tier of the properties (the deep one is `examples/meta.rs`): small fuzz histories
//! and a few suite parts, each property on each.
use forge_model::{Part, suite};
use forge_proof::props;

fn fuzz() -> Vec<(String, Part)> {
    (0..4)
        .map(|s| (format!("fuzz{s}"), suite::fuzz(s + 1000, 6)))
        .collect()
}

fn holds(prop: &str, f: props::Property, parts: &[(String, Part)]) {
    for (i, (name, part)) in parts.iter().enumerate() {
        let seed = i as u64 + 1;
        if let Some(hit) = f(part, seed) {
            panic!(
                "{prop} fails on {name} (seed {seed}): {}: {}",
                hit.kind, hit.detail
            );
        }
    }
}

#[test]
fn scaled_by_powers_of_two() {
    let mut parts = fuzz();
    parts.push(("shaft".into(), suite::shaft()));
    holds("scale", props::scale, &parts);
}

#[test]
fn neighbouring_tools_commute() {
    holds("order", props::order, &fuzz());
}

#[test]
fn boolean_identities() {
    holds("boolean", props::identities, &fuzz());
}

#[test]
fn certified() {
    let mut parts = fuzz();
    parts.push(("housing".into(), suite::housing()));
    holds("certify", props::certificate, &parts);
}

#[test]
fn brep_agrees_with_csg_point_by_point() {
    holds("membership", props::membership, &fuzz());
}

fn block(x: f64, y: f64, z: f64) -> forge_topo::Body {
    let part = Part {
        features: vec![forge_model::Feature::Extrude {
            profile: forge_ops::Profile {
                plane: forge_math::Frame::WORLD.with_origin(forge_math::Point3::new(x, y, z)),
                loops: vec![forge_ops::Profile::rectangle(
                    forge_math::Point2::ORIGIN,
                    10.0,
                    10.0,
                    0,
                )],
            },
            extent: forge_model::Extent::Distance(10.0),
            op: forge_model::Combine::New,
        }],
    };
    let mut b = part.regenerate().expect("a block");
    for f in &mut b.faces {
        f.name = forge_topo::Name::NONE;
    }
    for e in &mut b.edges {
        e.name = forge_topo::Name::NONE;
    }
    for v in &mut b.vertices {
        v.name = forge_topo::Name::NONE;
    }
    b
}

fn kinds(b: &forge_topo::Body) -> Vec<String> {
    forge_proof::cert::certify(b, 0.05)
        .iter()
        .map(|f| f.kind())
        .collect()
}

/// A block turned inside out (every face's normal and loop reversed) passes `check`, whose
/// probes are local, but not ray parity.
#[test]
fn certify_catches_an_inside_out_solid() {
    let mut b = block(0.0, 0.0, 0.0);
    assert!(kinds(&b).is_empty(), "{:?}", kinds(&b));
    for f in &mut b.faces {
        f.same_sense = !f.same_sense;
        f.query.clear();
    }
    for l in &mut b.loops {
        l.coedges.reverse();
    }
    for c in &mut b.coedges {
        c.forward = !c.forward;
    }
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    assert!(
        kinds(&b).iter().all(|k| k == "Inverted") && kinds(&b).len() == 6,
        "{:?}",
        kinds(&b)
    );
}

/// Two overlapping blocks in one body: each shell is closed and valid, but their faces cross.
#[test]
fn certify_catches_crossing_shells() {
    let mut b = block(0.0, 0.0, 0.0);
    b.append(&block(4.3, 3.7, 5.9));
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    let k = kinds(&b);
    assert!(
        k.iter().any(|k| k == "Crossing") && k.iter().any(|k| k == "NotBounding"),
        "{k:?}"
    );
}

/// Two blocks sharing one corner vertex: every edge has two faces, but the vertex has two fans.
#[test]
fn certify_catches_a_pinched_vertex() {
    let mut b = block(0.0, 0.0, 0.0);
    let n = b.vertices.len() as u32;
    b.append(&block(10.0, 10.0, 10.0));
    let corner = |b: &forge_topo::Body, p: forge_math::Point3| {
        (0..b.vertices.len() as u32).find(|&v| b.vertices[v as usize].point.distance(p) < 1e-9)
    };
    let p = forge_math::Point3::new(10.0, 10.0, 10.0);
    let keep = corner(&b, p).expect("the first block's corner");
    assert!(keep < n);
    let gone = (n..b.vertices.len() as u32)
        .find(|&v| b.vertices[v as usize].point.distance(p) < 1e-9)
        .expect("the second block's corner");
    for e in &mut b.edges {
        for end in [&mut e.start, &mut e.end] {
            if end.0 == gone {
                *end = forge_topo::VertexId(keep);
            }
        }
    }
    let probs = forge_topo::certify(&b);
    assert!(
        probs
            .iter()
            .any(|p| matches!(p, forge_topo::Problem::VertexFans { fans: 2, .. })),
        "{probs:?}"
    );
}

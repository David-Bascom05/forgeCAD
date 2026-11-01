//! Nothing a feature makes may follow the order in a body's arenas, to the bit: each feature on its
//! stage against the same stage with its arenas reversed.
use forge_agent::bench;
use forge_model::{History, Solid, step_solids};

/// The first name or point that tells two bodies apart.
fn differs(a: &forge_topo::Body, b: &forge_topo::Body) -> Option<String> {
    use std::collections::BTreeMap;
    let names = |b: &forge_topo::Body| {
        let mut m = BTreeMap::new();
        for f in b.face_ids() {
            m.insert(('f', b.face(f).name), None);
        }
        for e in b.edge_ids() {
            m.insert(('e', b.edge(e).name), None);
            for v in [b.edge(e).start, b.edge(e).end] {
                m.insert(('v', b.vertex(v).name), Some(b.vertex(v).point));
            }
        }
        m
    };
    let (x, y) = (names(a), names(b));
    for (k, p) in &x {
        match y.get(k) {
            None => {
                let there = p.and_then(|p| {
                    y.iter()
                        .find(|(_, q)| q.is_some_and(|q| (q - p).length() < 1e-6))
                        .map(|(n, _)| n.1)
                });
                return Some(format!(
                    "{k:?} at {p:?} only on the stage as it is; reversed there: {there:?}"
                ));
            }
            Some(q) => {
                if let (Some(p), Some(q)) = (p, q)
                    && p != q
                {
                    return Some(format!("{k:?} at {p:?} and {q:?}"));
                }
            }
        }
    }
    y.keys()
        .find(|k| !x.contains_key(k))
        .map(|k| format!("{k:?} only on the reversed stage"))
}

fn reversed(b: &forge_topo::Body) -> forge_topo::Body {
    use forge_topo::{Body, EdgeId};
    let mut o = Body::new();
    let shell = o.add_shell();
    let mut vs = std::collections::HashMap::new();
    for v in b.vertex_ids().collect::<Vec<_>>().into_iter().rev() {
        let x = b.vertex(v);
        vs.insert(v, o.add_tolerant_vertex(x.point, x.tol, x.name));
    }
    let mut es: std::collections::HashMap<EdgeId, EdgeId> = std::collections::HashMap::new();
    for e in b.edge_ids().collect::<Vec<_>>().into_iter().rev() {
        let x = b.edge(e);
        es.insert(
            e,
            o.add_edge(x.curve.clone(), x.range, vs[&x.start], vs[&x.end], x.name),
        );
    }
    for f in b.face_ids().collect::<Vec<_>>().into_iter().rev() {
        let x = b.face(f);
        let nf = o.add_face(shell, x.surface.clone(), x.same_sense, x.name);
        for &l in &x.loops {
            let uses: Vec<(EdgeId, bool)> = b
                .lp(l)
                .coedges()
                .map(|c| (es[&b.coedge(c).edge], b.coedge(c).forward))
                .collect();
            o.add_loop(nf, &uses);
        }
    }
    o.joins = b.joins.clone();
    o.split = b.split.clone();
    o.rebuild_shells();
    o
}

#[test]
fn names_do_not_follow_arena_order() {
    for part in bench::NAMES {
        let m = bench::model(bench::src(part).expect("a bench part"));
        let ids: Vec<u64> = m.lines.iter().map(|l| l.id).collect();
        for i in 0..m.built() {
            let h = History::new(&m.features[..=i], Some(&ids[..=i]));
            let before: Vec<Solid> = m.solids_after(i).to_vec();
            let flipped: Vec<Solid> = before
                .iter()
                .map(|s| Solid {
                    id: s.id,
                    body: reversed(&s.body),
                })
                .collect();
            let (x, y) = (step_solids(h, i, before), step_solids(h, i, flipped));
            let (x, y) = (x.expect("it builds"), y.expect("it builds reversed"));
            if let Some(why) = x.iter().zip(&y).find_map(|(p, q)| differs(&p.body, &q.body)) {
                panic!("{part} line {i} {}: {why}", m.kinds[i]);
            }
        }
    }
}

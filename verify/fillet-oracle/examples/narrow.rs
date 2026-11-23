//! Fillets whose radius exceeds a support face's width: narrow OUT.jsonl FILE.fg.. | LIST.txt
//!
//! For each fillet feature, each asked-for line edge and each plane face beside it, the face's width
//! across from the edge (the farthest of its vertices from the edge's line). One JSON line a fillet
//! with at least one edge whose radius exceeds a width: the edges, faces and widths, and whether
//! the kernel built the fillet. For finding parts like a block's top left narrower than r by an
//! earlier round (research/fillets/census.md).
use forge_geom::{Curve, Surface};
use forge_model::Feature;
use forge_topo::Body;

fn faces_of(b: &Body, e: forge_topo::EdgeId) -> Vec<forge_topo::FaceId> {
    b.edge(e).coedges.iter().map(|&c| b.coedge_face(c)).collect()
}

fn one(path: &str) -> Vec<String> {
    let id = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string();
    let Ok(src) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let m = forge_lang::fuzz::build(&src);
    let mut out = Vec::new();
    for (k, f) in m.features.iter().enumerate() {
        let Feature::Fillet { edges, .. } = f else { continue };
        let Some(s) = m.body_after(k) else { break };
        let built = k < m.built();
        let mut hits = Vec::new();
        for e in s.edge_ids() {
            let Some((_, rad)) = edges.iter().find(|(n, _)| *n == s.edge(e).name) else {
                continue;
            };
            let Some(r) = rad.constant() else { continue };
            let Curve::Line(line) = &s.edge(e).curve else { continue };
            for g in faces_of(&s, e) {
                if !matches!(s.face(g).surface, Surface::Plane(_)) {
                    continue;
                }
                let mut w: f64 = 0.0;
                for &lp in &s.face(g).loops {
                    for c in s.lp(lp).coedges() {
                        let x = s.edge(s.coedge(c).edge);
                        for v in [x.start, x.end] {
                            let p = s.vertex(v).point;
                            let q = p - line.origin;
                            let d = (q - line.dir.get() * q.dot(line.dir.get())).length();
                            w = w.max(d);
                        }
                    }
                }
                if w > 0.0 && r > w + 1e-9 {
                    hits.push(format!("[{},{},{r},{w:.4}]", e.index(), g.index()));
                }
            }
        }
        if !hits.is_empty() {
            out.push(format!(
                "{{\"id\":\"{id}\",\"feature\":{k},\"built\":{built},\"narrow\":[{}]}}",
                hits.join(",")
            ));
        }
        if !built {
            break;
        }
    }
    out
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let files: Vec<String> = if a.len() == 3 && a[2].ends_with(".txt") {
        std::fs::read_to_string(&a[2]).expect("a list").lines().map(String::from).collect()
    } else {
        a[2..].to_vec()
    };
    let mut rows = Vec::new();
    for f in &files {
        let r = std::panic::catch_unwind(|| one(f)).unwrap_or_default();
        for l in &r {
            println!("{l}");
        }
        rows.extend(r);
    }
    std::fs::write(&a[1], rows.join("\n") + "\n").expect("write");
}

//! One part file's fate as one JSON line, with every fillet audited: filletcheck FILE.fc
//! Run per process (research/fillets/run.py), so a hang or an abort costs one part.
use forge_agent::model::Model;
use forge_model::{Feature, Part};
use forge_proof::blends;

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '"' && *c != '\\' && (*c as u32) >= 32)
        .collect()
}

fn main() {
    let file = std::env::args().nth(1).expect("a part file");
    let src = std::fs::read_to_string(&file).expect("readable");
    std::panic::set_hook(Box::new(|i| eprintln!("{i}")));
    let m = Model::new(&src);
    let mut audits = Vec::new();
    for k in 0..m.features.len().min(m.built()) {
        let Feature::Fillet { edges, .. } = &m.features[k] else {
            continue;
        };
        let (Some(a), Some(b)) = (m.body_after(k), m.body_after(k + 1)) else {
            continue;
        };
        let radii = blends::Radii::of(edges.iter().map(|e| &e.1));
        let (lo, hi) = (0, 0);
        let x = blends::audit(a, b, &radii, k as u64 + 1);
        // bodies held as one overlap: points say nothing there
        let y = if m.solids_after(k).len() == 1 && m.solids_after(k + 1).len() == 1 {
            blends::points(a, b, 400, k as u64 + 1)
        } else {
            blends::Points::default()
        };
        let (off, pk) = blends::patch_seams(a, b);
        let needless = blends::needless_patches(a, b).len();
        audits.push(format!(
            "{{\"f\":{k},\"r\":[{lo},{hi}],\"faces\":{},\"pts\":{},\"gap\":{:e},\"unsup\":{},\"kink\":{:e},\"at\":\"{:?}\",\"kink_at\":\"{:?}\",\"tried\":{},\"wrong\":{},\"wrong_at\":\"{:?}\",\"patch_off\":{off:e},\"patch_kink\":{pk:e},\"needless\":{needless}}}",
            x.faces, x.points, x.worst_gap, x.unsupported, x.worst_kink, x.worst_at, x.kink_at, y.tried, y.wrong, y.worst
        ));
    }
    let part = Part {
        features: m.features.clone(),
    };
    if std::env::var_os("FILLETDUMP").is_some() {
        for sol in m.solids_after(m.built()) {
            let b = &sol.body;
            for p in forge_topo::check(b) {
                eprintln!("{p:?}");
                if let forge_topo::Problem::DuplicateName { name } = p {
                    for v in b.vertex_ids().filter(|&v| b.vertex(v).name == name) {
                        eprintln!("  vertex {:?}", b.vertex(v).point);
                    }
                    for e in b.edge_ids().filter(|&e| b.edge(e).name == name) {
                        use forge_geom::CurveGeom;
                        let x = b.edge(e);
                        eprintln!(
                            "  edge {:?} {:?}",
                            x.curve.eval(x.range.lo),
                            x.curve.eval(x.range.hi)
                        );
                    }
                    for f in b.face_ids().filter(|&f| b.face(f).name == name) {
                        eprintln!("  face {:?}", b.face_box(f));
                    }
                }
            }
        }
    }
    let problems: usize = m
        .solids_after(m.built())
        .iter()
        .map(|s| forge_topo::check(&s.body).len())
        .sum();
    let nary = if m.error.is_none() {
        match part.regenerate_nary(2) {
            Ok(b) => forge_topo::check(&b).len().to_string(),
            Err(e) => format!("\"{}\"", esc(&format!("{e:?}"))),
        }
    } else {
        "null".into()
    };
    let (sig, line) = m
        .error
        .as_ref()
        .map_or((String::new(), 0), |f| (f.msg.clone(), m.lines[f.at].no));
    if m.error.is_none() && problems == 0 {
        let mut m = m;
        let bb = m.stats().2;
        let (mut vol, mut area) = (0.0, 0.0);
        let mut massed = true;
        for s in m.solids_after(m.built()) {
            match forge_topo::mass::mass(&s.body) {
                Ok(p) => {
                    vol += p.volume;
                    area += p.area;
                }
                Err(_) => massed = false,
            }
        }
        println!(
            "{{\"v\":\"ok\",\"massed\":{massed},\"vol\":{vol},\"area\":{area},\"bbox\":[{},{},{},{},{},{}],\"problems\":0,\"nary\":{nary},\"audits\":[{}]}}",
            bb.x.lo,
            bb.y.lo,
            bb.z.lo,
            bb.x.hi,
            bb.y.hi,
            bb.z.hi,
            audits.join(",")
        );
        return;
    }
    println!(
        "{{\"v\":\"{}\",\"sig\":\"{}\",\"line\":{line},\"problems\":{problems},\"audits\":[{}]}}",
        if m.error.is_some() { "refused" } else { "invalid" },
        esc(&sig),
        audits.join(",")
    );
}

//! One STEP file from stdin or a path: what the reader makes of it, as one JSON line, for
//! research/step/rate.py. `--no-check` and `--no-mass` skip the slower measurements.
use forge_mesh::{Quality, tessellate};
use forge_topo::{check, mass};
use std::collections::BTreeMap;
use std::io::Read;

fn esc(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_graphic() || *c == ' ').map(|c| if c == '"' || c == '\\' { '_' } else { c }).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let mut text = Vec::new();
    match args.iter().find(|a| !a.starts_with("--")) {
        Some(p) => text = std::fs::read(p).expect("read"),
        None => {
            std::io::stdin().read_to_end(&mut text).expect("stdin");
        }
    }
    let t = std::time::Instant::now();
    let imp = match forge_step::import(&text, &forge_step::Limits::default()) {
        Ok(i) => i,
        Err(e) => {
            println!("{{\"err\":\"{}\"}}", esc(&e.to_string()));
            return;
        }
    };
    let read_ms = t.elapsed().as_secs_f64() * 1e3;
    let mut solids = Vec::new();
    for s in &imp.solids {
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        let mut first = String::new();
        if !flag("--no-check") {
            for p in check(&s.body) {
                let d = format!("{p:?}");
                if first.is_empty() {
                    first = d.clone();
                }
                let k = d.split([' ', '{']).next().unwrap_or("").to_string();
                *kinds.entry(k).or_default() += 1;
            }
        }
        let kj: Vec<String> = kinds.iter().map(|(k, n)| format!("\"{k}\":{n}")).collect();
        let (mut vol, mut area, mut merr) = (f64::NAN, f64::NAN, String::new());
        if !flag("--no-mass") && !kinds.contains_key("FaceNoLoops") {
            match mass::mass_rel(&s.body, 1e-6) {
                Ok(m) => {
                    vol = m.volume;
                    area = m.area;
                }
                Err(e) => merr = e.to_string(),
            }
        }
        let num = |x: f64| if x.is_finite() { format!("{x}") } else { "null".to_string() };
        let mut bx = "null".to_string();
        if !flag("--no-box") {
            if let Ok(m) = tessellate(&s.body, &Quality { chord: 0.05, ..Quality::default() }) {
                let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
                for p in &m.positions {
                    for (k, c) in [p.x, p.y, p.z].into_iter().enumerate() {
                        lo[k] = lo[k].min(c);
                        hi[k] = hi[k].max(c);
                    }
                }
                bx = format!("[{},{},{},{},{},{}]", lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
            }
        }
        solids.push(format!(
            "{{\"id\":{},\"faces\":{},\"edges\":{},\"problems\":{{{}}},\"first\":\"{}\",\"vol\":{},\"area\":{},\"merr\":\"{}\",\"box\":{}}}",
            s.entity, s.body.faces.len(), s.body.edges.len(), kj.join(","), esc(&first).chars().take(120).collect::<String>(), num(vol), num(area), esc(&merr), bx
        ));
    }
    let mut refs: BTreeMap<String, usize> = BTreeMap::new();
    let mut why = String::new();
    for r in &imp.refusals {
        *refs.entry(esc(&r.kind)).or_default() += 1;
        if why.is_empty() {
            why = format!("#{} {}: {}", r.entity, esc(&r.kind), esc(&r.why));
        }
    }
    let rj: Vec<String> = refs.iter().map(|(k, n)| format!("\"{k}\":{n}")).collect();
    let st = &imp.stats;
    let pj: Vec<String> = st.pairs.iter().map(|(k, v)| format!("\"{k}\":[{},{},{}]", v.0, v.1, v.2)).collect();
    println!(
        "{{\"system\":\"{}\",\"schema\":\"{}\",\"read_ms\":{:.1},\"all_ms\":{:.1},\"mm\":{},\"unc\":{},\"voff\":{},\"coff\":{},\"voffn\":{},\"coffn\":{},\"edges\":{},\"faces\":{},\"tangent\":{},\"pairs\":{{{}}},\"refusals\":{{{}}},\"why\":\"{}\",\"solids\":[{}]}}",
        esc(&imp.system), esc(&imp.schema), read_ms, t.elapsed().as_secs_f64() * 1e3, st.units_mm, st.uncertainty, st.vertex_off_curve, st.curve_off_surface, st.vertices_off, st.edges_off, st.edges, st.faces, st.tangent_off, pj.join(","), rj.join(","), why, solids.join(",")
    );
}

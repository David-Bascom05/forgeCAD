//! One STEP file from stdin or a path: what the reader makes of it, as one JSON line, for
//! research/step/rate.py. `--no-check` and `--no-mass` skip the slower measurements.
use forge_mesh::{Quality, tessellate};
use forge_topo::{check, mass};
use std::collections::BTreeMap;
use std::io::Read;

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .map(|c| if c == '"' || c == '\\' { '_' } else { c })
        .collect()
}

/// Runs one edit, panics held: "ok", "invalid:Kind", "err:why" or "panic".
fn trial(before: f64, f: impl FnOnce() -> Result<forge_topo::Body, String>) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Err(_) => "panic".into(),
        Ok(Err(e)) => format!("err:{}", esc(&e).chars().take(40).collect::<String>()),
        Ok(Ok(b)) => {
            let p = check(&b);
            if let Some(x) = p.first() {
                let d = format!("{x:?}");
                return format!("invalid:{}", d.split([' ', '{']).next().unwrap_or(""));
            }
            match mass::mass_rel(&b, 1e-6) {
                Ok(m) if m.volume > 0.0 && m.volume <= before * (1.0 + 1e-6) => "ok".into(),
                Ok(m) => format!("volume:{:.3}", m.volume / before),
                Err(_) => "mass".into(),
            }
        }
    }
}

fn hole(at: forge_math::Point3, z: forge_math::UnitVec3, r: f64, len: f64) -> Option<forge_topo::Body> {
    use forge_math::{Frame, Point2, Vec3};
    let o = at - Vec3::from(z) * (0.5 * len);
    let p = forge_ops::Profile {
        plane: Frame::from_z(o, z),
        loops: vec![forge_ops::Profile::circle(Point2::ORIGIN, r, 7001)],
    };
    forge_ops::extrude(&p, len, 7000).ok()
}

/// A hole through the middle, a fillet on a plane-plane edge, and where the body has tolerant
/// edges, a hole through one and a fillet on one.
fn edits(b: &forge_topo::Body, vol: f64) -> String {
    use forge_bool::{Op, boolean};
    use forge_geom::{Curve, CurveGeom, Surface};
    use forge_math::UnitVec3;
    use forge_ops::{FilletEdge, Radius, fillet};
    let bx = b.faces_box();
    let (w, h, d) = (bx.x.width(), bx.y.width(), bx.z.width());
    let small = w.min(h).min(d).max(1e-3);
    let centre = bx.center();
    let mut out = Vec::new();
    let cut = trial(vol, move || {
        let t = hole(centre, UnitVec3::Z, 0.13 * small, 3.0 * bx.diagonal()).ok_or("no tool")?;
        boolean(b, &t, Op::Subtract, 7002).map_err(|e| format!("{e:?}"))
    });
    out.push(format!("\"cut\":\"{cut}\""));
    let plain = b.edge_ids().find(|&e| {
        let x = b.edge(e);
        matches!(x.curve, Curve::Line(_))
            && x.tol <= forge_math::tol::LINEAR
            && x.coedges
                .iter()
                .all(|&c| matches!(b.face(b.coedge_face(c)).surface, Surface::Plane(_)))
            && x.range.width() > 0.1 * small
    });
    if let Some(e) = plain {
        let r = 0.03 * small;
        let f = trial(vol * 1.5, move || {
            fillet(
                b,
                &[FilletEdge {
                    edge: e,
                    radius: Radius::Constant(r),
                }],
                7003,
            )
            .map_err(|x| format!("{x:?}"))
        });
        out.push(format!("\"fillet\":\"{f}\""));
    }
    let tolerant = b
        .edge_ids()
        .find(|&e| b.edge(e).tol > forge_math::tol::LINEAR && b.edge(e).alive);
    if let Some(e) = tolerant {
        let x = b.edge(e);
        let at = x.curve.eval(x.range.mid());
        let len = b
            .edge_points(e, 8)
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum::<f64>();
        // a size that means something beside the edge's own error
        let r = (0.2 * len).min(0.05 * small).max(20.0 * x.tol);
        let dir = x.curve.deriv(x.range.mid()).d1.normalize().unwrap_or(UnitVec3::Z);
        let z = dir.any_perpendicular();
        let near = trial(vol, move || {
            let t = hole(at, z, r, 3.0 * bx.diagonal()).ok_or("no tool")?;
            boolean(b, &t, Op::Subtract, 7004).map_err(|e| format!("{e:?}"))
        });
        out.push(format!("\"near\":\"{near}\""));
        let on = trial(vol * 1.5, move || {
            fillet(
                b,
                &[FilletEdge {
                    edge: e,
                    radius: Radius::Constant(0.3 * r),
                }],
                7005,
            )
            .map_err(|x| format!("{x:?}"))
        });
        out.push(format!("\"ontol\":\"{on}\""));
    }
    out.join(",")
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
        let num = |x: f64| {
            if x.is_finite() {
                format!("{x}")
            } else {
                "null".to_string()
            }
        };
        let mut bx = "null".to_string();
        if !flag("--no-box")
            && let Ok(m) = tessellate(
                &s.body,
                &Quality {
                    chord: (2e-4 * s.body.faces_box().diagonal()).min(0.05),
                    ..Quality::default()
                },
            )
        {
            let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
            for p in &m.positions {
                for (k, c) in [p.x, p.y, p.z].into_iter().enumerate() {
                    lo[k] = lo[k].min(c);
                    hi[k] = hi[k].max(c);
                }
            }
            bx = format!("[{},{},{},{},{},{}]", lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
        }
        let mut ed = String::new();
        if flag("--edit") && kinds.is_empty() && vol.is_finite() && vol > 0.0 {
            std::panic::set_hook(Box::new(|_| {}));
            ed = edits(&s.body, vol);
        }
        solids.push(format!(
            "{{\"edit\":{{{ed}}},\"id\":{},\"faces\":{},\"edges\":{},\"problems\":{{{}}},\"first\":\"{}\",\"vol\":{},\"area\":{},\"merr\":\"{}\",\"box\":{}}}",
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
    let pj: Vec<String> = st
        .pairs
        .iter()
        .map(|(k, v)| format!("\"{k}\":[{},{},{}]", v.0, v.1, v.2))
        .collect();
    println!(
        "{{\"system\":\"{}\",\"schema\":\"{}\",\"read_ms\":{:.1},\"all_ms\":{:.1},\"mm\":{},\"unc\":{},\"voff\":{},\"coff\":{},\"voffn\":{},\"coffn\":{},\"edges\":{},\"faces\":{},\"tangent\":{},\"tolerant\":{},\"maxtol\":{},\"pairs\":{{{}}},\"refusals\":{{{}}},\"why\":\"{}\",\"solids\":[{}]}}",
        esc(&imp.system),
        esc(&imp.schema),
        read_ms,
        t.elapsed().as_secs_f64() * 1e3,
        st.units_mm,
        st.uncertainty,
        st.vertex_off_curve,
        st.curve_off_surface,
        st.vertices_off,
        st.edges_off,
        st.edges,
        st.faces,
        st.tangent_off,
        st.tolerant_edges,
        st.max_edge_tol,
        pj.join(","),
        rj.join(","),
        why,
        solids.join(",")
    );
}

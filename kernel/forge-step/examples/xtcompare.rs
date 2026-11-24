//! A Parasolid file against a STEP file of the same model: `xtcompare XT [STEP]`, one JSON line.
//! Each read's solids, refusals and time; solids paired by box and volume, and for each pair the
//! differences in faces, volume, area and box, and the faces' surface kinds. For
//! research/parasolid/validate.py.
use forge_geom::Surface;
use forge_step::{Imported, Limits, import};
use forge_topo::mass;
use std::collections::BTreeMap;
use std::time::Instant;

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .map(|c| if c == '"' || c == '\\' { '_' } else { c })
        .collect()
}

struct Sum {
    faces: usize,
    volume: f64,
    area: f64,
    bx: [f64; 6],
    kinds: BTreeMap<&'static str, usize>,
    valid: bool,
}

fn kind(s: &Surface) -> &'static str {
    match s {
        Surface::Plane(_) => "plane",
        Surface::Cylinder(_) => "cylinder",
        Surface::Cone(_) => "cone",
        Surface::Sphere(_) => "sphere",
        Surface::Torus(_) => "torus",
        Surface::Procedural(_) => "other",
    }
}

fn sum(b: &forge_topo::Body) -> Sum {
    let m = mass::mass_rel(b, 1e-6).ok();
    let bx = b.faces_box();
    let mut kinds = BTreeMap::new();
    for f in b.face_ids() {
        *kinds.entry(kind(&b.face(f).surface)).or_default() += 1;
    }
    Sum {
        faces: b.face_ids().count(),
        volume: m.as_ref().map_or(f64::NAN, |m| m.volume),
        area: m.as_ref().map_or(f64::NAN, |m| m.area),
        bx: [bx.x.lo, bx.x.hi, bx.y.lo, bx.y.hi, bx.z.lo, bx.z.hi],
        kinds,
        valid: forge_topo::check(b).is_empty(),
    }
}

fn read(path: &str) -> (Option<Imported>, f64, String) {
    let t = Instant::now();
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => return (None, 0.0, esc(&e.to_string())),
    };
    let unplaced = std::env::var_os("XT_UNPLACED").is_some() && forge_step::xt::is_xt(&data);
    let r = std::panic::catch_unwind(|| {
        if unplaced {
            forge_step::xt::import_unplaced(&data, &Limits::default())
        } else {
            import(&data, &Limits::default())
        }
    });
    let secs = t.elapsed().as_secs_f64();
    match r {
        Ok(Ok(i)) => (Some(i), secs, String::new()),
        Ok(Err(e)) => (None, secs, esc(&e.to_string())),
        Err(_) => (None, secs, "panic".into()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let xt = &args[0];
    let step = args.get(1).filter(|s| !s.is_empty());
    let (x, xs, xe) = read(xt);
    let mut out = format!("{{\"xt\":\"{}\",\"xt_secs\":{xs:.2},\"xt_error\":\"{xe}\"", esc(xt));
    let xsum: Vec<Sum> = x.as_ref().map(|i| i.solids.iter().map(|s| sum(&s.body)).collect()).unwrap_or_default();
    if let Some(i) = &x {
        let mut why: BTreeMap<String, usize> = BTreeMap::new();
        for r in &i.refusals {
            *why.entry(format!("{}: {}", r.kind, esc(&r.why).chars().take(80).collect::<String>())).or_default() += 1;
        }
        let w: Vec<String> = why.iter().map(|(k, n)| format!("\"{k}\":{n}")).collect();
        out += &format!(
            ",\"xt_solids\":{},\"xt_refusals\":{},\"xt_why\":{{{}}},\"xt_invalid\":{}",
            i.solids.len(),
            i.refusals.len(),
            w.join(","),
            xsum.iter().filter(|s| !s.valid).count()
        );
    }
    if let Some(sp) = step {
        let (st, ss, se) = read(sp);
        out += &format!(",\"step_secs\":{ss:.2},\"step_error\":\"{se}\"");
        if let Some(i) = &st {
            let ssum: Vec<Sum> = i.solids.iter().map(|s| sum(&s.body)).collect();
            out += &format!(",\"step_solids\":{},\"step_refusals\":{}", i.solids.len(), i.refusals.len());
            // pair each XT solid with the unused STEP solid nearest in box and volume
            let mut used = vec![false; ssum.len()];
            let (mut paired, mut face_eq, mut vol_ok, mut box_ok, mut kinds_eq) = (0, 0, 0, 0, 0);
            let mut worst_vol: f64 = 0.0;
            let mut worst_box: f64 = 0.0;
            for a in &xsum {
                let score = |b: &Sum| {
                    let d: f64 = (0..6).map(|k| (a.bx[k] - b.bx[k]).abs()).sum();
                    d + (a.volume - b.volume).abs().powf(1.0 / 3.0)
                };
                let best = ssum
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| !used[*k])
                    .min_by(|p, q| score(p.1).total_cmp(&score(q.1)));
                let Some((k, b)) = best else { continue };
                let size = (0..3).map(|j| b.bx[2 * j + 1] - b.bx[2 * j]).fold(0.0, f64::max).max(1e-9);
                let dbox = (0..6).map(|j| (a.bx[j] - b.bx[j]).abs()).fold(0.0, f64::max) / size;
                if dbox > 0.05 {
                    continue;
                }
                used[k] = true;
                paired += 1;
                face_eq += usize::from(a.faces == b.faces);
                kinds_eq += usize::from(a.kinds == b.kinds);
                let dv = (a.volume - b.volume).abs() / b.volume.abs().max(1e-12);
                if dv <= 1e-4 {
                    vol_ok += 1;
                }
                if dv.is_finite() {
                    worst_vol = worst_vol.max(dv);
                }
                if dbox <= 1e-5 {
                    box_ok += 1;
                }
                worst_box = worst_box.max(dbox);
            }
            out += &format!(
                ",\"paired\":{paired},\"faces_equal\":{face_eq},\"kinds_equal\":{kinds_eq},\"volume_ok\":{vol_ok},\"box_ok\":{box_ok},\"worst_volume\":{worst_vol:.3e},\"worst_box\":{worst_box:.3e}"
            );
            let xf: usize = xsum.iter().map(|s| s.faces).sum();
            let sf: usize = ssum.iter().map(|s| s.faces).sum();
            out += &format!(",\"xt_faces\":{xf},\"step_faces\":{sf}");
        }
    }
    out += "}";
    println!("{out}");
    if std::env::var_os("XT_DETAIL").is_some() {
        let show = |tag: &str, v: &[Sum]| {
            for (k, s) in v.iter().enumerate() {
                eprintln!(
                    "{tag} {k}: faces {} vol {:.6} area {:.4} valid {} box {:?} {:?}",
                    s.faces, s.volume, s.area, s.valid, s.bx.map(|x| (x * 1000.0).round() / 1000.0), s.kinds
                );
            }
        };
        show("xt", &xsum);
        if let Some(sp) = step
            && let (Some(i), _, _) = read(sp)
        {
            let v: Vec<Sum> = i.solids.iter().map(|s| sum(&s.body)).collect();
            show("step", &v);
        }
        if let Some(i) = &x {
            for (k, s) in i.solids.iter().enumerate() {
                for p in forge_topo::check(&s.body).iter().take(3) {
                    eprintln!("xt {k} problem: {}", esc(&format!("{p:?}")).chars().take(300).collect::<String>());
                    if let forge_topo::check::Problem::LoopsCross { face } = p {
                        use forge_geom::{CurveGeom, SurfaceGeom};
                        let b = &s.body;
                        let fc = &b.faces[face.0 as usize];
                        eprintln!("  face surface {:?} same {}", &fc.surface, fc.same_sense);
                        for lp in fc.loops.iter() {
                            eprintln!("  loop");
                            for c in b.loops[lp.0 as usize].coedges() {
                                let ce = &b.coedges[c.0 as usize];
                                let e = &b.edges[ce.edge.0 as usize];
                                let uv: Vec<String> = (0..=4)
                                    .map(|k| {
                                        let t = e.range.lo + e.range.width() * f64::from(k) / 4.0;
                                        let q = fc.surface.project(e.curve.eval(t));
                                        format!("({:.3},{:.3})", q.x, q.y)
                                    })
                                    .collect();
                                let ck = format!("{:?}", e.curve).chars().take(12).collect::<String>();
                                eprintln!("   {:?} fwd {} {ck} {}", ce.edge, ce.forward, uv.join(" "));
                            }
                        }
                    }
                    if let forge_topo::check::Problem::LoopBroken { lp, .. } = p {
                        let b = &s.body;
                        let l = &b.loops[lp.0 as usize];
                        eprintln!("  face {:?} surface {}", l.face, kind(&b.faces[l.face.0 as usize].surface));
                        for c in l.coedges() {
                            let ce = &b.coedges[c.0 as usize];
                            let e = &b.edges[ce.edge.0 as usize];
                            let (p0, p1) = (b.vertices[e.start.0 as usize].point, b.vertices[e.end.0 as usize].point);
                            let ck = format!("{:?}", e.curve).chars().take(12).collect::<String>();
                            eprintln!("   {:?} fwd {} {ck} {:?} -> {:?} range {:?}", ce.edge, ce.forward, p0, p1, e.range);
                        }
                    }
                }
            }
            for r in &i.refusals {
                eprintln!("refusal {} {}: {}", r.entity, r.kind, r.why);
            }
        }
    }
}

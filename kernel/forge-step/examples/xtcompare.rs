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
    /// Area by surface kind: what a face-by-face comparison can rest on where one file splits a
    /// closed face in two and the other does not.
    kind_area: BTreeMap<&'static str, f64>,
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

/// The box of the edges, sampled: a face's own box can be loose (a surface's over its domain).
fn edge_box(b: &forge_topo::Body) -> forge_math::Box3 {
    use forge_geom::CurveGeom;
    let mut pts = Vec::new();
    for e in &b.edges {
        for k in 0..=32 {
            pts.push(e.curve.eval(e.range.lo + e.range.width() * f64::from(k) / 32.0));
        }
    }
    forge_math::Box3::from_points(pts)
}

fn sum(s: &forge_step::Solid) -> Sum {
    let b = &s.body;
    let m = s.mass.clone().or_else(|| mass::mass_rel(b, 1e-6).ok());
    let bx = edge_box(b);
    let mut kinds = BTreeMap::new();
    let mut kind_area: BTreeMap<&'static str, f64> = BTreeMap::new();
    for f in b.face_ids() {
        let k = kind(&b.face(f).surface);
        *kinds.entry(k).or_default() += 1;
        *kind_area.entry(k).or_default() += mass::face_area(b, f).unwrap_or(f64::NAN);
    }
    Sum {
        faces: b.face_ids().count(),
        kind_area,
        volume: m.as_ref().map_or(f64::NAN, |m| m.volume),
        area: m.as_ref().map_or(f64::NAN, |m| m.area),
        bx: [bx.x.lo, bx.x.hi, bx.y.lo, bx.y.hi, bx.z.lo, bx.z.hi],
        kinds,
        valid: if std::env::var_os("XT_NOCHECK").is_some() {
            b.unverified.is_none()
        } else {
            if std::env::var_os("XT_TRACE").is_some() {
                eprintln!("checking a body of {} faces", b.face_ids().count());
            }
            // the check on a budget: a body read wrong can send its searches on for ever
            let _b = forge_geom::budget::scoped(1 << 24);
            forge_topo::check(b).is_empty()
        },
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
    let mut out = format!(
        "{{\"xt\":\"{}\",\"xt_secs\":{xs:.2},\"xt_error\":\"{xe}\"",
        esc(xt)
    );
    let xsum: Vec<Sum> = x
        .as_ref()
        .map(|i| i.solids.iter().map(sum).collect())
        .unwrap_or_default();
    if let Some(i) = &x {
        let mut why: BTreeMap<String, usize> = BTreeMap::new();
        for r in &i.refusals {
            *why.entry(format!(
                "{}: {}",
                r.kind,
                esc(&r.why).chars().take(80).collect::<String>()
            ))
            .or_default() += 1;
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
    if std::env::var_os("XT_TOTALBOX").is_some() {
        let mut t = [
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        for s in &xsum {
            for k in 0..3 {
                t[2 * k] = t[2 * k].min(s.bx[2 * k]);
                t[2 * k + 1] = t[2 * k + 1].max(s.bx[2 * k + 1]);
            }
        }
        eprintln!(
            "total box {:?} extents {:.3} {:.3} {:.3}",
            t,
            t[1] - t[0],
            t[3] - t[2],
            t[5] - t[4]
        );
        for s in &xsum {
            eprintln!("solidbox {:?}", s.bx);
        }
        if let Some(i) = &x {
            for (k, s) in i.solids.iter().enumerate() {
                for f in s.body.face_ids() {
                    if let Surface::Cylinder(c) = &s.body.face(f).surface {
                        let (o, z) = (c.frame.origin, forge_math::Vec3::from(c.frame.z()));
                        eprintln!(
                            "axis {k} {} {} {} {} {} {} {}",
                            o.x, o.y, o.z, z.x, z.y, z.z, c.radius
                        );
                    }
                }
            }
        }
    }
    if let Some(sp) = step {
        let (st, ss, se) = read(sp);
        out += &format!(",\"step_secs\":{ss:.2},\"step_error\":\"{se}\"");
        if let Some(i) = &st {
            let ssum: Vec<Sum> = i.solids.iter().map(sum).collect();
            out += &format!(
                ",\"step_solids\":{},\"step_refusals\":{}",
                i.solids.len(),
                i.refusals.len()
            );
            // pair each XT solid with the unused STEP solid nearest in box and volume
            let mut used = vec![false; ssum.len()];
            let (mut paired, mut face_eq, mut vol_ok, mut box_ok, mut kinds_eq) = (0, 0, 0, 0, 0);
            let (mut area_ok, mut kind_area_ok, mut vol_unknown) = (0, 0, 0);
            let mut worst_area: f64 = 0.0;
            let mut worst_vol: f64 = 0.0;
            let mut worst_box: f64 = 0.0;
            for a in &xsum {
                let score = |b: &Sum| {
                    let d: f64 = (0..6).map(|k| (a.bx[k] - b.bx[k]).abs()).sum();
                    let dv = forge_math::dmath::cbrt((a.volume - b.volume).abs());
                    d + if dv.is_finite() { dv } else { 0.0 }
                };
                let best = ssum
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| !used[*k])
                    .min_by(|p, q| score(p.1).total_cmp(&score(q.1)));
                let Some((k, b)) = best else { continue };
                let size = (0..3)
                    .map(|j| b.bx[2 * j + 1] - b.bx[2 * j])
                    .fold(0.0, f64::max)
                    .max(1e-9);
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
                if !dv.is_finite() {
                    vol_unknown += 1;
                }
                let da = (a.area - b.area).abs() / b.area.abs().max(1e-12);
                if da <= 1e-4 {
                    area_ok += 1;
                }
                if da.is_finite() {
                    worst_area = worst_area.max(da);
                }
                // each kind's area within 1e-3 of the body's: the faces are of the same surfaces
                let total = b.area.abs().max(1e-12);
                let keys: std::collections::BTreeSet<&&str> =
                    a.kind_area.keys().chain(b.kind_area.keys()).collect();
                if keys.iter().all(|k| {
                    let (x, y) = (
                        a.kind_area.get(*k).copied().unwrap_or(0.0),
                        b.kind_area.get(*k).copied().unwrap_or(0.0),
                    );
                    (x - y).abs() <= 1e-3 * total
                }) {
                    kind_area_ok += 1;
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
                ",\"paired\":{paired},\"faces_equal\":{face_eq},\"kinds_equal\":{kinds_eq},\"volume_ok\":{vol_ok},\"volume_unknown\":{vol_unknown},\"area_ok\":{area_ok},\"kind_area_ok\":{kind_area_ok},\"box_ok\":{box_ok},\"worst_volume\":{worst_vol:.3e},\"worst_area\":{worst_area:.3e},\"worst_box\":{worst_box:.3e}"
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
                    s.faces,
                    s.volume,
                    s.area,
                    s.valid,
                    s.bx.map(|x| (x * 1000.0).round() / 1000.0),
                    s.kinds
                );
            }
        };
        show("xt", &xsum);
        if let Some(sp) = step
            && let (Some(i), _, _) = read(sp)
        {
            let v: Vec<Sum> = i.solids.iter().map(sum).collect();
            show("step", &v);
        }
        if let Some(i) = &x {
            for (k, s) in i.solids.iter().enumerate() {
                let _b = forge_geom::budget::scoped(1 << 24);
                if std::env::var_os("XT_NOCHECK").is_some() {
                    continue;
                }
                for p in forge_topo::check(&s.body).iter().take(3) {
                    eprintln!(
                        "xt {k} problem: {}",
                        esc(&format!("{p:?}")).chars().take(300).collect::<String>()
                    );
                    if let forge_topo::check::Problem::FaceWrongSide { face, .. } = p {
                        let fc = &s.body.faces[face.0 as usize];
                        eprintln!("  surface {} same {}", fc.surface.kind(), fc.same_sense);
                    }
                    if let forge_topo::check::Problem::LoopsCross { face } = p {
                        use forge_geom::{CurveGeom, SurfaceGeom};
                        let b = &s.body;
                        let fc = &b.faces[face.0 as usize];
                        eprintln!("  face surface {:?} same {}", fc.surface, fc.same_sense);
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
                        eprintln!(
                            "  face {:?} surface {}",
                            l.face,
                            kind(&b.faces[l.face.0 as usize].surface)
                        );
                        for c in l.coedges() {
                            let ce = &b.coedges[c.0 as usize];
                            let e = &b.edges[ce.edge.0 as usize];
                            let (p0, p1) = (
                                b.vertices[e.start.0 as usize].point,
                                b.vertices[e.end.0 as usize].point,
                            );
                            let ck = format!("{:?}", e.curve).chars().take(12).collect::<String>();
                            eprintln!(
                                "   {:?} fwd {} {ck} {:?} -> {:?} range {:?}",
                                ce.edge, ce.forward, p0, p1, e.range
                            );
                        }
                    }
                }
            }
            for r in &i.refusals {
                eprintln!("refusal {} {}: {}", r.entity, r.kind, r.why);
            }
            for (k, s) in i.solids.iter().enumerate() {
                if let Some(u) = &s.body.unverified {
                    eprintln!("unverified {k} ({} faces): {u}", s.body.face_ids().count());
                    for f in s.body.face_ids() {
                        if let Err(e) = mass::face_area(&s.body, f) {
                            let fc = s.body.face(f);
                            eprintln!(
                                "  face {f:?} {} same {} loops {}: {e:?}",
                                fc.surface.kind(),
                                fc.same_sense,
                                fc.loops.len()
                            );
                            if std::env::var_os("XT_UV").is_some() {
                                use forge_geom::{CurveGeom, SurfaceGeom};
                                let b = &s.body;
                                eprintln!("   domain {:?}", fc.surface.domain());
                                for lp in fc.loops.iter() {
                                    for c in b.loops[lp.0 as usize].coedges() {
                                        let ce = &b.coedges[c.0 as usize];
                                        let e = &b.edges[ce.edge.0 as usize];
                                        let uv: Vec<String> = (0..=4)
                                            .map(|k| {
                                                let t = e.range.lo + e.range.width() * f64::from(k) / 4.0;
                                                let p = e.curve.eval(t);
                                                let q = fc.surface.project(p);
                                                format!(
                                                    "({:.3},{:.3} d{:.1e})",
                                                    q.x,
                                                    q.y,
                                                    fc.surface.eval(q).distance(p)
                                                )
                                            })
                                            .collect();
                                        let ck =
                                            format!("{:?}", e.curve).chars().take(12).collect::<String>();
                                        eprintln!(
                                            "    {:?} fwd {} {ck} {}",
                                            ce.edge,
                                            ce.forward,
                                            uv.join(" ")
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

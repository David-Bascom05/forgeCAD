//! Exports part histories and our results as JSON for the OpenCascade oracle (tools/occt/oracle.py).
//! cargo run --release -p forge-model --example oracle -- [sets] > parts.json
//! env: THREADS, REPS, CHORD (mesh chord for volume), ONLY=name, PREFIXES=k, KEEP=i,j,.., LITERAL (print each part as Rust to stderr)
//! sets: comma separated from suite, torture, fuzz:N, adv:N (seeds per family), degen; default suite,torture,fuzz:20
use forge_mesh::{Quality, tessellate};
use forge_model::adversarial::{self, canonical};
use forge_model::{Combine, Feature, Part, suite, torture};
use forge_ops::{Profile, Segment};
use std::fmt::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

fn num(v: f64) -> String {
    if v.is_finite() {
        format!("{v:?}")
    } else {
        "null".into()
    }
}

fn v3(v: [f64; 3]) -> String {
    format!("[{},{},{}]", num(v[0]), num(v[1]), num(v[2]))
}

fn frame(f: &forge_math::Frame) -> String {
    let (o, x, y, z) = (f.origin, f.x().get(), f.y().get(), f.z().get());
    format!(
        "{{\"o\":{},\"x\":{},\"y\":{},\"z\":{}}}",
        v3([o.x, o.y, o.z]),
        v3([x.x, x.y, x.z]),
        v3([y.x, y.y, y.z]),
        v3([z.x, z.y, z.z])
    )
}

fn profile(p: &Profile) -> String {
    let loops: Vec<String> = p
        .loops
        .iter()
        .map(|l| {
            let segs: Vec<String> = l
                .segments
                .iter()
                .map(|(s, _)| match *s {
                    Segment::Line { a, b } => {
                        format!(
                            "{{\"line\":[{},{},{},{}]}}",
                            num(a.x),
                            num(a.y),
                            num(b.x),
                            num(b.y)
                        )
                    }
                    Segment::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    } => format!(
                        "{{\"arc\":[{},{},{},{},{}]}}",
                        num(center.x),
                        num(center.y),
                        num(radius),
                        num(start),
                        num(sweep)
                    ),
                    Segment::Circle { center, radius, ccw } => format!(
                        "{{\"circle\":[{},{},{},{}]}}",
                        num(center.x),
                        num(center.y),
                        num(radius),
                        ccw
                    ),
                })
                .collect();
            format!("[{}]", segs.join(","))
        })
        .collect();
    format!("\"plane\":{},\"loops\":[{}]", frame(&p.plane), loops.join(","))
}

fn op(c: Combine) -> &'static str {
    match c {
        Combine::New => "new",
        Combine::Add => "add",
        Combine::Cut => "cut",
        Combine::Keep => "keep",
    }
}

fn feature(f: &Feature) -> String {
    match f {
        Feature::Extrude {
            profile: p,
            distance,
            op: c,
        } => format!(
            "{{\"kind\":\"extrude\",\"op\":\"{}\",{},\"distance\":{}}}",
            op(*c),
            profile(p),
            num(*distance)
        ),
        Feature::Revolve {
            profile: p,
            axis,
            angle,
            op: c,
        } => format!(
            "{{\"kind\":\"revolve\",\"op\":\"{}\",{},\"axis\":[{},{},{},{}],\"angle\":{}}}",
            op(*c),
            profile(p),
            num(axis.origin.x),
            num(axis.origin.y),
            num(axis.dir.x),
            num(axis.dir.y),
            num(*angle)
        ),
        Feature::Ball {
            frame: f,
            radius,
            op: c,
        } => format!(
            "{{\"kind\":\"ball\",\"op\":\"{}\",\"center\":{},\"radius\":{}}}",
            op(*c),
            v3([f.origin.x, f.origin.y, f.origin.z]),
            num(*radius)
        ),
    }
}

/// A face's surface as the oracle keys it: kind and defining numbers (frames in world space).
fn surface_key(f: &forge_geom::Surface) -> String {
    use forge_geom::Surface::*;
    let p = |q: forge_math::Point3| v3([q.x, q.y, q.z]);
    let d = |u: forge_math::UnitVec3| {
        let u = u.get();
        v3([u.x, u.y, u.z])
    };
    match f {
        Plane(s) => format!(
            "\"s\":\"plane\",\"p\":{},\"n\":{}",
            p(s.frame.origin),
            d(s.frame.z())
        ),
        Cylinder(s) => format!(
            "\"s\":\"cylinder\",\"p\":{},\"n\":{},\"r\":{}",
            p(s.frame.origin),
            d(s.frame.z()),
            num(s.radius)
        ),
        Cone(s) => format!(
            "\"s\":\"cone\",\"p\":{},\"n\":{},\"t\":{}",
            p(s.apex()),
            d(s.frame.z()),
            num(s.tan_a())
        ),
        Sphere(s) => format!(
            "\"s\":\"sphere\",\"p\":{},\"r\":{}",
            p(s.frame.origin),
            num(s.radius)
        ),
        Torus(s) => format!(
            "\"s\":\"torus\",\"p\":{},\"n\":{},\"R\":{},\"r\":{}",
            p(s.frame.origin),
            d(s.frame.z()),
            num(s.major),
            num(s.minor)
        ),
        Procedural(_) => "\"s\":\"procedural\"".into(),
    }
}

/// Volume and area of the body's mesh, and each face's surface, meshed area and meshed boundary
/// length (the length bounds how far a chord-`c` mesh's area can be off).
fn measure(b: &forge_topo::Body, chord: f64) -> Option<String> {
    let q = Quality {
        chord,
        max_angle: 1.0,
    };
    let m = catch_unwind(AssertUnwindSafe(|| tessellate(b, &q).ok()))
        .ok()
        .flatten()?;
    let n = b.faces.len();
    let mut area = vec![0.0; n];
    let mut sides: Vec<std::collections::HashMap<(u32, u32), i32>> = vec![Default::default(); n];
    for (t, &f) in m.triangles.iter().zip(&m.face_of) {
        let [a, bb, c] = t.map(|i| m.positions[i as usize]);
        area[f as usize] += (bb - a).cross(c - a).length() * 0.5;
        for (i, j) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *sides[f as usize].entry((i.min(j), i.max(j))).or_default() += 1;
        }
    }
    let faces: Vec<String> = (0..n)
        .filter(|&i| b.faces[i].alive)
        .map(|i| {
            let perim: f64 = sides[i]
                .iter()
                .filter(|e| *e.1 == 1)
                .map(|(e, _)| (m.positions[e.0 as usize] - m.positions[e.1 as usize]).length())
                .sum();
            format!(
                "{{{},\"a\":{},\"l\":{}}}",
                surface_key(&b.faces[i].surface),
                num(area[i]),
                num(perim)
            )
        })
        .collect();
    Some(format!(
        ",\"volume\":{},\"area\":{},\"chord\":{},\"face_areas\":[{}]",
        num(m.volume()),
        num(m.area()),
        num(chord),
        faces.join(",")
    ))
}

/// One regen's result: time (best of `reps`), outcome, sizes, and the mesh's measures.
fn run(reps: usize, chord: f64, f: impl Fn() -> Result<forge_topo::Body, forge_model::RegenError>) -> String {
    let mut best = f64::INFINITY;
    let mut out = None;
    for _ in 0..reps {
        let t = Instant::now();
        let r = catch_unwind(AssertUnwindSafe(&f));
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
        out = Some(r);
    }
    match out.expect("ran") {
        Err(_) => format!("{{\"ms\":{},\"status\":\"panic\"}}", num(best)),
        Ok(Err(e)) => format!(
            "{{\"ms\":{},\"status\":\"error\",\"error\":{:?}}}",
            num(best),
            format!("{e:?}").chars().take(200).collect::<String>()
        ),
        Ok(Ok(b)) => {
            let problems = forge_topo::check(&b);
            let mut s = format!(
                "{{\"ms\":{},\"status\":\"{}\",\"faces\":{},\"edges\":{},\"vertices\":{},\"shells\":{}",
                num(best),
                if problems.is_empty() { "ok" } else { "invalid" },
                b.face_ids().count(),
                b.edge_ids().count(),
                b.vertex_ids().count(),
                b.shells.len()
            );
            if let Some(p) = problems.first() {
                let _ = write!(
                    s,
                    ",\"problem\":{:?}",
                    format!("{p:?}").chars().take(160).collect::<String>()
                );
            }
            if let Some(m) = measure(&b, chord) {
                s.push_str(&m);
            }
            s.push('}');
            s
        }
    }
}

fn main() {
    let sets = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "suite,torture,fuzz:20".into());
    let env = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let threads = env("THREADS", 4.0) as usize;
    let reps = env("REPS", 3.0) as usize;
    let chord = env("CHORD", 0.002);
    let only = std::env::var("ONLY").ok();
    std::panic::set_hook(Box::new(|_| {}));
    let mut parts: Vec<(String, Part)> = Vec::new();
    for set in sets.split(',') {
        let (name, n) = set.split_once(':').unwrap_or((set, ""));
        let n: u64 = n.parse().unwrap_or(20);
        match name {
            "suite" => parts.extend(suite::parts().into_iter().map(|(n, p)| (n.to_string(), p))),
            "torture" => parts.push(("torture".into(), torture::part(221))),
            "fuzz" => parts.extend((1..=n).map(|s| (format!("fuzz{s}"), suite::fuzz(s, 80)))),
            "degen" => parts.extend(adversarial::degenerate_parts()),
            "adv" => {
                for (fam, g) in adversarial::families()
                    .into_iter()
                    .filter(|f| !f.0.starts_with("degen_"))
                {
                    parts.extend((0..n).map(|s| (format!("{fam}/{s}"), canonical(&g(s)))));
                }
            }
            _ => panic!("unknown set {name}"),
        }
    }
    // PREFIXES=k: each part also as its first k, 2k, ... features (to bisect a disagreement)
    let step = env("PREFIXES", 0.0) as usize;
    if step > 0 {
        parts = parts
            .into_iter()
            .filter(|(n, _)| only.as_deref().is_none_or(|o| o == n))
            .flat_map(|(n, p)| {
                let len = p.features.len();
                (1..=len.div_ceil(step)).map(move |i| {
                    let k = (i * step).min(len);
                    let features = p.features[..k].to_vec();
                    (format!("{n}#{k}"), Part { features })
                })
            })
            .collect();
    }
    // KEEP=0,5,9: only those features of each part
    if let Ok(keep) = std::env::var("KEEP") {
        let keep: Vec<usize> = keep.split(',').filter_map(|k| k.parse().ok()).collect();
        for (_, p) in &mut parts {
            p.features = keep.iter().filter_map(|&k| p.features.get(k).cloned()).collect();
        }
    }
    println!("{{\"threads\":{threads},\"parts\":[");
    let mut first = true;
    for (name, part) in parts {
        if step == 0 && only.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        eprintln!("{name}");
        if std::env::var_os("LITERAL").is_some() {
            eprintln!("{}", adversarial::literal(&canonical(&part)));
        }
        let seq = run(reps, chord, || part.regenerate());
        let nary1 = run(1, chord, || part.regenerate_nary(1));
        let nary = run(reps, chord, || part.regenerate_nary(threads));
        let fs: Vec<String> = part.features.iter().map(feature).collect();
        println!(
            "{}{{\"name\":{name:?},\"features\":[\n{}\n],\"forge\":{{\"seq\":{seq},\"nary1\":{nary1},\"nary\":{nary}}}}}",
            if first { "" } else { "," },
            fs.join(",\n")
        );
        first = false;
    }
    println!("]}}");
}

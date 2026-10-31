// research/nary-short-runs (copy into kernel/forge-model/examples to run): nx run SEED LEN seq|naryT [SEEDHI]
// | nx hash FROM TO LEN T | nx wall PART reps | nx cmp PART
use forge_math::hash::BitHasher;
use forge_model::suite;

fn hash(r: &Result<forge_topo::Body, forge_model::RegenError>) -> String {
    let mut h = BitHasher::new();
    match r {
        Ok(b) => {
            for v in &b.vertices {
                h.p3(v.point);
                h.f64(v.tol);
                h.u64(v.name.0);
            }
            for e in &b.edges {
                h.f64(e.range.lo);
                h.f64(e.range.hi);
                h.u64(e.name.0);
                h.u64(e.start.index() as u64);
                h.u64(e.end.index() as u64);
            }
            for c in &b.coedges {
                h.u64(c.edge.index() as u64 * 2 + c.forward as u64);
            }
            for f in &b.faces {
                h.u64(f.name.0);
                h.u64(f.same_sense as u64);
                for l in &f.loops {
                    h.u64(b.loops[l.index()].coedges().len() as u64);
                }
            }
            format!("{:016x} ok{}", h.finish(), forge_topo::check(b).is_empty())
        }
        Err(e) => format!("err {e:?}"),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let n = |i: usize| a[i].parse::<usize>().unwrap();
    match a[0].as_str() {
        "run" => {
            let reps: usize = std::env::var("NX_REPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            let lo = n(1);
            let hi = a.get(4).map_or(lo, |x| x.parse().unwrap());
            for _ in 0..reps {
                for s in lo..=hi {
                    let p = suite::fuzz(s as u64, n(2));
                    let _ = if a[3] == "seq" {
                        p.regenerate()
                    } else {
                        p.regenerate_nary(a[3][4..].parse().unwrap())
                    };
                }
            }
        }
        "wall" => {
            // nx wall PART|fuzzS.L reps: best-of ms for seq and nary 1 2 4
            let part = if let Some(x) = a[1].strip_prefix("fuzz") {
                let (s, l) = x.split_once('.').unwrap();
                suite::fuzz(s.parse().unwrap(), l.parse().unwrap())
            } else if a[1] == "torture" {
                forge_model::torture::part(221)
            } else {
                suite::part(&a[1]).unwrap()
            };
            let reps = n(2);
            let best = |f: &dyn Fn()| {
                (0..reps)
                    .map(|_| {
                        let t = std::time::Instant::now();
                        f();
                        t.elapsed().as_secs_f64() * 1e3
                    })
                    .fold(f64::INFINITY, f64::min)
            };
            let s = best(&|| {
                part.regenerate().unwrap();
            });
            let ts: Vec<String> = [1, 2, 4]
                .iter()
                .map(|&t| {
                    format!(
                        "{:.2}",
                        best(&|| {
                            part.regenerate_nary(t).unwrap();
                        })
                    )
                })
                .collect();
            println!("{:<10} seq {s:.2} nary1/2/4 {}", a[1], ts.join(" "));
        }
        "cmp" => {
            let p = forge_model::suite::part(&a[1]).expect("part");
            let (s, t) = (p.regenerate().unwrap(), p.regenerate_nary(1).unwrap());
            let desc = |b: &forge_topo::Body| -> Vec<String> {
                let mut v: Vec<String> = b
                    .face_ids()
                    .map(|f| {
                        let fc = b.face(f);
                        let bx = forge_bool::classify::face_box(b, f);
                        let ne: usize = fc.loops.iter().map(|l| b.lp(*l).coedges().len()).sum();
                        format!(
                            "{} loops {} box ({:.2},{:.2},{:.2})-({:.2},{:.2},{:.2}) e{} {:016x}",
                            format!("{:?}", fc.surface).chars().take(8).collect::<String>(),
                            fc.loops.len(),
                            bx.x.lo,
                            bx.y.lo,
                            bx.z.lo,
                            bx.x.hi,
                            bx.y.hi,
                            bx.z.hi,
                            ne,
                            fc.name.0
                        )
                    })
                    .collect();
                v.sort();
                v
            };
            let (ds, dt) = (desc(&s), desc(&t));
            let key = |x: &String| x.split(" e").next().unwrap().to_string();
            let (ks, kt): (Vec<String>, Vec<String>) =
                (ds.iter().map(key).collect(), dt.iter().map(key).collect());
            let edges = |b: &forge_topo::Body, name: &str| {
                for f in b.face_ids() {
                    if format!("{:016x}", b.face(f).name.0) != name {
                        continue;
                    }
                    for l in &b.face(f).loops {
                        for c in b.lp(*l).coedges() {
                            let e = b.edge(b.coedge(c).edge);
                            let (p, q) = (b.vertex(e.start).point, b.vertex(e.end).point);
                            println!(
                                "      {} ({:.3},{:.3},{:.3})->({:.3},{:.3},{:.3}) {:016x}",
                                format!("{:?}", e.curve).chars().take(8).collect::<String>(),
                                p.x,
                                p.y,
                                p.z,
                                q.x,
                                q.y,
                                q.z,
                                e.name.0
                            );
                        }
                    }
                }
            };
            for x in &ds {
                if !kt.contains(&key(x)) {
                    println!("seq only:  {x}");
                    edges(&s, x.rsplit(' ').next().unwrap());
                }
            }
            for x in &dt {
                if !ks.contains(&key(x)) {
                    println!("nary only: {x}");
                    edges(&t, x.rsplit(' ').next().unwrap());
                }
            }
        }
        _ => {
            for s in n(1)..=n(2) {
                let p = suite::fuzz(s as u64, n(3));
                let t = n(4);
                if t == 0 {
                    println!("{s} seq {}", hash(&p.regenerate()));
                } else {
                    println!("{s} nary {}", hash(&p.regenerate_nary(t)));
                }
            }
        }
    }
}

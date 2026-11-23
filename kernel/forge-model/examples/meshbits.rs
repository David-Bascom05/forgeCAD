//! A hash of every mesh the kernel makes from its parts, and of the constrained Delaunay
//! triangulator's on random regions: what tools/speed/bits.txt does not cover (it hashes the
//! B-rep). Two builds that print the same lines mesh alike, bit for bit.
//!
//!   meshbits            one line per part (suite, fuzz histories, bench parts), quality and CDT case
//!
//! As wasm (`cargo build --profile wasm -p forge-model --example meshbits --target
//! wasm32-wasip1`, run with tools/wasi-run.mjs) it prints the same lines: native = wasm.
use forge_math::Point2;
use forge_math::hash::BitHasher;
use forge_math::rng::Rng;
use forge_mesh::cdt::{Refine, triangulate_as_far, triangulate_within};
use forge_mesh::{Quality, tessellate};
use forge_model::{Part, suite};

fn mesh_hash(part: &Part, ids: Option<&[u64]>, q: &Quality) -> u64 {
    let mut h = BitHasher::new();
    let bodies = match ids {
        None => part.regenerate().map(|b| vec![b]),
        Some(ids) => part
            .regenerate_solids_ids(Some(ids))
            .map(|s| s.into_iter().map(|s| s.body).collect()),
    };
    match bodies {
        Err(_) => h.u64(u64::MAX),
        Ok(bodies) => {
            for body in &bodies {
                mesh_into(&mut h, body, q);
            }
        }
    }
    h.finish()
}

fn mesh_into(h: &mut BitHasher, body: &forge_topo::Body, q: &Quality) {
    {
        match tessellate(body, q) {
            Err(e) => {
                h.u64(u64::MAX - 1);
                for b in format!("{e:?}").bytes() {
                    h.u64(u64::from(b));
                }
            }
            Ok(m) => {
                for p in &m.positions {
                    h.p3(*p);
                }
                for t in &m.triangles {
                    h.u64(u64::from(t[0]) | u64::from(t[1]) << 21 | u64::from(t[2]) << 42);
                }
                for f in &m.face_of {
                    h.u64(u64::from(*f));
                }
                for e in &m.edges {
                    h.u64(e.len() as u64);
                    for v in e {
                        h.u64(u64::from(*v));
                    }
                }
                for f in &m.unsettled {
                    h.u64(u64::from(*f));
                }
            }
        }
    }
}

fn bench_parts() -> Vec<(String, Part, Option<Vec<u64>>)> {
    let dir = format!(
        "{}/../../interface/forge-lang/parts/bench",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the bench parts")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "fg"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    // (not std::env::temp_dir, which WASI does not have: meshbits runs as wasm too)
    let tmp = std::env::var_os("TMPDIR").map_or_else(|| std::path::PathBuf::from("/tmp"), Into::into);
    let fonts = tmp.join("forge-meshbits");
    std::fs::create_dir_all(&fonts).expect("temp dir");
    std::fs::write(
        fonts.join(forge_lang::bench::FONT),
        forge_lang::bench::font_bytes(),
    )
    .expect("font");
    names
        .into_iter()
        .map(|n| {
            let src = std::fs::read_to_string(format!("{dir}/{n}.fg")).expect("a bench part");
            let opts = forge_lang::Options {
                dir: Some(fonts.clone()),
                checks: false,
                ..forge_lang::Options::default()
            };
            let (_, cache) = forge_lang::run(&src, opts, Default::default());
            let (f, ids) = cache.history();
            (n, Part { features: f.to_vec() }, Some(ids.to_vec()))
        })
        .collect()
}

/// Splits edges longer than `len` and triangles wider than `area`.
struct Coarse {
    len: f64,
    area: f64,
}

impl Refine for Coarse {
    fn edge(&self, _: u32, _: u32, a: Point2, b: Point2) -> bool {
        a.distance(b) > self.len
    }
    fn triangle(&self, _: [u32; 3], p: [Point2; 3]) -> bool {
        0.5 * forge_math::predicates::orient2d(p[0], p[1], p[2]) > self.area
    }
    fn triangles(&self) -> bool {
        self.area > 0.0
    }
}

fn cdt_hash(r: &Result<(forge_mesh::Triangulation, bool), forge_mesh::CdtError>) -> u64 {
    let mut h = BitHasher::new();
    match r {
        Err(e) => {
            h.u64(u64::MAX);
            for b in format!("{e:?}").bytes() {
                h.u64(u64::from(b));
            }
        }
        Ok((t, settled)) => {
            h.u64(u64::from(*settled));
            for p in &t.points {
                h.f64(p.x);
                h.f64(p.y);
            }
            for v in &t.triangles {
                h.u64(u64::from(v[0]) | u64::from(v[1]) << 21 | u64::from(v[2]) << 42);
            }
        }
    }
    h.finish()
}

/// A random region: a star outline, star holes inside it, scattered interior points; or, for a
/// tangled case, loops of random points that cross themselves and each other, sometimes repeating
/// a point or a chord. Some points snapped to a coarse grid, for collinear and cocircular ties.
fn region(rng: &mut Rng, tangled: bool) -> (Vec<Point2>, Vec<Vec<u32>>, Vec<Point2>) {
    let mut pts = Vec::new();
    let mut loops = Vec::new();
    let grid = rng.below(3) == 0;
    let snap = |v: f64| if grid { (v * 8.0).round() / 8.0 } else { v };
    if tangled {
        for _ in 0..1 + rng.below(3) {
            let n = 3 + rng.below(12);
            let mut lp = Vec::new();
            for _ in 0..n {
                if !pts.is_empty() && rng.below(6) == 0 {
                    lp.push(rng.below(pts.len()) as u32);
                } else {
                    lp.push(pts.len() as u32);
                    pts.push(Point2::new(snap(rng.unit() * 10.0), snap(rng.unit() * 10.0)));
                }
            }
            loops.push(lp);
        }
    } else {
        let star = |pts: &mut Vec<Point2>, rng: &mut Rng, c: Point2, r: f64, n: usize, ccw: bool| {
            let mut lp: Vec<u32> = Vec::new();
            for k in 0..n {
                let a = std::f64::consts::TAU * k as f64 / n as f64;
                let rr = r * (0.6 + 0.4 * rng.unit());
                lp.push(pts.len() as u32);
                pts.push(Point2::new(
                    c.x + rr * forge_math::dmath::cos(a),
                    c.y + rr * forge_math::dmath::sin(a),
                ));
            }
            if !ccw {
                lp.reverse();
            }
            lp
        };
        let n = 5 + rng.below(40);
        loops.push(star(&mut pts, rng, Point2::new(0.0, 0.0), 10.0, n, true));
        for h in 0..rng.below(3) {
            let c = Point2::new(-3.0 + 6.0 * h as f64, 0.0);
            let m = 3 + rng.below(10);
            loops.push(star(&mut pts, rng, c, 1.5, m, false));
        }
    }
    let interior = (0..rng.below(30))
        .map(|_| Point2::new(snap(rng.unit() * 20.0 - 10.0), snap(rng.unit() * 20.0 - 10.0)))
        .collect();
    (pts, loops, interior)
}

fn main() {
    let mut parts: Vec<(String, Part, Option<Vec<u64>>)> = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p, None))
        .collect();
    for seed in 1..=24u64 {
        parts.push((format!("fuzz{seed}"), suite::fuzz(seed, 40), None));
    }
    parts.extend(bench_parts());
    let qualities = [
        Quality::default(),
        Quality {
            chord: 0.25,
            max_angle: 0.7,
        },
        Quality {
            chord: 0.02,
            max_angle: 0.3,
        },
    ];
    for (name, part, ids) in &parts {
        for q in &qualities {
            println!("{name} {} {:016x}", q.chord, mesh_hash(part, ids.as_deref(), q));
        }
    }
    let mut rng = Rng::new(0x6d65_7368);
    for case in 0..400 {
        let tangled = case % 4 == 3;
        let (pts, loops, interior) = region(&mut rng, tangled);
        let coarse = [
            None,
            Some(Coarse { len: 2.0, area: 0.0 }),
            Some(Coarse { len: 1.0, area: 0.5 }),
        ];
        for (k, c) in coarse.iter().enumerate() {
            let refine = c.as_ref().map(|c| c as &dyn Refine);
            let r = triangulate_within(&pts, &loops, &interior, refine, tangled).map(|t| (t, true));
            println!("cdt{case} r{k} {:016x}", cdt_hash(&r));
            let r = triangulate_as_far(&pts, &loops, &interior, refine, tangled);
            println!("cdt{case} r{k} far {:016x}", cdt_hash(&r));
        }
    }
}

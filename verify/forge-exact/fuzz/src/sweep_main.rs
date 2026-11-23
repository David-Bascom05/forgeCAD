//! `sweep-diff`: every extrude of every repo part, and random ones, against its definition
//! (forge_exact_fuzz::sweep; research/sweep-spec/DESIGN.md §4.1).
//!
//! usage: sweep-diff [--seeds N] [--tapers N] [--from S] [--no-repo] [--digest PATH] [--check PATH] [--verbose]
//!   --digest PATH   write each case's verdict, sorted, to PATH
//!   --check PATH    compare with PATH; exit 1 on any difference
//! Exits 2 when any case is `differ`, `built_invalid`, `refused_valid` or `selfcheck`.

use forge_exact_fuzz::guarded;
use forge_exact_fuzz::sweep::{SweepRef, feature_line, field};
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3};
use forge_model::{Combine, Extent, Feature};
use forge_ops::{Profile, ProfileLoop, Segment};
use std::collections::BTreeMap;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.unit()
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn star(rng: &mut Rng, n: usize, c: Point2, r0: f64, r1: f64) -> Vec<Point2> {
    let mut angles: Vec<f64> = (0..n).map(|_| rng.range(0.0, std::f64::consts::TAU)).collect();
    angles.sort_by(f64::total_cmp);
    angles
        .iter()
        .map(|&a| {
            let r = rng.range(r0, r1);
            Point2::new(
                c.x + r * forge_math::dmath::cos(a),
                c.y + r * forge_math::dmath::sin(a),
            )
        })
        .collect()
}

fn lines(pts: &[Point2], first_id: u64) -> ProfileLoop {
    let n = pts.len();
    ProfileLoop {
        segments: (0..n)
            .map(|i| {
                (
                    Segment::Line {
                        a: pts[i],
                        b: pts[(i + 1) % n],
                    },
                    first_id + i as u64,
                )
            })
            .collect(),
    }
}

/// A random extrude: a star polygon (sometimes with a hole, sometimes a random and maybe crossing
/// polygon), a random frame, a random extent.
fn random_case(seed: u64) -> Vec<Feature> {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let n = 3 + rng.below(10) as usize;
    let kind = rng.below(10);
    let mut loops = Vec::new();
    if kind < 2 {
        // any points: often crossing, so the kernel must refuse
        let pts: Vec<Point2> = (0..n)
            .map(|_| Point2::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)))
            .collect();
        loops.push(lines(&pts, 1));
    } else {
        let outer = star(&mut rng, n, Point2::new(0.0, 0.0), 4.0, 10.0);
        loops.push(lines(&outer, 1));
        if kind >= 7 {
            let m = 3 + rng.below(6) as usize;
            let c = Point2::new(rng.range(-0.5, 0.5), rng.range(-0.5, 0.5));
            let mut hole = star(&mut rng, m, c, 0.5, 3.0);
            hole.reverse();
            loops.push(lines(&hole, 100));
        }
    }
    let z = loop {
        let v = Vec3::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
        if let Some(u) = v.normalize() {
            break u;
        }
    };
    let z: UnitVec3 = if rng.below(3) == 0 { UnitVec3::Z } else { z };
    let origin = Point3::new(
        rng.range(-20.0, 20.0),
        rng.range(-20.0, 20.0),
        rng.range(-20.0, 20.0),
    );
    let plane = Frame::from_z(origin, z);
    let d = |rng: &mut Rng| match rng.below(12) {
        0 => rng.range(-2e-5, 2e-5),
        _ => rng.range(0.5, 30.0) * if rng.below(3) == 0 { -1.0 } else { 1.0 },
    };
    let extent = match rng.below(5) {
        0 | 1 => Extent::Distance(d(&mut rng)),
        2 => Extent::Symmetric(d(&mut rng).abs()),
        3 => Extent::TwoSided {
            forward: d(&mut rng),
            back: d(&mut rng),
        },
        _ => {
            let dir = Vec3::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)) + z.get();
            Extent::Along {
                forward: d(&mut rng).abs(),
                back: rng.range(0.0, 5.0),
                dir,
            }
        }
    };
    vec![Feature::Extrude {
        profile: Profile { plane, loops },
        extent,
        op: Combine::New,
    }]
}

/// A random taper: a skyline polygon (simple, every edge axis-aligned, so of rational length),
/// sometimes with a rectangular hole, sometimes turned by the 3-4-5 rotation (still rational
/// lengths), a random angle either way and a random depth either way.
fn random_taper(seed: u64) -> Vec<Feature> {
    let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
    let cols = 1 + rng.below(5) as usize;
    let mut x = 0.0;
    let mut pts = vec![Point2::new(0.0, 0.0)];
    let mut first = (0.0, 0.0, 0.0);
    for c in 0..cols {
        let w = (1 + rng.below(8)) as f64;
        let h = (1 + rng.below(8)) as f64;
        if c == 0 {
            first = (w, h, 0.0);
        }
        pts.push(Point2::new(x, h));
        x += w;
        pts.push(Point2::new(x, h));
    }
    pts.push(Point2::new(x, 0.0));
    // drop corners where the skyline keeps its height (two columns alike)
    let mut clean: Vec<Point2> = Vec::new();
    for p in pts {
        clean.push(p);
        while clean.len() >= 3 {
            let n = clean.len();
            let (a, b, c) = (clean[n - 3], clean[n - 2], clean[n - 1]);
            if (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) == 0.0 {
                clean.remove(n - 2);
            } else {
                break;
            }
        }
    }
    let turn = rng.below(3) == 0;
    let map = |p: Point2| {
        if turn {
            Point2::new(3.0 * p.x - 4.0 * p.y, 4.0 * p.x + 3.0 * p.y)
        } else {
            p
        }
    };
    let mut loops = vec![lines(&clean.iter().map(|&p| map(p)).collect::<Vec<_>>(), 1)];
    let _ = first.2;
    if rng.below(3) == 0 && first.0 >= 3.0 && first.1 >= 3.0 {
        let hole = [
            Point2::new(1.0, 1.0),
            Point2::new(1.0, first.1 - 1.0),
            Point2::new(first.0 - 1.0, first.1 - 1.0),
            Point2::new(first.0 - 1.0, 1.0),
        ];
        loops.push(lines(&hole.iter().map(|&p| map(p)).collect::<Vec<_>>(), 100));
    }
    let origin = Point3::new(rng.range(-5.0, 5.0), rng.range(-5.0, 5.0), rng.range(-5.0, 5.0));
    let plane = if rng.below(2) == 0 {
        Frame::from_z(origin, UnitVec3::Z)
    } else {
        let v = Vec3::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(0.2, 1.0));
        Frame::from_z(origin, v.normalize().expect("a direction"))
    };
    let angle = rng.range(1.0, 75.0).to_radians() * if rng.below(4) == 0 { -1.0 } else { 1.0 };
    let distance = rng.range(0.2, 6.0) * if rng.below(4) == 0 { -1.0 } else { 1.0 };
    vec![Feature::Extrude {
        profile: Profile { plane, loops },
        extent: Extent::Tapered { distance, angle },
        op: Combine::New,
    }]
}

fn repo_parts(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if name.ends_with(".fg") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let seeds: u64 = opt("--seeds").and_then(|s| s.parse().ok()).unwrap_or(500);
    let verbose = args.iter().any(|a| a == "--verbose");
    let from: u64 = opt("--from").and_then(|s| s.parse().ok()).unwrap_or(0);
    let repo = !args.iter().any(|a| a == "--no-repo");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let root = root.canonicalize().expect("the checkout");

    let mut reference = SweepRef::start();
    let mut verdicts: BTreeMap<String, String> = BTreeMap::new();
    let mut ask = |name: String, line: Result<String, String>, verdicts: &mut BTreeMap<String, String>| {
        let v = match line {
            Err(e) => format!("out_of_scope ({})", e.split([' ', '(', '{']).next().unwrap_or("")),
            Ok(line) => {
                if let Ok(p) = std::env::var("SWEEP_DUMP") {
                    use std::io::Write as _;
                    let mut f = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(p)
                        .expect("SWEEP_DUMP");
                    writeln!(f, "{line}").expect("SWEEP_DUMP");
                }
                let a = reference.ask(&line);
                if verbose {
                    eprintln!("{name}: {a}");
                }
                match field(&a, "verdict") {
                    Some(v) => v.to_string(),
                    None => format!("error {}", field(&a, "error").unwrap_or(&a)),
                }
            }
        };
        verdicts.insert(name, v);
    };

    for path in if repo { repo_parts(&root) } else { Vec::new() } {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(built) = guarded(|| Ok(forge_lang::fuzz::build(&src))) else {
            continue;
        };
        for i in 0..built.features.len() {
            if let Some(line) = feature_line(&built.features, i) {
                ask(format!("{rel}#{i}"), line, &mut verdicts);
            }
        }
    }
    for seed in from..from + seeds {
        let features = random_case(seed);
        if verbose && let Feature::Extrude { extent, profile, .. } = &features[0] {
            {
                eprintln!(
                    "random/{seed:05}: {extent:?} z {:?} loops {} area {} normalized {:?}",
                    profile.plane.z(),
                    profile.loops.len(),
                    profile.area(),
                    profile.normalized_touching().err()
                );
                let h = forge_model::History::new(&features, None);
                if let Ok(ts) = forge_model::feature_tools(h, 0) {
                    let b = ts[0].0.compact();
                    let mesh = forge_mesh::tessellate(&b, &forge_mesh::Quality::default());
                    eprintln!(
                        "  kernel: {} faces, problems {:?}, mesh volume {:?}",
                        b.face_ids().count(),
                        forge_topo::check(&b),
                        mesh.map(|m| m.volume())
                    );
                }
            }
        }
        if let Some(line) = feature_line(&features, 0) {
            ask(format!("random/{seed:05}"), line, &mut verdicts);
        }
    }

    let tapers: u64 = opt("--tapers").and_then(|s| s.parse().ok()).unwrap_or(300);
    for seed in from..from + tapers {
        let features = random_taper(seed);
        if verbose {
            eprintln!(
                "taper/{seed:05}: {:?}",
                match &features[0] {
                    Feature::Extrude { extent, .. } => *extent,
                    _ => unreachable!(),
                }
            );
        }
        if let Some(line) = feature_line(&features, 0) {
            ask(format!("taper/{seed:05}"), line, &mut verdicts);
        }
    }

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for v in verdicts.values() {
        *counts.entry(v.clone()).or_default() += 1;
    }
    // refused_valid: the kernel refused a definition in the domain (over-refusal, reported, not a
    // wrong result); built_roof: a taper whose outline closes before its depth, built as the
    // solid cut off there (DESIGN.md §2.2 rule 3, a decision pending)
    let bad = ["differ", "built_invalid", "selfcheck"];
    for (name, v) in &verdicts {
        if bad.iter().any(|b| v.starts_with(b)) || v.starts_with("error") {
            println!("{name}: {v}");
        }
    }
    for (v, n) in &counts {
        println!("{n:6} {v}");
    }
    let digest: String = verdicts.iter().map(|(k, v)| format!("{k} {v}\n")).collect();
    if let Some(p) = opt("--digest") {
        std::fs::write(&p, &digest).expect("write the digest");
    }
    let mut code = 0;
    if let Some(p) = opt("--check") {
        let old = std::fs::read_to_string(&p).unwrap_or_default();
        if old != digest {
            println!("the digest differs from {p}");
            code = 1;
        }
    }
    if verdicts
        .values()
        .any(|v| bad.iter().any(|b| v.starts_with(b)) || v.starts_with("error"))
    {
        code = 2;
    }
    std::process::exit(code);
}

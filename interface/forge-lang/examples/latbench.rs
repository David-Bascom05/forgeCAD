//! The user-latency bench (tools/speed/latency.sh): wall-clock and CPU time of what a user waits
//! for, on the bench parts (parts/bench/*.fg), at whatever thread count the process is given
//! (FORGECAD_THREADS, the CPUs taskset leaves it).
//!
//!   latbench ops PART [K]     one round on PART: `open` (the cold build, `forgecad build`'s:
//!                             checks on), then K timed reps (after one untimed warm-up) of each
//!                             of: `edit` (one dimension changed, rebuilt from the open's cache:
//!                             a new value every rep), `step` (STEP AP214 export), `mesh` (3MF
//!                             export, checked), `draw` (hidden-line views front, top, right, iso),
//!                             `nary` (the kernel's n-ary regen at the process's thread count);
//!                             an op's reps stop once they took LATBENCH_BUDGET s (default 2),
//!                             and LATBENCH_SKIP=op,.. leaves ops out
//!   latbench kern NAME [K]    the kernel's suite part NAME (forge_model::suite: coplanar, upto,
//!                             shaft, mixed, ..): `kseq` (the sequential regen) and `nary` (the
//!                             n-ary regen at the process's thread count), warm-up then K reps
//!   latbench exec CMD ARG..   one timed run of a child process (the CAM certify line)
//!   latbench bits             each part's result hashed, seq and n-ary at the process's thread
//!                             count (compared across FORGECAD_THREADS=1, 2, N by latency.sh)
//! Output: one line per sample, `OP REP WALL_NS CPU_NS` (CPU: user+sys of the whole process, all
//! threads, over the op, in 10 ms ticks), REP 0 for `open`.
//!
//! Validation knobs (latency.sh --selftest; never set in a real comparison):
//!   LATBENCH_SLOW=PART:OP:FRAC,..  after OP on PART, spin until (1+FRAC) x its time has passed:
//!                                  a deliberate slowdown of exactly FRAC on that line
//!   LATBENCH_PARVIEWS=PART,..      `draw` on PART makes its views on all threads at once
//!                                  (forge_math::par::map) instead of one after another: a
//!                                  deliberate threading speed-up (the same views, the same bits)
use forge_model::Part;
use std::time::Instant;

/// Fields of /proc/self/stat after the command: `(utime + stime, cutime + cstime)` in ns.
/// The kernel keeps each thread's run time to the ns and adds a thread's to its process when it
/// ends; this file reports the sum in clock ticks (USER_HZ, 100 on Linux), so a sample's CPU is
/// quantised to 10 ms: latstat.py takes a round's CPU as the mean over its reps (their total).
fn proc_cpu() -> (u64, u64) {
    let s = std::fs::read_to_string("/proc/self/stat").expect("/proc/self/stat");
    let f: Vec<u64> = s[s.rfind(')').expect("stat") + 2..]
        .split(' ')
        .map(|x| x.parse().unwrap_or(0))
        .collect();
    let tick = 10_000_000;
    ((f[11] + f[12]) * tick, (f[13] + f[14]) * tick)
}

/// CPU time (user + sys) of this process so far, every thread's, ended ones included, in ns.
fn cpu_ns() -> u64 {
    proc_cpu().0
}

/// CPU time (user + sys) of this process's waited-for children so far, in ns.
fn children_cpu_ns() -> u64 {
    proc_cpu().1
}

fn font_dir() -> std::path::PathBuf {
    let dir = forge_dirs::temp_dir().join("forge-latbench");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join(forge_lang::bench::FONT), forge_lang::bench::font_bytes()).expect("font");
    dir
}

fn source(name: &str) -> String {
    let path = match std::env::var("FORGE_BENCH_PARTS") {
        Ok(dir) => format!("{dir}/{name}.fg"),
        Err(_) => format!("{}/parts/bench/{name}.fg", env!("CARGO_MANIFEST_DIR")),
    };
    std::fs::read_to_string(&path).expect("a bench part")
}

fn names() -> Vec<String> {
    let dir = format!("{}/parts/bench", env!("CARGO_MANIFEST_DIR"));
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .expect("the bench parts")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "fg"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    v.sort();
    v
}

/// The text with its first param's value scaled by `k`: an edit of one dimension.
fn edited(src: &str, k: f64) -> String {
    let mut out = String::with_capacity(src.len() + 8);
    let mut done = false;
    for line in src.split_inclusive('\n') {
        let body = line.trim_end();
        let head = body
            .strip_prefix("shared ")
            .or_else(|| body.strip_prefix("param "));
        if !done
            && let Some((_, v)) = head.and_then(|h| h.split_once(" = "))
            && let Ok(x) = v.split('#').next().unwrap_or("").trim().parse::<f64>()
        {
            let prefix = &line[..line.find(" = ").expect("found") + 3];
            out.push_str(&format!("{prefix}{}\n", x * k));
            done = true;
            continue;
        }
        out.push_str(line);
    }
    assert!(done, "a param to edit");
    out
}

/// `LATBENCH_SLOW`'s fraction for this part and op.
fn slow(part: &str, op: &str) -> f64 {
    std::env::var("LATBENCH_SLOW").ok().map_or(0.0, |s| {
        s.split(',')
            .filter_map(|e| {
                let mut it = e.split(':');
                let (p, o, f) = (it.next()?, it.next()?, it.next()?);
                (p == part && o == op).then(|| f.parse::<f64>().ok())?
            })
            .sum()
    })
}

fn par_views(part: &str) -> bool {
    std::env::var("LATBENCH_PARVIEWS")
        .ok()
        .is_some_and(|s| s.split(',').any(|p| p == part))
}

/// Times `f`, prints its sample, and returns its value.
fn timed<T>(part: &str, op: &str, rep: usize, f: impl FnOnce() -> T) -> T {
    let extra = slow(part, op);
    let (c0, t0) = (cpu_ns(), Instant::now());
    let v = std::hint::black_box(f());
    if extra > 0.0 {
        let until = t0 + t0.elapsed().mul_f64(1.0 + extra);
        while Instant::now() < until {
            std::hint::spin_loop();
        }
    }
    let wall = t0.elapsed();
    let cpu = cpu_ns() - c0;
    if rep != usize::MAX {
        println!("{op} {rep} {} {cpu}", wall.as_nanos());
    }
    v
}

fn options(dir: &std::path::Path) -> forge_lang::Options {
    forge_lang::Options {
        dir: Some(dir.to_path_buf()),
        ..forge_lang::Options::default()
    }
}

fn views(name: &str, body: &forge_topo::Body) -> usize {
    let inst = [forge_draw::Instance {
        body,
        pose: forge_math::Frame::WORLD,
    }];
    let opts = forge_draw::Options::default();
    let names = ["front", "top", "right", "iso"];
    let one = |k: usize| {
        let v = forge_draw::View::named(names[k]).expect("a named view");
        forge_draw::hlr(&inst, &v, &opts).len()
    };
    if par_views(name) {
        forge_math::par::map(names.len(), one).into_iter().sum()
    } else {
        (0..names.len()).map(one).sum()
    }
}

fn ops(name: &str, k: usize) {
    let dir = font_dir();
    let src = source(name);
    let opts = options(&dir);
    let (run, cache) = timed(name, "open", 0, || {
        forge_lang::run(&src, opts.clone(), Default::default())
    });
    assert!(run.errors.is_empty(), "{name}: {:?}", run.errors);
    let body = run.body.clone().expect("a solid");
    // edit: a new value of the first dimension every rep, each rebuilt from the last one's cache
    let mut cache = Some(cache);
    let t0 = Instant::now();
    for rep in 0..=k {
        if rep > 1 && t0.elapsed() > budget() {
            break;
        }
        let s = edited(&src, 1.0 + 0.01 * (rep + 1) as f64);
        let r = if rep == 0 { usize::MAX } else { rep };
        let (run, c) = timed(name, "edit", r, || {
            forge_lang::run(&s, opts.clone(), cache.take().expect("cache"))
        });
        assert!(run.errors.is_empty(), "{name} edited: {:?}", run.errors);
        cache = Some(c);
    }
    let mut sopt = forge_step::Options::default();
    if !run.sheets.is_empty() {
        sopt.tol = forge_step::SHEET_TOL;
    }
    sopt.name = name.to_string();
    let mopt = forge_lang::meshfile::Options::new(forge_lang::meshfile::Format::ThreeMf);
    // the part's kernel features as the build sent them, for the n-ary regen
    let (f, ids) = {
        let o = forge_lang::Options {
            checks: false,
            ..opts.clone()
        };
        let (_, c) = forge_lang::run(&src, o, Default::default());
        let (f, ids) = c.history();
        (Part { features: f.to_vec() }, ids.to_vec())
    };
    let ops: [(&str, &dyn Fn() -> usize); 4] = [
        ("step", &|| forge_step::export(&body, &sopt).expect("step").text.len()),
        ("mesh", &|| {
            let e = forge_lang::meshfile::export(&run, name, Some(&dir), &mopt).expect("mesh");
            assert!(e.checks.iter().all(|c| c.watertight()), "{name}: mesh not watertight");
            e.bytes.len()
        }),
        ("draw", &|| views(name, &body)),
        ("nary", &|| {
            f.regenerate_nary_solids_ids(forge_math::par::threads(), Some(&ids))
                .expect("nary")
                .len()
        }),
    ];
    for (op, go) in ops {
        if skip(op) {
            continue;
        }
        timed(name, op, usize::MAX, go);
        let t0 = Instant::now();
        for rep in 1..=k {
            timed(name, op, rep, go);
            if t0.elapsed() > budget() {
                break;
            }
        }
    }
}

/// The time an op's reps may take in a round (LATBENCH_BUDGET seconds, default 2): after it, no
/// more reps (at least one is timed).
fn budget() -> std::time::Duration {
    std::time::Duration::from_secs_f64(
        std::env::var("LATBENCH_BUDGET").ok().and_then(|b| b.parse().ok()).unwrap_or(2.0),
    )
}

/// An op LATBENCH_SKIP (comma-separated) leaves out.
fn skip(op: &str) -> bool {
    std::env::var("LATBENCH_SKIP").ok().is_some_and(|s| s.split(',').any(|o| o == op))
}

fn hash_solids(r: &[forge_model::Solid]) -> u64 {
    let mut h = forge_math::hash::BitHasher::new();
    for b in r.iter().map(|s| &s.body) {
        for v in &b.vertices {
            h.p3(v.point);
        }
        for e in &b.edges {
            h.f64(e.range.lo);
            h.f64(e.range.hi);
        }
        h.u64(b.faces.len() as u64);
    }
    h.finish()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("ops") => ops(
            &args[1],
            args.get(2).map_or(5, |k| k.parse().expect("K")),
        ),
        Some("kern") => {
            let name = &args[1];
            let k = args.get(2).map_or(5, |k| k.parse().expect("K"));
            let part = forge_model::suite::part(name).expect("a suite part");
            let ops: [(&str, &dyn Fn() -> bool); 2] = [
                ("kseq", &|| part.regenerate().is_ok()),
                ("nary", &|| part.regenerate_nary(forge_math::par::threads()).is_ok()),
            ];
            for (op, go) in ops {
                assert!(timed(name, op, usize::MAX, go), "{name} {op} failed");
                let t0 = Instant::now();
                for rep in 1..=k {
                    timed(name, op, rep, go);
                    if t0.elapsed() > budget() {
                        break;
                    }
                }
            }
        }
        Some("exec") => {
            let (c0, t0) = (children_cpu_ns(), Instant::now());
            let st = std::process::Command::new(&args[1])
                .args(&args[2..])
                .stdout(std::process::Stdio::null())
                .status()
                .expect("spawn");
            let wall = t0.elapsed();
            assert!(st.success(), "{} failed: {st}", args[1]);
            println!("exec 0 {} {}", wall.as_nanos(), children_cpu_ns() - c0);
        }
        Some("bits") => {
            let dir = font_dir();
            let t = forge_math::par::threads();
            for name in names() {
                let o = forge_lang::Options {
                    checks: false,
                    ..options(&dir)
                };
                let (run, c) = forge_lang::run(&source(&name), o, Default::default());
                assert!(run.errors.is_empty(), "{name}: {:?}", run.errors);
                let (f, ids) = c.history();
                let part = Part { features: f.to_vec() };
                let seq = part.regenerate_solids_ids(Some(ids)).expect("seq");
                let nary = part.regenerate_nary_solids_ids(t, Some(ids)).expect("nary");
                let draw = run.body.as_ref().map_or(0, |b| views("", b));
                println!(
                    "{name} seq {:016x} nary {:016x} views {draw}",
                    hash_solids(&seq),
                    hash_solids(&nary)
                );
            }
        }
        _ => {
            eprintln!("usage: latbench ops PART [K] | exec CMD ARG.. | bits");
            std::process::exit(64);
        }
    }
}

//! The boolean's decision log over the kernel's test inputs (research/kernel-audit, phase 0):
//! per fact kind, how often one fact was decided two ways within one boolean and by which pairs
//! of sites; per rung of the retry ladder, how often it fired, on which families, and how its
//! try ended.
//!
//! cargo run --release -p forge-proof --features decisions --example decisions --
//!     [only SECTION,..] [rel N] [fuzz N] [seeds N] [count N] [adv N] [ulp N] [threads N] [out FILE.md]
//! Sections: reliability (the scoreboard's parts, both paths), suite (FUZZ_SEEDS fuzz histories of
//! 80 features, as CI runs them), meta (at the nightly depth, `seeds` fuzz histories of `count`
//! features and `adv` seeds of each adversarial family, under the properties tools/proof/known.txt
//! files failures of), lang (forge_lang::fuzz over its test seeds,
//! fuzz_known.txt's), hair (the box a hair under a tolerance below a notched top, adv
//! near_tolerance 2's first three features, under N ulp nudges), bench (parts/bench/*.fg).
//! The n-ary regens run on one thread, so their decisions land in the plan's ledger.
use forge_bool::decisions::{self, Kind, Stats};
use forge_model::adversarial::{canonical, degenerate_parts, families};
use forge_model::{Part, suite};
use forge_proof::props::{self, Property};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

enum Job {
    /// Both regens (sequential, n-ary on one thread), or the sequential alone.
    Regen(Part, bool),
    Prop(&'static str, Property, Part, u64),
    Lang(u64),
    Bench(String),
}

struct Task {
    family: String,
    name: String,
    job: Job,
}

fn task(family: impl Into<String>, name: impl Into<String>, job: Job) -> Task {
    Task {
        family: family.into(),
        name: name.into(),
        job,
    }
}

fn bench_dir() -> String {
    format!(
        "{}/../../interface/forge-lang/parts/bench",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn font_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-decisions-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join(forge_lang::bench::FONT), forge_lang::bench::font_bytes()).expect("font");
    dir
}

/// Did a run fail: an error, an invalid body, a finding, a panic.
fn run(job: &Job, fonts: &std::path::Path) -> bool {
    let valid = |r: Result<forge_topo::Body, forge_model::RegenError>| {
        r.is_ok_and(|b| forge_topo::check(&b).is_empty())
    };
    match job {
        Job::Regen(p, nary) => {
            let a = valid(p.regenerate());
            let b = !*nary || valid(p.regenerate_nary(1));
            !(a && b)
        }
        Job::Prop(name, f, p, seed) => props::run(name, *f, p, *seed).is_some(),
        Job::Lang(seed) => !forge_lang::fuzz::run(*seed, 12).fails.is_empty(),
        Job::Bench(name) => {
            let src = std::fs::read_to_string(format!("{}/{name}.fg", bench_dir())).expect("a bench part");
            let opts = forge_lang::Options {
                dir: Some(fonts.to_path_buf()),
                checks: false,
                ..forge_lang::Options::default()
            };
            let (r, cache) = forge_lang::run(&src, opts, Default::default());
            let (f, ids) = cache.history();
            let part = Part { features: f.to_vec() };
            let nary = part.regenerate_nary_solids_ids(1, Some(ids));
            !r.errors.is_empty() || nary.is_err()
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let num = |k: &str, d: u64| opt(k).and_then(|s| s.parse().ok()).unwrap_or(d);
    let only = opt("only");
    let on = |s: &str| only.as_ref().is_none_or(|o| o.split(',').any(|x| x == s));
    let (rel, fuzz, ulps) = (num("rel", 24), num("fuzz", 150), num("ulp", 400));
    let (seeds, count, adv) = (num("seeds", 150), num("count", 20) as usize, num("adv", 6));
    let threads = num("threads", 4) as usize;
    std::panic::set_hook(Box::new(|_| {}));

    let mut tasks: Vec<Task> = Vec::new();
    if on("reliability") {
        for (fam, g) in families() {
            if fam.starts_with("degen_") {
                continue;
            }
            for s in 0..rel {
                tasks.push(task(
                    format!("rel {fam}"),
                    format!("{fam}/{s}"),
                    Job::Regen(canonical(&g(s)), true),
                ));
            }
        }
        for (name, p) in degenerate_parts() {
            let fam = format!("rel degen_{}", name.split('/').next().unwrap_or(""));
            tasks.push(task(fam, name, Job::Regen(canonical(&p), true)));
        }
        for (name, p) in suite::parts() {
            tasks.push(task("rel suite", name, Job::Regen(p, true)));
        }
        for s in 1..=rel {
            tasks.push(task(
                "rel fuzz",
                format!("fuzz/{s}"),
                Job::Regen(suite::fuzz(s, 40), true),
            ));
        }
    }
    if on("suite") {
        for s in 1..=fuzz {
            tasks.push(task(
                "suite fuzz80",
                format!("seed {s}"),
                Job::Regen(suite::fuzz(s, 80), false),
            ));
        }
    }
    if on("meta") {
        // the properties known.txt lists failures of, over meta's parts: fuzz histories and seeds
        // of each adversarial family (even property seeds: exact motions only), as meta has them
        let props: Vec<(&'static str, Property)> = props::all()
            .into_iter()
            .filter(|(n, _)| {
                [
                    "rigid", "mirror", "order", "boolean", "backends", "certify", "ulp",
                ]
                .contains(n)
            })
            .collect();
        let mut parts: Vec<(String, String, Part)> = (0..seeds)
            .map(|s| {
                (
                    "fuzz".to_string(),
                    format!("fuzz{s}"),
                    suite::fuzz(s + 1000, count),
                )
            })
            .collect();
        parts.extend(
            suite::parts()
                .into_iter()
                .map(|(n, p)| ("suite".to_string(), n.to_string(), p)),
        );
        for (fam, g) in families() {
            parts.extend((0..adv).map(|s| (fam.to_string(), format!("adv {fam} {s}"), canonical(&g(s)))));
        }
        for (name, f) in &props {
            for (i, (fam, pname, part)) in parts.iter().enumerate() {
                let seed = if pname.starts_with("adv") {
                    2 * (i as u64 + 1)
                } else {
                    i as u64 + 1
                };
                let job = Job::Prop(name, *f, part.clone(), seed);
                tasks.push(task(format!("meta {name} {fam}"), format!("{name} {pname}"), job));
            }
        }
    }
    if on("lang") {
        for &s in forge_lang::fuzz::SEEDS.iter() {
            tasks.push(task("lang fuzz", format!("lang seed {s}"), Job::Lang(s)));
        }
    }
    if on("hair") {
        let (_, g) = families()
            .into_iter()
            .find(|(f, _)| *f == "near_tolerance")
            .expect("the family");
        let mut part = canonical(&g(2));
        part.features.truncate(3);
        for seed in 0..ulps {
            let job = Job::Prop("ulp", props::ulp, part.clone(), seed);
            tasks.push(task(
                "hair-under-notched-top ulp",
                format!("ulp seed {seed}"),
                job,
            ));
        }
    }
    if on("bench") {
        let mut names: Vec<String> = std::fs::read_dir(bench_dir())
            .expect("the bench parts")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "fg"))
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .collect();
        names.sort();
        for n in names {
            tasks.push(task(format!("bench {n}"), n.clone(), Job::Bench(n)));
        }
    }

    let fonts = font_dir();
    let t0 = Instant::now();
    let next = AtomicUsize::new(0);
    // per family: runs, failed, panicked
    let outcome: Mutex<BTreeMap<String, [usize; 3]>> = Mutex::new(BTreeMap::new());
    eprintln!("{} runs on {threads} threads", tasks.len());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(t) = tasks.get(i) else { break };
                    decisions::input(&t.family, &t.name);
                    let r = catch_unwind(AssertUnwindSafe(|| run(&t.job, &fonts)));
                    let mut o = outcome.lock().expect("outcomes");
                    let e = o.entry(t.family.clone()).or_default();
                    e[0] += 1;
                    match r {
                        Ok(failed) => e[1] += failed as usize,
                        Err(_) => e[2] += 1,
                    }
                    if i.is_multiple_of(200) {
                        eprintln!("{i}/{} {:.0} s", tasks.len(), t0.elapsed().as_secs_f64());
                    }
                }
            });
        }
    });
    let secs = t0.elapsed().as_secs_f64();
    let stats = decisions::take();
    let outcome = outcome.into_inner().expect("outcomes");
    let text = table(&stats, &outcome, &args, tasks.len(), secs);
    match opt("out") {
        Some(f) => std::fs::write(&f, &text).expect("write the table"),
        None => print!("{text}"),
    }
}

/// The family's section ("rel", "meta", ..) for the rung table: the first word.
fn section(f: &str) -> String {
    let mut w = f.split(' ');
    let head = w.next().unwrap_or("");
    match head {
        "meta" => format!("meta {}", w.next().unwrap_or("")),
        h => h.to_string(),
    }
}

fn top(m: &BTreeMap<String, u64>, n: usize) -> String {
    let mut v: Vec<(&String, &u64)> = m.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let mut s: Vec<String> = v.iter().take(n).map(|(k, c)| format!("{k} {c}")).collect();
    if v.len() > n {
        s.push(format!("+{} more", v.len() - n));
    }
    s.join(", ")
}

fn table(
    s: &Stats,
    outcome: &BTreeMap<String, [usize; 3]>,
    args: &[String],
    runs: usize,
    secs: f64,
) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "# The boolean's decision log: contradictions and retries\n");
    let _ = writeln!(
        o,
        "`cargo run --release -p forge-proof --features decisions --example decisions -- {}`: {runs} runs \
         in {secs:.0} s. Written by that run (the notes above it are by hand).\n",
        args.join(" ")
    );
    let _ = writeln!(o, "## Per fact kind\n");
    let _ = writeln!(
        o,
        "| fact | decisions | facts | facts decided at 2+ sites | contradictions | of which by one site only |"
    );
    let _ = writeln!(o, "|---|---:|---:|---:|---:|---:|");
    for k in Kind::ALL {
        let e = s.kinds.get(&k).cloned().unwrap_or_default();
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} |",
            k.name(),
            e.decisions,
            e.facts,
            e.multi_site,
            e.contradictions,
            e.same_site
        );
    }
    let _ = writeln!(
        o,
        "\nDecisions recorded outside any plan (an n-ary pass's helper threads): {}.\n",
        s.unscoped
    );

    let _ = writeln!(o, "## Plans: does a contradiction go with a lost way?\n");
    let _ = writeln!(
        o,
        "| path | plans | with a contradiction | of those ok / internal / other / unknown | without: ok / internal / other / unknown |"
    );
    let _ = writeln!(o, "|---|---:|---:|---|---|");
    for (path, p) in &s.plans {
        let n: u64 = p.iter().flatten().sum();
        let w: u64 = p[1].iter().sum();
        let row = |r: &[u64; 4]| format!("{} / {} / {} / {}", r[0], r[1], r[2], r[3]);
        let _ = writeln!(o, "| {path} | {n} | {w} | {} | {} |", row(&p[1]), row(&p[0]));
    }

    let _ = writeln!(o, "\n## Site pairs that disagree\n");
    let _ = writeln!(
        o,
        "Per fact kind, the pairs of sites that gave one fact two answers, most facts first.\n"
    );
    let _ = writeln!(o, "| fact | site | other site | facts | paths | families |");
    let _ = writeln!(o, "|---|---|---|---:|---|---|");
    let mut pairs: Vec<(&(Kind, String, String), &decisions::PairStat)> = s.pairs.iter().collect();
    pairs.sort_by_key(|a| (a.0.0, std::cmp::Reverse(a.1.count)));
    for ((k, a, b), p) in &pairs {
        let paths: BTreeMap<String, u64> = p.paths.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let _ = writeln!(
            o,
            "| {} | `{a}` | `{b}` | {} | {} | {} |",
            k.name(),
            p.count,
            top(&paths, 3),
            top(&p.families, 4)
        );
    }
    let _ = writeln!(
        o,
        "\nAn example of each (input [path; retry flags] entities and points: each site's answer, its measure vs its limit):\n"
    );
    for ((k, a, b), p) in &pairs {
        let _ = writeln!(o, "- {} / `{a}` vs `{b}`: {}", k.name(), p.example);
    }

    let _ = writeln!(o, "\n## Retry rungs\n");
    let _ = writeln!(
        o,
        "Each try a rung made, by how it ended (ok: the plan came out; internal: lost its way again; other: \
         refused otherwise; unknown: a rung that only marks the boolean as retried).\n"
    );
    let _ = writeln!(
        o,
        "| rung | tries | ok | internal | other | unknown | by section (tries/ok) | inputs (first 12) |"
    );
    let _ = writeln!(o, "|---|---:|---:|---:|---:|---:|---|---|");
    for (name, r) in &s.rungs {
        let n: u64 = r.outcomes.iter().sum();
        let mut by: BTreeMap<String, [u64; 2]> = BTreeMap::new();
        for (f, c) in &r.families {
            let e = by.entry(section(f)).or_default();
            e[0] += c.iter().sum::<u64>();
            e[1] += c[0];
        }
        let by: Vec<String> = by.iter().map(|(k, v)| format!("{k} {}/{}", v[0], v[1])).collect();
        let inputs: Vec<&str> = r.inputs.iter().map(String::as_str).collect();
        let _ = writeln!(
            o,
            "| {name} | {n} | {} | {} | {} | {} | {} | {} |",
            r.outcomes[0],
            r.outcomes[1],
            r.outcomes[2],
            r.outcomes[3],
            by.join(", "),
            inputs.join("; ")
        );
    }
    let _ = writeln!(o, "\n## Retry rungs by family\n");
    let _ = writeln!(o, "| rung | family | tries | ok | internal | other | unknown |");
    let _ = writeln!(o, "|---|---|---:|---:|---:|---:|---:|");
    for (name, r) in &s.rungs {
        for (f, c) in &r.families {
            let _ = writeln!(
                o,
                "| {name} | {f} | {} | {} | {} | {} | {} |",
                c.iter().sum::<u64>(),
                c[0],
                c[1],
                c[2],
                c[3]
            );
        }
    }

    let _ = writeln!(o, "\n## Inputs\n");
    let _ = writeln!(
        o,
        "| family | runs | failed (error, invalid, finding) | panicked | plans with a contradiction |"
    );
    let _ = writeln!(o, "|---|---:|---:|---:|---:|");
    for (f, c) in outcome {
        let _ = writeln!(
            o,
            "| {f} | {} | {} | {} | {} |",
            c[0],
            c[1],
            c[2],
            s.contradicted.get(f).copied().unwrap_or(0)
        );
    }
    let _ = writeln!(o, "\n## Decisions per site\n");
    let _ = writeln!(o, "| fact | site | decisions |");
    let _ = writeln!(o, "|---|---|---:|");
    for ((k, site), n) in &s.sites {
        let _ = writeln!(o, "| {} | `{site}` | {n} |", k.name());
    }
    o
}

//! Random whole-kernel .fg programs (`forge_lang::fuzz`): every statement builds a valid part or
//! is refused with a clear error, and every finished part meshes closed and regenerates the same
//! n-ary. Failures the kernel still has are listed by kind in fuzz_known.txt; any other kind fails
//! here. Hunt and shrink: cargo run --release -p forge-model --example shrink -- gen 0 200
use forge_lang::fuzz::{self, MAKERS, SEEDS, Verdict, run};
use std::collections::BTreeMap;

const LEN: usize = 12;
/// Seeds whose finished part the n-ary regen builds only through the sequential one.
const RELY_ON_NAME_FALLBACK: &[u64] = &[30];

/// The feature calls of the language a generated program may make, by the maker that makes each
/// (an import needs a file: its round trips are forge-step's and the language's own tests; sheet
/// metal, thermoforms and assemblies are not kernel histories).
const FEATURES: &[(&str, &str)] = &[
    ("extrude", "extrude"),
    ("cut", "extrude"),
    ("intersect", "extrude"),
    ("revolve", "revolve"),
    ("fillet", "fillet"),
    ("chamfer", "chamfer"),
    ("shell", "shell"),
    ("loft", "loft"),
    ("hole", "hole"),
    ("thread", "thread"),
    ("pattern", "pattern"),
    ("workplane", "workplane"),
    ("workaxis", "workaxis"),
    ("workpoint", "workpoint"),
    ("ucs", "ucs"),
    ("combine", "combine"),
    ("split", "split"),
    ("splitface", "splitface"),
    ("copybody", "copybody"),
    ("movebody", "movebody"),
    ("text", "text"),
];

#[test]
fn every_kind_has_a_maker() {
    // (`dome` is a loft's end, not a feature)
    let shapes = forge_lang::shapes::NAMES
        .iter()
        .filter(|n| **n != "dome")
        .map(|n| (*n, *n));
    for (call, maker) in FEATURES.iter().copied().chain(shapes) {
        // `sphere` is `ball`
        let maker = if maker == "sphere" { "ball" } else { maker };
        assert!(
            MAKERS.iter().any(|m| m.name == maker),
            "no maker for '{call}': add a row to forge_lang::fuzz::MAKERS"
        );
    }
}

/// Every seed fails only in kinds fuzz_known.txt lists (a line there matches any failure whose
/// kind holds it), and every line there still matches a failure: the list only shrinks. The seeds
/// run on up to eight threads (each seed's run is a function of the seed alone).
#[test]
#[cfg_attr(target_family = "wasm", ignore = "a kernel panic aborts the wasm run")]
fn random_programs() {
    forge_topo::check::check_against_both_ways(true);
    let known: Vec<&str> = include_str!("fuzz_known.txt")
        .lines()
        .map(|l| l.split(" #").next().unwrap_or("").trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let runs: Vec<(u64, fuzz::Run)> = std::thread::scope(|s| {
        let threads = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .clamp(4, 8);
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(&seed) = SEEDS.get(i) else { break };
                        let t = std::time::Instant::now();
                        let r = run(seed, LEN);
                        eprintln!(
                            "seed {seed}: {:.1} s, {} statements",
                            t.elapsed().as_secs_f64(),
                            fuzz::statements(&r.src).len()
                        );
                        out.push((seed, r));
                    }
                    out
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("fuzz worker"))
            .collect()
    });
    let mut kinds: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut new = Vec::new();
    let mut seen = vec![false; known.len()];
    for (seed, r) in &runs {
        for (k, why) in &r.tried {
            let e = kinds.entry(k).or_default();
            e.0 += 1;
            e.1 += why.is_none() as usize;
        }
        for (what, src) in &r.fails {
            let mut hit = false;
            for (i, k) in known.iter().enumerate() {
                if what.contains(k) {
                    seen[i] = true;
                    hit = true;
                }
            }
            if !hit {
                new.push(format!("seed {seed}: {what}\n{src}"));
            }
        }
    }
    eprintln!("kind (tried, built): {kinds:?}");
    // every maker drawn, and every one built at least once over the seeds
    for m in MAKERS.iter().filter(|m| !fuzz::SWARM_ONLY.contains(&m.name)) {
        let (tried, built) = kinds.get(m.name).copied().unwrap_or_default();
        assert!(tried > 0 && built > 0, "{}: tried {tried}, built {built}", m.name);
    }
    assert!(
        new.is_empty(),
        "{} failures of a kind not in fuzz_known.txt (save one as F.fg and shrink it: \
         cargo run --release -p forge-model --example shrink -- F.fg):\n{}",
        new.len(),
        new.join("\n")
    );
    // seeds whose finished part the n-ary regen builds only through the sequential one (a later
    // feature's reference named as only that one names it, docs/naming.md): the list only shrinks
    let mut relying: Vec<u64> = runs
        .iter()
        .filter(|(_, r)| r.name_fallbacks > 0)
        .map(|(s, _)| *s)
        .collect();
    relying.sort_unstable();
    assert_eq!(
        relying, RELY_ON_NAME_FALLBACK,
        "seeds relying on the n-ary name fallback"
    );
    let gone: Vec<&str> = known
        .iter()
        .zip(&seen)
        .filter(|(_, s)| !**s)
        .map(|(k, _)| *k)
        .collect();
    assert!(
        gone.is_empty(),
        "no seed fails as these any more: remove them from fuzz_known.txt\n{}",
        gone.join("\n")
    );
}

/// Sheets and thin walls among the kinds that make and change solids and move bodies (a swarm of
/// its own, so the seeds' programs in `random_programs` stay what they were): every statement
/// builds valid or is refused with a clear error, and every finished part's bodies, sheets too,
/// mesh as they should and regenerate the same n-ary.
#[test]
#[cfg_attr(target_family = "wasm", ignore = "a kernel panic aborts the wasm run")]
fn sheet_and_thin_programs() {
    forge_topo::check::check_against_both_ways(true);
    let known: Vec<&str> = include_str!("fuzz_known_swarm.txt")
        .lines()
        .map(|l| l.split(" #").next().unwrap_or("").trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let mask = fuzz::kinds_of(&[
        "extrude", "revolve", "fillet", "chamfer", "hole", "pattern", "thicken", "movebody", "copybody",
        "sheet", "thin",
    ]);
    let mut kinds: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut new = Vec::new();
    let mut seen = vec![false; known.len()];
    for &seed in &SEEDS {
        let r = fuzz::run_kinds(seed, LEN, mask);
        for (k, why) in &r.tried {
            let e = kinds.entry(k).or_default();
            e.0 += 1;
            e.1 += why.is_none() as usize;
        }
        for (what, src) in &r.fails {
            let mut hit = false;
            for (i, k) in known.iter().enumerate() {
                if what.contains(k) {
                    seen[i] = true;
                    hit = true;
                }
            }
            if !hit {
                new.push(format!("seed {seed}: {what}\n{src}"));
            }
        }
    }
    eprintln!("kind (tried, built): {kinds:?}");
    for k in fuzz::SWARM_ONLY {
        let (tried, built) = kinds.get(k).copied().unwrap_or_default();
        assert!(tried > 0 && built > 0, "{k}: tried {tried}, built {built}");
    }
    assert!(
        new.is_empty(),
        "{} failures of a kind not in fuzz_known_swarm.txt:\n{}",
        new.len(),
        new.join("\n")
    );
    let gone: Vec<&str> = known
        .iter()
        .zip(&seen)
        .filter(|(_, s)| !**s)
        .map(|(k, _)| *k)
        .collect();
    assert!(
        gone.is_empty(),
        "no seed fails as these any more: remove them from fuzz_known_swarm.txt\n{}",
        gone.join("\n")
    );
}

/// The same seed makes the same program, whatever else ran before.
#[test]
fn a_seed_makes_one_program() {
    let a = run(5, 6).src;
    let _ = run(6, 3);
    assert_eq!(a, run(5, 6).src);
}

/// The shrinker keeps a failure's verdict and cuts what does not carry it.
#[test]
fn shrinking_keeps_the_verdict() {
    // an unknown name is a statement the language cannot read: the generator's bug
    let src = "sketch S0(on: xy) {\n  o = rect(corner: (0, 0), size: (40, 30))\n}\nL0 = extrude(S0, 10)\n\
               sketch S1(on: xy) {\n  o = circle((5, 5), 2)\n}\nL1 = cut(S1, thru)\nL2 = fillet(edges(of: nosuch), 1)\n";
    let v = fuzz::judge(src);
    assert!(
        matches!(&v, Verdict::Fail(w) if w.contains("UnknownName")),
        "{v:?}"
    );
    let small = fuzz::shrink(src, &v);
    assert_eq!(fuzz::judge(&small), v);
    assert!(
        fuzz::statements(&small).len() < fuzz::statements(src).len(),
        "{small}"
    );
}

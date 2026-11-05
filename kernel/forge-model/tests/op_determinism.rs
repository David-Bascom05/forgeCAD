//! What the op memo's proof assumes of the kernel (research/forge-lang/lean/ForgeLang/OpMemo.lean;
//! forgecad-gate/instant-load/op-memo-proof.md "Audit"): a regen is a function of the part.
//!
//! - `nary_is_the_same_on_every_backend`: the n-ary regen, bit for bit with every face's and
//!   edge's name, on the CPU batch, the GPU batch (when a GPU opens here; the test says whether
//!   it did) and Auto, at 1 and 4 threads (audit item 5).
//! - `after_a_caught_panic_*`: a regen on a thread where an earlier op was cut short by a panic
//!   with thread-local state set (audit item 3: the work budget; item 1: a boolean retry switch)
//!   gives what a fresh thread gives. Both are guarded (drop guards).
use forge_gpu::Backend;
use forge_model::{Part, suite, torture};

fn bits(r: &Result<forge_topo::Body, forge_model::RegenError>) -> u64 {
    let mut h = forge_math::hash::BitHasher::new();
    match r {
        Ok(b) => {
            for v in &b.vertices {
                h.p3(v.point);
            }
            for e in &b.edges {
                h.f64(e.range.lo);
                h.f64(e.range.hi);
            }
            for f in b.face_ids() {
                h.u64(b.face(f).name.0);
            }
            for e in b.edge_ids() {
                h.u64(b.edge(e).name.0);
            }
            h.u64(b.faces.len() as u64);
        }
        Err(e) => {
            h.u64(u64::MAX);
            for b in format!("{e:?}").bytes() {
                h.u64(b as u64);
            }
        }
    }
    h.finish()
}

fn parts() -> Vec<(String, Part)> {
    let mut out: Vec<(String, Part)> = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    out.push(("torture".into(), torture::part(221)));
    for seed in [1, 2, 5] {
        out.push((format!("fuzz{seed}"), suite::fuzz(seed, 80)));
    }
    out
}

#[test]
fn nary_is_the_same_on_every_backend() {
    let gpu = forge_gpu::gpu().is_some();
    eprintln!(
        "nary_is_the_same_on_every_backend: a GPU {}",
        if gpu {
            "opened"
        } else {
            "did not open: Gpu runs on the CPU"
        }
    );
    let mut bad = vec![];
    for (name, part) in parts() {
        let base = bits(&part.regenerate_nary_on(1, Backend::Cpu));
        for (threads, backend, what) in [
            (4, Backend::Cpu, "cpu, 4 threads"),
            (1, Backend::Gpu, "gpu, 1 thread"),
            (4, Backend::Gpu, "gpu, 4 threads"),
            (4, Backend::Auto, "auto, 4 threads"),
        ] {
            if bits(&part.regenerate_nary_on(threads, backend)) != base {
                bad.push(format!("{name}: {what} differs from cpu on 1 thread"));
            }
        }
        if bits(&part.regenerate_nary_on(1, Backend::Cpu)) != base {
            bad.push(format!("{name}: cpu again differs"));
        }
    }
    assert!(
        bad.is_empty(),
        "the n-ary regen depends on its backend:\n{}",
        bad.join("\n")
    );
}

/// A regen on a fresh thread, and on a thread where `spoil` ran and panicked first.
fn fresh_and_spoiled(part: &Part, spoil: fn()) -> (u64, u64) {
    // resume_unwind: a panic without the hook's message
    let p1 = part.clone();
    let fresh = std::thread::spawn(move || bits(&p1.regenerate()))
        .join()
        .expect("fresh");
    let p2 = part.clone();
    let spoiled = std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(spoil);
        bits(&p2.regenerate())
    })
    .join()
    .expect("spoiled");
    (fresh, spoiled)
}

/// Audit item 3: a STEP read sets the work budget and a panic inside it leaves it set; mass
/// properties (what a fillet's retries and thermo accept or refuse by) on that thread then fail.
#[test]
fn after_a_caught_panic_the_work_budget_is_back() {
    let mut bad = vec![];
    for (name, part) in parts() {
        let Ok(body) = part.regenerate() else { continue };
        let mass = |b: &forge_topo::Body| {
            let m = forge_topo::mass::mass(b).map(|m| m.volume.to_bits());
            let t = forge_mesh::tessellate(b, &forge_mesh::Quality::default()).map(|m| m.triangles.len());
            format!("{m:?} {t:?}")
        };
        let b1 = body.clone();
        let fresh = std::thread::spawn(move || mass(&b1)).join().expect("fresh");
        let spoiled = std::thread::spawn(move || {
            let _ = std::panic::catch_unwind(|| {
                let _b = forge_geom::budget::scoped(1);
                std::panic::resume_unwind(Box::new("a read cut short"));
            });
            mass(&body)
        })
        .join()
        .expect("spoiled");
        if fresh != spoiled {
            bad.push(format!("{name}: {fresh} vs {spoiled}"));
        }
    }
    assert!(
        bad.is_empty(),
        "mass after a panic in a budgeted read differs:\n{}",
        bad.join("\n")
    );
}

/// Audit item 1: a boolean retry sets its switches and a panic in the retry leaves them set; the
/// one switch outside forge-bool (`ABSORB_HAIR_ROOTS`) stands for them.
#[test]
fn after_a_caught_panic_the_boolean_switches_are_back() {
    let mut bad = vec![];
    for (name, part) in parts() {
        let (a, b) = fresh_and_spoiled(&part, || {
            forge_geom::intersect::ABSORB_HAIR_ROOTS.with(|k| k.set(true));
            std::panic::resume_unwind(Box::new("a retry cut short"));
        });
        if a != b {
            bad.push(name);
        }
    }
    assert!(
        bad.is_empty(),
        "a regen after a panic in a boolean retry differs: {bad:?}"
    );
}

//! Opens parts: from the build cache, or built by the kernel and then cached.

use crate::mesh::PartMesh;
use crate::{cache, tess};
use forge_agent::bench;
use forge_agent::cmd::Session;

/// The part's file text; a bench part's name or a path.
pub fn source(part: &str) -> Result<String, String> {
    match bench::src(part) {
        Some(s) => Ok(s.to_string()),
        None => std::fs::read_to_string(part).map_err(|e| format!("{part}: {e}")),
    }
}

/// The part's body, built by the kernel: the one its mesh's entity numbers mean.
pub fn body(part: &str) -> Result<forge_topo::Body, String> {
    let bench_model;
    let mut session;
    let model = match bench::src(part) {
        Some(src) => {
            bench_model = bench::model(src);
            &bench_model
        }
        None => {
            session = Session::default();
            session.model(part)?
        }
    };
    if let Some(e) = &model.error {
        return Err(format!("{part}: line {}: {}", model.lines[e.at].no, e.msg));
    }
    model.body().cloned().ok_or(format!("{part}: nothing built"))
}

fn build(part: &str) -> Result<PartMesh, String> {
    tess::build_part(&body(part)?).map_err(|e| format!("{part}: {e:?}"))
}

/// The mesh of a part, and whether it came from the cache.
pub fn open(part: &str, use_cache: bool) -> Result<(PartMesh, bool), String> {
    let src = source(part)?;
    if use_cache && let Some(m) = cache::load(&src) {
        return Ok((m, true));
    }
    let m = build(part)?;
    if use_cache {
        cache::store(&src, &m);
    }
    Ok((m, false))
}

type Opened = (usize, Result<(PartMesh, bool), String>);

/// Several parts, on up to four threads.
pub fn open_all(parts: &[&str], use_cache: bool) -> Result<(Vec<PartMesh>, usize), String> {
    let at = std::sync::atomic::AtomicUsize::new(0);
    let mut out: Vec<Opened> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                s.spawn(|| {
                    let mut got = Vec::new();
                    loop {
                        let i = at.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(p) = parts.get(i) else { return got };
                        got.push((i, open(p, use_cache)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("a worker panicked"))
            .collect()
    });
    out.sort_by_key(|x| x.0);
    let mut meshes = Vec::with_capacity(out.len());
    let mut hits = 0;
    for (_, r) in out {
        let (m, hit) = r?;
        hits += hit as usize;
        meshes.push(m);
    }
    Ok((meshes, hits))
}

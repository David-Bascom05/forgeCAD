//! Opens parts: from the build cache, or built by the kernel and then cached.

use crate::mesh::PartMesh;
use crate::{cache, tess};
use forge_agent::bench;
use forge_agent::cmd::Session;

/// The part's file text; a bench part's name or a path.
pub fn source(part: &str) -> Result<String, String> {
    match bench::src(part) {
        Some(s) => Ok(s.to_string()),
        None if !part.ends_with(".fc") && !part.ends_with(".fg") => {
            Err(format!("{part}: no bench part or .fc or .fg part file"))
        }
        None => std::fs::read_to_string(part).map_err(|e| format!("{part}: {e}")),
    }
}

/// The part's body, built by the kernel: the one its mesh's entity numbers mean.
pub fn body(part: &str) -> Result<forge_topo::Body, String> {
    if part.ends_with(".fg") {
        return fg_body(part);
    }
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
        return Err(format!("{part}: line {}: {}", model.lines[e.at].no, e.message()));
    }
    model.body().cloned().ok_or(format!("{part}: nothing built"))
}

/// A Forge part's finished body, built by forge-lang; its first error if it doesn't build.
fn fg_body(part: &str) -> Result<forge_topo::Body, String> {
    let src = source(part)?;
    let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    if let Some(e) = run.errors.first() {
        let line = e.span().map_or(0, |s| s.lines(&src).0);
        return Err(format!("{part}: line {line}: {}: {}", e.kind(), e.text()));
    }
    run.body.ok_or(format!("{part}: nothing built"))
}

/// The part as tools act on it: a bench part in memory, a file with its commits going to the file.
pub fn tool_part(part: &str) -> Result<forge_agent::tools::Part, String> {
    let p = match bench::src(part) {
        Some(src) => forge_agent::tools::Part::of(bench::model(src)),
        None => Session::default().take_part(part)?,
    };
    if let Some(e) = &p.model.error {
        return Err(format!("{part}: line {}: {}", p.model.lines[e.at].no, e.message()));
    }
    Ok(p)
}

fn build(part: &str) -> Result<PartMesh, String> {
    tess::build_part(&body(part)?).map_err(|e| format!("{part}: {e:?}"))
}

thread_local! {
    /// Why the last `open` on this thread couldn't mesh what it opened.
    pub static MESH_FAILED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// The mesh of a part, and whether it came from the cache.
pub fn open(part: &str, use_cache: bool) -> Result<(PartMesh, bool), String> {
    let src = source(part)?;
    // a new part has nothing to build yet
    if src.trim().is_empty() {
        return Ok((PartMesh::empty(), false));
    }
    if use_cache && let Some(m) = cache::load(&src) {
        return Ok((m, true));
    }
    // sketches with no solid yet draw nothing; a solid that can't be meshed still opens, to be fixed
    let m = match build(part) {
        Err(e) if e.ends_with("nothing built") => return Ok((PartMesh::empty(), false)),
        Err(e) if body(part).is_ok() => {
            eprintln!("forge-app: can't mesh it: {e}");
            MESH_FAILED.with(|f| *f.borrow_mut() = Some(e));
            return Ok((PartMesh::empty(), false));
        }
        m => m?,
    };
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

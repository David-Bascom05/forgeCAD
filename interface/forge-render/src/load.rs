//! Opens parts: from the build cache, or built by forge-lang and then cached. A bench part opens
//! by its name: its Forge port in `interface/forge-lang/parts/bench`.

use crate::mesh::PartMesh;
use crate::{cache, tess};
use std::path::{Path, PathBuf};

/// The bench parts (`forge_lang::bench`), in the order the stress bench places them.
pub const BENCH: [&str; 15] = [
    "gearbox", "volute", "crank", "valve", "heatsink", "quadarm", "conrod", "jack", "frame", "manifold",
    "bracket", "insert", "clamp", "hub", "guard",
];

/// A bench part's Forge text.
pub fn bench_src(name: &str) -> Option<&'static str> {
    forge_lang::bench::src(name)
}

/// Where the bench parts are written to be opened as files, with the block font their text is
/// cut in beside them: once a process, each file only if it isn't already what it should be.
pub fn bench_dir() -> Result<PathBuf, String> {
    static DIR: std::sync::OnceLock<Result<PathBuf, String>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let base = std::env::var_os("FORGE_RENDER_CACHE")
            .map(PathBuf::from)
            .or_else(forge_dirs::cache)
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join("forgecad").join("bench");
        forge_dirs::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for n in BENCH {
            let src = bench_src(n).expect("a bench part");
            write_if_changed(&dir.join(format!("{n}.fg")), src.as_bytes())?;
        }
        // the font is made by code, which the kernel hash follows: made only when that changed
        let font = dir.join(forge_lang::bench::FONT);
        let made = dir.join(format!("{}.made", forge_lang::bench::FONT));
        let key = env!("FORGE_KERNEL_HASH");
        let fresh =
            forge_dirs::fs::exists(&font) && forge_dirs::fs::read_to_string(&made).is_ok_and(|k| k == key);
        if !fresh {
            write_if_changed(&font, &forge_lang::bench::font_bytes())?;
            write_if_changed(&made, key.as_bytes())?;
        }
        Ok(dir)
    })
    .clone()
}

// written whole and then moved into place, so a reader never sees half a file
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if forge_dirs::fs::read(path).is_ok_and(|b| b == bytes) {
        return Ok(());
    }
    let tmp = path.with_extension(format!("{}.tmp", forge_dirs::process_id()));
    forge_dirs::fs::write(&tmp, bytes)
        .and_then(|_| forge_dirs::fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = forge_dirs::fs::remove_file(&tmp);
            format!("{}: {e}", path.display())
        })
}

/// A bench part's file, by its name; none if it's no bench part's name.
pub fn bench_file(name: &str) -> Option<Result<String, String>> {
    bench_src(name)?;
    Some(bench_dir().map(|d| d.join(format!("{name}.fg")).to_string_lossy().into_owned()))
}

/// The part's file text; a bench part's name or a path.
pub fn source(part: &str) -> Result<String, String> {
    match bench_src(part) {
        Some(s) => Ok(s.to_string()),
        None if !part.ends_with(".fg") => Err(format!("{part}: no bench part or .fg part file")),
        None => forge_dirs::fs::read_to_string(part).map_err(|e| format!("{part}: {e}")),
    }
}

/// Where a part's fonts and imported files are found: beside it.
fn dir_of(part: &str) -> Result<Option<PathBuf>, String> {
    if bench_src(part).is_some() {
        return bench_dir().map(Some);
    }
    Ok(Path::new(part).parent().map(Path::to_path_buf))
}

/// The part's finished body, built by forge-lang: the one its mesh's entity numbers mean; its
/// first error if it doesn't build.
pub fn body(part: &str) -> Result<forge_topo::Body, String> {
    let src = source(part)?;
    // the shape only: its measures and expects are no part of its mesh
    let opts = forge_lang::Options {
        dir: dir_of(part)?,
        checks: false,
        expects: false,
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, Default::default());
    if let Some(e) = run.errors.first() {
        let line = e.span().map_or(0, |s| s.lines(&src).0);
        return Err(format!("{part}: line {line}: {}: {}", e.kind(), e.text()));
    }
    run.body.ok_or(format!("{part}: nothing built"))
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
    let out: Vec<Opened> = crate::plat::par_map(parts, 4, |p| (0, open(p, use_cache)));
    let mut meshes = Vec::with_capacity(out.len());
    let mut hits = 0;
    for (_, r) in out {
        let (m, hit) = r?;
        hits += hit as usize;
        meshes.push(m);
    }
    Ok((meshes, hits))
}

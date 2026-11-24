//! Every reader of a file ForgeCAD opens, fed real files made wrong in the ways files go wrong:
//! cut short, numbers made extreme, references pointed elsewhere, lists nested deep, counts made
//! huge, archive members corrupted and packed again. Each must end in a value or a typed error, in
//! bounded time and memory; a panic, a hang, an abort or a runaway is a finding.
//!
//! `read-fuzz` (src/main.rs) drives the readers here from seed files and keeps what it finds;
//! tests/corpus.rs replays what was found and fixed.

pub mod mutate;
pub mod zip;

use std::panic::{AssertUnwindSafe, catch_unwind};

/// One reader of one kind of file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// `forge_step::import` on STEP (part 21) text, then the import's own check and mesh.
    Step,
    /// The same reader, sniffing an IGES file.
    Iges,
    /// The same reader, sniffing OpenCascade `.brep` text.
    Brep,
    /// FreeCAD `.FCStd`: a zip of XML and `.brp` members, translated to Forge parts.
    Fcstd,
    /// DWG, any version.
    Dwg,
    /// A `.fg` part read and evaluated, stopping before the kernel (`fg check`).
    Fg,
    /// A `.fg` part built with the kernel (`fg build`).
    FgBuild,
    /// TrueType fonts (text features; drawings).
    Ttf,
    /// AutoCAD SHX fonts.
    Shx,
    /// JSON (MCP requests, edit ops).
    Json,
    /// Material libraries.
    Material,
    /// zlib and raw deflate streams (FCStd members, git objects).
    Inflate,
    /// Git trees, commits, tags, the index and pack deltas (forge-git reads a project's history).
    Git,
    /// STL, binary and ASCII, read as a mesh.
    Stl,
    /// Wavefront OBJ, read as a mesh.
    Obj,
    /// 3MF packages, read as a mesh.
    ThreeMf,
}

pub const TARGETS: &[(&str, Target)] = &[
    ("step", Target::Step),
    ("iges", Target::Iges),
    ("brep", Target::Brep),
    ("fcstd", Target::Fcstd),
    ("dwg", Target::Dwg),
    ("fg", Target::Fg),
    ("fg-build", Target::FgBuild),
    ("ttf", Target::Ttf),
    ("shx", Target::Shx),
    ("json", Target::Json),
    ("material", Target::Material),
    ("inflate", Target::Inflate),
    ("git", Target::Git),
    ("stl", Target::Stl),
    ("obj", Target::Obj),
    ("3mf", Target::ThreeMf),
];

impl Target {
    pub fn by_name(s: &str) -> Option<Target> {
        TARGETS.iter().find(|(n, _)| *n == s).map(|x| x.1)
    }
    pub fn name(self) -> &'static str {
        TARGETS.iter().find(|x| x.1 == self).map(|x| x.0).unwrap_or("?")
    }
    /// The file extensions this target's seeds come from.
    pub fn exts(self) -> &'static [&'static str] {
        match self {
            Target::Step => &["step", "stp"],
            Target::Iges => &["igs", "iges"],
            Target::Brep => &["brep", "brp"],
            Target::Fcstd => &["fcstd"],
            Target::Dwg => &["dwg"],
            Target::Fg | Target::FgBuild => &["fg"],
            Target::Ttf => &["ttf", "otf"],
            Target::Shx => &["shx"],
            Target::Json => &["json"],
            Target::Material => &["toml"],
            Target::Inflate => &["z", "zlib", "deflate"],
            Target::Git => &["gitobj"],
            Target::Stl => &["stl"],
            Target::Obj => &["obj"],
            Target::ThreeMf => &["3mf"],
        }
    }
    /// The mutations that know this format.
    pub fn format(self) -> mutate::Format {
        match self {
            Target::Step => mutate::Format::Step,
            Target::Iges => mutate::Format::Iges,
            Target::Brep => mutate::Format::Text,
            Target::Fcstd | Target::ThreeMf => mutate::Format::Zip,
            Target::Obj => mutate::Format::Text,
            Target::Stl => mutate::Format::Binary,
            Target::Fg | Target::FgBuild => mutate::Format::Fg,
            Target::Json | Target::Material => mutate::Format::Text,
            Target::Dwg | Target::Ttf | Target::Shx | Target::Inflate | Target::Git => mutate::Format::Binary,
        }
    }
}

/// How one input fared. `Ok` and `Refused` are both fine; `Panic` is a finding.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Ok,
    /// A typed error, its first words.
    Refused(String),
    /// Where it panicked and why.
    Panic(String),
}

impl Outcome {
    pub fn finding(&self) -> bool {
        matches!(self, Outcome::Panic(_))
    }
}

fn refused(e: impl std::fmt::Display) -> Outcome {
    let s = e.to_string();
    Outcome::Refused(s.chars().take(80).collect())
}

/// Runs one reader on one input, a panic caught and said where.
pub fn run(t: Target, data: &[u8]) -> Outcome {
    match catch_unwind(AssertUnwindSafe(|| read(t, data))) {
        Ok(o) => o,
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            let at = LAST_PANIC.with(|p| p.take()).unwrap_or_default();
            Outcome::Panic(format!("{at}: {}", msg.chars().take(160).collect::<String>()))
        }
    }
}

thread_local! {
    static LAST_PANIC: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Keeps each panic's location for `run`, and prints nothing.
pub fn quiet_panics() {
    std::panic::set_hook(Box::new(|info| {
        let at = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        LAST_PANIC.with(|p| *p.borrow_mut() = Some(at));
    }));
}

fn read(t: Target, data: &[u8]) -> Outcome {
    match t {
        Target::Step | Target::Iges | Target::Brep => step(data),
        Target::Fcstd => match forge_import::fcstd::translate(data) {
            Ok(_) => Outcome::Ok,
            Err(e) => refused(e),
        },
        Target::Dwg => match forge_dwg::file::read(data) {
            Ok(_) => Outcome::Ok,
            Err(e) => refused(e.0),
        },
        Target::Fg | Target::FgBuild => {
            let Ok(src) = std::str::from_utf8(data) else {
                return refused("not UTF-8");
            };
            let opts = forge_lang::Options {
                build: t == Target::FgBuild,
                ..Default::default()
            };
            let (r, _) = forge_lang::run(src, opts, Default::default());
            match r.errors.first() {
                None => Outcome::Ok,
                Some(e) => refused(format!("{e:?}")),
            }
        }
        Target::Ttf => {
            let a = forge_font::Font::parse(data.to_vec()).is_ok();
            let b = forge_dwg::fonts::ttf::TtfFont::parse(data.to_vec()).is_ok();
            if a && b { Outcome::Ok } else { refused("font") }
        }
        Target::Shx => match forge_dwg::fonts::shx::ShxFont::parse(data.to_vec()) {
            Ok(_) => Outcome::Ok,
            Err(e) => refused(format!("{e:?}")),
        },
        Target::Json => match forge_agent::json::Json::parse(&String::from_utf8_lossy(data)) {
            Ok(_) => Outcome::Ok,
            Err(e) => refused(e),
        },
        Target::Material => match forge_material::parse(&String::from_utf8_lossy(data)) {
            Ok(_) => Outcome::Ok,
            Err(e) => refused(format!("{e:?}")),
        },
        Target::Inflate => {
            let a = forge_git::zlib::inflate_zlib(data, 64 << 20).is_ok();
            let b = forge_git::zlib::inflate(data, 64 << 20).is_ok();
            if a || b { Outcome::Ok } else { refused("inflate") }
        }
        Target::Git => git(data),
        Target::Stl | Target::Obj | Target::ThreeMf => {
            let m = match t {
                Target::Stl => forge_mesh::read::stl(data),
                Target::Obj => forge_mesh::read::obj(data),
                _ => forge_import::threemf::read(data),
            };
            match m {
                Ok(m) => {
                    let _ = forge_mesh::read::summary(&m);
                    Outcome::Ok
                }
                Err(e) => refused(e),
            }
        }
    }
}

/// STEP, IGES or `.brep`: the import, then what a user does next with what it made (the check
/// and a mesh, as `fg check` and the viewer do).
fn step(data: &[u8]) -> Outcome {
    // READ_FUZZ_TRACE=1: each stage's time on stderr, to find what a slow case spends it on
    let trace = std::env::var_os("READ_FUZZ_TRACE").is_some();
    let t0 = std::time::Instant::now();
    let imp = match forge_step::import(data, &forge_step::Limits::default()) {
        Ok(i) => i,
        Err(e) => return refused(e),
    };
    if trace {
        eprintln!(
            "import {:?}: {} solids, {} refusals",
            t0.elapsed(),
            imp.solids.len(),
            imp.refusals.len()
        );
    }
    for s in &imp.solids {
        let t1 = std::time::Instant::now();
        let p = forge_topo::check(&s.body);
        let t2 = std::time::Instant::now();
        let q = forge_mesh::Quality {
            chord: (2e-3 * s.body.faces_box().diagonal()).clamp(1e-3, 1.0),
            ..forge_mesh::Quality::default()
        };
        let m = forge_mesh::tessellate::tessellate_tolerant(&s.body, &q);
        if trace {
            eprintln!(
                "solid #{} faces {}: check {:?} ({} problems), mesh {:?} ({})",
                s.entity,
                s.body.faces.len(),
                t2 - t1,
                p.len(),
                t2.elapsed(),
                if m.is_ok() { "ok" } else { "refused" }
            );
        }
    }
    Outcome::Ok
}

/// A git object or index, by its first byte: the rest is read as that kind.
fn git(data: &[u8]) -> Outcome {
    use forge_git::{Commit, HashKind, Tag};
    let Some((&k, d)) = data.split_first() else {
        return refused("empty");
    };
    let ok = match k % 5 {
        0 => forge_git::object::parse_tree(HashKind::Sha1, d).is_ok(),
        1 => Commit::parse(HashKind::Sha1, d).is_ok(),
        2 => Tag::parse(HashKind::Sha1, d).is_ok(),
        3 => forge_git::index::Index::parse(d, HashKind::Sha1).is_ok(),
        _ => {
            // a delta against the first half
            let h = d.len() / 2;
            forge_git::pack::apply_delta(&d[..h], &d[h..]).is_ok()
        }
    };
    if ok { Outcome::Ok } else { refused("git") }
}

/// Peak resident memory since the last `reset_peak`, in kB (Linux; 0 elsewhere).
pub fn peak_kb() -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Current resident memory, in kB (Linux; 0 elsewhere).
pub fn rss_kb() -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Starts the peak over from now (Linux: clear_refs 5).
pub fn reset_peak() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

/// Seeds for a target: the files under `dirs` with its extensions, largest first cut to `max_bytes`
/// each, plus what it makes itself where the repository holds no file of the kind.
pub fn seeds(t: Target, dirs: &[std::path::PathBuf], max_bytes: usize) -> Vec<(String, Vec<u8>)> {
    let mut out = vec![];
    for d in dirs {
        walk(d, &mut |p| {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if t.exts().contains(&ext.as_str())
                && let Ok(b) = std::fs::read(p)
                && b.len() <= max_bytes
            {
                out.push((p.display().to_string(), b));
            }
        });
    }
    out.extend(made(t));
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn walk(d: &std::path::Path, f: &mut dyn FnMut(&std::path::Path)) {
    let Ok(rd) = std::fs::read_dir(d) else {
        if d.is_file() {
            f(d);
        }
        return;
    };
    let mut es: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    es.sort();
    for p in es {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if p.is_dir() {
            walk(&p, f);
        } else {
            f(&p);
        }
    }
}

/// Seeds made here: small FreeCAD documents, deflate streams, git objects and JSON.
fn made(t: Target) -> Vec<(String, Vec<u8>)> {
    match t {
        Target::Fcstd => zip::fcstd_seeds(),
        Target::Stl | Target::Obj | Target::ThreeMf => zip::mesh_seeds(t),
        Target::Inflate => {
            let text = include_bytes!("lib.rs");
            vec![
                ("made/zlib".into(), forge_git::zlib::deflate_zlib(text)),
                ("made/raw".into(), forge_git::zlib::deflate(text)),
                ("made/zeros".into(), forge_git::zlib::deflate(&[0u8; 1 << 16])),
            ]
        }
        Target::Git => {
            let mut tree = vec![0u8];
            for (m, n) in [("100644", "a.fg"), ("40000", "dir"), ("100755", "run.sh")] {
                tree.extend_from_slice(format!("{m} {n}\0").as_bytes());
                tree.extend_from_slice(&[0xab; 20]);
            }
            let commit = b"\x01tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\nparent 4b825dc642cb6eb9a060e54bf8d69288fbee4904\nauthor A <a@b> 1700000000 +0000\ncommitter A <a@b> 1700000000 +0000\n\nmessage\n".to_vec();
            let tag = b"\x02object 4b825dc642cb6eb9a060e54bf8d69288fbee4904\ntype commit\ntag v1\ntagger A <a@b> 1700000000 +0000\n\nv1\n".to_vec();
            // a delta: base size 8, result size 12, copy 0..8, insert 4
            let mut delta = vec![4u8];
            delta.extend_from_slice(b"abcdefgh");
            delta.extend_from_slice(&[8, 12, 0x90, 8, 4]);
            delta.extend_from_slice(b"ijkl");
            vec![
                ("made/tree".into(), tree),
                ("made/commit".into(), commit),
                ("made/tag".into(), tag),
                ("made/delta".into(), delta),
            ]
        }
        Target::Json => [
            r#"{"op":"SetParam","param":"t","value":{"value":3,"dim":"Length"}}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"build","arguments":{"file":"a.fg","ops":[{"op":"Undo"},{"op":"AddFeature","before":null,"text":"x = 1"}]}}}"#,
            r#"[1, -2.5e-3, true, false, null, "aé\n", [], {}]"#,
        ]
        .iter()
        .enumerate()
        .map(|(i, s)| (format!("made/{i}.json"), s.as_bytes().to_vec()))
        .collect(),
        _ => vec![],
    }
}

//! The commands, each answering in a few lines of text and in JSON, over a session that keeps
//! every file's build.

use crate::json::Json;
use crate::model::Model;
use crate::select::{self, Ent};
use crate::tools::Part;
use crate::{measure, render, short};
use forge_topo::{Body, check};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Args {
    pub file: String,
    /// The selector, the entities to measure, the view, or the other file.
    pub words: Vec<String>,
    /// Ask about the part after this feature, not the finished one.
    pub at: Option<String>,
    pub labels: Option<String>,
    pub px: Option<usize>,
    /// `act`: refused if the part has moved on from this version.
    pub base: Option<u64>,
}

pub struct Reply {
    pub ok: bool,
    pub text: String,
    pub json: Json,
    pub png: Option<Vec<u8>>,
}

fn fail(msg: impl Into<String>) -> Reply {
    let msg = msg.into();
    Reply {
        ok: false,
        text: format!("error: {msg}"),
        json: Json::obj(vec![("ok", false.into()), ("error", msg.into())]),
        png: None,
    }
}

pub const COMMANDS: [&str; 9] = [
    "act", "build", "query", "measure", "render", "check", "diff", "help", "export",
];

#[derive(Default)]
pub struct Session {
    parts: HashMap<String, Part>,
    /// Where files may be read from (serve and mcp); any `.fc` file when `None`.
    root: Option<std::path::PathBuf>,
    /// Bodies read from STEP files, by path, with the bytes they were read from: parsing is dear,
    /// and a file is known by its bytes, never its length and time.
    imported: Imported,
    /// Whether `act` may write files (off unless asked for).
    writable: bool,
}

type Imported = std::sync::Arc<
    std::sync::Mutex<HashMap<std::path::PathBuf, (std::sync::Arc<[u8]>, std::sync::Arc<forge_topo::Body>)>>,
>;

/// STEP files larger than this are refused unread.
const STEP_LIMIT: u64 = 2 << 30;

/// The solid an import line names: every solid of a STEP file as one body, found with links
/// resolved first and, in a rooted session, only under the root. What the file does not make a
/// valid solid of is refused, with the entities and problems named.
fn import_file(
    dir: &std::path::Path,
    root: Option<&std::path::Path>,
    name: &str,
    cache: &Imported,
) -> Result<std::sync::Arc<forge_topo::Body>, String> {
    let real = std::fs::canonicalize(dir.join(name)).ok().filter(|r| {
        r.extension().is_some_and(|x| {
            ["step", "stp", "brep", "igs", "iges"]
                .iter()
                .any(|k| x.eq_ignore_ascii_case(k))
        })
    });
    let refused = || format!("{name}: not a STEP, .brep or IGES file under the root");
    let r = match (root, real) {
        (Some(root), Some(r)) if r.starts_with(root) => r,
        (Some(_), _) => return Err(refused()),
        (None, Some(r)) => r,
        (None, None) => return Err(format!("{name}: no such .step, .stp, .brep, .igs or .iges file")),
    };
    let meta = std::fs::metadata(&r).map_err(|e| format!("{name}: {e}"))?;
    if !meta.is_file() || meta.len() > STEP_LIMIT {
        return Err(format!(
            "{name}: not a regular file of at most {} MB",
            STEP_LIMIT >> 20
        ));
    }
    let data = std::fs::read(&r).map_err(|e| format!("{name}: {e}"))?;
    if let Ok(c) = cache.lock()
        && let Some((d, b)) = c.get(&r)
        && **d == data[..]
    {
        return Ok(b.clone());
    }
    let imp =
        forge_step::import(&data, &forge_step::Limits::default()).map_err(|e| format!("{name}: {e}"))?;
    // as a part built here: what reads is imported, a solid that fails the check with the check's
    // marks, and what was refused is said; only a file with nothing to build is refused
    let said = |x: &forge_step::Refusal| format!("#{} {}: {}", x.entity, x.kind, x.why);
    if imp.solids.is_empty() {
        let first = imp.refusals.first().map_or_else(|| "no solid".to_string(), said);
        return Err(format!("{name}: nothing could be read; {first}"));
    }
    let mut body = forge_topo::Body::new();
    for s in &imp.solids {
        body.append(&s.body);
    }
    if !imp.refusals.is_empty() {
        let all: Vec<String> = imp.refusals.iter().map(said).collect();
        body.left_out = Some(format!(
            "{} of the file's {} solids could not be read and are left out: {}",
            imp.refusals.len(),
            imp.refusals.len() + imp.solids.len(),
            all.join("; ")
        ));
    }
    let body = std::sync::Arc::new(body);
    if let Ok(mut c) = cache.lock() {
        c.insert(r, (data.into(), body.clone()));
    }
    Ok(body)
}

/// Fonts larger than this are refused unread.
const FONT_LIMIT: u64 = 32 << 20;

/// The font a text line names: a .ttf or .otf file, relative to the part file's directory, read
/// with links resolved first and, in a rooted session, only under the root. Rooted, every refusal
/// reads the same and carries no OS error, as for part files.
fn font_file(
    dir: &std::path::Path,
    root: Option<&std::path::Path>,
    name: &str,
) -> Result<std::sync::Arc<forge_font::Font>, String> {
    use std::io::Read;
    let real = std::fs::canonicalize(dir.join(name)).ok().filter(|r| {
        r.extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("ttf") || x.eq_ignore_ascii_case("otf"))
    });
    let read = |r: &std::path::Path| -> Result<Vec<u8>, String> {
        let meta = std::fs::metadata(r).map_err(|e| e.to_string())?;
        if !meta.is_file() {
            return Err("not a regular file".into());
        }
        if meta.len() > FONT_LIMIT {
            return Err(format!("larger than {} MB", FONT_LIMIT >> 20));
        }
        // the limit again while reading, should the file grow in between
        let mut data = Vec::new();
        std::fs::File::open(r)
            .and_then(|f| f.take(FONT_LIMIT + 1).read_to_end(&mut data))
            .map_err(|e| e.to_string())?;
        if data.len() as u64 > FONT_LIMIT {
            return Err(format!("larger than {} MB", FONT_LIMIT >> 20));
        }
        Ok(data)
    };
    let parse = |data: Vec<u8>| forge_font::Font::parse(data).map(std::sync::Arc::new);
    match (root, real) {
        (Some(root), Some(r)) if r.starts_with(root) => read(&r)
            .ok()
            .and_then(|d| parse(d).ok())
            .ok_or_else(|| format!("{name}: not a font file under the root")),
        (Some(_), _) => Err(format!("{name}: not a font file under the root")),
        (None, Some(r)) => {
            // what the font reader says of it would be its bytes: only that it is no font
            parse(read(&r).map_err(|e| format!("{name}: {e}"))?)
                .map_err(|_| format!("{name}: not a font file"))
        }
        (None, None) => Err(format!("{name}: no such .ttf or .otf font file")),
    }
}

impl Session {
    /// A session that reads `.fc` files under `root` only.
    pub fn rooted(root: std::path::PathBuf) -> Session {
        Session {
            root: Some(root),
            ..Session::default()
        }
    }

    /// The session, letting `act` write the files it changes or not.
    pub fn writable(mut self, on: bool) -> Session {
        self.writable = on;
        self
    }

    /// The file's model, brought up to date with what is on disk.
    pub fn model(&mut self, file: &str) -> Result<&mut Model, String> {
        self.part_at(file).map(|p| &mut p.0.model)
    }

    fn model_at(&mut self, file: &str) -> Result<(&mut Model, std::path::PathBuf), String> {
        self.part_at(file).map(|p| (&mut p.0.model, p.1))
    }

    /// The file's part for its own use (a live session), its commits going to the file.
    pub fn take_part(&mut self, file: &str) -> Result<Part, String> {
        let (p, path) = self.part_at(file)?;
        p.open(&path);
        Ok(self.parts.remove(file).expect("just opened"))
    }

    /// `part_at`, for what would change the file.
    fn part_file(&mut self, file: &str) -> Result<(&mut Part, std::path::PathBuf), String> {
        let (p, path) = self.part_at(file)?;
        if !is_part(&p.model) {
            return Err(format!(
                "{file}: line 1 is not a feature; act changes part files only"
            ));
        }
        Ok((p, path))
    }

    /// The file's part, brought up to date, and its real path.
    fn part_at(&mut self, file: &str) -> Result<(&mut Part, std::path::PathBuf), String> {
        // every check, and the read, on the one path with links resolved. A directory on it
        // swapped for a link between the check and the read still escapes the root; closing
        // that needs openat with O_NOFOLLOW per component, which std does not offer.
        let real = std::fs::canonicalize(file)
            .ok()
            .filter(|r| r.extension().is_some_and(|x| x == "fc"));
        let (src, real) = match (&self.root, real) {
            // rooted, every refusal reads the same and no OS error goes back: a client learns
            // nothing about paths outside the root
            (Some(root), Some(r)) if r.starts_with(root) => (
                std::fs::read_to_string(&r).map_err(|_| format!("{file}: cannot read"))?,
                r,
            ),
            (Some(_), _) => return Err(format!("{file}: not a part file under the root")),
            (None, Some(r)) => (
                std::fs::read_to_string(&r).map_err(|e| format!("{file}: {e}"))?,
                r,
            ),
            (None, None) => return Err(format!("{file}: no such .fc part file")),
        };
        let dir = std::fs::canonicalize(file)
            .ok()
            .and_then(|r| r.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default();
        let dir2 = dir.clone();
        let root = self.root.clone();
        let part = self.parts.entry(file.to_string()).or_default();
        let m = &mut part.model;
        m.fonts = Some(std::sync::Arc::new(move |name: &str| {
            font_file(&dir, root.as_deref(), name)
        }));
        let (dir, root, cache) = (dir2, self.root.clone(), self.imported.clone());
        m.imports = Some(std::sync::Arc::new(move |name: &str| {
            import_file(&dir, root.as_deref(), name, &cache)
        }));
        m.update(&src);
        Ok((part, real))
    }

    /// Commits lines to the part: all of them or none.
    fn act(&mut self, a: &Args) -> Result<Reply, String> {
        if !self.writable {
            return Err("this session does not write files (serve and mcp take --writable)".into());
        }
        self.create(&a.file);
        let (p, path) = self.part_file(&a.file)?;
        p.open(&path);
        let d = p.act(&a.words.join("\n"), a.base);
        Ok(Reply {
            ok: d.ok,
            text: d.text.clone(),
            json: d.json(),
            png: None,
        })
    }

    /// A part file that isn't there yet, made empty: under the root, if there is one.
    fn create(&self, file: &str) {
        let path = std::path::Path::new(file);
        if path.exists() || path.extension().is_none_or(|x| x != "fc") {
            return;
        }
        let dir = match path.parent() {
            Some(d) if !d.as_os_str().is_empty() => d,
            _ => std::path::Path::new("."),
        };
        let inside = std::fs::canonicalize(dir)
            .is_ok_and(|d| self.root.as_ref().is_none_or(|root| d.starts_with(root)));
        if inside {
            let _ = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path);
        }
    }

    /// What `query` answers beside selectors: the file, numbers, history, what can be dragged.
    fn query_more(&mut self, a: &Args, what: &str) -> Result<Reply, String> {
        match what {
            "stat" => return self.build(a),
            "check" => return self.check(a),
            _ => {}
        }
        let (p, path) = self.part_at(&a.file)?;
        if what == "file" && !one_name(&path) {
            return Err(format!("{}: not a plain file with one name; not read", a.file));
        }
        p.open(&path);
        let text = match what {
            "file" => format!("v{}\n{}", p.version(), p.model.src()),
            "since" => {
                let n = a
                    .words
                    .get(1)
                    .and_then(|n| n.parse().ok())
                    .ok_or("query F since VERSION")?;
                p.since(n)
            }
            _ => p
                .dims()
                .iter()
                .map(|d| format!("{} = {}", d.name, short(d.value)))
                .collect::<Vec<_>>()
                .join("\n"),
        };
        Ok(Reply {
            ok: true,
            json: Json::obj(vec![("ok", true.into()), ("text", text.clone().into())]),
            text,
            png: None,
        })
    }

    pub fn run(&mut self, cmd: &str, a: &Args) -> Reply {
        if a.at.is_some() && cmd != "query" {
            return fail("--at goes with query only");
        }
        if (a.labels.is_some() || a.px.is_some()) && cmd != "render" {
            return fail("--labels and --px go with render only");
        }
        if a.base.is_some() && cmd != "act" {
            return fail("--base goes with act only");
        }
        let r = match cmd {
            "build" => self.build(a),
            "query" => self.query(a),
            "measure" => self.measure(a),
            "render" => self.render(a),
            "check" => self.check(a),
            "diff" => self.diff(a),
            "act" => self.act(a),
            "help" => Ok(help_reply(&a.file, a)),
            "export" => self.export(a),
            _ => Err(format!(
                "unknown command '{cmd}' (one of: {})",
                COMMANDS.join(" ")
            )),
        };
        r.unwrap_or_else(fail)
    }

    fn build(&mut self, a: &Args) -> Result<Reply, String> {
        let writable = self.writable;
        let (m, path) = self.model_at(&a.file)?;
        // a varying fillet's start, once found, goes into the file; told where it cannot
        let laws = m.fill_law_starts();
        if !laws.is_empty() && writable {
            replace(&path, m.src()).map_err(|e| format!("{}: {e}", a.file))?;
        }
        let unwritten = if writable { Vec::new() } else { laws };
        let n = m.lines.len();
        let mut text = String::new();
        let mut j: Vec<(&str, Json)> = vec![("lines", n.into()), ("built", m.built().into())];
        let ok = m.error.is_none();
        match m.error.clone() {
            None => {
                text.push_str(&format!("ok {n} lines {:.2}ms", m.ms));
                // a body a cut took whole: not an error, but not to pass unnoticed
                let removed = m.removed_bodies();
                for (line, body) in &removed {
                    text.push_str(&format!(
                        "\nline {line} removed the body made on line {body}: nothing of it was left"
                    ));
                }
                j.push((
                    "removed_bodies",
                    Json::Arr(
                        removed
                            .iter()
                            .map(|&(line, body)| {
                                Json::obj(vec![("line", line.into()), ("body", body.into())])
                            })
                            .collect(),
                    ),
                ));
                // a body that fails its own check is not to pass as good: written, and said
                let problems: Vec<_> = m
                    .solids_after(m.built())
                    .iter()
                    .flat_map(|s| check(&s.body))
                    .collect();
                if let Some(w) = not_valid(&problems) {
                    text.push_str(&w);
                }
                j.push(("problems", problems.len().into()));
                // a selector that picks fewer than meant fails quietly otherwise
                let picks = m.picks();
                if !picks.is_empty() {
                    let p: Vec<String> = picks.iter().map(|(l, c)| format!("line {l}: {c}")).collect();
                    text.push_str(&format!("\nselectors picked: {}", p.join(", ")));
                }
                // asked for, built, and it changed nothing: say so
                let idle = m.no_effect();
                if !idle.is_empty() {
                    let l: Vec<String> = idle.iter().map(|n| n.to_string()).collect();
                    text.push_str(&format!(
                        "\nchanged nothing (a tool that misses, or inside what it adds to): line {}",
                        l.join(", ")
                    ));
                    j.push(("no_effect", Json::Arr(idle.iter().map(|&n| n.into()).collect())));
                }
                // what a font left out (its kerning, say)
                for (line, f) in m.lines.iter().zip(&m.features) {
                    if let forge_model::Feature::Text(t) = f
                        && let Some(font) = &t.font.font
                    {
                        for w in &font.warnings {
                            text.push_str(&format!("\nwarning line {}: {}: {w}", line.no, t.font.name));
                        }
                    }
                }
            }
            Some(f) => {
                let line = &m.lines[f.at];
                // the line is echoed only once the file has shown itself a part file
                let echo = if f.at > 0 {
                    // a sketch block's header; its message quotes the row
                    line.toks.split(|t| t == ";").next().unwrap_or_default().join(" ")
                } else {
                    String::new()
                };
                text.push_str(&format!("error line {}: {}", line.no, f.msg));
                if !echo.is_empty() {
                    text.push_str(&format!("\n  {echo}"));
                }
                text.push_str(&format!("\nhint: {}", f.hint));
                j.push((
                    "error",
                    Json::obj(vec![
                        ("line", line.no.into()),
                        ("text", echo.into()),
                        ("msg", f.msg.into()),
                        ("hint", f.hint.into()),
                    ]),
                ));
            }
        }
        j.push(("ms", m.ms.into()));
        j.push(("rebuilt", m.rebuilt.into()));
        if m.body().is_some() {
            let (volume, area, bb, unmeshed, approx) = m.stats();
            let body = m.body().expect("built");
            let bbox = [bb.x.lo, bb.y.lo, bb.z.lo, bb.x.hi, bb.y.hi, bb.z.hi];
            let (f, e, v) = counts(body);
            // a failed build's numbers are in the JSON; the text keeps to the error
            let solids = m.solids_after(m.built());
            let ids: Vec<usize> = solids.iter().map(|s| s.id).collect();
            // each body's exact volume, else its share from the faces' cached triangles
            let exact: Vec<Option<f64>> = solids
                .iter()
                .map(|s| forge_topo::mass::mass(&s.body).ok().map(|p| p.volume))
                .collect();
            let mut vols = vec![0.0; ids.len()];
            let n = m.built();
            let faces = m.faces().to_vec();
            for (f, fm) in &faces {
                if let (Some(fm), Some(b)) = (fm, m.body_of(n, Ent::Face(*f)))
                    && let Some(k) = ids.iter().position(|&i| i == b)
                {
                    vols[k] += fm.volume;
                }
            }
            for (v, x) in vols.iter_mut().zip(exact) {
                *v = x.unwrap_or(*v);
            }
            if ok {
                text.push_str(&format!(
                    "\nvolume {} area {}{} bbox {}\nfaces {f} edges {e} verts {v}",
                    short(volume),
                    short(area),
                    approx
                        .as_ref()
                        .map_or(String::new(), |why| format!(" (approx: {why})")),
                    bbox.map(short).join(" ")
                ));
                if ids.len() > 1 {
                    let v: Vec<String> = ids
                        .iter()
                        .zip(&vols)
                        .map(|(&i, x)| format!("{} {}", m.feat_name(i), short(*x)))
                        .collect();
                    text.push_str(&format!("\nbodies {}: {}", ids.len(), v.join(", ")));
                }
            }
            j.push((
                "body_volumes",
                Json::Arr(vols.into_iter().map(Json::Num).collect()),
            ));
            if unmeshed > 0 {
                text.push_str(&format!("\n({unmeshed} faces would not mesh: bbox approximate)"));
            }
            for (no, sel) in &unwritten {
                text.push_str(&format!(
                    "\nline {no}: the law runs from {sel} (not written in: this session does not write files)"
                ));
            }
            j.push((
                "laws",
                Json::Arr(
                    unwritten
                        .into_iter()
                        .map(|(no, sel)| Json::obj(vec![("line", no.into()), ("from", sel.into())]))
                        .collect(),
                ),
            ));
            j.extend([
                ("volume", volume.into()),
                ("area", area.into()),
                ("bbox", Json::Arr(bbox.map(Json::Num).to_vec())),
                ("faces", f.into()),
                ("edges", e.into()),
                ("verts", v.into()),
                ("bodies", ids.len().into()),
            ]);
        }
        j.insert(0, ("ok", ok.into()));
        Ok(Reply {
            ok,
            text,
            json: Json::obj(j),
            png: None,
        })
    }

    /// The part after `--at`'s line, or as far as it built.
    fn upto(m: &Model, a: &Args) -> Result<Option<usize>, String> {
        a.at.as_deref()
            .map(|at| m.feat(m.built(), at).map(|i| i + 1))
            .transpose()
    }

    fn query(&mut self, a: &Args) -> Result<Reply, String> {
        if let Some(w) = a.words.first()
            && matches!(w.as_str(), "file" | "stat" | "check" | "since" | "dims")
        {
            let w = w.clone();
            return self.query_more(a, &w);
        }
        let m = self.model(&a.file)?;
        let sel = a.words.join(" ");
        let at = Self::upto(m, a)?;
        let (body, ents) = m.select(&sel, at)?;
        let n = at.unwrap_or(m.built()).min(m.built());
        let several = m.solids_after(n).len() > 1;
        let handles = select::handles(body);
        let mut list = Vec::new();
        // entities described alike share a line
        let mut lines: Vec<(Vec<String>, String)> = Vec::new();
        for &e in &ents {
            let h = handles[&select::name(body, e)].clone();
            let mut d = select::describe(body, e);
            if let Some(b) = m.body_of(n, e).filter(|_| several) {
                d.push_str(&format!(" body={}", m.feat_name(b)));
            }
            list.push(Json::obj(vec![
                ("h", h.as_str().into()),
                ("d", d.as_str().into()),
            ]));
            match lines.iter_mut().find(|l| l.1 == d) {
                Some(l) => l.0.push(h),
                None => lines.push((vec![h], d)),
            }
        }
        let mut text = format!("{} found", ents.len());
        for (hs, d) in lines.iter().take(25) {
            text.push_str(&format!("\n{} {d}", hs.join(" ")));
        }
        if lines.len() > 25 {
            text.push_str(&format!(
                "\n... {} more lines; narrow the selector",
                lines.len() - 25
            ));
        }
        Ok(Reply {
            ok: true,
            text,
            json: Json::obj(vec![("ok", true.into()), ("found", Json::Arr(list))]),
            png: None,
        })
    }

    fn measure(&mut self, a: &Args) -> Result<Reply, String> {
        let m = self.model(&a.file)?;
        let words: Vec<&str> = a.words.iter().map(|s| s.as_str()).collect();
        let ents = match words[..] {
            ["mass"] | ["wall"] => vec![],
            _ => words
                .iter()
                .map(|w| one(m, w))
                .collect::<Result<Vec<Ent>, String>>()?,
        };
        let mass = m.mass().cloned();
        let (exact, inexact) = match mass {
            Ok(x) => (Some(x), None),
            Err(why) => (None, Some(why)),
        };
        let (body, mesh) = m.ready()?;
        let mut pairs: Vec<(&str, f64)> = match words[..] {
            ["mass"] => match &exact {
                Some(ms) => vec![
                    ("volume", ms.volume),
                    ("area", ms.area),
                    ("cx", ms.centroid.x),
                    ("cy", ms.centroid.y),
                    ("cz", ms.centroid.z),
                    ("ixx", ms.inertia[0][0]),
                    ("iyy", ms.inertia[1][1]),
                    ("izz", ms.inertia[2][2]),
                    ("ixy", ms.inertia[0][1]),
                    ("iyz", ms.inertia[1][2]),
                    ("izx", ms.inertia[2][0]),
                    ("i1", ms.principal[0]),
                    ("i2", ms.principal[1]),
                    ("i3", ms.principal[2]),
                ],
                None => {
                    let ms = measure::mass(mesh);
                    vec![
                        ("volume", ms.volume),
                        ("area", ms.area),
                        ("cx", ms.centroid.x),
                        ("cy", ms.centroid.y),
                        ("cz", ms.centroid.z),
                        ("ixx", ms.inertia[0]),
                        ("iyy", ms.inertia[1]),
                        ("izz", ms.inertia[2]),
                        ("approx", 1.0),
                    ]
                }
            },
            ["wall"] => match measure::wall(body, mesh) {
                Some((t, p)) => vec![("wall", t), ("x", p.x), ("y", p.y), ("z", p.z)],
                None => return Err("no wall found".into()),
            },
            [_] => measure::one(body, mesh, ents[0]),
            [_, _] => measure::between(body, mesh, ents[0], ents[1]),
            _ => return Err("measure takes mass, wall, or one or two @handles / (selectors)".into()),
        };
        // a face's area from its surface
        if let ([Ent::Face(f)], Some(ms)) = (&ents[..], &exact)
            && let Some(&(_, a)) = ms.faces.iter().find(|x| x.0 == *f)
        {
            pairs.iter_mut().filter(|p| p.0 == "area").for_each(|p| p.1 = a);
        }
        let mut text = pairs
            .iter()
            .map(|(k, v)| format!("{k} {}", short(*v)))
            .collect::<Vec<_>>()
            .join(" ");
        let mut j = vec![("ok", true.into())];
        j.extend(pairs.iter().map(|&(k, v)| (k, Json::Num(v))));
        // from the mesh, and why not from the surfaces
        if let (["mass"], Some(why)) = (&words[..], &inexact) {
            text.push_str(&format!(" (from the mesh: {why})"));
            j.push(("why", why.as_str().into()));
        }
        Ok(Reply {
            ok: true,
            text,
            json: Json::obj(j),
            png: None,
        })
    }

    /// The part as a STEP file, in the reply's bytes.
    fn export(&mut self, a: &Args) -> Result<Reply, String> {
        let mut opt = forge_step::Options::default();
        let mut words = a.words.iter();
        while let Some(w) = words.next() {
            match w.as_str() {
                "ap214" => opt.schema = forge_step::Schema::Ap214,
                "ap242" => opt.schema = forge_step::Schema::Ap242,
                "tol" => {
                    opt.tol = words
                        .next()
                        .and_then(|t| t.parse().ok())
                        .filter(|t: &f64| *t > 0.0)
                        .ok_or("tol takes a length in mm")?;
                }
                _ => return Err(format!("export: unknown word '{w}' (ap214 ap242 tol MM)")),
            }
        }
        opt.name = std::path::Path::new(&a.file)
            .file_stem()
            .map_or_else(|| "part".into(), |n| n.to_string_lossy().into_owned());
        let m = self.model(&a.file)?;
        let body = m.body().ok_or("the part has no solid to export")?;
        let e = forge_step::export(body, &opt).map_err(|e| e.to_string())?;
        let text = format!(
            "{} solids, {} faces, {} edges; fitted {} faces (within {:.1e} mm), {} edges (within {:.1e} mm); accuracy stated {:.1e} mm",
            e.solids,
            e.faces,
            e.edges,
            e.fit_faces.count,
            e.fit_faces.error,
            e.fit_edges.count,
            e.fit_edges.error,
            e.uncertainty
        );
        let json = Json::obj(vec![
            ("ok", true.into()),
            ("solids", e.solids.into()),
            ("faces", e.faces.into()),
            ("edges", e.edges.into()),
            ("fit_faces", e.fit_faces.count.into()),
            ("fit_face_error", e.fit_faces.error.into()),
            ("fit_edges", e.fit_edges.count.into()),
            ("fit_edge_error", e.fit_edges.error.into()),
        ]);
        Ok(Reply {
            ok: true,
            text,
            json,
            png: Some(e.text.into_bytes()),
        })
    }

    fn render(&mut self, a: &Args) -> Result<Reply, String> {
        let m = self.model(&a.file)?;
        let view = a.words.first().map_or("iso", |s| s.as_str());
        let mut labels = Vec::new();
        if let Some(sel) = &a.labels {
            let (body, ents) = m.select(sel, None)?;
            let handles = select::handles(body);
            for e in ents.into_iter().take(40) {
                let h = handles[&select::name(body, e)]
                    .trim_start_matches('@')
                    .to_string();
                labels.push((h, select::centre(body, e)));
            }
        }
        let (body, mesh) = m.ready()?;
        let png = render::render(body, mesh, view, a.px.unwrap_or(400), &labels)?;
        let (w, h) = (
            u32::from_be_bytes(png[16..20].try_into().expect("IHDR")),
            u32::from_be_bytes(png[20..24].try_into().expect("IHDR")),
        );
        Ok(Reply {
            ok: true,
            text: format!(
                "{view} {w}x{h} {} bytes{}",
                png.len(),
                if labels.is_empty() {
                    ""
                } else {
                    ", labels are handles without @"
                }
            ),
            json: Json::obj(vec![
                ("ok", true.into()),
                ("w", (w as usize).into()),
                ("h", (h as usize).into()),
                ("bytes", png.len().into()),
            ]),
            png: Some(png),
        })
    }

    fn check(&mut self, a: &Args) -> Result<Reply, String> {
        let m = self.model(&a.file)?;
        let mut bad = Vec::new();
        // each body on its own: separate bodies may overlap, one body's shells may not
        for i in 0..m.built() {
            for s in m.solids_after(i + 1) {
                let p = check(&s.body);
                if !p.is_empty() {
                    bad.push((m.lines[i].no, p.len(), format!("{:?}", p[0])));
                    break;
                }
            }
        }
        let ok = bad.is_empty() && m.error.is_none();
        let mut text = if bad.is_empty() {
            format!("valid after each of {} lines", m.built())
        } else {
            bad.iter()
                .map(|(l, n, p)| format!("line {l}: {n} problems, first {p}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        if let Some(f) = &m.error {
            text.push_str(&format!("\nline {} does not build: {}", m.lines[f.at].no, f.msg));
        }
        // what a file's body leaves undone: said, not failed
        let warned = crate::model::tangled(m.solids_after(m.built()));
        for w in &warned {
            text.push_str(&format!("\nwarning ({}): {}", w.rule, w.reason));
        }
        let mut sketches = Vec::new();
        for (i, f) in m.features.iter().enumerate() {
            if let forge_model::Feature::Sketch(s) = f {
                let (dof, free) = crate::sketch::freedom(s);
                let no = m.lines[i].no;
                text.push_str(&match dof {
                    0 => format!("\nline {no}: sketch fully constrained"),
                    _ => format!(
                        "\nline {no}: sketch has {dof} degrees of freedom; free: {}",
                        free.join(" ")
                    ),
                });
                let mut driven = Vec::new();
                for &(k, v) in &s.driven {
                    let row = s.rows[k].join(" ");
                    let v = if s.rows[k][1] == "ang" {
                        v / forge_math::dmath::PI * 180.0
                    } else {
                        v
                    };
                    text.push_str(&format!("\n  `{row}` = {}", short(v)));
                    driven.push(Json::obj(vec![("row", row.into()), ("value", v.into())]));
                }
                let free = Json::Arr(free.into_iter().map(Json::from).collect());
                sketches.push(Json::obj(vec![
                    ("line", no.into()),
                    ("dof", dof.into()),
                    ("free", free),
                    ("driven", Json::Arr(driven)),
                ]));
            }
        }
        let list = bad
            .into_iter()
            .map(|(l, n, p)| {
                Json::obj(vec![
                    ("line", l.into()),
                    ("problems", n.into()),
                    ("first", p.into()),
                ])
            })
            .collect();
        Ok(Reply {
            ok,
            text,
            json: Json::obj(vec![
                ("ok", ok.into()),
                ("bad", Json::Arr(list)),
                (
                    "warnings",
                    Json::Arr(warned.into_iter().map(|w| Json::from(w.rule)).collect()),
                ),
                ("sketches", Json::Arr(sketches)),
            ]),
            png: None,
        })
    }

    fn diff(&mut self, a: &Args) -> Result<Reply, String> {
        let other = a.words.first().ok_or("diff A B: two files")?;
        let mut side = |f: &str| -> Result<_, String> {
            let m = self.model(f)?;
            let built = m.body().is_some().then(|| m.stats().0);
            Ok((m.lines.clone(), m.body().cloned(), built, m.error.is_some()))
        };
        let (la, ba, ma, ea) = side(&a.file)?;
        let (lb, bb, mb, eb) = side(other)?;
        let same = |x: &crate::text::Line, y: &crate::text::Line| x.label == y.label && x.toks == y.toks;
        let first = la
            .iter()
            .zip(&lb)
            .position(|(x, y)| !same(x, y))
            .or((la.len() != lb.len()).then(|| la.len().min(lb.len())));
        let mut text = match first {
            None => "same features".to_string(),
            Some(k) => format!(
                "first change: line {} (a has {} lines, b {})",
                lb.get(k).or(la.get(k)).map_or(0, |l| l.no),
                la.len(),
                lb.len()
            ),
        };
        let mut j = vec![
            ("ok", true.into()),
            (
                "first",
                first.map_or(Json::Null, |k| lb.get(k).or(la.get(k)).map_or(0, |l| l.no).into()),
            ),
        ];
        if ea || eb {
            text.push_str(&format!("\n{} does not build", if ea { "a" } else { "b" }));
        }
        if let (Some(va), Some(vb), Some(ba), Some(bb)) = (ma, mb, ba, bb) {
            let dv = short(vb - va);
            text.push_str(&format!(
                "\nvolume {} -> {} ({}{dv})",
                short(va),
                short(vb),
                if vb >= va { "+" } else { "" }
            ));
            j.push(("dvolume", (vb - va).into()));
            let names = |b: &Body| -> HashMap<forge_topo::Name, Ent> {
                b.face_ids()
                    .map(|f| (b.face(f).name, Ent::Face(f)))
                    .chain(b.edge_ids().map(|e| (b.edge(e).name, Ent::Edge(e))))
                    .collect()
            };
            let (na, nb) = (names(&ba), names(&bb));
            let hb = select::handles(&bb);
            let ha = select::handles(&ba);
            let added: Vec<_> = nb.iter().filter(|x| !na.contains_key(x.0)).collect();
            let gone: Vec<_> = na.iter().filter(|x| !nb.contains_key(x.0)).collect();
            let faces =
                |v: &[(&forge_topo::Name, &Ent)]| v.iter().filter(|x| matches!(x.1, Ent::Face(_))).count();
            text.push_str(&format!(
                "\nfaces +{} -{}, edges +{} -{}",
                faces(&added),
                faces(&gone),
                added.len() - faces(&added),
                gone.len() - faces(&gone)
            ));
            let mut shown = 0;
            for (n, e) in &added {
                if matches!(e, Ent::Face(_)) && shown < 5 {
                    text.push_str(&format!("\n+ {} {}", hb[n], select::describe(&bb, **e)));
                    shown += 1;
                }
            }
            for (n, e) in &gone {
                if matches!(e, Ent::Face(_)) && shown < 8 {
                    text.push_str(&format!("\n- {} {}", ha[n], select::describe(&ba, **e)));
                    shown += 1;
                }
            }
            j.extend([
                ("faces_added", faces(&added).into()),
                ("faces_removed", faces(&gone).into()),
            ]);
        }
        Ok(Reply {
            ok: true,
            text,
            json: Json::obj(j),
            png: None,
        })
    }
}

/// O_NOFOLLOW: open fails on a symbolic link (the value differs by platform).
#[cfg(unix)]
pub(crate) const NOFOLLOW: i32 = if cfg!(any(target_os = "macos", target_os = "ios", target_os = "freebsd")) {
    0x100
} else if cfg!(any(target_arch = "aarch64", target_arch = "arm")) {
    0o100000
} else {
    0o400000
};

/// Writes `src` over the part file at `path` (canonical, checked against the root): only if
/// the name is still that file, a regular file with no other names (not a link swapped in
/// since the check, nor a hard link to a file elsewhere), and through a new file in the same
/// directory renamed into place.
#[cfg(unix)]
/// The part file's metadata, if it may be written: a plain file with one name, here on its own
/// path. Refused, it has not been touched, and nothing of it should go anywhere else.
#[cfg(unix)]
pub(crate) fn writable_file(path: &std::path::Path) -> Result<std::fs::Metadata, String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let checked = std::fs::metadata(path).map_err(|_| "cannot write".to_string())?;
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(NOFOLLOW)
        .open(path)
        .map_err(|_| "not a plain file with one name; not written".to_string())?;
    let held = f.metadata().map_err(|_| "cannot write".to_string())?;
    if !held.file_type().is_file()
        || held.nlink() != 1
        || (held.dev(), held.ino()) != (checked.dev(), checked.ino())
    {
        return Err("not a plain file with one name; not written".into());
    }
    Ok(held)
}

#[cfg(unix)]
pub(crate) fn replace(path: &std::path::Path, src: &str) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let held = writable_file(path)?;
    let dir = path.parent().ok_or("cannot write")?;
    let name = path.file_name().ok_or("cannot write")?.to_string_lossy();
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(held.mode() & 0o777)
        .open(&tmp)
        .and_then(|mut t| std::io::Write::write_all(&mut t, src.as_bytes()).and_then(|_| t.sync_all()))
        .and_then(|_| std::fs::rename(&tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return Err("cannot write".into());
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn writable_file(_: &std::path::Path) -> Result<std::fs::Metadata, String> {
    Err("writing part files needs a unix system".into())
}

#[cfg(not(unix))]
pub(crate) fn replace(_: &std::path::Path, _: &str) -> Result<(), String> {
    Err("writing part files needs a unix system".into())
}

/// A file named `.fc` is a part, however broken (only those are opened at all), unless its first
/// line starts with no feature or parameter: then it isn't changed.
fn is_part(m: &Model) -> bool {
    let Some(f) = m.error.as_ref().filter(|f| f.at == 0) else {
        return true;
    };
    let t = &m.lines[f.at].toks;
    crate::text::kind(&t[0]).is_some() || t.get(1).is_some_and(|w| w == "=")
}

/// A file read back whole has one name, as one that is written must: a second name in the root
/// is no way to read a file that was never a part.
#[cfg(unix)]
fn one_name(path: &std::path::Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.nlink() == 1)
}

#[cfg(not(unix))]
fn one_name(_: &std::path::Path) -> bool {
    true
}

fn help_reply(topic: &str, a: &Args) -> Reply {
    let mut words = vec![topic];
    words.extend(a.words.iter().map(String::as_str));
    let text = crate::help::help(&words.join(" "));
    Reply {
        ok: true,
        json: Json::obj(vec![("ok", true.into()), ("text", text.clone().into())]),
        text,
        png: None,
    }
}

fn counts(b: &Body) -> (usize, usize, usize) {
    (b.face_ids().count(), b.edge_ids().count(), b.vertices.len())
}

/// The one entity of the finished part a handle or selector names.
fn one(m: &Model, r: &str) -> Result<Ent, String> {
    if r.starts_with('@') {
        return select::by_handle(m.body().ok_or("no part yet")?, r);
    }
    let (_, ents) = m.select(r, None)?;
    match ents[..] {
        [e] => Ok(e),
        _ => Err(format!(
            "'{r}' selects {} entities; measure needs one",
            ents.len()
        )),
    }
}

/// What `build` says of a body that fails its check.
fn not_valid(problems: &[forge_topo::Problem]) -> Option<String> {
    let first = problems.first()?;
    Some(format!(
        "\nwarning: the part is not valid: {} problems, first {first:?}",
        problems.len()
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) struct Dir(pub(crate) std::path::PathBuf);

    impl Dir {
        pub(crate) fn new(name: &str) -> Dir {
            // wasi has no temp_dir or pid; its runner opens the whole filesystem
            let (base, id) = if cfg!(target_os = "wasi") {
                let t = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock");
                (std::path::PathBuf::from("/tmp"), t.as_nanos() as u64)
            } else {
                (std::env::temp_dir(), std::process::id() as u64)
            };
            let d = base.join(format!("forge-agent-{id}-{name}"));
            std::fs::create_dir_all(&d).expect("temp dir");
            Dir(d)
        }
        pub(crate) fn file(&self, name: &str, src: &str) -> String {
            let p = self.0.join(name);
            std::fs::write(&p, src).expect("write");
            p.to_string_lossy().into_owned()
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn args(file: &str, words: &[&str]) -> Args {
        Args {
            file: file.into(),
            words: words.iter().map(|w| w.to_string()).collect(),
            ..Args::default()
        }
    }

    const V1: &str = "base: extrude 8 rect -40 -25 80 50
wall: extrude 8 xz at 0 25 0 rect -40 0 80 45
hole 6.5 -xy at -28 -13 8 thru
pattern hole1 lin 1 0 0 3 28
hole 8 -y at 0 25 30 thru
fillet 4 (edges between base wall concave)
";

    /// A body that fails its check is said to; a sound one says nothing.
    #[test]
    fn build_warns_of_an_invalid_body() {
        let bad = forge_topo::Problem::FaceWrongSide {
            face: forge_topo::FaceId(3),
            coedge: forge_topo::CoedgeId(7),
        };
        assert!(not_valid(&[bad]).is_some_and(|w| w.contains("not valid: 1 problems")));
        assert!(not_valid(&[]).is_none());
        let dir = Dir::new("invalid");
        let g = dir.file("b.fc", "extrude 8 rect 0 0 20 10\n");
        let r = Session::default().run("build", &args(&g, &[]));
        assert!(r.ok && !r.text.contains("warning"), "{}", r.text);
    }

    /// A varying fillet law left to its default start gets it written into the file at its
    /// first build, which builds the same part and stays put.
    #[cfg(unix)]
    #[test]
    fn law_starts_are_written_in() {
        let dir = Dir::new("laws");
        let src = "base: extrude 8 rect 0 0 20 10\nfillet 1..3 (edges front top)\n";
        let f = dir.file("a.fc", src);
        let mut s = Session::default().writable(true);
        let r = s.run("build", &args(&f, &[]));
        assert!(r.ok, "{}", r.text);
        let written = std::fs::read_to_string(&f).expect("file");
        assert!(
            written.contains("fillet 1..3 from (verts near 0 0 8) (edges front top)"),
            "{written}"
        );
        let volume = |t: &str| {
            t.split_whitespace()
                .skip_while(|w| *w != "volume")
                .nth(1)
                .map(String::from)
        };
        let r2 = s.run("build", &args(&f, &[]));
        assert_eq!(std::fs::read_to_string(&f).expect("file"), written);
        assert_eq!(volume(&r.text), volume(&r2.text));
        // a session that does not write says what it would have written
        let g = dir.file("b.fc", src);
        let r3 = Session::default().run("build", &args(&g, &[]));
        assert!(
            r3.text.contains("line 2: the law runs from (verts near 0 0 8)"),
            "{}",
            r3.text
        );
        assert_eq!(std::fs::read_to_string(&g).expect("file"), src);
    }

    const BODIES: &str = "base: extrude 10 rect 0 0 10 10
lid: extrude new 4 at 0 0 8 rect 0 0 10 10
extrude cut 12 at 5 5 -1 circle 0 0 2 in base
fillet 1 (edges body lid top along +x)
split plane at 0 0 10 in lid
splitface (faces body base normal -y) by plane z 1 0 0 at 5 0 0
spare: copybody by 30 0 0 in base
movebody rot 0 0 0 0 0 1 90 in spare
combine cut base lid keep
";

    /// A part of several bodies: built, checked body by body, queried by body, and the same
    /// as the part regenerated whole.
    #[test]
    fn several_bodies() {
        let dir = Dir::new("bodies");
        let f = dir.file("b.fc", BODIES);
        let mut s = Session::default();
        let r = s.run("build", &args(&f, &[]));
        assert!(r.ok, "{}", r.text);
        assert!(r.text.contains("\nbodies 4: base "), "{}", r.text);
        let r = s.run("check", &args(&f, &[]));
        assert!(r.ok, "{}", r.text);
        let r = s.run("query", &args(&f, &["faces body spare"]));
        assert!(r.text.starts_with("8 found"), "{}", r.text);
        assert!(
            r.text.lines().skip(1).all(|l| l.ends_with("body=spare")),
            "{}",
            r.text
        );
        let r = s.run("query", &args(&f, &["faces body lid"]));
        let lid = r.text.lines().skip(1).count();
        let r = s.run("query", &args(&f, &["faces body split1"]));
        let upper = r.text.lines().skip(1).count();
        assert!(lid > 0 && upper > 0, "{}", r.text);
        // the file is what the part regenerates
        let m = s.model(&f).expect("model");
        let part = forge_model::Part {
            features: m.features.clone(),
        };
        let whole = part.regenerate_solids().expect("regenerates");
        let built = m.solids_after(m.built());
        assert_eq!(whole.len(), built.len());
        for (a, b) in whole.iter().zip(built) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.body, b.body.compact());
        }
        let nary = part.regenerate_nary_solids(2).expect("n-ary");
        assert_eq!(nary.iter().map(|s| s.id).collect::<Vec<_>>(), [0, 1, 4, 6]);
        // and prints back as it reads
        let back = crate::text::parse(&crate::text::print(&part)).expect("parses");
        assert_eq!(back, part);
    }

    /// A file whose lines keep ids away from their places builds the same through every entry
    /// point: the edits that put them there, a fresh read, the CLI's session and the regens
    /// given the ids. Read as a plain part, by places, it is refused.
    #[test]
    fn ids_out_of_place_build_alike_everywhere() {
        use forge_model::Solid;
        use forge_topo::Name;
        let mut m = Model::new(BODIES);
        let items = m.doc.read_items("t = 1");
        let first = m.doc.items[0].id;
        m.edit(forge_doc::Op::Insert {
            before: Some(first),
            items,
        })
        .expect("insert");
        let src = m.src().to_string();
        assert!(src.starts_with("id 10 t = 1\nbase: id 1 extrude"), "{src}");
        let want: Vec<Body> = Model::new(BODIES)
            .solids_after(9)
            .iter()
            .map(|s| s.body.compact())
            .collect();
        let bodies = |m: &Model| -> Vec<Body> {
            m.solids_after(m.built())
                .iter()
                .map(|s| s.body.compact())
                .collect()
        };
        assert_eq!(bodies(&m), want, "edited");
        assert_eq!(bodies(&Model::new(&src)), want, "read afresh");
        let dir = Dir::new("ids");
        let f = dir.file("i.fc", &src);
        let mut s = Session::default();
        let r = s.run("build", &args(&f, &[]));
        assert!(r.ok && r.text.contains("\nbodies 4: base "), "{}", r.text);
        assert_eq!(bodies(s.model(&f).expect("model")), want, "session");
        let ids: Vec<u64> = m.lines.iter().map(|l| l.id).collect();
        let part = forge_model::Part {
            features: m.features.clone(),
        };
        let seq = part.regenerate_solids_ids(Some(&ids)).expect("regenerates");
        assert_eq!(
            seq.iter().map(|s| s.body.clone()).collect::<Vec<_>>(),
            want,
            "regen"
        );
        let nary = part.regenerate_nary_solids_ids(2, Some(&ids)).expect("n-ary");
        let names = |b: &[Solid]| {
            let mut n: Vec<Name> = b
                .iter()
                .flat_map(|s| s.body.face_ids().map(|f| s.body.face(f).name))
                .collect();
            n.sort();
            n
        };
        assert_eq!(names(&nary), names(&seq), "n-ary");
        // by places, the names the file holds are not found
        assert!(part.regenerate_solids().is_err());
        assert!(crate::text::parse(&src).is_err());
    }

    /// The loop an agent runs: write a bracket, build, fix an error, query an edge, fillet
    /// it, render. Everything it writes and reads is counted.
    #[test]
    fn agent_loop_is_small() {
        let dir = Dir::new("loop");
        let f = dir.file("b.fc", V1);
        let mut s = Session::default();
        let mut cost: Vec<(&str, usize)> = vec![("write file", V1.len())];
        let step = |what: &'static str,
                    cmd: &str,
                    words: &[&str],
                    s: &mut Session,
                    cost: &mut Vec<(&str, usize)>| {
            let r = s.run(cmd, &args(&f, words));
            let line = format!("forgecad {cmd} b.fc {}", words.join(" "));
            cost.push((what, line.len() + r.text.len()));
            eprintln!("$ {line}\n{}", r.text);
            r
        };
        let r = step("build", "build", &[], &mut s, &mut cost);
        assert!(!r.ok && r.text.starts_with("error line 5"), "{}", r.text);
        let (old, new) = ("hole 8 -y at", "hole 8 xz at");
        std::fs::write(&f, V1.replace(old, new)).expect("write");
        let r = step("fix + build", "build", &[], &mut s, &mut cost);
        cost.push(("edit", old.len() + new.len()));
        assert!(r.ok, "{}", r.text);
        let r = step(
            "query",
            "query",
            &["edges convex top along +x"],
            &mut s,
            &mut cost,
        );
        let handles: Vec<String> = r
            .text
            .lines()
            .skip(1)
            .map(|l| l.split(' ').next().expect("handle").to_string())
            .collect();
        assert_eq!(handles.len(), 2, "{}", r.text);
        let add = format!("fillet 2 {}\n", handles.join(" "));
        std::fs::write(&f, V1.replace(old, new) + &add).expect("write");
        cost.push(("add fillet", add.len()));
        let r = step("build", "build", &[], &mut s, &mut cost);
        assert!(r.ok, "{}", r.text);
        let r = step("render", "render", &[], &mut s, &mut cost);
        let (w, h) = (
            r.json.get("w").and_then(Json::num).expect("w"),
            r.json.get("h").and_then(Json::num).expect("h"),
        );
        // an image costs about (w h / 750) tokens
        let image = (w * h / 750.0) as usize;
        let chars: usize = cost.iter().map(|c| c.1).sum();
        for (what, n) in &cost {
            eprintln!("{what:>12} {n:>5} chars");
        }
        eprintln!(
            "total {chars} chars, ~{} tokens + image ~{image} tokens",
            chars / 4
        );
        assert!(chars < 1600, "{chars} chars");
    }

    #[test]
    fn reads_only_part_files_under_the_root() {
        let dir = Dir::new("root");
        let inside = dir.file("p.fc", "extrude 5 rect 0 0 10 10\n");
        let key = dir.file("key.pem", "-----BEGIN PRIVATE KEY-----\n");
        let outside = Dir::new("outside");
        let away = outside.file("q.fc", "extrude 5 rect 0 0 10 10\n");
        let mut s = Session {
            root: Some(std::fs::canonicalize(&dir.0).expect("root")),
            ..Session::default()
        };
        assert!(s.run("build", &args(&inside, &[])).ok);
        let r = s.run("build", &args(&key, &[]));
        assert!(
            !r.ok && r.text.contains("not a part file under the root") && !r.text.contains("BEGIN"),
            "{}",
            r.text
        );
        let r = s.run("build", &args(&away, &[]));
        assert!(
            !r.ok && r.text.contains("not a part file under the root"),
            "{}",
            r.text
        );
        // outside the root, a file that exists and one that does not read the same
        let gone = format!("{}/nothing.fc", outside.0.display());
        let (a, b) = (
            s.run("build", &args(&away, &[])),
            s.run("build", &args(&gone, &[])),
        );
        assert_eq!(a.text.replace(&away, "F"), b.text.replace(&gone, "F"));
        assert!(!b.text.contains("os error"), "{}", b.text);
        let sneaky = format!(
            "{}/../{}/q.fc",
            dir.0.display(),
            outside.0.file_name().expect("name").to_string_lossy()
        );
        assert!(!s.run("build", &args(&sneaky, &[])).ok);
        // links are followed before any check: to a file outside, or to one not a part file
        #[cfg(unix)]
        {
            let out_link = dir.0.join("out.fc");
            let key_link = dir.0.join("key.fc");
            std::os::unix::fs::symlink(&away, &out_link).expect("link");
            std::os::unix::fs::symlink(&key, &key_link).expect("link");
            let r = s.run("build", &args(&out_link.to_string_lossy(), &[]));
            assert!(
                !r.ok && r.text.contains("not a part file under the root"),
                "{}",
                r.text
            );
            let r = s.run("build", &args(&key_link.to_string_lossy(), &[]));
            assert!(
                !r.ok && r.text.contains("not a part file under the root"),
                "{}",
                r.text
            );
            let r = Session::default().run("build", &args(&key_link.to_string_lossy(), &[]));
            assert!(!r.ok && !r.text.contains("BEGIN"), "{}", r.text);
        }
        // a file that is not a part file is never echoed, even named .fc
        let secret = dir.file("s.fc", "hunter2 swordfish\nmore\n");
        let r = s.run("build", &args(&secret, &[]));
        assert!(
            !r.ok && !r.text.contains("hunter2") && !r.json.to_string().contains("hunter2"),
            "{}",
            r.text
        );
        // nor changed, nor echoed by an edit, even in a session that writes
        let mut w = Session::rooted(std::fs::canonicalize(&dir.0).expect("root")).writable(true);
        let r = w.run("act", &args(&secret, &["mv f1 end"]));
        assert!(
            !r.ok && !r.text.contains("hunter2") && !r.json.to_string().contains("hunter2"),
            "{}",
            r.text
        );
        assert_eq!(
            std::fs::read_to_string(&secret).expect("read"),
            "hunter2 swordfish\nmore\n"
        );
    }

    /// A part exported, and imported into another with a hole in it: the STEP file beside the
    /// part file, features on top, names steady; rooted, never outside the root.
    #[test]
    fn a_step_file_is_a_feature() {
        let dir = Dir::new("steproot");
        let block = dir.file("block.fc", "extrude 10 rect 0 0 40 20\n");
        let mut s = Session::default();
        let e = s.run("export", &args(&block, &[]));
        assert!(e.ok, "{}", e.text);
        let step = dir.0.join("b.step");
        std::fs::write(&step, e.png.expect("the file")).expect("write");
        let part = dir.file("p.fc", "import \"b.step\"\nhole 4 -xy at 20 10 10 thru\n");
        let r = s.run("measure", &args(&part, &["mass"]));
        assert!(r.ok, "{}", r.text);
        // the hole goes right through: a through cut sees what was imported
        let want = 8000.0 - forge_math::dmath::PI * 4.0 * 10.0;
        assert!(r.text.contains(&format!("volume {want:.1}")), "{}", r.text);
        let c = s.run("check", &args(&part, &[]));
        assert!(c.ok && !c.text.contains("warning"), "{}", c.text);
        // a file with its first point lifted: its integrals and mesh disagree, imported and warned
        let text = String::from_utf8(std::fs::read(&step).expect("read")).expect("text");
        let off = text.replacen("(0.,0.,0.)", "(0.,0.,4.)", 1);
        assert_ne!(off, text);
        std::fs::write(dir.0.join("off.step"), off).expect("write");
        let warned = dir.file("w.fc", "import \"off.step\"\n");
        let c = s.run("check", &args(&warned, &[]));
        assert!(
            c.ok && c.text.contains("warning (import-not-verified)"),
            "{}",
            c.text
        );
        // a file with a second solid that cannot be read: the first is imported, the other said
        let more = text.replacen(
            "ENDSEC;\nEND-ISO",
            "#999999=MANIFOLD_SOLID_BREP('',#999998);\nENDSEC;\nEND-ISO",
            1,
        );
        assert_ne!(more, text);
        std::fs::write(dir.0.join("more.step"), more).expect("write");
        let part = dir.file("m.fc", "import \"more.step\"\n");
        let c = s.run("check", &args(&part, &[]));
        assert!(
            c.ok && c
                .text
                .contains("warning (import-incomplete): 1 of the file's 2 solids"),
            "{}",
            c.text
        );
        // a solid that fails the check is imported, with the check's own marks
        // (a side face said the wrong way round: wrong side of its loop, still closed)
        let k = text.match_indices("ADVANCED_FACE").nth(2).expect("a face").0;
        let bad = format!("{}{}", &text[..k], text[k..].replacen(".T.)", ".F.)", 1));
        std::fs::write(dir.0.join("bad.step"), bad).expect("write");
        let part = dir.file("n.fc", "import \"bad.step\"\n");
        let b = s.run("build", &args(&part, &[]));
        assert!(b.ok, "{}", b.text);
        let c = s.run("check", &args(&part, &[]));
        assert!(!c.ok && c.text.contains("problems"), "{}", c.text);
        // no file: an error on its line
        let gone = dir.file("q.fc", "import \"nope.step\"\n");
        assert!(!s.run("build", &args(&gone, &[])).ok);
        // a file that is no STEP: an error, not a panic
        dir.file("bad.step", "this is not one");
        let bad = dir.file("r.fc", "import \"bad.step\"\n");
        assert!(!s.run("build", &args(&bad, &[])).ok);
        // rooted, a file outside the root is refused
        let outside = Dir::new("stepout");
        std::fs::write(outside.0.join("o.step"), std::fs::read(&step).expect("read")).expect("write");
        let mut rooted = Session {
            root: Some(std::fs::canonicalize(&dir.0).expect("root")),
            ..Session::default()
        };
        let away = dir.file(
            "s.fc",
            &format!(
                "import \"../{}/o.step\"\n",
                outside.0.file_name().expect("name").to_string_lossy()
            ),
        );
        assert!(!rooted.run("build", &args(&away, &[])).ok);
    }

    /// Text finds its font beside the part file; rooted, never outside the root, never a file
    /// that is not a font, a link out, or a huge file, and every refusal reads the same.
    #[test]
    fn fonts_are_read_like_part_files() {
        let font = forge_font::testing::build(
            1000,
            &[
                (
                    'I',
                    forge_font::testing::TestGlyph::bar(100.0, 0.0, 200.0, 700.0, 300.0),
                ),
                ('O', forge_font::testing::square_ring()),
            ],
            &[],
        );
        let dir = Dir::new("fontroot");
        let outside = Dir::new("fontout");
        std::fs::write(dir.0.join("f.ttf"), &font).expect("write");
        std::fs::write(outside.0.join("o.ttf"), &font).expect("write");
        let key = dir.file("key.pem", "-----BEGIN PRIVATE KEY-----\n");
        std::fs::create_dir_all(dir.0.join("d.ttf")).expect("dir");
        // a huge file costs nothing on disk: it is never read
        std::fs::File::create(dir.0.join("big.ttf"))
            .and_then(|f| f.set_len(FONT_LIMIT + 1))
            .expect("big");
        let part = |font: &str| {
            let name = format!("p{}.fc", font.len());
            dir.file(
                &name,
                &format!("extrude 5 rect 0 0 40 20\ntext \"IOI\" font \"{font}\" 10 1 xy at 2 2 5\n"),
            )
        };
        let mut s = Session {
            root: Some(std::fs::canonicalize(&dir.0).expect("root")),
            ..Session::default()
        };
        let r = s.run("build", &args(&part("f.ttf"), &[]));
        assert!(r.ok, "{}", r.text);
        let away = format!(
            "../{}/o.ttf",
            outside.0.file_name().expect("name").to_string_lossy()
        );
        let mut refused = vec![
            away.clone(),
            "key.pem".into(),
            "d.ttf".into(),
            "big.ttf".into(),
            "none.ttf".into(),
        ];
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.0.join("o.ttf"), dir.0.join("out.ttf")).expect("link");
            std::os::unix::fs::symlink(&key, dir.0.join("key.ttf")).expect("link");
            refused.extend(["out.ttf".into(), "key.ttf".into()]);
        }
        for name in &refused {
            let r = s.run("build", &args(&part(name), &[]));
            assert!(!r.ok, "{name}: {}", r.text);
            assert!(
                r.text
                    .contains(&format!("{name}: not a font file under the root")),
                "{}",
                r.text
            );
            assert!(
                !r.text.contains("os error") && !r.text.contains("BEGIN") && !r.text.contains("MB"),
                "{}",
                r.text
            );
        }
        // not rooted, a font anywhere reads, and refusals say why
        let mut free = Session::default();
        assert!(free.run("build", &args(&part(&away), &[])).ok);
        let r = free.run("build", &args(&part("big.ttf"), &[]));
        assert!(!r.ok && r.text.contains("larger than 32 MB"), "{}", r.text);
        let r = free.run("build", &args(&part("key.pem"), &[]));
        assert!(!r.ok && !r.text.contains("BEGIN"), "{}", r.text);
    }

    /// Parts that once came out with two entities of one name: a half disc (two segments that
    /// meet twice), a chamfer on a half bore, a groove across overlapping half-round bosses.
    #[test]
    fn names_stay_unique() {
        let half_disc = "extrude 36 loop line -33 0 33 0 arc 0 0 33 0 -180\n";
        let half_bore = "boss: extrude 36 xz at -72 87 95 loop line -33.5 0 33.5 0 arc 0 0 33.5 0 -180\n\
            bore: extrude cut thru both xz at -72 0 95 circle 0 0 23.5\n\
            chamfer 1 (edges of bore circle back)\n";
        let groove = "extrude 36 xz at -8 87 95 loop line -41 0 41 0 arc 0 0 41 0 -90 arc 0 0 41 -90 -90\n\
            extrude 36 xz at 62 87 95 loop line -42 0 42 0 arc 0 0 42 0 -90 arc 0 0 42 -90 -90\n\
            revolve cut yz at 62 0 95 axis x rect 79 33 2.2 3.4\n";
        let dir = Dir::new("unique");
        for (name, src) in [
            ("half-disc", half_disc),
            ("half-bore", half_bore),
            ("groove", groove),
        ] {
            let f = dir.file(&format!("{name}.fc"), src);
            let r = Session::default().run("check", &args(&f, &[]));
            assert!(
                r.ok && r.text.starts_with("valid after each"),
                "{name}: {}",
                r.text
            );
        }
        // and on the n-ary path, where the lines need no part to resolve against
        for src in [half_disc, groove] {
            let part = crate::text::parse(src).expect("parses");
            let body = part.regenerate_nary(1).expect("n-ary regen");
            assert!(
                forge_topo::check(&body).is_empty(),
                "{:?}",
                forge_topo::check(&body)
            );
        }
    }

    /// The example AGENTS.md opens with builds, handles and all.
    #[test]
    fn the_agents_example_builds() {
        let doc = include_str!("../../../AGENTS.md");
        let dir = Dir::new("example");
        let example = doc.split("```").nth(1).expect("an example block");
        let f = dir.file("e.fc", example);
        let r = Session::default().run("build", &args(&f, &[]));
        assert!(r.ok, "{}", r.text);
    }

    #[cfg(unix)]
    #[test]
    fn acts_write_the_file() {
        let dir = Dir::new("edit");
        let f = dir.file("e.fc", "w = 4\nextrude w rect 0 0 1 1  # a block\n");
        let r = Session::default().run("act", &args(&f, &["w = 5"]));
        assert!(r.text.contains("does not write files"), "{}", r.text);
        let mut s = Session::default().writable(true);
        let r = s.run("act", &args(&f, &["w = 5"]));
        assert_eq!(r.text, "ok v2; volume +1 mm3, valid");
        let r = s.run(
            "act",
            &args(&f, &["before extrude1 h = 2", "before extrude1 d = h"]),
        );
        assert!(r.ok, "{}", r.text);
        assert_eq!(
            std::fs::read_to_string(&f).expect("written"),
            "w = 5\nid 3 h = 2\nid 4 d = h\nid 2 extrude w rect 0 0 1 1  # a block\n"
        );
        let r = s.run("act", &args(&f, &["dim extrude1 x 3"]));
        assert!(r.text.contains("extrude1 is not a sketch"), "{}", r.text);
    }

    /// An edit never changes which bodies there are: a solid inserted above the first keeps that
    /// line a new body (its `new` written), and its `in` follows it to its new number.
    #[cfg(unix)]
    #[test]
    fn inserts_keep_the_bodies() {
        let dir = Dir::new("edit-in");
        let f = dir.file(
            "e.fc",
            "extrude 10 rect 0 0 10 10\nextrude new 4 at 20 0 0 rect 0 0 10 10\nextrude cut 12 at 5 5 -1 circle 0 0 2 in extrude1\n",
        );
        let mut s = Session::default().writable(true);
        let before = s.run("build", &args(&f, &[]));
        assert!(before.text.contains("bodies 2"), "{}", before.text);
        let r = s.run(
            "act",
            &args(&f, &["before extrude1 extrude 3 at -30 0 0 rect 0 0 5 5"]),
        );
        assert!(r.ok && r.text.contains("3 bodies"), "{}", r.text);
        let text = std::fs::read_to_string(&f).expect("written");
        assert_eq!(
            text,
            "id 4 extrude 3 at -30 0 0 rect 0 0 5 5\nid 1 extrude new 10 rect 0 0 10 10\nid 2 extrude new 4 at 20 0 0 rect 0 0 10 10\nid 3 extrude cut 12 at 5 5 -1 circle 0 0 2 in extrude2\n"
        );
        // the bodies before are as they were, the new one beside them
        assert!(r.text.contains("volume +75 mm3"), "{}", r.text);
        let _ = before;
    }

    /// Under a root, an edit never writes through a link to a file outside it.
    #[cfg(unix)]
    #[test]
    fn edits_refuse_links_out_of_the_root() {
        let dir = Dir::new("wroot");
        let outside = Dir::new("woutside");
        let away = outside.file("q.fc", "w = 4\nextrude w rect 0 0 1 1\n");
        let mut s = Session::rooted(std::fs::canonicalize(&dir.0).expect("root")).writable(true);
        let soft = dir.0.join("soft.fc");
        let hard = dir.0.join("hard.fc");
        std::os::unix::fs::symlink(&away, &soft).expect("link");
        std::fs::hard_link(&away, &hard).expect("link");
        for (l, why) in [
            (&soft, "not a part file under the root"),
            (&hard, "not a plain file with one name"),
        ] {
            let r = s.run("act", &args(&l.to_string_lossy(), &["w = 9"]));
            assert!(!r.ok && r.text.contains(why), "{}", r.text);
        }
        assert_eq!(
            std::fs::read_to_string(&away).expect("read"),
            "w = 4\nextrude w rect 0 0 1 1\n"
        );
        let plain = dir.file("p.fc", "w = 4\nextrude w rect 0 0 1 1\n");
        assert!(s.run("act", &args(&plain, &["w = 9"])).ok);
        assert_eq!(
            std::fs::read_dir(&dir.0).expect("dir").count(),
            4,
            "the two links with no log beside them, the part and its log, no temporary file"
        );
    }

    /// Only paths ending in `.fc` are opened, read through links first, and made new. Within
    /// them a broken part reads back so it can be mended, and says why its first line fails.
    #[cfg(unix)]
    #[test]
    fn only_fc_paths_open_and_a_broken_part_reads_back() {
        let dir = Dir::new("only-fc");
        let env = dir.file(".env", "API_KEY = s3kr1t\nOTHER = hunter2\n");
        let notes = dir.file("notes.txt", "w = 1\n");
        let link = dir.0.join("env.fc");
        std::os::unix::fs::symlink(&env, &link).expect("link");
        let mut s = Session::default().writable(true);
        for f in [env.clone(), notes.clone(), link.to_string_lossy().to_string()] {
            for (cmd, words) in [
                ("query", &["file"][..]),
                ("query", &["dims"]),
                ("query", &["stat"]),
                ("build", &[]),
                ("check", &[]),
                ("act", &["w = 3"]),
            ] {
                let r = s.run(cmd, &args(&f, words));
                assert!(!r.ok && !r.text.contains("hunter2"), "{f} {cmd}: {}", r.text);
                assert!(!r.text.contains("s3kr1t"), "{f} {cmd}: {}", r.text);
            }
        }
        assert_eq!(
            std::fs::read_to_string(&env).expect("read"),
            "API_KEY = s3kr1t\nOTHER = hunter2\n"
        );
        let made = dir.0.join("new.txt").to_string_lossy().to_string();
        assert!(!s.run("act", &args(&made, &["w = 3"])).ok);
        assert!(!std::path::Path::new(&made).exists());

        // a typo on line 1 of a real part says what it is, and the part reads back to be mended
        let f = dir.file("typo.fc", "extrude 5 rectt 0 0 1 1\nfillet 1 (edges top)\n");
        let r = s.run("build", &args(&f, &[]));
        assert!(!r.ok && r.text.contains("rectt"), "{}", r.text);
        let r = s.run("query", &args(&f, &["file"]));
        assert!(r.ok && r.text.contains("rectt"), "{}", r.text);
        let r = s.run("act", &args(&f, &["edit f1 extrude 5 rect 0 0 1 1"]));
        assert!(r.ok, "{}", r.text);
        // a broken line lower down too
        let f = dir.file("low.fc", "w = 3\nextrude w rect 0 0 1 1\nnope 3\n");
        assert!(s.run("query", &args(&f, &["file"])).text.contains("nope 3"));
        // comments alone are a part yet to come
        let f = dir.file("c.fc", "# a plate, later\n");
        assert!(s.run("query", &args(&f, &["file"])).text.contains("a plate"));
        assert!(s.run("act", &args(&f, &["w = 3", "extrude w rect 0 0 1 1"])).ok);
        // what starts with no feature at all isn't changed, or logged
        let f = dir.file("s.fc", "hunter2 swordfish\n");
        let r = s.run("act", &args(&f, &["w = 3"]));
        assert!(!r.ok && !r.text.contains("hunter2"), "{}", r.text);
        assert_eq!(std::fs::read_to_string(&f).expect("read"), "hunter2 swordfish\n");
        assert!(!dir.0.join("s.fc.log").exists());
        // a second name is no way to read a file
        let away = dir.file("key.txt", "hunter2\n");
        let hard = dir.0.join("hard.fc");
        std::fs::hard_link(&away, &hard).expect("link");
        let r = s.run("query", &args(&hard.to_string_lossy(), &["file"]));
        assert!(!r.ok && !r.text.contains("hunter2"), "{}", r.text);
    }

    #[cfg(unix)]
    #[test]
    fn a_new_part_is_made_by_its_first_act() {
        let dir = Dir::new("new-part");
        let f = dir.0.join("new.fc").to_string_lossy().to_string();
        let mut s = Session::default().writable(true);
        let r = s.run("act", &args(&f, &["extrude 5 rectt 0 0 1 1"]));
        assert!(!r.ok && r.text.contains("rectt"), "{}", r.text);
        assert!(s.run("act", &args(&f, &["w = 3", "extrude w rect 0 0 1 1"])).ok);
        assert!(s.run("query", &args(&f, &["file"])).text.contains("extrude w"));
    }

    /// The log is never followed through a link, and what isn't a log is
    /// refused, not overwritten.
    #[cfg(unix)]
    #[test]
    fn the_log_never_overwrites_what_is_not_a_log() {
        let dir = Dir::new("log-kinds");
        let outside = Dir::new("log-outside");
        let away = outside.file("v.txt", "precious\n");
        let part = "w = 4\nextrude w rect 0 0 1 1\n";
        let mut s = Session::default().writable(true);
        // a file that is no log
        let f = dir.file("a.fc", part);
        std::fs::write(dir.0.join("a.fc.log"), "my notes\n").expect("write");
        let r = s.run("act", &args(&f, &["w = 5"]));
        assert!(!r.ok, "{}", r.text);
        assert_eq!(
            std::fs::read_to_string(dir.0.join("a.fc.log")).expect("read"),
            "my notes\n"
        );
        assert_eq!(std::fs::read_to_string(&f).expect("read"), part);
        // a link, to a file or nowhere
        let f = dir.file("b.fc", part);
        std::os::unix::fs::symlink(&away, dir.0.join("b.fc.log")).expect("link");
        assert!(!s.run("act", &args(&f, &["w = 5"])).ok);
        let f = dir.file("c.fc", part);
        let gone = outside.0.join("not-yet");
        std::os::unix::fs::symlink(&gone, dir.0.join("c.fc.log")).expect("link");
        assert!(!s.run("act", &args(&f, &["w = 5"])).ok);
        assert!(!gone.exists(), "not created through the link");
        // a second name
        let f = dir.file("d.fc", part);
        std::fs::hard_link(&away, dir.0.join("d.fc.log")).expect("link");
        assert!(!s.run("act", &args(&f, &["w = 5"])).ok);
        assert_eq!(std::fs::read_to_string(&away).expect("read"), "precious\n");
        // a whole entry that does not read is damage, not a torn tail: left for a person
        let f = dir.file("e.fc", part);
        assert!(s.run("act", &args(&f, &["w = 5"])).ok);
        let log = dir.0.join("e.fc.log");
        let mut bytes = std::fs::read(&log).expect("read");
        bytes.extend_from_slice(&[3, 9, 9, 9]);
        std::fs::write(&log, &bytes).expect("write");
        let r = Session::default()
            .writable(true)
            .run("act", &args(&f, &["w = 6"]));
        assert!(!r.ok, "{}", r.text);
        assert_eq!(std::fs::read(&log).expect("read"), bytes);
    }

    /// The log holds the part's text, so no one reads it who can't read the part.
    #[cfg(unix)]
    #[test]
    fn the_log_is_as_private_as_the_part() {
        use std::os::unix::fs::PermissionsExt;
        let dir = Dir::new("log-mode");
        let mode = |p: &std::path::Path| std::fs::metadata(p).expect("meta").permissions().mode() & 0o777;
        let set = |p: &std::path::Path, m| {
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(m)).expect("chmod")
        };
        let f = dir.file("a.fc", "w = 4\nextrude w rect 0 0 1 1\n");
        let log = dir.0.join("a.fc.log");
        set(std::path::Path::new(&f), 0o600);
        assert!(
            Session::default()
                .writable(true)
                .run("act", &args(&f, &["w = 5"]))
                .ok
        );
        assert_eq!(mode(&log), 0o600);
        // one left broader than the part is tightened
        set(&log, 0o644);
        assert!(
            Session::default()
                .writable(true)
                .run("act", &args(&f, &["w = 6"]))
                .ok
        );
        assert_eq!(mode(&log), 0o600);
        assert_eq!(mode(std::path::Path::new(&f)), 0o600);
    }

    /// A part that may not be written leaves nothing of itself in a log, and a
    /// write that fails takes its entry back.
    #[cfg(unix)]
    #[test]
    fn a_refused_write_leaves_no_log_entry() {
        let dir = Dir::new("log-refused");
        let outside = Dir::new("log-refused-out");
        let away = outside.file("q.fc", "w = 4\nextrude w rect 0 0 1 1\n");
        let hard = dir.0.join("hard.fc");
        std::fs::hard_link(&away, &hard).expect("link");
        let mut s = Session::default().writable(true);
        let r = s.run("act", &args(&hard.to_string_lossy(), &["w = 9"]));
        assert!(
            !r.ok && r.text.contains("not a plain file with one name"),
            "{}",
            r.text
        );
        assert!(
            !dir.0.join("hard.fc.log").exists(),
            "no copy of it beside the link"
        );
        // the write itself fails, for root too: a directory where the temporary file must go
        let f = dir.file("p.fc", "w = 4\nextrude w rect 0 0 1 1\n");
        assert!(s.run("act", &args(&f, &["w = 5"])).ok);
        let log = dir.0.join("p.fc.log");
        let before = std::fs::read(&log).expect("read");
        let text = std::fs::read_to_string(&f).expect("read");
        let tmp = dir.0.join(format!(".p.fc.{}.tmp", std::process::id()));
        std::fs::create_dir(&tmp).expect("in the way");
        let r = s.run("act", &args(&f, &["w = 6"]));
        std::fs::remove_dir(&tmp).expect("out of the way");
        assert!(!r.ok, "{}", r.text);
        assert_eq!(std::fs::read(&log).expect("read"), before, "the entry went back");
        assert_eq!(std::fs::read_to_string(&f).expect("read"), text);
        // and the part goes on from where it was, one version on
        let r = s.run("act", &args(&f, &["w = 7"]));
        assert!(r.ok && r.text.starts_with("ok v3;"), "{}", r.text);
    }

    #[test]
    fn commands_answer() {
        let dir = Dir::new("cmds");
        let a = dir.file("a.fc", &V1.replace("-y at", "xz at"));
        let b = dir.file(
            "b.fc",
            &(V1.replace("-y at", "xz at") + "chamfer 1 (edges of hole2)\n"),
        );
        let mut s = Session::default();
        assert!(s.run("build", &args(&a, &[])).ok);
        let q = s.run("query", &args(&a, &["faces cyl r=4 of hole2"]));
        // the hole's wall is one face, not two half-turns
        assert!(q.text.starts_with("1 found"), "{}", q.text);
        let h = q
            .text
            .lines()
            .nth(1)
            .and_then(|l| l.split(' ').next())
            .expect("a handle")
            .to_string();
        let m = s.run("measure", &args(&a, &[&h]));
        assert!(m.text.contains("radius 4"), "{}", m.text);
        // a face's handle, its kind first; a few digits of it no longer name anything
        assert!(h.starts_with("@f") && h.len() >= 10, "{h}");
        let m = s.run("measure", &args(&a, &[&h[..6]]));
        assert!(!m.ok && m.text.contains("not a handle"), "{}", m.text);
        let m = s.run(
            "measure",
            &args(&a, &["(faces normal -z near 0 0 0)", "(faces normal +z top)"]),
        );
        assert!(m.text.starts_with("distance 45 angle 0"), "{}", m.text);
        let m = s.run("measure", &args(&a, &["wall"]));
        assert!(m.text.starts_with("wall 8 "), "{}", m.text);
        let m = s.run("measure", &args(&a, &["mass"]));
        assert!(m.ok, "{}", m.text);
        let c = s.run("check", &args(&a, &[]));
        assert!(
            c.ok && c.text.starts_with("valid after each of 6 lines"),
            "{}",
            c.text
        );
        let d = s.run("diff", &args(&a, &[&b]));
        assert!(d.text.starts_with("first change: line 7"), "{}", d.text);
        assert!(d.text.contains("faces +"), "{}", d.text);
        let r = s.run(
            "render",
            &Args {
                labels: Some("faces cyl".into()),
                ..args(&a, &["top"])
            },
        );
        assert!(
            r.png.as_ref().is_some_and(|p| p.starts_with(b"\x89PNG")),
            "{}",
            r.text
        );
        let bad = s.run("query", &args(&a, &["edges of nothing"]));
        assert!(
            !bad.ok && bad.text.contains("no feature 'nothing'"),
            "{}",
            bad.text
        );
        let j = s.run("build", &args(&a, &[])).json.to_string();
        assert!(j.starts_with("{\"ok\":true,\"lines\":6,\"built\":6,"), "{j}");
    }
}

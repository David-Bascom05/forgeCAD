//! Forge (.fg) parts for agents: build, check, query, measure, render, export, and `edit`, which
//! takes typed ops. Every reply is typed JSON: errors, warnings
//! and lints with their spans and their fixes as ops, which `edit` takes back as they are. Each
//! file's build stages are kept, so a rebuild starts at the first feature that changed.

use forge_agent::json::Json;
use forge_agent::select::{self, Ent, View};
use forge_agent::{facemesh, measure, render};
use forge_lang::build::Cache;
use forge_lang::error::{Error, Op};
use forge_lang::{Options, Run, edit, report};
use forge_topo::Body;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The tools a part file takes, on the command line and over `serve`/`mcp`.
pub const COMMANDS: [&str; 8] = [
    "build", "check", "query", "measure", "render", "export", "edit", "diff",
];

/// A tool's arguments: the file, and its words (a selector, the refs to measure, the view, the
/// ops).
#[derive(Clone, Debug, Default)]
pub struct Args {
    pub file: String,
    pub words: Vec<String>,
    /// `render`: a selector whose entities are labelled.
    pub labels: Option<String>,
    /// `render`: the picture's long side.
    pub px: Option<usize>,
    /// `query`, `measure`, `render`: the part as it stood after this feature (its id).
    pub at: Option<String>,
    /// `edit`: the version (`version` in a reply) the ops were written against; refused if the
    /// file has moved on.
    pub base: Option<String>,
}

/// A tool's answer: plain text, its typed JSON, and the bytes of a picture or a file.
pub struct Reply {
    pub ok: bool,
    pub text: String,
    pub json: Json,
    pub png: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct Session {
    root: Option<PathBuf>,
    writable: bool,
    /// Each file's stages, by its real path.
    caches: HashMap<PathBuf, Cache>,
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

/// A typed error as the reply: its record, and its fixes.
fn typed(e: &Error, src: &str) -> Reply {
    let j = e.json(src);
    Reply {
        ok: false,
        text: format!("error {}: {}", e.kind(), e.text()),
        json: Json::obj(vec![("ok", false.into()), ("error", j)]),
        png: None,
    }
}

fn obj_push(j: &mut Json, k: &str, v: Json) {
    if let Json::Obj(f) = j {
        f.push((k.into(), v));
    }
}

/// A file's version: a hash of its text, which `edit --base` takes back.
pub fn version(text: &str) -> String {
    format!("{:016x}", forge_math::hash::hash128(text.as_bytes()) as u64)
}

/// The part a selector, measure or picture looks at: the finished part, or with `--at` the part
/// as it stood after that feature; how many kernel features it holds, and the file it is of.
struct Staged {
    body: Body,
    n: usize,
    path: PathBuf,
    /// Each built feature's id and kernel index, in the document's order.
    feats: Vec<(String, usize)>,
    /// Each solid's faces and edges in `body`, and the feature that made it.
    spans: Vec<forge_lang::build::Span>,
}

/// The features an id names: itself, or every feature a call or loop of that name made
/// (`lug` holds `lug.boss`, `bolt` holds `bolt[1]`). Their kernel indices.
fn feats_named(feats: &[(String, usize)], name: &str) -> Vec<usize> {
    let exact: Vec<usize> = feats.iter().filter(|f| f.0 == name).map(|f| f.1).collect();
    if !exact.is_empty() {
        return exact;
    }
    feats
        .iter()
        .filter(|f| {
            f.0.strip_prefix(name)
                .is_some_and(|r| r.starts_with('.') || r.starts_with('['))
        })
        .map(|f| f.1)
        .collect()
}

fn no_feature(feats: &[(String, usize)], name: &str) -> String {
    let mut ids: Vec<&str> = feats.iter().map(|f| f.0.as_str()).collect();
    ids.dedup();
    let more = if ids.len() > 30 { " .." } else { "" };
    ids.truncate(30);
    format!(
        "'{name}' is no built feature of the part (features: {}{more})",
        ids.join(" ")
    )
}

impl Session {
    pub fn new(root: Option<PathBuf>, writable: bool) -> Session {
        Session {
            root,
            writable,
            caches: HashMap::new(),
        }
    }

    /// The file's real path: a .fg file, under the root if there is one. Rooted, every refusal
    /// reads the same, so a client learns nothing of paths outside it.
    fn path(&self, file: &str) -> Result<PathBuf, String> {
        let real = std::fs::canonicalize(file)
            .ok()
            .filter(|r| r.extension().is_some_and(|x| x == "fg"));
        match (&self.root, real) {
            (Some(root), Some(r)) if r.starts_with(root) => Ok(r),
            (Some(_), _) => Err(format!("{file}: not a .fg part file under the root")),
            (None, Some(r)) => Ok(r),
            (None, None) => Err(format!("{file}: no such .fg part file")),
        }
    }

    fn read(&self, file: &str) -> Result<(PathBuf, String), String> {
        let p = self.path(file)?;
        let src = std::fs::read_to_string(&p).map_err(|_| format!("{file}: cannot read"))?;
        Ok((p, src))
    }

    /// Runs the text through the file's kept stages.
    fn run_src(&mut self, path: &Path, src: &str, build: bool) -> Run {
        let cache = self.caches.remove(path).unwrap_or_default();
        let opts = Options {
            build,
            dir: path.parent().map(Path::to_path_buf),
            root: self.root.clone(),
            // a refused round's or shell's size comes back with the largest that builds
            fit_sizes: build,
            ..Options::default()
        };
        let (run, cache) = forge_lang::run(src, opts, cache);
        // a check builds nothing: the stages are the last build's still
        if build {
            self.caches.insert(path.to_path_buf(), cache);
        }
        run
    }

    fn built(&mut self, file: &str) -> Result<(Run, String), String> {
        let (p, src) = self.read(file)?;
        let run = self.run_src(&p, &src, true);
        Ok((run, src))
    }

    /// The run as the reply: the report, each sheet's bends, and the file's version.
    fn report(run: &Run, src: &str) -> Reply {
        let mut json = report::json(run, src);
        obj_push(&mut json, "version", version(src).into());
        if !run.sheets.is_empty() {
            obj_push(
                &mut json,
                "sheets",
                Json::Arr(run.sheets.iter().map(|s| s.json()).collect()),
            );
        }
        Reply {
            ok: run.ok(),
            text: report::text(run, src),
            json,
            png: None,
        }
    }

    /// `params` holds a tool's arguments as sent (an op may be an object, not text).
    pub fn run(&mut self, cmd: &str, a: &Args, params: &Json) -> Reply {
        if a.at.is_some() && !matches!(cmd, "query" | "measure" | "render") {
            return fail("--at goes with query, measure and render");
        }
        if a.base.is_some() && cmd != "edit" {
            return fail("--base goes with edit");
        }
        let r = match cmd {
            "build" => self.built(&a.file).map(|(run, src)| Self::report(&run, &src)),
            "check" => self.read(&a.file).map(|(p, src)| {
                let run = self.run_src(&p, &src, false);
                Self::report(&run, &src)
            }),
            "query" => self.query(a),
            "measure" => self.measure(a),
            "render" => self.render(a),
            "export" => self.export(a),
            "edit" => self.edit(a, params),
            "diff" => self.diff(a),
            _ => Err(format!(
                "unknown command '{cmd}' ({}; a part changes by edit F OP)",
                COMMANDS.join(" ")
            )),
        };
        r.unwrap_or_else(fail)
    }

    fn body(run: &Run) -> Result<&Body, String> {
        run.body.as_ref().ok_or_else(|| match run.errors.first() {
            Some(e) => format!("the part did not build: {}: {}", e.kind(), e.text()),
            None => "the part has no solid".to_string(),
        })
    }

    /// The part to look at: built, and with `--at FEAT` as it stood after that feature (the
    /// last of what a call or loop of that name made), as far as the part built.
    fn staged(&mut self, a: &Args) -> Result<Staged, String> {
        let (run, _) = self.built(&a.file)?;
        let path = self.path(&a.file)?;
        let cache = self.caches.get(&path).ok_or("the part was not built")?;
        let built = cache.built();
        let feats: Vec<(String, usize)> = run
            .feats
            .iter()
            .filter(|f| f.idx < built)
            .map(|f| (f.id.clone(), f.idx))
            .collect();
        let Some(at) = &a.at else {
            let spans = cache.spans_after(built).to_vec();
            let body = match run.body {
                Some(b) => b,
                None => Self::body(&run)?.clone(),
            };
            return Ok(Staged {
                body,
                n: built,
                path,
                feats,
                spans,
            });
        };
        let Some(&last) = feats_named(&feats, at).iter().max() else {
            // named, but not built: say where the part stops
            if run.feats.iter().any(|f| f.id == *at) {
                let stop = run
                    .errors
                    .first()
                    .map_or(String::new(), |e| format!(": {}: {}", e.kind(), e.text()));
                return Err(format!("'{at}' did not build{stop}"));
            }
            return Err(no_feature(&feats, at));
        };
        let body = cache
            .body_after(last + 1)
            .ok_or_else(|| format!("the part has no solid after '{at}'"))?
            .clone();
        Ok(Staged {
            body,
            n: last + 1,
            path,
            feats,
            spans: cache.spans_after(last + 1).to_vec(),
        })
    }

    /// A selector on the staged part. `of FEAT..` and `between FEAT FEAT` read which feature
    /// made each face (by its id: a call's or loop's name takes all it made), as a part's own
    /// `edges(of: ..)` does; a pattern's copies are the pattern's.
    fn select(&mut self, st: &Staged, sel: &str) -> Result<Vec<Ent>, String> {
        let toks: Vec<&str> = sel
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split_whitespace()
            .collect();
        let mut origin = HashMap::new();
        // a name for several features (a call, a loop) stands for them all: an index of its
        // own, past every feature's, added to the lineage of each face any of them made
        let mut groups: Vec<(String, usize)> = vec![];
        if toks.iter().any(|t| matches!(*t, "of" | "between" | "body")) {
            // (the run's lineage as the selector's own map)
            origin = self
                .caches
                .get_mut(&st.path)
                .ok_or("the part was not built")?
                .origin(st.n)
                .iter()
                .map(|(k, v)| (*k, v.to_vec()))
                .collect();
            for t in &toks {
                let set = feats_named(&st.feats, t);
                if set.len() > 1 && !groups.iter().any(|g| g.0 == *t) {
                    let id = usize::MAX / 2 + groups.len();
                    for o in origin.values_mut() {
                        if o.iter().any(|i| set.contains(i)) {
                            o.push(id);
                        }
                    }
                    groups.push((t.to_string(), id));
                }
            }
        }
        let feats = &st.feats;
        let feat = |r: &str| -> Result<usize, String> {
            if let Some(g) = groups.iter().find(|g| g.0 == r) {
                return Ok(g.1);
            }
            match feats_named(feats, r)[..] {
                [i] => Ok(i),
                _ => Err(no_feature(feats, r)),
            }
        };
        // the solid an entity is on, by the feature that made it (`body FEAT`)
        let body = &st.body;
        let span_of =
            |f: forge_topo::FaceId| st.spans.iter().find(|s| s.faces.contains(&f.0)).map(|s| s.solid);
        let body_of = |e: Ent| match e {
            Ent::Face(f) => span_of(f),
            Ent::Edge(x) => select::faces_of_edge(body, x).first().and_then(|&f| span_of(f)),
            Ent::Vert(v) => body
                .edge_ids()
                .find(|&x| body.edge(x).start == v || body.edge(x).end == v)
                .and_then(|x| select::faces_of_edge(body, x).first().copied())
                .and_then(span_of),
        };
        select::select(
            &View {
                body: &st.body,
                origin: &origin,
                feat: &feat,
                body_of: &body_of,
            },
            sel,
        )
    }

    fn one(&mut self, st: &Staged, r: &str) -> Result<Ent, String> {
        if r.starts_with('@') {
            return select::by_handle(&st.body, r);
        }
        match self.select(st, r)?[..] {
            [e] => Ok(e),
            ref many => Err(format!(
                "'{r}' selects {} entities; measure needs one",
                many.len()
            )),
        }
    }

    fn query(&mut self, a: &Args) -> Result<Reply, String> {
        let what = a.words.first().map(String::as_str).unwrap_or("");
        if a.at.is_some()
            && matches!(
                what,
                "stat" | "check" | "file" | "handles" | "params" | "features"
            )
        {
            return Err("--at goes with a selector, measure and render".into());
        }
        match what {
            "stat" => return self.built(&a.file).map(|(run, src)| Self::report(&run, &src)),
            // every feature's status as the part builds (its stages kept: right after a build this
            // costs nothing); `check F` alone stops before the kernel
            "check" => return self.built(&a.file).map(|(run, src)| Self::report(&run, &src)),
            "file" => {
                let (_, src) = self.read(&a.file)?;
                return Ok(Reply {
                    ok: true,
                    json: Json::obj(vec![
                        ("ok", true.into()),
                        ("text", src.as_str().into()),
                        ("version", version(&src).into()),
                    ]),
                    text: src,
                    png: None,
                });
            }
            "handles" | "params" | "features" => {
                // built, so each feature's status is what it is, not "not built"
                let (run, src) = self.built(&a.file)?;
                let j = report::json(&run, &src);
                let key = if what == "features" { "features" } else { "params" };
                let list = j.get(key).cloned().unwrap_or(Json::Arr(vec![]));
                let text = match &list {
                    Json::Arr(xs) => xs
                        .iter()
                        .map(|x| {
                            let mut s = String::new();
                            x.write(&mut s);
                            s
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => String::new(),
                };
                return Ok(Reply {
                    ok: true,
                    text,
                    json: Json::obj(vec![("ok", true.into()), (key, list)]),
                    png: None,
                });
            }
            _ => {}
        }
        // numbers from the built part or assembly (`forge_lang::query`); `faces` alone, not a selector
        // `faces of F ..` on a part, or where F is a feature, is a selector; `faces of OCC` an
        // assembly's faces. With --at, `faces` alone is a selector too
        let of_feature = what == "faces"
            && a.words.get(1).is_some_and(|w| w == "of")
            && a.words.get(2).is_some_and(|w| {
                self.built(&a.file).is_ok_and(|(run, _)| {
                    let ids: Vec<(String, usize)> = run.feats.iter().map(|f| (f.id.clone(), f.idx)).collect();
                    run.asm.is_none() || !feats_named(&ids, w).is_empty()
                })
            });
        let asks = matches!(
            what,
            "holes" | "section" | "aligned" | "gap" | "interference" | "frame" | "bendsim" | "dfm"
        ) || (what == "faces"
            && !of_feature
            && a.words.get(1).map_or(a.at.is_none(), |w| w == "of" || w == "in"));
        if asks && a.at.is_none() {
            let p = self.path(&a.file)?;
            let r = forge_lang::query::ask_file(&p, self.root.clone(), &a.words.join(" "));
            return Ok(Reply {
                ok: r.ok,
                text: r.text,
                json: r.json,
                png: None,
            });
        }
        if asks {
            return Err("--at goes with a selector, measure and render".into());
        }
        let st = self.staged(a)?;
        let ents = self.select(&st, &a.words.join(" "))?;
        let body = &st.body;
        let handles = select::handles(body);
        let mut list = Vec::new();
        let mut text = format!("{} found", ents.len());
        for &e in &ents {
            let h = handles[&select::name(body, e)].clone();
            let d = select::describe(body, e);
            if list.len() < 25 {
                text.push_str(&format!("\n{h} {d}"));
            }
            list.push(Json::obj(vec![
                ("h", h.as_str().into()),
                ("d", d.as_str().into()),
            ]));
        }
        Ok(Reply {
            ok: true,
            text,
            json: Json::obj(vec![("ok", true.into()), ("found", Json::Arr(list))]),
            png: None,
        })
    }

    fn measure(&mut self, a: &Args) -> Result<Reply, String> {
        let st = self.staged(a)?;
        let body = &st.body;
        let mut words: Vec<&str> = a.words.iter().map(String::as_str).collect();
        // `exact`: every digit in the text, for checks finer than the reply's rounding
        let exact = words.last() == Some(&"exact");
        if exact {
            words.pop();
        }
        let mut cache = facemesh::Cache::default();
        let mesh = facemesh::join(&cache.faces(body));
        let pairs: Vec<(&str, f64)> = match words[..] {
            ["mass"] => {
                let m = forge_topo::mass::mass(body).map_err(|e| format!("mass: {e:?}"))?;
                vec![
                    ("volume", m.volume),
                    ("area", m.area),
                    ("cx", m.centroid.x),
                    ("cy", m.centroid.y),
                    ("cz", m.centroid.z),
                    ("ixx", m.inertia[0][0]),
                    ("iyy", m.inertia[1][1]),
                    ("izz", m.inertia[2][2]),
                    ("ixy", m.inertia[0][1]),
                    ("iyz", m.inertia[1][2]),
                    ("izx", m.inertia[2][0]),
                    ("i1", m.principal[0]),
                    ("i2", m.principal[1]),
                    ("i3", m.principal[2]),
                ]
            }
            ["wall"] => match measure::wall(body, &mesh) {
                Some((t, p)) => vec![("wall", t), ("x", p.x), ("y", p.y), ("z", p.z)],
                None => return Err("no wall found".into()),
            },
            [r] => measure::one(body, &mesh, self.one(&st, r)?),
            [r, s] => measure::between(body, &mesh, self.one(&st, r)?, self.one(&st, s)?),
            _ => return Err(
                "measure takes mass, wall, or one or two @handles / (selectors), then exact for every digit"
                    .into(),
            ),
        };
        let text = pairs
            .iter()
            .map(|(k, v)| {
                if exact {
                    format!("{k} {v:?}")
                } else {
                    format!("{k} {}", forge_agent::short(*v))
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut j = vec![("ok", true.into())];
        j.extend(pairs.iter().map(|&(k, v)| (k, Json::Num(v))));
        Ok(Reply {
            ok: true,
            text,
            json: Json::obj(j),
            png: None,
        })
    }

    fn render(&mut self, a: &Args) -> Result<Reply, String> {
        let st = self.staged(a)?;
        let body = &st.body;
        let view = a.words.first().map_or("iso", String::as_str);
        let mut labels = Vec::new();
        if let Some(sel) = &a.labels {
            let handles = select::handles(body);
            for e in self.select(&st, sel)?.into_iter().take(40) {
                let h = handles[&select::name(body, e)]
                    .trim_start_matches('@')
                    .to_string();
                labels.push((h, select::centre(body, e)));
            }
        }
        let mut cache = facemesh::Cache::default();
        let mesh = facemesh::join(&cache.faces(body));
        let png = render::render(body, &mesh, view, a.px.unwrap_or(400), &labels)?;
        let (w, h) = (
            u32::from_be_bytes(png[16..20].try_into().expect("IHDR")),
            u32::from_be_bytes(png[20..24].try_into().expect("IHDR")),
        );
        Ok(Reply {
            ok: true,
            text: format!("{view} {w}x{h} {} bytes", png.len()),
            json: Json::obj(vec![
                ("ok", true.into()),
                ("w", (w as usize).into()),
                ("h", (h as usize).into()),
                ("bytes", png.len().into()),
            ]),
            png: Some(png),
        })
    }

    /// STEP of the part, or `dxf [SHEET]`: a sheet's flat pattern. The text goes in the reply's
    /// bytes.
    fn export(&mut self, a: &Args) -> Result<Reply, String> {
        let (run, _) = self.built(&a.file)?;
        if let Some(fmt) = a.words.first().filter(|w| *w == "dxf" || *w == "dwg") {
            let dwg = fmt == "dwg";
            let s = match a.words.get(1) {
                Some(id) => run.sheets.iter().find(|s| s.id == *id),
                None if run.sheets.len() <= 1 => run.sheets.first(),
                None => {
                    let ids: Vec<&str> = run.sheets.iter().map(|s| s.id.as_str()).collect();
                    return Err(format!(
                        "several sheets: export F {fmt} ID, one of {}",
                        ids.join(" ")
                    ));
                }
            }
            .ok_or("the part has no such sheet")?;
            let bytes = if dwg {
                s.dwg()
            } else {
                s.dxf().map(String::into_bytes)
            }
            .map_err(|e| format!("{}: no flat pattern: {e:?}", s.id))?;
            return Ok(Reply {
                ok: true,
                text: format!(
                    "{} flat pattern, {} bytes of {}",
                    s.id,
                    bytes.len(),
                    fmt.to_uppercase()
                ),
                json: Json::obj(vec![
                    ("ok", true.into()),
                    ("sheet", s.id.as_str().into()),
                    ("bytes", bytes.len().into()),
                ]),
                png: Some(bytes),
            });
        }
        let mut opt = forge_step::Options::default();
        // sheet metal's bends with cuts across them are fitted: tight enough that a reader which
        // sets each edge's tolerance from a few samples finds no edge over it
        if !run.sheets.is_empty() {
            opt.tol = forge_step::SHEET_TOL;
        }
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
                _ => {
                    return Err(format!(
                        "export: unknown word '{w}' (ap214 ap242 tol MM, or dxf [SHEET])"
                    ));
                }
            }
        }
        opt.name = Path::new(&a.file)
            .file_stem()
            .map_or_else(|| "part".into(), |n| n.to_string_lossy().into_owned());
        let body = Self::body(&run)?;
        let e = forge_step::export(body, &opt).map_err(|e| e.to_string())?;
        Ok(Reply {
            ok: true,
            text: format!("{} solids, {} faces, {} edges", e.solids, e.faces, e.edges),
            json: Json::obj(vec![
                ("ok", true.into()),
                ("solids", e.solids.into()),
                ("faces", e.faces.into()),
                ("edges", e.edges.into()),
            ]),
            png: Some(e.text.into_bytes()),
        })
    }

    /// What part B changed from part A: features and params added, removed and changed (with
    /// what changed in each), faces by name, and with `volume` the exact volume added and
    /// removed (`forge_lang::compare`, as `compare` gives it between two saves of one file).
    fn diff(&mut self, a: &Args) -> Result<Reply, String> {
        let Some(other) = a.words.first() else {
            return Err("diff takes two parts: diff A B [volume]".into());
        };
        let side = |file: &str| -> Result<forge_lang::compare::Side, String> {
            let (p, text) = self.read(file)?;
            let opts = Options {
                dir: p.parent().map(Path::to_path_buf),
                root: self.root.clone(),
                render: true,
                anchors: true,
                ..Options::default()
            };
            let (run, _) = forge_lang::run(&text, opts, Default::default());
            Ok(forge_lang::compare::Side {
                label: file.to_string(),
                text,
                run,
            })
        };
        let c = forge_lang::compare::compare(side(&a.file)?, side(other)?);
        let vol =
            a.words
                .iter()
                .any(|w| w == "volume")
                .then(|| match (&c.base.run.body, &c.target.run.body) {
                    (Some(x), Some(y)) => forge_lang::compare::volume(x, y),
                    _ => Err("a part did not build".into()),
                });
        let mut json = c.json(vol.as_ref());
        obj_push(&mut json, "ok", true.into());
        Ok(Reply {
            ok: true,
            text: json.to_string(),
            json,
            png: None,
        })
    }

    /// Typed ops, all applied or none: each checked as it goes (`apply_checked`), the file written
    /// once, then built. The reply: each op with what it reached, and the build's report.
    fn edit(&mut self, a: &Args, params: &Json) -> Result<Reply, String> {
        if !self.writable {
            return Err("this session does not write files (serve and mcp take --writable)".into());
        }
        let step = undo_step(a, params);
        let ops = if step.is_some() {
            vec![]
        } else {
            ops_of(a, params)?
        };
        let (p, src) = self.read(&a.file)?;
        let now = version(&src);
        if let Some(base) = a.base.as_deref().filter(|b| *b != now) {
            let msg = format!(
                "the file is at version {now}, not {base}: it changed since that reply; read it again (query F file) and send ops against {now}"
            );
            return Ok(Reply {
                ok: false,
                text: format!("error StaleBase: {msg}"),
                json: Json::obj(vec![
                    ("ok", false.into()),
                    (
                        "error",
                        Json::obj(vec![
                            ("kind", "StaleBase".into()),
                            ("text", msg.into()),
                            ("base", base.into()),
                            ("version", now.as_str().into()),
                        ]),
                    ),
                    ("version", now.as_str().into()),
                ]),
                png: None,
            });
        }
        // the file's last edit taken back, or put back (its log beside it, `F.fg.log`)
        if let Some(s) = step {
            let text = forge_lang::undo::step(&p, s)?;
            let _ = forge_lang::vtools::save(&p, &format!("{s:?}"), Some(&agent()));
            let run = self.run_src(&p, &text, true);
            let mut r = Self::report(&run, &text);
            obj_push(
                &mut r.json,
                "applied",
                Json::Arr(vec![Json::obj(vec![(
                    "op",
                    Json::obj(vec![("op", format!("{s:?}").into())]),
                )])]),
            );
            r.text = format!("{s:?}\n{}", r.text);
            return Ok(r);
        }
        let mut text = src.clone();
        let mut done = Vec::new();
        for (k, op) in ops.iter().enumerate() {
            match edit::apply_checked_in(&text, op, p.parent().map(Path::to_path_buf)) {
                Ok((next, reach)) => {
                    let mut j = Json::obj(vec![("op", op.json())]);
                    if let Some(r) = reach {
                        obj_push(&mut j, "drives_values", r.values.into());
                        obj_push(
                            &mut j,
                            "features",
                            Json::Arr(r.features.iter().map(|x| Json::Str(x.clone())).collect()),
                        );
                        obj_push(&mut j, "structural", r.structural.into());
                    }
                    done.push(j);
                    text = next;
                }
                Err(e) => {
                    let mut r = typed(&e, &text);
                    obj_push(&mut r.json, "refused", k.into());
                    obj_push(&mut r.json, "op", op.json());
                    obj_push(&mut r.json, "version", now.as_str().into());
                    r.text = format!("op {k} refused, nothing applied: {}", r.text);
                    return Ok(r);
                }
            }
        }
        // logged, so `{"op":"Undo"}` takes it back
        forge_lang::undo::write(&p, &src, &text).map_err(|_| format!("{}: cannot write", a.file))?;
        // an agent's edit is a save of its own, where the folder keeps history
        let kinds: Vec<String> = done
            .iter()
            .filter_map(|j| {
                j.get("op")
                    .and_then(|o| o.get("op"))
                    .and_then(|k| k.str())
                    .map(String::from)
            })
            .collect();
        let _ = forge_lang::vtools::save(&p, &kinds.join(", "), Some(&agent()));
        let run = self.run_src(&p, &text, true);
        let mut r = Self::report(&run, &text);
        obj_push(&mut r.json, "applied", Json::Arr(done));
        r.text = format!("applied {} ops\n{}", ops.len(), r.text);
        Ok(r)
    }
}

fn agent() -> String {
    std::env::var("FORGECAD_AGENT").unwrap_or_else(|_| "agent".into())
}

/// `{"op":"Undo"}` or `{"op":"Redo"}` as an edit's one op.
fn undo_step(a: &Args, params: &Json) -> Option<forge_lang::undo::Step> {
    let j = match (params.get("op"), params.get("ops"), &a.words[..]) {
        (Some(Json::Str(t)), _, _) => Json::parse(t).ok()?,
        (Some(o), _, _) => o.clone(),
        (None, Some(Json::Arr(xs)), _) if xs.len() == 1 => match &xs[0] {
            Json::Str(t) => Json::parse(t).ok()?,
            o => o.clone(),
        },
        (None, None, [w]) => Json::parse(w).ok()?,
        _ => return None,
    };
    match j.get("op").and_then(Json::str)? {
        "Undo" => Some(forge_lang::undo::Step::Undo),
        "Redo" => Some(forge_lang::undo::Step::Redo),
        _ => None,
    }
}

/// The ops of an `edit`: `op` (an object, or its JSON as text), or `ops` (a list of them); on the
/// command line, each word an op's JSON.
fn ops_of(a: &Args, params: &Json) -> Result<Vec<Op>, String> {
    let one = |j: &Json| -> Result<Op, String> {
        let j = match j {
            Json::Str(s) => Json::parse(s).map_err(|e| format!("op: {e}"))?,
            other => other.clone(),
        };
        let mut s = String::new();
        j.write(&mut s);
        Op::from_json(&j).ok_or(format!("not an op: {s}"))
    };
    let list: Vec<Json> = match (params.get("op"), params.get("ops")) {
        (Some(o), _) => vec![o.clone()],
        (None, Some(Json::Arr(xs))) => xs.clone(),
        (None, Some(Json::Str(s))) => match Json::parse(s).map_err(|e| format!("ops: {e}"))? {
            Json::Arr(xs) => xs,
            o => vec![o],
        },
        _ => a.words.iter().map(|w| Json::Str(w.clone())).collect(),
    };
    if list.is_empty() {
        return Err("edit takes an op: {\"op\":\"SetParam\",\"param\":\"t\",\"value\":{\"value\":3,\"dim\":\"Length\"}}".into());
    }
    list.iter().map(one).collect()
}

/// In process, so they run in the wasm suite too (the browser's arithmetic).
#[cfg(test)]
mod tests {
    use super::*;

    const PART: &str = "shared t = 6\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (40, 40))\n}\nbase = extrude(plan, t)\n";

    fn dir() -> PathBuf {
        // wasi has no temp_dir or pid; its runner opens the whole filesystem
        let (base, id) = if cfg!(target_os = "wasi") {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock");
            (PathBuf::from("/tmp"), t.as_nanos() as u64)
        } else {
            (std::env::temp_dir(), std::process::id() as u64)
        };
        let d = base.join(format!("forgecad-fg-unit-{id}"));
        std::fs::create_dir_all(&d).expect("temp dir");
        std::fs::canonicalize(d).expect("real path")
    }

    #[test]
    fn a_fix_from_the_report_goes_back_through_edit_and_the_stages_are_kept() {
        let d = dir();
        let f = d.join("p.fg");
        std::fs::write(&f, PART).unwrap();
        let file = f.to_string_lossy().into_owned();
        let mut s = Session::new(Some(d.clone()), true);
        let a = Args {
            file: file.clone(),
            ..Args::default()
        };
        let r = s.run("build", &a, &Json::Null);
        assert!(r.ok, "{}", r.text);
        let Some(Json::Arr(ws)) = r.json.get("warnings") else {
            panic!("{}", r.json)
        };
        let lint = ws
            .iter()
            .find(|w| w.get("kind").and_then(Json::str) == Some("LooseLiteral"))
            .unwrap();
        let Some(Json::Arr(fixes)) = lint.get("fixes") else {
            panic!("{lint}")
        };
        let fix = fixes[0].clone();
        let e = s.run("edit", &a, &Json::obj(vec![("op", fix.clone())]));
        assert!(e.ok, "{}", e.text);
        let want = edit::apply(PART, &Op::from_json(&fix).unwrap()).unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), want);
        // unchanged: every stage reused
        let again = s.run("build", &a, &Json::Null);
        assert_eq!(again.json.get("rebuilt").and_then(Json::num), Some(0.0));
        // a wrong dimension is refused, typed, and nothing is written
        let bad = Json::parse(r#"{"op":"SetParam","param":"t","value":{"value":3,"dim":"Angle"}}"#).unwrap();
        let e = s.run("edit", &a, &Json::obj(vec![("op", bad)]));
        assert!(!e.ok);
        assert_eq!(
            e.json
                .get("error")
                .and_then(|x| x.get("kind"))
                .and_then(Json::str),
            Some("EditDim")
        );
        assert_eq!(std::fs::read_to_string(&f).unwrap(), want);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_numbers_queries_answer_through_query_and_selectors_still_select() {
        let f =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sheetmetal/tray_asm.fg");
        let mut s = Session::new(None, false);
        let a = Args {
            file: f.to_string_lossy().into_owned(),
            words: "gap rack/right_ear.bottom tray/right.bottom"
                .split(' ')
                .map(String::from)
                .collect(),
            ..Args::default()
        };
        let r = s.run("query", &a, &Json::Null);
        assert!(r.ok, "{}", r.text);
        assert!((r.json.get("gap").and_then(Json::num).unwrap() - 0.2).abs() < 1e-9);
        let part = f.with_file_name("load_tray.fg");
        let a = Args {
            file: part.to_string_lossy().into_owned(),
            words: vec!["faces".into(), "cyl".into()],
            ..Args::default()
        };
        let r = s.run("query", &a, &Json::Null);
        assert!(r.ok && r.json.get("found").is_some(), "{}", r.text);
    }
}

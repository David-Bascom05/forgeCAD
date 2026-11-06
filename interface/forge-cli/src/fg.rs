//! Forge (.fg) parts for agents: the same tools as .fc (build, check, query, measure, render,
//! export, help) plus `edit`, which takes typed ops. Every reply is typed JSON: errors, warnings
//! and lints with their spans and their fixes as ops, which `edit` takes back as they are. Each
//! file's build stages are kept, so a rebuild starts at the first feature that changed.

use forge_agent::cmd::{Args, Reply};
use forge_agent::json::Json;
use forge_agent::select::{self, Ent, View};
use forge_agent::{facemesh, measure, render};
use forge_lang::build::Cache;
use forge_lang::error::{Error, Op};
use forge_lang::{Options, Run, edit, report};
use forge_topo::Body;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn is_fg(file: &str) -> bool {
    file.ends_with(".fg")
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

    /// The run as the reply: the report, and each sheet's bends.
    fn report(run: &Run, src: &str) -> Reply {
        let mut json = report::json(run, src);
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
            "act" => Err(format!(
                "{}: a .fg part changes by typed ops: edit F OP (an op as errors' fixes give it)",
                a.file
            )),
            "diff" => Err("diff takes .fc parts".into()),
            _ => Err(format!("unknown command '{cmd}' for a .fg part")),
        };
        r.unwrap_or_else(fail)
    }

    fn body(run: &Run) -> Result<&Body, String> {
        run.body.as_ref().ok_or_else(|| match run.errors.first() {
            Some(e) => format!("the part did not build: {}: {}", e.kind(), e.text()),
            None => "the part has no solid".to_string(),
        })
    }

    /// Selectors on the finished body: geometric filters; `of`/`between` features are .fc's
    /// lineage, which a .fg part asks in its own selections (`edges(of: F, ..)`).
    fn select(body: &Body, sel: &str) -> Result<Vec<Ent>, String> {
        let origin = HashMap::new();
        let feat = |r: &str| -> Result<usize, String> {
            Err(format!(
                "'{r}': on a .fg part, select by feature in the part's own text (edges(of: {r}, ..))"
            ))
        };
        let body_of = |_: Ent| None;
        select::select(
            &View {
                body,
                origin: &origin,
                feat: &feat,
                body_of: &body_of,
            },
            sel,
        )
    }

    fn one(body: &Body, r: &str) -> Result<Ent, String> {
        if r.starts_with('@') {
            return select::by_handle(body, r);
        }
        match Self::select(body, r)?[..] {
            [e] => Ok(e),
            ref many => Err(format!(
                "'{r}' selects {} entities; measure needs one",
                many.len()
            )),
        }
    }

    fn query(&mut self, a: &Args) -> Result<Reply, String> {
        let what = a.words.first().map(String::as_str).unwrap_or("");
        match what {
            "stat" => return self.built(&a.file).map(|(run, src)| Self::report(&run, &src)),
            "check" => return Ok(self.run("check", a, &Json::Null)),
            "file" => {
                let (_, src) = self.read(&a.file)?;
                return Ok(Reply {
                    ok: true,
                    json: Json::obj(vec![("ok", true.into()), ("text", src.as_str().into())]),
                    text: src,
                    png: None,
                });
            }
            "handles" | "params" | "features" => {
                let (p, src) = self.read(&a.file)?;
                let run = self.run_src(&p, &src, false);
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
        if a.at.is_some() {
            return Err("--at: a .fg part is asked as it finished".into());
        }
        let (run, _) = self.built(&a.file)?;
        let body = Self::body(&run)?;
        let ents = Self::select(body, &a.words.join(" "))?;
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
        let (run, _) = self.built(&a.file)?;
        let body = Self::body(&run)?;
        let words: Vec<&str> = a.words.iter().map(String::as_str).collect();
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
            [r] => measure::one(body, &mesh, Self::one(body, r)?),
            [r, s] => measure::between(body, &mesh, Self::one(body, r)?, Self::one(body, s)?),
            _ => return Err("measure takes mass, wall, or one or two @handles / (selectors)".into()),
        };
        let text = pairs
            .iter()
            .map(|(k, v)| format!("{k} {}", forge_agent::short(*v)))
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
        let (run, _) = self.built(&a.file)?;
        let body = Self::body(&run)?;
        let view = a.words.first().map_or("iso", String::as_str);
        let mut labels = Vec::new();
        if let Some(sel) = &a.labels {
            let handles = select::handles(body);
            for e in Self::select(body, sel)?.into_iter().take(40) {
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

    /// Typed ops, all applied or none: each checked as it goes (`apply_checked`), the file written
    /// once, then built. The reply: each op with what it reached, and the build's report.
    fn edit(&mut self, a: &Args, params: &Json) -> Result<Reply, String> {
        if !self.writable {
            return Err("this session does not write files (serve and mcp take --writable)".into());
        }
        let ops = ops_of(a, params)?;
        let (p, src) = self.read(&a.file)?;
        let mut text = src.clone();
        let mut done = Vec::new();
        for (k, op) in ops.iter().enumerate() {
            match edit::apply_checked(&text, op) {
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
                    r.text = format!("op {k} refused, nothing applied: {}", r.text);
                    return Ok(r);
                }
            }
        }
        std::fs::write(&p, &text).map_err(|_| format!("{}: cannot write", a.file))?;
        let run = self.run_src(&p, &text, true);
        let mut r = Self::report(&run, &text);
        obj_push(&mut r.json, "applied", Json::Arr(done));
        r.text = format!("applied {} ops\n{}", ops.len(), r.text);
        Ok(r)
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
}

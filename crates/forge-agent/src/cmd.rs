//! The commands, each answering in a few lines of text and in JSON, over a session that keeps
//! every file's build.

use crate::json::Json;
use crate::model::Model;
use crate::select::{self, Ent};
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

pub const COMMANDS: [&str; 7] = ["build", "query", "measure", "render", "check", "diff", "edit"];

#[derive(Default)]
pub struct Session {
    models: HashMap<String, Model>,
    /// Where files may be read from (serve and mcp); any `.fc` file when `None`.
    root: Option<std::path::PathBuf>,
    /// Whether `edit` may write files (off unless asked for).
    writable: bool,
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
            parse(read(&r).map_err(|e| format!("{name}: {e}"))?).map_err(|e| format!("{name}: {e:?}"))
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

    /// The session, letting `edit` write the files it changes or not.
    pub fn writable(mut self, on: bool) -> Session {
        self.writable = on;
        self
    }

    /// The file's model, brought up to date with what is on disk.
    pub fn model(&mut self, file: &str) -> Result<&mut Model, String> {
        self.model_at(file).map(|m| m.0)
    }

    /// `model`, and the file's real path.
    fn model_at(&mut self, file: &str) -> Result<(&mut Model, std::path::PathBuf), String> {
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
        let root = self.root.clone();
        let m = self.models.entry(file.to_string()).or_default();
        m.fonts = Some(std::sync::Arc::new(move |name: &str| {
            font_file(&dir, root.as_deref(), name)
        }));
        m.update(&src);
        Ok((m, real))
    }

    /// Changes the file through its document, writes it back and builds it.
    fn edit(&mut self, a: &Args) -> Result<Reply, String> {
        use forge_doc::Edit;
        if !self.writable {
            return Err("this session does not write files (serve and mcp take --writable)".into());
        }
        let (m, path) = self.model_at(&a.file)?;
        let w = &a.words;
        let word = |k: usize| {
            w.get(k)
                .map(String::as_str)
                .ok_or("edit needs more words (see AGENTS.md)")
        };
        let rest = |k: usize| {
            w.get(k..)
                .filter(|r| !r.is_empty())
                .map(|r| r.join(" "))
                .ok_or("edit needs a value")
        };
        let at = |k: usize| -> Result<(usize, Option<usize>), String> {
            let no: usize = word(k)?
                .parse()
                .map_err(|_| format!("'{}' is not a line number", word(k).unwrap_or("")))?;
            m.doc
                .at_line(no)
                .ok_or_else(|| format!("the file has no line {no}"))
        };
        let id = |k: usize| at(k).map(|(i, _)| m.doc.items[i].id);
        let e = match word(0)? {
            "param" => Edit::SetParam {
                name: word(1)?.to_string(),
                expr: rest(2)?,
            },
            "dim" => {
                let (i, row) = at(1)?;
                Edit::SetDimension {
                    item: m.doc.items[i].id,
                    row: row.ok_or("that line is not a sketch row")?,
                    value: rest(2)?,
                }
            }
            "arg" => Edit::SetArg {
                item: id(1)?,
                arg: word(2)?
                    .parse::<usize>()
                    .map_err(|_| "arg LINE K TEXT: K counts from 1")?
                    .wrapping_sub(1),
                text: rest(3)?,
            },
            "insert" => Edit::Insert {
                at: if word(1)? == "end" { usize::MAX } else { at(1)?.0 },
                text: rest(2)?.replace("\\n", "\n"),
            },
            "delete" => Edit::Delete { item: id(1)? },
            "move" => Edit::Move {
                item: id(1)?,
                to: if word(2)? == "end" { usize::MAX } else { at(2)?.0 },
            },
            op => return Err(format!("unknown edit '{op}' (param dim arg insert delete move)")),
        };
        m.edit(e)?;
        let src = m.src().to_string();
        replace(&path, &src).map_err(|e| format!("{}: {e}", a.file))?;
        self.build(a)
    }

    pub fn run(&mut self, cmd: &str, a: &Args) -> Reply {
        if a.at.is_some() && cmd != "query" {
            return fail("--at goes with query only");
        }
        if (a.labels.is_some() || a.px.is_some()) && cmd != "render" {
            return fail("--labels and --px go with render only");
        }
        let r = match cmd {
            "build" => self.build(a),
            "query" => self.query(a),
            "measure" => self.measure(a),
            "render" => self.render(a),
            "check" => self.check(a),
            "diff" => self.diff(a),
            "edit" => self.edit(a),
            _ => Err(format!(
                "unknown command '{cmd}' (one of: {})",
                COMMANDS.join(" ")
            )),
        };
        r.unwrap_or_else(fail)
    }

    fn build(&mut self, a: &Args) -> Result<Reply, String> {
        let m = self.model(&a.file)?;
        let n = m.lines.len();
        let mut text = String::new();
        let mut j: Vec<(&str, Json)> = vec![("lines", n.into()), ("built", m.built().into())];
        let ok = m.error.is_none();
        match m.error.clone() {
            None => {
                text.push_str(&format!("ok {n} lines {:.2}ms", m.ms));
                // a selector that picks fewer than meant fails quietly otherwise
                let picks = m.picks();
                if !picks.is_empty() {
                    let p: Vec<String> = picks.iter().map(|(l, c)| format!("line {l}: {c}")).collect();
                    text.push_str(&format!("\nselectors picked: {}", p.join(", ")));
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
            let (volume, area, bb, unmeshed) = m.stats();
            let body = m.body().expect("built");
            let bbox = [bb.x.lo, bb.y.lo, bb.z.lo, bb.x.hi, bb.y.hi, bb.z.hi];
            let (f, e, v) = counts(body);
            // a failed build's numbers are in the JSON; the text keeps to the error
            let bodies = m.solids_after(m.built()).len();
            if ok {
                text.push_str(&format!(
                    "\nvolume {} area {} (approx) bbox {}\nfaces {f} edges {e} verts {v}",
                    short(volume),
                    short(area),
                    bbox.map(short).join(" ")
                ));
                if bodies > 1 {
                    text.push_str(&format!(" bodies {bodies}"));
                }
            }
            if unmeshed > 0 {
                text.push_str(&format!(
                    "\n({unmeshed} faces would not mesh: volume, area approximate)"
                ));
            }
            let laws = law_starts(m);
            for (no, e, v) in &laws {
                text.push_str(&format!("\nline {no}: the law on {e} runs from {v}"));
            }
            j.push((
                "laws",
                Json::Arr(
                    laws.into_iter()
                        .map(|(no, e, v)| {
                            Json::obj(vec![("line", no.into()), ("edge", e.into()), ("from", v.into())])
                        })
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
                ("bodies", bodies.into()),
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
        let m = self.model(&a.file)?;
        let sel = a.words.join(" ");
        let at = Self::upto(m, a)?;
        let (body, ents) = m.select(&sel, at)?;
        let n = at.unwrap_or(m.built()).min(m.built());
        let several = m.solids_after(n).len() > 1;
        let handles = select::handles(body);
        let mut list = Vec::new();
        // pieces alike (a hole's two half faces) share a line
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
        let (body, mesh) = m.ready()?;
        let pairs: Vec<(&str, f64)> = match words[..] {
            ["mass"] => {
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
                ]
            }
            ["wall"] => match measure::wall(body, mesh) {
                Some((t, p)) => vec![("wall", t), ("x", p.x), ("y", p.y), ("z", p.z)],
                None => return Err("no wall found".into()),
            },
            [_] => measure::one(body, mesh, ents[0]),
            [_, _] => measure::between(body, mesh, ents[0], ents[1]),
            _ => return Err("measure takes mass, wall, or one or two @handles / (selectors)".into()),
        };
        let text = pairs
            .iter()
            .map(|(k, v)| format!("{k} {}", short(*v)))
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

/// Every built fillet's varying law whose start vertex was left to the default (the one
/// named first): its line, the edge's handle and the vertex's, on the part before that line.
fn law_starts(m: &Model) -> Vec<(usize, String, String)> {
    let mut out = Vec::new();
    for i in 0..m.built().min(m.features.len()) {
        let forge_model::Feature::Fillet { edges, starts, .. } = &m.features[i] else {
            continue;
        };
        let Some(body) = m.body_after(i) else {
            continue;
        };
        let handles = select::handles(body);
        for (n, r) in edges {
            if r.constant().is_some() || starts.iter().any(|s| s.0 == *n) {
                continue;
            }
            let Some(e) = body.edge_by_name(*n) else {
                continue;
            };
            let v = body.vertex(forge_ops::law_start(body, e)).name;
            if let (Some(eh), Some(vh)) = (handles.get(n), handles.get(&v)) {
                out.push((m.lines[i].no, eh.clone(), vh.clone()));
            }
        }
    }
    out
}

/// O_NOFOLLOW: open fails on a symbolic link (the value differs by platform).
#[cfg(unix)]
const NOFOLLOW: i32 = if cfg!(any(target_os = "macos", target_os = "ios", target_os = "freebsd")) {
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
fn replace(path: &std::path::Path, src: &str) -> Result<(), String> {
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
fn replace(_: &std::path::Path, _: &str) -> Result<(), String> {
    Err("writing part files needs a unix system".into())
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

    /// A varying fillet law left to its default start is reported with the vertex it runs
    /// from; naming that start in the file builds the same part and is not reported.
    #[test]
    fn law_starts_are_reported() {
        let dir = Dir::new("laws");
        let src = "base: extrude 8 rect 0 0 20 10\nfillet 1..3 (edges front top)\n";
        let f = dir.file("a.fc", src);
        let mut s = Session::default();
        let r = s.run("build", &args(&f, &[]));
        assert!(r.ok, "{}", r.text);
        let line = r
            .text
            .lines()
            .find(|l| l.contains("runs from"))
            .expect("reported")
            .to_string();
        let v = line.rsplit(' ').next().expect("vertex").to_string();
        let volume = |t: &str| {
            t.split_whitespace()
                .skip_while(|w| *w != "volume")
                .nth(1)
                .map(String::from)
        };
        let named = dir.file("b.fc", &src.replace("1..3 ", &format!("1..3 from {v} ")));
        let r2 = s.run("build", &args(&named, &[]));
        assert!(r2.ok && !r2.text.contains("runs from"), "{}", r2.text);
        assert_eq!(volume(&r.text), volume(&r2.text));
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
        assert!(r.text.ends_with("bodies 4"), "{}", r.text);
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

    /// The example AGENTS.md opens with builds, handles and all.
    #[test]
    fn the_agents_example_builds() {
        let doc = include_str!("../../../AGENTS.md");
        let dir = Dir::new("example");
        // the first part, and the parameters and sketch
        for k in [1, 3] {
            let example = doc.split("```").nth(k).expect("an example block");
            let f = dir.file("e.fc", example);
            let r = Session::default().run("build", &args(&f, &[]));
            assert!(r.ok, "{}", r.text);
        }
    }

    #[cfg(unix)]
    #[test]
    fn edits_write_the_file() {
        let dir = Dir::new("edit");
        let f = dir.file("e.fc", "w = 4\nextrude w rect 0 0 1 1  # a block\n");
        let r = Session::default().run("edit", &args(&f, &["param", "w", "5"]));
        assert!(r.text.contains("does not write files"), "{}", r.text);
        let mut s = Session::default().writable(true);
        let r = s.run("edit", &args(&f, &["param", "w", "5"]));
        assert!(r.ok && r.text.contains("bbox 0 0 0 1 1 5"), "{}", r.text);
        let r = s.run("edit", &args(&f, &["insert", "2", "h", "=", "2\\nd", "=", "h"]));
        assert!(r.ok, "{}", r.text);
        assert_eq!(
            std::fs::read_to_string(&f).expect("written"),
            "w = 5\nh = 2\nd = h\nextrude w rect 0 0 1 1  # a block\n"
        );
        assert_eq!(
            s.run("edit", &args(&f, &["dim", "1", "3"])).text,
            "error: that line is not a sketch row"
        );
    }

    /// A line's `in` body follows its line when an edit renumbers the lines above.
    #[cfg(unix)]
    #[test]
    fn body_references_follow_edits() {
        let dir = Dir::new("edit-in");
        let f = dir.file(
            "e.fc",
            "extrude new 10 rect 0 0 10 10\nextrude new 4 at 20 0 0 rect 0 0 10 10\nextrude cut 12 at 5 5 -1 circle 0 0 2 in extrude1\n",
        );
        let mut s = Session::default().writable(true);
        assert!(s.run("build", &args(&f, &[])).ok);
        let r = s.run(
            "edit",
            &args(
                &f,
                &[
                    "insert", "1", "extrude", "new", "3", "at", "-30", "0", "0", "rect", "0", "0", "5", "5",
                ],
            ),
        );
        assert!(r.ok, "{}", r.text);
        let text = std::fs::read_to_string(&f).expect("written");
        assert!(text.ends_with("circle 0 0 2 in extrude2\n"), "{text}");
        assert!(r.text.contains("bodies 3"), "{}", r.text);
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
            let r = s.run("edit", &args(&l.to_string_lossy(), &["param", "w", "9"]));
            assert!(!r.ok && r.text.contains(why), "{}", r.text);
        }
        assert_eq!(
            std::fs::read_to_string(&away).expect("read"),
            "w = 4\nextrude w rect 0 0 1 1\n"
        );
        let plain = dir.file("p.fc", "w = 4\nextrude w rect 0 0 1 1\n");
        assert!(s.run("edit", &args(&plain, &["param", "w", "9"])).ok);
        assert_eq!(
            std::fs::read_dir(&dir.0).expect("dir").count(),
            3,
            "no temporary file left"
        );
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
        assert!(q.text.starts_with("2 found"), "{}", q.text);
        let h = q
            .text
            .lines()
            .nth(1)
            .and_then(|l| l.split(' ').next())
            .expect("a handle")
            .to_string();
        let m = s.run("measure", &args(&a, &[&h]));
        assert!(m.text.contains("radius 4"), "{}", m.text);
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

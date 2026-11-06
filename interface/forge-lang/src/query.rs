//! Questions asked of a built part or assembly, answered in numbers: where its holes are, its
//! section by a plane, which holes share an axis, the gap between two faces, where parts clash,
//! where a sketch sits on a sheet. One text form for the CLI, the agent and the app; answers are
//! typed JSON, the same every run. Lengths mm, angles degrees, in the assembly's frame (a part
//! alone: the part's) unless asked `in OCC`.
//!
//! Names are stable: a face is its anchor text where the part's anchors name it, else its handle
//! (`@f..`, from the face's lineage); a hole is its wall's anchor text, else `hole[k]` in the
//! part's hole order (radius, then centre). In an assembly each is prefixed `OCC/`.

use crate::anchors::{Anchors, Asked};
use crate::{Options, Run};
use forge_agent::facemesh;
use forge_agent::features::{self, Hole, Piece};
use forge_agent::json::Json;
use forge_agent::measure::{self, Pick, Quantity};
use forge_agent::select::{self, Ent};
use forge_math::{Frame, Point3, UnitVec3, Vec3, dmath};
use forge_topo::{Body, FaceId, Name};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum Cut {
    /// `x|y|z = V`
    Axis(usize, f64),
    /// `at X Y Z normal NX NY NZ`
    Plane(Point3, Vec3),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Query {
    Holes {
        of: Option<String>,
        d: Option<f64>,
    },
    Section {
        cut: Cut,
        of: Option<String>,
    },
    Aligned {
        pair: Option<(String, String)>,
        tol: f64,
    },
    Gap {
        a: String,
        b: String,
    },
    Interference {
        pair: Option<(String, String)>,
    },
    Frame {
        what: String,
        of: Option<String>,
    },
    Faces {
        of: Option<String>,
    },
}

/// A query and the frame its answer is in (`in OCC`: that occurrence's part frame).
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub q: Query,
    pub local: Option<String>,
}

pub const HELP: &str = "holes [of OCC] [d D] | section x|y|z=V [of OCC] | section at X Y Z normal NX NY NZ [of OCC] | \
aligned [HOLE HOLE] [tol T] | gap FACE FACE | interference [OCC OCC] | frame SKETCH|PLANE [of OCC] | faces [of OCC]; \
any of them `in OCC` for that part's frame. In an assembly a name is OCC/NAME.";

fn num(w: Option<&String>, what: &str) -> Result<f64, String> {
    w.and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{what} takes a number"))
}

/// Words, with a parenthesised group (an anchor's text) kept as one.
fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for c in text.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            c if c.is_whitespace() && depth <= 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn parse(text: &str) -> Result<Request, String> {
    let mut w = words(text);
    let mut local = None;
    if let Some(k) = w.iter().position(|x| x == "in") {
        let occ = w.get(k + 1).cloned().ok_or("`in` takes an occurrence")?;
        w.drain(k..k + 2);
        local = Some(occ);
    }
    let mut of = None;
    if let Some(k) = w.iter().position(|x| x == "of") {
        let occ = w.get(k + 1).cloned().ok_or("`of` takes an occurrence")?;
        w.drain(k..k + 2);
        of = Some(occ);
    }
    let mut take = |key: &str| -> Result<Option<f64>, String> {
        match w.iter().position(|x| x == key) {
            Some(k) => {
                let v = num(w.get(k + 1), key)?;
                w.drain(k..k + 2);
                Ok(Some(v))
            }
            None => Ok(None),
        }
    };
    let d = take("d")?;
    let tol = take("tol")?;
    let head = w.first().map(String::as_str).unwrap_or("");
    let rest: Vec<String> = w.iter().skip(1).cloned().collect();
    let pair = |r: &[String]| -> Result<Option<(String, String)>, String> {
        match r {
            [] => Ok(None),
            [a, b] => Ok(Some((a.clone(), b.clone()))),
            _ => Err("takes two names or none".into()),
        }
    };
    let q = match head {
        "holes" if rest.is_empty() => Query::Holes { of, d },
        "section" => {
            let joined = rest.join(" ");
            let cut = if let Some((ax, v)) = joined.split_once('=') {
                let k = match ax.trim() {
                    "x" => 0,
                    "y" => 1,
                    "z" => 2,
                    o => return Err(format!("section {o}=..: x, y or z")),
                };
                Cut::Axis(k, num(Some(&v.trim().to_string()), "section")?)
            } else {
                match &rest[..] {
                    [at, x, y, z, nw, nx, ny, nz] if at == "at" && nw == "normal" => {
                        let p = Point3::new(num(Some(x), "at")?, num(Some(y), "at")?, num(Some(z), "at")?);
                        let n = Vec3::new(
                            num(Some(nx), "normal")?,
                            num(Some(ny), "normal")?,
                            num(Some(nz), "normal")?,
                        );
                        if n.normalize().is_none() {
                            return Err("section: the normal is zero".into());
                        }
                        Cut::Plane(p, n)
                    }
                    _ => return Err("section x|y|z=V, or section at X Y Z normal NX NY NZ".into()),
                }
            };
            Query::Section { cut, of }
        }
        "aligned" => Query::Aligned {
            pair: pair(&rest)?,
            tol: tol.unwrap_or(0.05),
        },
        "gap" => match &rest[..] {
            [a, b] => Query::Gap {
                a: a.clone(),
                b: b.clone(),
            },
            _ => return Err("gap takes two faces".into()),
        },
        "interference" => Query::Interference { pair: pair(&rest)? },
        "frame" => match &rest[..] {
            [x] => Query::Frame { what: x.clone(), of },
            _ => return Err("frame takes a sketch or a plane (`frame rh`, `frame right.top`)".into()),
        },
        "faces" if rest.is_empty() => Query::Faces { of },
        _ => return Err(format!("not a query: '{text}'. {HELP}")),
    };
    Ok(Request { q, local })
}

/// One placed body: an occurrence of a part (its path) or the part alone (no path).
struct Item {
    occ: String,
    body: Arc<Body>,
    pose: Frame,
    part: usize,
}

/// What a part knows of itself: its anchors, what was asked of it, its sheets.
struct Part {
    anchors: Anchors,
    asked: Vec<(String, Asked)>,
    sheets: Vec<crate::sheet::SheetOut>,
    handles: HashMap<Name, String>,
    holes: Vec<(String, Hole)>,
}

struct Scene {
    items: Vec<Item>,
    parts: Vec<Part>,
    assembly: bool,
}

/// An error, typed.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryError {
    pub kind: &'static str,
    pub text: String,
}

fn qe(kind: &'static str, text: impl Into<String>) -> QueryError {
    QueryError {
        kind,
        text: text.into(),
    }
}

/// An answer: its record and its words.
#[derive(Clone, Debug)]
pub struct Answer {
    pub ok: bool,
    pub json: Json,
    pub text: String,
}

impl Answer {
    fn err(e: QueryError) -> Answer {
        Answer {
            ok: false,
            text: format!("error {}: {}", e.kind, e.text),
            json: Json::obj(vec![
                ("ok", false.into()),
                (
                    "error",
                    Json::obj(vec![("kind", e.kind.into()), ("text", e.text.as_str().into())]),
                ),
            ]),
        }
    }
}

fn r9(v: f64) -> f64 {
    let r = (v * 1e9).round() / 1e9;
    if r == 0.0 { 0.0 } else { r }
}

fn pt(p: Point3) -> Json {
    Json::Arr(vec![r9(p.x).into(), r9(p.y).into(), r9(p.z).into()])
}

fn vt(v: Vec3) -> Json {
    Json::Arr(vec![r9(v.x).into(), r9(v.y).into(), r9(v.z).into()])
}

fn sh(v: f64) -> String {
    forge_agent::short(r9(v))
}

fn pts(p: Point3) -> String {
    format!("({}, {}, {})", sh(p.x), sh(p.y), sh(p.z))
}

/// The texts a query names (faces, holes, planes) that a part must evaluate, by occurrence.
fn refs(q: &Query) -> Vec<String> {
    match q {
        Query::Gap { a, b } => vec![a.clone(), b.clone()],
        Query::Aligned {
            pair: Some((a, b)), ..
        } => vec![a.clone(), b.clone()],
        Query::Frame { what, .. } => vec![what.clone()],
        _ => vec![],
    }
}

/// Runs the document at `path` and answers the query on it.
pub fn ask_file(path: &Path, root: Option<PathBuf>, text: &str) -> Answer {
    match std::fs::read_to_string(path) {
        Ok(src) => ask(&src, path.parent().map(Path::to_path_buf), root, text),
        Err(e) => Answer::err(qe("File", format!("{}: {e}", path.display()))),
    }
}

/// Runs `src` (a part or an assembly, its files beside `dir`) and answers the query.
pub fn ask(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>, text: &str) -> Answer {
    let req = match parse(text) {
        Ok(r) => r,
        Err(e) => return Answer::err(qe("Query", e)),
    };
    match scene(src, dir, root, &req).and_then(|s| answer(&s, &req)) {
        Ok(a) => a,
        Err(e) => Answer::err(e),
    }
}

fn part_opts(dir: Option<PathBuf>, root: Option<PathBuf>, asks: Vec<String>) -> Options {
    Options {
        dir,
        root,
        anchors: true,
        checks: false,
        expects: false,
        overlap: false,
        meshes: false,
        render: true,
        asks,
        ..Options::default()
    }
}

/// `OCC/TEXT` split at the longest occurrence path it starts with; a part alone: no path.
fn split<'a>(items: &[Item], assembly: bool, r: &'a str) -> Result<(usize, &'a str), QueryError> {
    if !assembly {
        return Ok((0, r));
    }
    let mut best: Option<(usize, usize)> = None;
    for (k, it) in items.iter().enumerate() {
        if let Some(rest) = r.strip_prefix(it.occ.as_str())
            && rest.starts_with('/')
            && best.is_none_or(|b| it.occ.len() > b.1)
        {
            best = Some((k, it.occ.len()));
        }
    }
    match best {
        Some((k, n)) => Ok((k, &r[n + 1..])),
        None => Err(qe(
            "NoSuchOccurrence",
            format!(
                "'{r}': an assembly's names are OCC/NAME, OCC one of {}",
                occ_list(items)
            ),
        )),
    }
}

fn occ_list(items: &[Item]) -> String {
    let mut v: Vec<&str> = items.iter().map(|i| i.occ.as_str()).collect();
    v.dedup();
    v.join(", ")
}

fn scene(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>, req: &Request) -> Result<Scene, QueryError> {
    let opts = part_opts(dir.clone(), root.clone(), vec![]);
    let (run, _) = crate::run(src, opts, Default::default());
    let assembly = run.asm.as_ref().is_some_and(|a| !a.occurrences.is_empty());
    let texts = refs(&req.q);
    if !assembly {
        let asks: Vec<String> = texts.iter().filter(|t| !plain_name(t)).cloned().collect();
        let run = if asks.is_empty() {
            run
        } else {
            crate::run(src, part_opts(dir, root, asks), Default::default()).0
        };
        let body = run.body.clone().ok_or_else(|| not_built(&run))?;
        let part = part_of(&run, &body);
        return Ok(Scene {
            items: vec![Item {
                occ: String::new(),
                body: Arc::new(body),
                pose: Frame::WORLD,
                part: 0,
            }],
            parts: vec![part],
            assembly: false,
        });
    }
    let a = run.asm.as_ref().expect("an assembly");
    if !run.errors.is_empty() && a.draw.is_empty() {
        return Err(not_built(&run));
    }
    let mut items = Vec::new();
    let mut k = 0;
    for (i, (occ, _, _)) in a.occurrences.iter().enumerate() {
        if a.occ_part.get(i).copied().flatten().is_none() {
            continue;
        }
        let (body, pose) = &a.draw[k];
        k += 1;
        items.push((occ.clone(), body.clone(), *pose, a.occurrences[i].1.clone()));
    }
    // each part file once, asked what the query names in any of its occurrences
    let mut files: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (occ, _, _, file) in &items {
        let e = files.entry(file.clone()).or_default();
        for t in &texts {
            if let Some(rest) = t.strip_prefix(occ.as_str()).and_then(|r| r.strip_prefix('/'))
                && !plain_name(rest)
                && !e.iter().any(|x| x == rest)
            {
                e.push(rest.to_string());
            }
        }
    }
    let mut parts = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::new();
    for (occ, body, pose, file) in items {
        let k = match index.get(&file) {
            Some(&k) => k,
            None => {
                let p = Path::new(&file);
                let part_run = std::fs::read_to_string(p).ok().map(|text| {
                    let o = part_opts(
                        p.parent().map(Path::to_path_buf),
                        root.clone(),
                        files[&file].clone(),
                    );
                    crate::run(&text, o, Default::default()).0
                });
                let part = match part_run {
                    // its own build names faces as the placed one does (same text, same lineage)
                    Some(r) => part_of(&r, &body),
                    None => part_of_body(&body),
                };
                parts.push(part);
                index.insert(file.clone(), parts.len() - 1);
                parts.len() - 1
            }
        };
        out.push(Item {
            occ,
            body,
            pose,
            part: k,
        });
    }
    Ok(Scene {
        items: out,
        parts,
        assembly: true,
    })
}

fn not_built(run: &Run) -> QueryError {
    match run.errors.first() {
        Some(e) => qe(
            "NotBuilt",
            format!("the document did not build: {}: {}", e.kind(), e.text()),
        ),
        None => qe("NoGeometry", "the document has no solid"),
    }
}

fn plain_name(t: &str) -> bool {
    t.starts_with('@') || t.starts_with("hole[") || t.contains('#')
}

fn part_of_body(body: &Body) -> Part {
    let mut p = Part {
        anchors: Anchors::default(),
        asked: vec![],
        sheets: vec![],
        handles: select::handles(body),
        holes: vec![],
    };
    p.holes = name_holes(body, &p);
    p
}

fn part_of(run: &Run, body: &Body) -> Part {
    let mut p = part_of_body(body);
    p.anchors = run.anchors.clone().unwrap_or_default();
    p.asked = run.asked.clone();
    p.sheets = run.sheets.clone();
    p.holes = name_holes(body, &p);
    p
}

fn face_name(body: &Body, part: &Part, f: FaceId) -> String {
    let n = body.face(f).name;
    part.anchors
        .faces
        .get(&n)
        .or_else(|| part.handles.get(&n))
        .cloned()
        .unwrap_or_else(|| format!("@f{:x}", n.0))
}

/// Each hole with its name: its wall's anchor text, numbered (`#k`, in hole order) where that
/// text names more than one hole; `hole[k]` where no anchor names it.
fn name_holes(body: &Body, part: &Part) -> Vec<(String, Hole)> {
    let all = features::holes(body);
    let texts: Vec<Option<String>> = all
        .iter()
        .map(|h| {
            let t: Vec<&String> = h
                .faces
                .iter()
                .filter_map(|f| part.anchors.faces.get(&body.face(*f).name))
                .collect();
            t.first()
                .filter(|x| t.iter().all(|y| y == *x))
                .map(|x| x.to_string())
        })
        .collect();
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut out = Vec::new();
    for (k, h) in all.iter().enumerate() {
        let name = match &texts[k] {
            Some(t) => {
                let many = texts.iter().filter(|x| x.as_ref() == Some(t)).count() > 1;
                let i = seen.entry(t.as_str()).or_default();
                *i += 1;
                if many {
                    format!("{t}#{}", *i - 1)
                } else {
                    t.clone()
                }
            }
            None => format!("hole[{k}]"),
        };
        out.push((name, h.clone()));
    }
    out
}

impl Scene {
    fn full(&self, it: &Item, name: &str) -> String {
        if self.assembly {
            format!("{}/{name}", it.occ)
        } else {
            name.to_string()
        }
    }

    /// The frame answers are in: the world, or an occurrence's part frame.
    fn out_frame(&self, local: &Option<String>) -> Result<(Frame, String), QueryError> {
        match local {
            None => Ok((
                Frame::WORLD,
                if self.assembly { "assembly" } else { "part" }.into(),
            )),
            Some(o) => {
                let it = self.items.iter().find(|i| i.occ == *o).ok_or_else(|| {
                    qe(
                        "NoSuchOccurrence",
                        format!("in {o}: no such occurrence; one of {}", occ_list(&self.items)),
                    )
                })?;
                Ok((it.pose.inverse(), o.clone()))
            }
        }
    }

    fn filter(&self, of: &Option<String>) -> Result<Vec<usize>, QueryError> {
        let ks: Vec<usize> = (0..self.items.len())
            .filter(|&k| {
                of.as_ref().is_none_or(|o| {
                    self.items[k].occ == *o || self.items[k].occ.starts_with(&format!("{o}."))
                })
            })
            .collect();
        if ks.is_empty() {
            return Err(qe(
                "NoSuchOccurrence",
                format!(
                    "of {}: no such occurrence; one of {}",
                    of.as_deref().unwrap_or(""),
                    occ_list(&self.items)
                ),
            ));
        }
        Ok(ks)
    }

    /// The faces a name stands for: a handle, a hole, an anchor's text, or what the part asked.
    fn faces(&self, r: &str) -> Result<(usize, Vec<FaceId>), QueryError> {
        let (k, t) = split(&self.items, self.assembly, r)?;
        let it = &self.items[k];
        let part = &self.parts[it.part];
        let body = &it.body;
        if t.starts_with('@') {
            return match select::by_handle(body, t) {
                Ok(Ent::Face(f)) => Ok((k, vec![f])),
                Ok(_) => Err(qe("NotAFace", format!("{r} is not a face"))),
                Err(e) => Err(qe("NoSuchFace", e)),
            };
        }
        if let Some((_, h)) = part.holes.iter().find(|(n, _)| n == t) {
            return Ok((k, h.faces.clone()));
        }
        let named: Vec<FaceId> = body
            .face_ids()
            .filter(|f| {
                part.anchors
                    .faces
                    .get(&body.face(*f).name)
                    .is_some_and(|x| x == t)
            })
            .collect();
        if !named.is_empty() {
            return Ok((k, named));
        }
        match part.asked.iter().find(|(x, _)| x == t).map(|x| &x.1) {
            Some(Asked::Faces(names, _)) => {
                let fs: Vec<FaceId> = names
                    .iter()
                    .flat_map(|n| crate::build::role_faces(body, *n))
                    .collect();
                if fs.is_empty() {
                    Err(qe(
                        "NoSuchFace",
                        format!("{r} names no face of the finished part"),
                    ))
                } else {
                    Ok((k, fs))
                }
            }
            Some(Asked::Nothing) | None => Err(qe(
                "NoSuchFace",
                format!(
                    "{r}: not a face of the part (an anchor's text, @handle or hole name; `faces` lists them)"
                ),
            )),
            Some(other) => Err(qe("NotAFace", format!("{r} is {}, not faces", asked_kind(other)))),
        }
    }

    fn hole(&self, r: &str) -> Result<(usize, &str, &Hole), QueryError> {
        let (k, t) = split(&self.items, self.assembly, r)?;
        let it = &self.items[k];
        let part = &self.parts[it.part];
        if let Some((n, h)) = part.holes.iter().find(|(n, _)| n == t) {
            return Ok((k, n, h));
        }
        // a face of a hole, by any of its names
        let (_, fs) = self.faces(r)?;
        part.holes
            .iter()
            .find(|(_, h)| fs.iter().any(|f| h.faces.contains(f)))
            .map(|(n, h)| (k, n.as_str(), h))
            .ok_or_else(|| qe("NotAHole", format!("{r} is not a round hole's wall")))
    }
}

fn asked_kind(a: &Asked) -> String {
    match a {
        Asked::Plane(_) => "a plane".into(),
        Asked::Faces(..) => "faces".into(),
        Asked::Edges(_) => "edges".into(),
        Asked::Point(_) => "a point".into(),
        Asked::Axis(..) => "an axis".into(),
        Asked::Other(t) => format!("a {t}"),
        Asked::Nothing => "nothing".into(),
    }
}

/// A hole as the answer frame sees it: centre, axis.
fn hole_at(it: &Item, h: &Hole, to: &Frame) -> (Point3, Vec3) {
    let f = to.compose(&it.pose);
    (f.point_to_world(h.centre), f.vec_to_world(h.axis))
}

fn hole_json(name: String, occ: &str, it: &Item, h: &Hole, to: &Frame) -> Json {
    let (c, a) = hole_at(it, h, to);
    Json::obj(vec![
        ("name", name.into()),
        ("occurrence", occ.into()),
        ("centre", pt(c)),
        ("axis", vt(a)),
        ("r", r9(h.radius).into()),
        ("d", r9(2.0 * h.radius).into()),
        ("depth", r9(h.depth).into()),
        ("thru", h.thru.into()),
    ])
}

fn answer(s: &Scene, req: &Request) -> Result<Answer, QueryError> {
    let (to, frame_name) = s.out_frame(&req.local)?;
    let mut fields: Vec<(&str, Json)> = vec![("ok", true.into())];
    let mut text = String::new();
    match &req.q {
        Query::Holes { of, d } => {
            let mut list = Vec::new();
            for k in s.filter(of)? {
                let it = &s.items[k];
                for (n, h) in &s.parts[it.part].holes {
                    if d.is_some_and(|d| (2.0 * h.radius - d).abs() > 1e-3) {
                        continue;
                    }
                    let name = s.full(it, n);
                    let (c, a) = hole_at(it, h, &to);
                    text.push_str(&format!(
                        "{name}  d {}  centre {}  axis {}  depth {}{}\n",
                        sh(2.0 * h.radius),
                        pts(c),
                        pts(Point3::ORIGIN + a),
                        sh(h.depth),
                        if h.thru { "  thru" } else { "" }
                    ));
                    list.push(hole_json(name, &it.occ, it, h, &to));
                }
            }
            text.insert_str(0, &format!("{} holes ({frame_name} frame)\n", list.len()));
            fields.push(("query", "holes".into()));
            fields.push(("holes", Json::Arr(list)));
        }
        Query::Section { cut, of } => {
            let world = match cut {
                Cut::Axis(k, v) => {
                    let mut o = [0.0; 3];
                    o[*k] = *v;
                    let n = [UnitVec3::X, UnitVec3::Y, UnitVec3::Z][*k];
                    let x = [Vec3::Y, Vec3::Z, Vec3::X][*k];
                    Frame::from_zx(Point3::new(o[0], o[1], o[2]), n, x).expect("square axes")
                }
                Cut::Plane(p, n) => Frame::from_z(*p, n.normalize().expect("checked")),
            };
            // the plane given in the answer's frame
            let world = to.inverse().compose(&world);
            let mut loops = Vec::new();
            let (mut total, mut count) = (0.0, 0);
            for k in s.filter(of)? {
                let it = &s.items[k];
                let local = it.pose.inverse().compose(&world);
                let ls = features::section(&it.body, &local).map_err(|e| qe("SectionFailed", e))?;
                let back = to.compose(&it.pose);
                for l in ls {
                    count += 1;
                    total += l.area;
                    let mut pieces = Vec::new();
                    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
                    for (f, p) in &l.pieces {
                        let face = s.full(it, &face_name(&it.body, &s.parts[it.part], *f));
                        for q in p.points() {
                            let q = back.point_to_world(q);
                            for (j, v) in [q.x, q.y, q.z].into_iter().enumerate() {
                                lo[j] = lo[j].min(v);
                                hi[j] = hi[j].max(v);
                            }
                        }
                        pieces.push(match p {
                            Piece::Line(a, b) => Json::obj(vec![
                                ("kind", "line".into()),
                                ("from", pt(back.point_to_world(*a))),
                                ("to", pt(back.point_to_world(*b))),
                                ("face", face.into()),
                            ]),
                            Piece::Arc(c, r, q) => Json::obj(vec![
                                ("kind", "arc".into()),
                                ("centre", pt(back.point_to_world(*c))),
                                ("r", r9(*r).into()),
                                ("from", pt(back.point_to_world(q[0]))),
                                ("to", pt(back.point_to_world(q[q.len() - 1]))),
                                ("face", face.into()),
                            ]),
                            Piece::Curve(q) => Json::obj(vec![
                                ("kind", "curve".into()),
                                (
                                    "points",
                                    Json::Arr(q.iter().map(|x| pt(back.point_to_world(*x))).collect()),
                                ),
                                ("face", face.into()),
                            ]),
                        });
                    }
                    let kind = if !l.closed {
                        "open"
                    } else if l.area > 0.0 {
                        "outline"
                    } else {
                        "hole"
                    };
                    text.push_str(&format!(
                        "{}{kind} area {} length {} x {}..{} y {}..{} z {}..{}, {} pieces\n",
                        if s.assembly {
                            format!("{}: ", it.occ)
                        } else {
                            String::new()
                        },
                        sh(l.area.abs()),
                        sh(l.length),
                        sh(lo[0]),
                        sh(hi[0]),
                        sh(lo[1]),
                        sh(hi[1]),
                        sh(lo[2]),
                        sh(hi[2]),
                        l.pieces.len()
                    ));
                    for p in &l.pieces {
                        text.push_str(&format!("  {}\n", piece_text(&p.1, &back)));
                    }
                    loops.push(Json::obj(vec![
                        ("occurrence", it.occ.as_str().into()),
                        ("kind", kind.into()),
                        ("area", r9(l.area.abs()).into()),
                        ("length", r9(l.length).into()),
                        ("min", pt(Point3::new(lo[0], lo[1], lo[2]))),
                        ("max", pt(Point3::new(hi[0], hi[1], hi[2]))),
                        ("pieces", Json::Arr(pieces)),
                    ]));
                }
            }
            text.insert_str(
                0,
                &format!(
                    "{count} loops, material area {} ({frame_name} frame)\n",
                    sh(total)
                ),
            );
            fields.push(("query", "section".into()));
            fields.push((
                "plane",
                Json::obj(vec![
                    ("origin", pt(to.point_to_world(world.origin))),
                    ("normal", vt(to.vec_to_world(world.z().get()))),
                ]),
            ));
            fields.push(("area", r9(total).into()));
            fields.push(("loops", Json::Arr(loops)));
        }
        Query::Aligned { pair, tol } => {
            fields.push(("query", "aligned".into()));
            let line = |a: (usize, &str, &Hole), b: (usize, &str, &Hole)| {
                let (ia, ib) = (&s.items[a.0], &s.items[b.0]);
                let (ca, da) = hole_at(ia, a.2, &Frame::WORLD);
                let (cb, db) = hole_at(ib, b.2, &Frame::WORLD);
                let ang = dmath::acos(da.dot(db).abs().clamp(0.0, 1.0)) * 180.0 / dmath::PI;
                let w = cb - ca;
                let off = (w - da * w.dot(da)).length();
                (off, ang)
            };
            if let Some((a, b)) = pair {
                let ha = s.hole(a)?;
                let hb = s.hole(b)?;
                let (off, ang) = line(ha, hb);
                let ok = off <= *tol && ang <= 0.05;
                text = format!(
                    "{} {} and {}: axis offset {} angle {} (d {} and {}), tol {}\n",
                    if ok { "coaxial" } else { "NOT coaxial" },
                    s.full(&s.items[ha.0], ha.1),
                    s.full(&s.items[hb.0], hb.1),
                    sh(off),
                    sh(ang),
                    sh(2.0 * ha.2.radius),
                    sh(2.0 * hb.2.radius),
                    sh(*tol),
                );
                fields.extend([
                    ("coaxial", ok.into()),
                    ("offset", r9(off).into()),
                    ("angle", r9(ang).into()),
                    ("tol", r9(*tol).into()),
                    (
                        "a",
                        hole_json(
                            s.full(&s.items[ha.0], ha.1),
                            &s.items[ha.0].occ,
                            &s.items[ha.0],
                            ha.2,
                            &to,
                        ),
                    ),
                    (
                        "b",
                        hole_json(
                            s.full(&s.items[hb.0], hb.1),
                            &s.items[hb.0].occ,
                            &s.items[hb.0],
                            hb.2,
                            &to,
                        ),
                    ),
                ]);
            } else {
                // every hole on one axis with a hole of another occurrence (or another part's)
                let all: Vec<(usize, &str, &Hole)> = s
                    .items
                    .iter()
                    .enumerate()
                    .flat_map(|(k, it)| {
                        s.parts[it.part]
                            .holes
                            .iter()
                            .map(move |(n, h)| (k, n.as_str(), h))
                    })
                    .collect();
                let mut group: Vec<Option<usize>> = vec![None; all.len()];
                let mut groups: Vec<Vec<usize>> = Vec::new();
                for i in 0..all.len() {
                    for j in i + 1..all.len() {
                        if all[i].0 == all[j].0 && s.assembly {
                            continue;
                        }
                        let (off, ang) = line(all[i], all[j]);
                        if off > *tol || ang > 0.05 {
                            continue;
                        }
                        match (group[i], group[j]) {
                            (Some(g), None) => {
                                groups[g].push(j);
                                group[j] = Some(g);
                            }
                            (None, Some(g)) => {
                                groups[g].push(i);
                                group[i] = Some(g);
                            }
                            (None, None) => {
                                groups.push(vec![i, j]);
                                group[i] = Some(groups.len() - 1);
                                group[j] = Some(groups.len() - 1);
                            }
                            (Some(_), Some(_)) => {}
                        }
                    }
                }
                let mut out = Vec::new();
                for g in &groups {
                    let first = all[g[0]];
                    let mut worst: f64 = 0.0;
                    let mut members = Vec::new();
                    for &m in g {
                        let (off, _) = line(first, all[m]);
                        worst = worst.max(off);
                        let it = &s.items[all[m].0];
                        members.push(hole_json(s.full(it, all[m].1), &it.occ, it, all[m].2, &to));
                    }
                    let names: Vec<String> =
                        g.iter().map(|&m| s.full(&s.items[all[m].0], all[m].1)).collect();
                    let (c, a) = hole_at(&s.items[first.0], first.2, &to);
                    text.push_str(&format!(
                        "axis through {} along {}: {} (offset up to {})\n",
                        pts(c),
                        pts(Point3::ORIGIN + a),
                        names.join(", "),
                        sh(worst)
                    ));
                    out.push(Json::obj(vec![
                        ("axis", vt(a)),
                        ("through", pt(c)),
                        ("offset", r9(worst).into()),
                        ("holes", Json::Arr(members)),
                    ]));
                }
                text.insert_str(
                    0,
                    &format!(
                        "{} shared axes, tol {} ({frame_name} frame)\n",
                        out.len(),
                        sh(*tol)
                    ),
                );
                fields.push(("tol", r9(*tol).into()));
                fields.push(("axes", Json::Arr(out)));
            }
        }
        Query::Gap { a, b } => {
            let (ka, fa) = s.faces(a)?;
            let (kb, fb) = s.faces(b)?;
            let (ia, ib) = (&s.items[ka], &s.items[kb]);
            let ma = mesh_of(s, ia);
            let mb = mesh_of(s, ib);
            let mut best: Option<(f64, Point3, Point3, FaceId, FaceId, bool)> = None;
            for &x in &fa {
                for &y in &fb {
                    let pa = Pick {
                        body: &ia.body,
                        mesh: &ma,
                        at: ia.pose,
                        ent: Ent::Face(x),
                    };
                    let pb = Pick {
                        body: &ib.body,
                        mesh: &mb,
                        at: ib.pose,
                        ent: Ent::Face(y),
                    };
                    let r = measure::read_two(&pa, &pb);
                    let Some(d) = r.get(Quantity::Distance) else {
                        continue;
                    };
                    let (p, q) = r
                        .shown
                        .iter()
                        .find_map(|x| match x {
                            measure::Shown::Line(p, q, Quantity::Distance) => Some((*p, *q)),
                            _ => None,
                        })
                        .unwrap_or((Point3::ORIGIN, Point3::ORIGIN));
                    let par = match (
                        select::plane_normal(&ia.body, x).map(|n| ia.pose.vec_to_world(n)),
                        select::plane_normal(&ib.body, y).map(|n| ib.pose.vec_to_world(n)),
                    ) {
                        (Some(u), Some(v)) => u.cross(v).length() < 1e-9,
                        _ => false,
                    };
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, p, q, x, y, par));
                    }
                }
            }
            let (d, p, q, x, y, par) = best.ok_or_else(|| qe("NoGeometry", "no distance between them"))?;
            let na = s.full(ia, &face_name(&ia.body, &s.parts[ia.part], x));
            let nb = s.full(ib, &face_name(&ib.body, &s.parts[ib.part], y));
            let (p, q) = (to.point_to_world(p), to.point_to_world(q));
            text = format!(
                "gap {} between {na} and {nb}{}: {} to {}\n",
                sh(d),
                if par { " (parallel planes)" } else { "" },
                pts(p),
                pts(q)
            );
            fields.extend([
                ("query", "gap".into()),
                ("gap", r9(d).into()),
                ("parallel", par.into()),
                ("a", Json::obj(vec![("face", na.into()), ("at", pt(p))])),
                ("b", Json::obj(vec![("face", nb.into()), ("at", pt(q))])),
            ]);
        }
        Query::Interference { pair } => {
            let pairs: Vec<(usize, usize)> = match pair {
                Some((a, b)) => {
                    let ka = s.filter(&Some(a.clone()))?;
                    let kb = s.filter(&Some(b.clone()))?;
                    ka.iter()
                        .flat_map(|&x| kb.iter().map(move |&y| (x, y)))
                        .filter(|(x, y)| x != y)
                        .collect()
                }
                None => (0..s.items.len())
                    .flat_map(|x| (x + 1..s.items.len()).map(move |y| (x, y)))
                    .collect(),
            };
            let world: Vec<Option<Body>> = s.items.iter().map(|_| None).collect();
            let mut world = world;
            let boxes: Vec<forge_math::Box3> = s
                .items
                .iter()
                .map(|it| {
                    let b = &it.body;
                    let bb = b
                        .face_ids()
                        .fold(forge_math::Box3::EMPTY, |x, f| x.hull(&b.face_box(f)));
                    let mut w = forge_math::Box3::EMPTY;
                    for c in 0..8 {
                        let p = Point3::new(
                            if c & 1 == 0 { bb.x.lo } else { bb.x.hi },
                            if c & 2 == 0 { bb.y.lo } else { bb.y.hi },
                            if c & 4 == 0 { bb.z.lo } else { bb.z.hi },
                        );
                        w = w.hull(&forge_math::Box3::from_point(it.pose.point_to_world(p)));
                    }
                    w
                })
                .collect();
            let mut list = Vec::new();
            let mut volume = 0.0;
            for (x, y) in pairs {
                if !boxes[x].overlaps(&boxes[y]) {
                    continue;
                }
                for k in [x, y] {
                    if world[k].is_none() {
                        world[k] = Some(forge_asm::check::rigid_copy(&s.items[k].body, &s.items[k].pose));
                    }
                }
                let (bx, by) = (world[x].as_ref().expect("made"), world[y].as_ref().expect("made"));
                let c = match features::clash(bx, by) {
                    Ok(Some(c)) => c,
                    Ok(None) => continue,
                    Err(e) => {
                        return Err(qe(
                            "BooleanFailed",
                            format!("{} and {}: {e}", s.items[x].occ, s.items[y].occ),
                        ));
                    }
                };
                let (ix, iy) = (&s.items[x], &s.items[y]);
                let names = |it: &Item, fs: &[FaceId]| -> Vec<String> {
                    let mut v: Vec<String> = fs
                        .iter()
                        .map(|f| s.full(it, &face_name(&it.body, &s.parts[it.part], *f)))
                        .collect();
                    v.sort();
                    v.dedup();
                    v
                };
                let (na, nb) = (names(ix, &c.faces_a), names(iy, &c.faces_b));
                let (lo, hi, cen) = (
                    to.point_to_world(c.lo),
                    to.point_to_world(c.hi),
                    to.point_to_world(c.centroid),
                );
                let (lo, hi) = (
                    Point3::new(lo.x.min(hi.x), lo.y.min(hi.y), lo.z.min(hi.z)),
                    Point3::new(lo.x.max(hi.x), lo.y.max(hi.y), lo.z.max(hi.z)),
                );
                volume += c.volume;
                text.push_str(&format!(
                    "{} x {}: volume {} centre {} box {}..{}\n  {}: {}\n  {}: {}\n",
                    ix.occ,
                    iy.occ,
                    sh(c.volume),
                    pts(cen),
                    pts(lo),
                    pts(hi),
                    ix.occ,
                    na.join(", "),
                    iy.occ,
                    nb.join(", ")
                ));
                let arr = |v: Vec<String>| Json::Arr(v.into_iter().map(Json::from).collect());
                list.push(Json::obj(vec![
                    ("a", ix.occ.as_str().into()),
                    ("b", iy.occ.as_str().into()),
                    ("volume", r9(c.volume).into()),
                    ("centre", pt(cen)),
                    ("min", pt(lo)),
                    ("max", pt(hi)),
                    ("faces_a", arr(na)),
                    ("faces_b", arr(nb)),
                ]));
            }
            text.insert_str(
                0,
                &format!(
                    "{} interferences, volume {} ({frame_name} frame)\n",
                    list.len(),
                    sh(volume)
                ),
            );
            fields.push(("query", "interference".into()));
            fields.push(("volume", r9(volume).into()));
            fields.push(("interferences", Json::Arr(list)));
        }
        Query::Frame { what, of } => {
            let (k, t) = match of {
                Some(o) => (*s.filter(&Some(o.clone()))?.first().expect("one"), what.as_str()),
                None => split(&s.items, s.assembly, what)?,
            };
            let it = &s.items[k];
            let part = &s.parts[it.part];
            let frame = match part.asked.iter().find(|(x, _)| x == t).map(|x| &x.1) {
                Some(Asked::Plane(f)) | Some(Asked::Faces(_, Some(f))) => *f,
                Some(Asked::Nothing) | None => {
                    return Err(qe(
                        "NoSuchPlane",
                        format!("{what}: not a sketch or plane of the part"),
                    ));
                }
                Some(other) => {
                    return Err(qe(
                        "NotAPlane",
                        format!("{what} is {}, not a plane", asked_kind(other)),
                    ));
                }
            };
            let w = to.compose(&it.pose).compose(&frame);
            let (x, y, z) = (w.x().get(), w.y().get(), w.z().get());
            text = format!(
                "{what}: origin {} x {} y {} z {} ({frame_name} frame)\n",
                pts(w.origin),
                pts(Point3::ORIGIN + x),
                pts(Point3::ORIGIN + y),
                pts(Point3::ORIGIN + z)
            );
            fields.extend([
                ("query", "frame".into()),
                ("name", s.full(it, t).into()),
                ("origin", pt(w.origin)),
                ("x", vt(x)),
                ("y", vt(y)),
                ("z", vt(z)),
            ]);
            if let Some((sheet, wall, at, dx)) = on_sheet(part, &frame) {
                let th = dmath::atan2(dx.1, dx.0) * 180.0 / dmath::PI;
                text.push_str(&format!(
                    "on {sheet}'s wall {wall}; on the flat: origin ({}, {}), x turned {} deg from the flat's x\n",
                    sh(at.0),
                    sh(at.1),
                    sh(th)
                ));
                fields.push((
                    "sheet",
                    Json::obj(vec![
                        ("sheet", sheet.into()),
                        ("wall", wall.into()),
                        ("flat_origin", Json::Arr(vec![r9(at.0).into(), r9(at.1).into()])),
                        ("flat_x", Json::Arr(vec![r9(dx.0).into(), r9(dx.1).into()])),
                    ]),
                ));
            }
        }
        Query::Faces { of } => {
            let mut list = Vec::new();
            for k in s.filter(of)? {
                let it = &s.items[k];
                let part = &s.parts[it.part];
                let back = to.compose(&it.pose);
                let mut rows: Vec<(String, Json, String)> = Vec::new();
                for f in it.body.face_ids() {
                    let name = s.full(it, &face_name(&it.body, part, f));
                    let kind = select::surface_kind(&it.body.face(f).surface);
                    let c = back.point_to_world(select::centre(&it.body, Ent::Face(f)));
                    let n = select::plane_normal(&it.body, f).map(|n| back.vec_to_world(n));
                    let mut j = vec![
                        ("name", name.as_str().into()),
                        ("kind", kind.into()),
                        ("centre", pt(c)),
                    ];
                    if let Some(n) = n {
                        j.push(("normal", vt(n)));
                    }
                    let line = format!(
                        "{name}  {kind}  centre {}{}",
                        pts(c),
                        n.map(|n| format!("  normal {}", pts(Point3::ORIGIN + n)))
                            .unwrap_or_default()
                    );
                    rows.push((name, Json::obj(j), line));
                }
                rows.sort_by(|a, b| a.0.cmp(&b.0));
                for (_, j, l) in rows {
                    text.push_str(&l);
                    text.push('\n');
                    list.push(j);
                }
            }
            text.insert_str(0, &format!("{} faces ({frame_name} frame)\n", list.len()));
            fields.push(("query", "faces".into()));
            fields.push(("faces", Json::Arr(list)));
        }
    }
    fields.push(("frame", frame_name.as_str().into()));
    Ok(Answer {
        ok: true,
        json: Json::obj(fields),
        text,
    })
}

fn piece_text(p: &Piece, back: &Frame) -> String {
    match p {
        Piece::Line(a, b) => format!(
            "line {} {}",
            pts(back.point_to_world(*a)),
            pts(back.point_to_world(*b))
        ),
        Piece::Arc(c, r, q) => format!(
            "arc centre {} r {} from {} to {}",
            pts(back.point_to_world(*c)),
            sh(*r),
            pts(back.point_to_world(q[0])),
            pts(back.point_to_world(q[q.len() - 1]))
        ),
        Piece::Curve(q) => format!("curve through {} points", q.len()),
    }
}

fn mesh_of(_s: &Scene, it: &Item) -> forge_mesh::TriMesh {
    let mut c = facemesh::Cache::default();
    facemesh::join(&c.faces(&it.body))
}

/// A sheet, its wall, and a point and direction on the flat.
type OnSheet = (String, String, (f64, f64), (f64, f64));

/// The sheet wall a plane lies on (in the part's frame), and the plane's origin and x on the flat.
fn on_sheet(part: &Part, f: &Frame) -> Option<OnSheet> {
    let mut best: Option<(f64, OnSheet)> = None;
    for s in &part.sheets {
        let Some(lay) = s.layout.as_ref() else { continue };
        let t = s.sheet.gauge.thickness;
        for wp in &lay.walls {
            let wz = wp.frame.z().get();
            if wz.cross(f.z().get()).length() > 1e-9 {
                continue;
            }
            let o = wp.frame.point_to_local(f.origin);
            if o.z.abs() > t + 1e-6 {
                continue;
            }
            // how far the origin is outside the wall's outline box (a wall beside it lies in-plane too)
            let away = s
                .sheet
                .walls
                .iter()
                .find(|w| w.id == wp.wall)
                .and_then(|w| w.outline.first())
                .map_or(0.0, |l| {
                    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
                    for (seg, _) in &l.segments {
                        for p in seg.samples(8) {
                            lo = [lo[0].min(p.x), lo[1].min(p.y)];
                            hi = [hi[0].max(p.x), hi[1].max(p.y)];
                        }
                    }
                    let dx = (lo[0] - o.x).max(o.x - hi[0]).max(0.0);
                    let dy = (lo[1] - o.y).max(o.y - hi[1]).max(0.0);
                    (dx * dx + dy * dy).sqrt()
                });
            let x = wp.frame.vec_to_local(f.x().get());
            let d0 = wp.developed.apply(forge_math::Point2::new(o.x, o.y));
            let d1 = wp.developed.apply(forge_math::Point2::new(o.x + x.x, o.y + x.y));
            let wall = s
                .walls
                .iter()
                .find(|w| w.0 == wp.wall)
                .map_or_else(|| format!("{}", wp.wall.0), |w| w.1.clone());
            let got = (s.id.clone(), wall, (d0.x, d0.y), (d1.x - d0.x, d1.y - d0.y));
            if best.as_ref().is_none_or(|b| away < b.0) {
                best = Some((away, got));
            }
        }
    }
    best.map(|b| b.1)
}

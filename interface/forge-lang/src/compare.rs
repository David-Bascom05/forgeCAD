//! Compare two states of a document: two saves, or a save and the working copy. Each side is built
//! as that save holds it (its parts and used files too, read from the same save). Features and
//! params are matched by name; faces by their stable names first, then, for names lost, by
//! coincident surfaces with overlapping boxes; added and removed material is exact, from the
//! booleans.

use crate::{Options, Run};
use forge_agent::json::Json;
use forge_git::Oid;
use forge_history::History;
use forge_topo::{Body, FaceId};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Same,
    Changed,
    Added,
    Removed,
    Moved,
}

impl Change {
    pub fn word(self) -> &'static str {
        match self {
            Change::Same => "same",
            Change::Changed => "changed",
            Change::Added => "added",
            Change::Removed => "removed",
            Change::Moved => "moved",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub name: String,
    pub change: Change,
    pub old: Option<String>,
    pub new: Option<String>,
    /// The feature's kind (`hole`, `extrude`, `sketch`), for its icon; None for a param.
    pub kind: Option<String>,
    /// A param's unit (`mm`, `deg`), or "" for a number.
    pub unit: String,
    /// What changed in it, said shortly: each argument whose value changed, old then new.
    pub deltas: Vec<(String, String, String)>,
    /// A short human word for an added or removed feature's size (`Ø8`, `10 deep`).
    pub size: String,
    /// The faces it made, by their place in each side's body (base, target).
    pub faces: (Vec<usize>, Vec<usize>),
}

impl Item {
    fn bare(name: &str, change: Change, old: Option<String>, new: Option<String>) -> Item {
        Item {
            name: name.to_string(),
            change,
            old,
            new,
            kind: None,
            unit: String::new(),
            deltas: vec![],
            size: String::new(),
            faces: (vec![], vec![]),
        }
    }

    /// One line a person reads: `Hole Ø8 added`, `w 40 to 50 mm`, `plan size (w, 30) to (w, 36)`.
    pub fn summary(&self) -> String {
        let kind = self.kind.as_deref().map(title).unwrap_or_default();
        match self.change {
            Change::Added | Change::Removed => {
                let what = if kind.is_empty() { self.name.clone() } else { kind };
                let size = if self.size.is_empty() {
                    String::new()
                } else {
                    format!(" {}", self.size)
                };
                format!("{what}{size} {}", self.change.word())
            }
            Change::Moved => format!("{} moved", if kind.is_empty() { &self.name } else { &kind }),
            _ if self.kind.is_none() => {
                let u = if self.unit.is_empty() {
                    String::new()
                } else {
                    format!(" {}", self.unit)
                };
                format!(
                    "{} to {}{u}",
                    self.old.as_deref().unwrap_or(""),
                    self.new.as_deref().unwrap_or("")
                )
            }
            _ => match self.deltas.first() {
                Some((k, a, b)) => format!("{k} {a} to {b}"),
                None => format!("{kind} changed"),
            },
        }
    }
}

/// `fillet` as `Fillet`, `sheet_flange` as `Sheet flange`.
pub fn title(kind: &str) -> String {
    let k = kind.replace('_', " ");
    let mut c = k.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A call's arguments as written: positional ones keyed `#1`, `#2`, named ones by name.
pub fn call_args(text: &str) -> Vec<(String, String)> {
    let Some(open) = text.find('(') else { return vec![] };
    let b = text.as_bytes();
    let (mut depth, mut start, mut out, mut pos) = (0i32, open + 1, vec![], 0);
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    push_arg(&text[start..i], &mut out, &mut pos);
                    break;
                }
            }
            b',' if depth == 1 => {
                push_arg(&text[start..i], &mut out, &mut pos);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

fn push_arg(a: &str, out: &mut Vec<(String, String)>, pos: &mut usize) {
    let a = a.trim();
    if a.is_empty() {
        return;
    }
    let named = a
        .split_once(':')
        .filter(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || c == '_'));
    match named {
        Some((k, v)) => out.push((k.trim().to_string(), v.trim().to_string())),
        None => {
            *pos += 1;
            out.push((format!("#{pos}"), a.to_string()));
        }
    }
}

/// What changed between two writings of a statement: arguments of its call, or of each line of
/// a block (a sketch's rows) that changed, named `row arg`.
pub fn deltas(old: &str, new: &str) -> Vec<(String, String, String)> {
    let mut out = vec![];
    let (ol, nl): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let lines: Vec<(&str, &str)> = if ol.len() == nl.len() {
        ol.iter()
            .zip(&nl)
            .map(|(a, b)| (*a, *b))
            .filter(|(a, b)| a != b)
            .collect()
    } else {
        vec![(old, new)]
    };
    for (a, b) in lines {
        let row = a
            .split_once('=')
            .map(|x| x.0.trim())
            .filter(|r| !r.contains('('))
            .unwrap_or("");
        let (aa, ba) = (call_args(a), call_args(b));
        let mut any = false;
        for (k, v) in &ba {
            let was = aa
                .iter()
                .find(|x| &x.0 == k)
                .map(|x| x.1.clone())
                .unwrap_or_default();
            if &was != v {
                let key = match (row.is_empty() || ol.len() == 1, k.starts_with('#')) {
                    (true, false) => k.clone(),
                    (true, true) => format!("argument {}", &k[1..]),
                    (false, false) => format!("{row} {k}"),
                    (false, true) => row.to_string(),
                };
                out.push((key, was, v.clone()));
                any = true;
            }
        }
        if !any && a.trim() != b.trim() {
            out.push((row.to_string(), a.trim().to_string(), b.trim().to_string()));
        }
    }
    out
}

/// An added or removed feature's size as a person says it.
fn size_of(kind: &str, text: &str) -> String {
    let args = call_args(text);
    let get = |k: &str| args.iter().find(|x| x.0 == k).map(|x| x.1.clone());
    match kind {
        "hole" => get("d")
            .map(|d| format!("Ø{d}"))
            .or(get("screw").map(|s| s.trim_matches('"').to_string())),
        "fillet" | "round" => get("r").or(get("#2")).map(|r| format!("R{r}")),
        "chamfer" => get("d").or(get("#2")).map(|d| format!("{d} mm")),
        "extrude" | "pad" | "pocket" => get("#2").or(get("depth")).map(|d| format!("{d} deep")),
        "shell" => get("t").or(get("#2")).map(|t| format!("{t} wall")),
        _ => None,
    }
    .unwrap_or_default()
}

/// A face's class, by side: the base's faces are same, changed or removed; the target's are
/// same, changed or added.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Faces {
    pub base: Vec<Change>,
    pub target: Vec<Change>,
    pub changed: usize,
    pub added: usize,
    pub removed: usize,
    pub same: usize,
    /// Pairs found by geometry after their names were lost.
    pub fallback: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Volume {
    pub added: f64,
    pub removed: f64,
    pub added_body: Option<Body>,
    pub removed_body: Option<Body>,
}

pub struct Side {
    pub label: String,
    pub text: String,
    pub run: Run,
}

pub struct Comparison {
    pub base: Side,
    pub target: Side,
    pub features: Vec<Item>,
    pub params: Vec<Item>,
    pub occurrences: Vec<Item>,
    pub faces: Option<Faces>,
}

/// Where a document at a save is built from: a directory inside that save.
fn dir_at(h: &History, rel: &str, at: Option<&Oid>) -> PathBuf {
    let file = match at {
        Some(c) => crate::pins::path_of(&crate::pins::Pin {
            work: h.work().to_path_buf(),
            n: 0,
            save: c.hex(),
            rel: rel.to_string(),
        }),
        None => h.abs(rel),
    };
    file.parent().map(|p| p.to_path_buf()).unwrap_or_default()
}

/// The document `rel` built as save `at` holds it (None: the working copy, as on disk).
pub fn build_at(h: &History, rel: &str, at: Option<&Oid>, label: &str) -> Result<Side, String> {
    build_at_step(h, rel, at, label, None)
}

/// `build_at`, an assembly's STEP written too under `step`'s product name.
pub fn build_at_step(
    h: &History,
    rel: &str,
    at: Option<&Oid>,
    label: &str,
    step: Option<String>,
) -> Result<Side, String> {
    let text = h
        .content(rel, at)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{rel} is not in {label}"))?;
    let text = String::from_utf8(text).map_err(|_| format!("{rel} in {label} is not text"))?;
    let opts = Options {
        dir: Some(dir_at(h, rel, at)),
        root: Some(h.work().to_path_buf()),
        render: true,
        anchors: true,
        step,
        ..Options::default()
    };
    let (run, _) = crate::run(&text, opts, Default::default());
    Ok(Side {
        label: label.to_string(),
        text,
        run,
    })
}

fn slice(src: &str, s: crate::syntax::Span) -> String {
    src.get(s.lo as usize..s.hi as usize)
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Items keyed by name in two orders: same, changed, added, removed, and moved where the common
/// ones changed order (those off a longest common subsequence).
fn keyed(a: &[(String, String)], b: &[(String, String)]) -> Vec<Item> {
    let am: HashMap<&str, &str> = a.iter().map(|x| (x.0.as_str(), x.1.as_str())).collect();
    let bm: HashMap<&str, &str> = b.iter().map(|x| (x.0.as_str(), x.1.as_str())).collect();
    let ca: Vec<&str> = a
        .iter()
        .map(|x| x.0.as_str())
        .filter(|n| bm.contains_key(n))
        .collect();
    let cb: Vec<&str> = b
        .iter()
        .map(|x| x.0.as_str())
        .filter(|n| am.contains_key(n))
        .collect();
    let keep = lcs(&ca, &cb);
    let mut out = vec![];
    for (n, t) in b {
        let item = match am.get(n.as_str()) {
            None => Item::bare(n, Change::Added, None, Some(t.clone())),
            Some(o) => Item::bare(
                n,
                if *o != t {
                    Change::Changed
                } else if !keep.contains(n.as_str()) {
                    Change::Moved
                } else {
                    Change::Same
                },
                Some(o.to_string()),
                Some(t.clone()),
            ),
        };
        out.push(item);
    }
    for (n, t) in a {
        if !bm.contains_key(n.as_str()) {
            out.push(Item::bare(n, Change::Removed, Some(t.clone()), None));
        }
    }
    out
}

fn lcs<'a>(a: &[&'a str], b: &[&'a str]) -> std::collections::HashSet<&'a str> {
    let (n, m) = (a.len(), b.len());
    let mut t = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i][j] = if a[i] == b[j] {
                t[i + 1][j + 1] + 1
            } else {
                t[i + 1][j].max(t[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = std::collections::HashSet::new();
    while i < n && j < m {
        if a[i] == b[j] {
            out.insert(a[i]);
            i += 1;
            j += 1;
        } else if t[i + 1][j] >= t[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

fn features(s: &Side) -> Vec<(String, String)> {
    s.run
        .feats
        .iter()
        .filter(|f| f.top_level)
        .map(|f| (f.id.clone(), slice(&s.text, f.stmt)))
        .collect()
}

fn params(s: &Side) -> Vec<(String, String)> {
    s.run
        .shared
        .iter()
        .chain(&s.run.locals)
        .map(|p| (p.key.to_string(), slice(&s.text, p.rhs)))
        .collect()
}

fn occurrences(s: &Side) -> Vec<(String, String)> {
    s.run.asm.as_ref().map_or_else(Vec::new, |a| {
        a.occurrences
            .iter()
            .map(|(id, file, f)| {
                let o = f.origin;
                (
                    id.clone(),
                    format!("{file} at ({:.3}, {:.3}, {:.3})", o.x, o.y, o.z),
                )
            })
            .collect()
    })
}

const REL: f64 = 1e-9;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= REL * a.abs().max(b.abs()).max(1.0)
}

fn same_surface(a: &forge_geom::Surface, b: &forge_geom::Surface) -> bool {
    a == b || forge_bool::ssi::coincident(a, b)
}

fn same_face(a: &Body, fa: FaceId, b: &Body, fb: FaceId) -> bool {
    let (x, y) = (a.face(fa), b.face(fb));
    if x.loops.len() != y.loops.len() || !same_surface(&x.surface, &y.surface) {
        return false;
    }
    let (ba, bb) = (a.face_box(fa), b.face_box(fb));
    let ends = |q: &forge_math::Box3| [q.x.lo, q.x.hi, q.y.lo, q.y.hi, q.z.lo, q.z.hi];
    if !ends(&ba)
        .iter()
        .zip(ends(&bb))
        .all(|(p, q)| (p - q).abs() <= 1e-7 * p.abs().max(q.abs()).max(1.0))
    {
        return false;
    }
    match (
        forge_topo::mass::face_area(a, fa),
        forge_topo::mass::face_area(b, fb),
    ) {
        (Ok(p), Ok(q)) => close(p, q),
        _ => false,
    }
}

/// How much two faces' boxes overlap (negative: not at all), a flat face ranked by its two
/// largest extents.
fn overlap(a: &forge_math::Box3, b: &forge_math::Box3) -> f64 {
    let d = [
        a.x.hi.min(b.x.hi) - a.x.lo.max(b.x.lo),
        a.y.hi.min(b.y.hi) - a.y.lo.max(b.y.lo),
        a.z.hi.min(b.z.hi) - a.z.lo.max(b.z.lo),
    ];
    if d.iter().any(|&e| e < -1e-9) {
        return -1.0;
    }
    let mut e = d.map(|x| x.max(0.0));
    e.sort_by(|p, q| q.total_cmp(p));
    e[0] * e[1].max(1e-9)
}

pub fn faces(a: &Body, b: &Body) -> Faces {
    let fa: Vec<FaceId> = a.face_ids().collect();
    let fb: Vec<FaceId> = b.face_ids().collect();
    let mut f = Faces {
        base: vec![Change::Removed; fa.len()],
        target: vec![Change::Added; fb.len()],
        ..Faces::default()
    };
    // names that are one face on each side pair at once
    let count = |body: &Body, ids: &[FaceId]| {
        let mut m: HashMap<u64, Vec<usize>> = HashMap::new();
        for (i, id) in ids.iter().enumerate() {
            m.entry(body.face(*id).name.0).or_default().push(i);
        }
        m
    };
    let (na, nb) = (count(a, &fa), count(b, &fb));
    let mut paired_a = vec![false; fa.len()];
    let mut paired_b = vec![false; fb.len()];
    for (name, ia) in &na {
        let Some(ib) = nb.get(name) else {
            continue;
        };
        if *name == 0 || ia.len() != 1 || ib.len() != 1 {
            continue;
        }
        let (i, j) = (ia[0], ib[0]);
        let c = if same_face(a, fa[i], b, fb[j]) {
            Change::Same
        } else {
            Change::Changed
        };
        f.base[i] = c;
        f.target[j] = c;
        paired_a[i] = true;
        paired_b[j] = true;
    }
    // the rest by geometry: coincident surfaces, the most overlap first
    let mut cands = vec![];
    for (i, &x) in fa.iter().enumerate() {
        if paired_a[i] {
            continue;
        }
        for (j, &y) in fb.iter().enumerate() {
            if paired_b[j] || !same_surface(&a.face(x).surface, &b.face(y).surface) {
                continue;
            }
            let o = overlap(&a.face_box(x), &b.face_box(y));
            if o >= 0.0 {
                cands.push((o, i, j));
            }
        }
    }
    cands.sort_by(|p, q| q.0.total_cmp(&p.0).then((p.1, p.2).cmp(&(q.1, q.2))));
    for (_, i, j) in cands {
        if paired_a[i] || paired_b[j] {
            continue;
        }
        let c = if same_face(a, fa[i], b, fb[j]) {
            Change::Same
        } else {
            Change::Changed
        };
        f.base[i] = c;
        f.target[j] = c;
        paired_a[i] = true;
        paired_b[j] = true;
        f.fallback += 1;
    }
    for c in &f.target {
        match c {
            Change::Same => f.same += 1,
            Change::Changed => f.changed += 1,
            Change::Added => f.added += 1,
            _ => {}
        }
    }
    f.removed = f.base.iter().filter(|c| **c == Change::Removed).count();
    f
}

/// Material added (target less base) and removed (base less target), exactly.
pub fn volume(a: &Body, b: &Body) -> Result<Volume, String> {
    let vol = |x: &Body| forge_topo::mass::mass(x).map(|m| m.volume).unwrap_or(0.0);
    let added = forge_bool::boolean(b, a, forge_bool::Op::Subtract, 0).map_err(|e| format!("{e:?}"))?;
    let removed = forge_bool::boolean(a, b, forge_bool::Op::Subtract, 0).map_err(|e| format!("{e:?}"))?;
    Ok(Volume {
        added: vol(&added),
        removed: vol(&removed),
        added_body: Some(added),
        removed_body: Some(removed),
    })
}

/// Each face of a side's body, by place, and the feature its anchor names (`hole1.wall`: hole1).
fn face_owners(s: &Side) -> Vec<Option<String>> {
    let (Some(b), Some(a)) = (&s.run.body, &s.run.anchors) else {
        return vec![];
    };
    b.face_ids()
        .map(|f| {
            a.faces.get(&b.face(f).name).map(|t| {
                t.chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
        })
        .collect()
}

fn kinds(s: &Side) -> HashMap<String, &'static str> {
    s.run.feats.iter().map(|f| (f.id.clone(), f.kind)).collect()
}

fn units(s: &Side) -> HashMap<String, &'static str> {
    s.run
        .shared
        .iter()
        .chain(&s.run.locals)
        .map(|p| {
            let u = match &p.value {
                crate::eval::Value::Num(n) => match n.dim {
                    crate::values::Dim::Length => "mm",
                    crate::values::Dim::Angle => "deg",
                    _ => "",
                },
                _ => "",
            };
            (p.key.to_string(), u)
        })
        .collect()
}

pub fn compare(base: Side, target: Side) -> Comparison {
    let faces = match (&base.run.body, &target.run.body) {
        (Some(a), Some(b)) if base.run.asm.is_none() => Some(faces(a, b)),
        _ => None,
    };
    let (ob, ot) = (face_owners(&base), face_owners(&target));
    let (kb, kt) = (kinds(&base), kinds(&target));
    let mut features = keyed(&features(&base), &features(&target));
    for x in &mut features {
        x.kind = kt.get(&x.name).or(kb.get(&x.name)).map(|k| k.to_string());
        let kind = x.kind.clone().unwrap_or_default();
        match (&x.old, &x.new) {
            (Some(o), Some(n)) if x.change == Change::Changed => x.deltas = deltas(o, n),
            (_, Some(n)) if x.change == Change::Added => x.size = size_of(&kind, n),
            (Some(o), _) if x.change == Change::Removed => x.size = size_of(&kind, o),
            _ => {}
        }
        let mine = |owners: &[Option<String>]| -> Vec<usize> {
            owners
                .iter()
                .enumerate()
                .filter(|(_, o)| o.as_deref() == Some(x.name.as_str()))
                .map(|(i, _)| i)
                .collect()
        };
        x.faces = (mine(&ob), mine(&ot));
    }
    let (ub, ut) = (units(&base), units(&target));
    let mut params = keyed(&params(&base), &params(&target));
    for p in &mut params {
        p.unit = ut
            .get(&p.name)
            .or(ub.get(&p.name))
            .map(|u| u.to_string())
            .unwrap_or_default();
    }
    Comparison {
        features,
        params,
        occurrences: keyed(&occurrences(&base), &occurrences(&target)),
        faces,
        base,
        target,
    }
}

/// Compare a document between two references (`v3`, a save id, `current`).
pub fn between(h: &History, rel: &str, a: &str, b: &str) -> Result<Comparison, String> {
    let ra = h.resolve(rel, a).map_err(|e| e.to_string())?;
    let rb = h.resolve(rel, b).map_err(|e| e.to_string())?;
    let base = build_at(h, rel, ra.as_ref(), a)?;
    let target = build_at(h, rel, rb.as_ref(), b)?;
    Ok(compare(base, target))
}

fn items_json(xs: &[Item]) -> Json {
    Json::Arr(
        xs.iter()
            .filter(|x| x.change != Change::Same)
            .map(|x| {
                let mut f = vec![
                    ("name", Json::Str(x.name.clone())),
                    ("change", Json::Str(x.change.word().into())),
                    ("summary", Json::Str(x.summary())),
                ];
                if let Some(k) = &x.kind {
                    f.push(("kind", Json::Str(k.clone())));
                }
                if let Some(o) = &x.old {
                    f.push(("old", Json::Str(o.clone())));
                }
                if let Some(n) = &x.new {
                    f.push(("new", Json::Str(n.clone())));
                }
                Json::obj(f)
            })
            .collect(),
    )
}

fn names_of(body: &Body, cls: &[Change], want: Change) -> Json {
    let ids: Vec<FaceId> = body.face_ids().collect();
    Json::Arr(
        ids.iter()
            .zip(cls)
            .filter(|(_, c)| **c == want)
            .map(|(id, _)| Json::Str(format!("{:016x}", body.face(*id).name.0)))
            .collect(),
    )
}

impl Comparison {
    pub fn json(&self, vol: Option<&Result<Volume, String>>) -> Json {
        let mut f = vec![
            ("base", Json::Str(self.base.label.clone())),
            ("target", Json::Str(self.target.label.clone())),
            ("features", items_json(&self.features)),
            ("params", items_json(&self.params)),
        ];
        if self.base.run.asm.is_some() || self.target.run.asm.is_some() {
            f.push(("occurrences", items_json(&self.occurrences)));
        }
        if let (Some(fc), Some(a), Some(b)) = (&self.faces, &self.base.run.body, &self.target.run.body) {
            f.push((
                "faces",
                Json::obj(vec![
                    ("changed", names_of(b, &fc.target, Change::Changed)),
                    ("added", names_of(b, &fc.target, Change::Added)),
                    ("removed", names_of(a, &fc.base, Change::Removed)),
                    ("same", Json::Num(fc.same as f64)),
                    ("fallback", Json::Num(fc.fallback as f64)),
                ]),
            ));
        }
        match vol {
            Some(Ok(v)) => f.push((
                "volume",
                Json::obj(vec![
                    ("added_mm3", Json::Num(v.added)),
                    ("removed_mm3", Json::Num(v.removed)),
                ]),
            )),
            Some(Err(e)) => f.push(("volume_error", Json::Str(e.clone()))),
            None => {}
        }
        let errs = |s: &Side| Json::Arr(s.run.errors.iter().map(|e| Json::Str(e.text())).collect());
        if !self.base.run.errors.is_empty() || !self.target.run.errors.is_empty() {
            f.push((
                "errors",
                Json::obj(vec![("base", errs(&self.base)), ("target", errs(&self.target))]),
            ));
        }
        Json::obj(f)
    }

    /// Counts for a status line: `4 changed · 2 added · 1 removed`.
    pub fn counts(&self) -> BTreeMap<&'static str, usize> {
        let mut m = BTreeMap::new();
        if let Some(f) = &self.faces {
            m.insert("changed", f.changed);
            m.insert("added", f.added);
            m.insert("removed", f.removed);
        }
        m
    }
}

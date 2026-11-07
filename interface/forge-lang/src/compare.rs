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
            None => Item {
                name: n.clone(),
                change: Change::Added,
                old: None,
                new: Some(t.clone()),
            },
            Some(o) => Item {
                name: n.clone(),
                change: if *o != t {
                    Change::Changed
                } else if !keep.contains(n.as_str()) {
                    Change::Moved
                } else {
                    Change::Same
                },
                old: Some(o.to_string()),
                new: Some(t.clone()),
            },
        };
        out.push(item);
    }
    for (n, t) in a {
        if !bm.contains_key(n.as_str()) {
            out.push(Item {
                name: n.clone(),
                change: Change::Removed,
                old: Some(t.clone()),
                new: None,
            });
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

pub fn compare(base: Side, target: Side) -> Comparison {
    let faces = match (&base.run.body, &target.run.body) {
        (Some(a), Some(b)) if base.run.asm.is_none() => Some(faces(a, b)),
        _ => None,
    };
    Comparison {
        features: keyed(&features(&base), &features(&target)),
        params: keyed(&params(&base), &params(&target)),
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
                ];
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

//! The part as built from the program so far: its edges, for references resolved against the
//! geometry, and the Forge selections that name them. One build cache per body, so a longer
//! program rebuilds only its tail.

use crate::Fail;
use crate::py::V3;
use forge_agent::select::{self, Ent};
use forge_geom::CurveGeom;
use forge_lang::anchors::{Anchors, Asked};
use forge_lang::build::Cache;
use forge_topo::{Body, Name};
use std::collections::{BTreeSet, HashMap};

pub struct Edge {
    pub h: Name,
    pub c: &'static str,
    pub v: [usize; 2],
    pub t: [Option<V3>; 2],
    /// 17 points evenly along it.
    pub p: Vec<V3>,
    /// The faces it bounds, by name.
    pub f: Vec<Name>,
}

/// A planar face of the part: a point on it and its outward normal.
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub at: V3,
    pub n: V3,
}

#[derive(Default)]
pub struct State {
    cache: Option<Cache>,
    src: String,
    body: Option<Body>,
    anchors: Option<Anchors>,
    /// The patterns and holes of the part as `edges` last built it: their copies and places
    /// (`P[k]`) name edges alike in all else apart.
    placed: Vec<String>,
    /// The part's top-level features as `edges` last built it, other than sketches: what
    /// `edges(of: F)` and `faces(of: F)` may name.
    made: Vec<String>,
}

/// A message as it travels in JSON: quotes, backslashes, controls blanked.
fn esc(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '"' || c == '\\' || (c as u32) < 32 {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// A piece of a selection: the probe's text (numbers written out), the text written (numbers
/// read from params), and those params.
type Cand = (String, String, Vec<(String, f64)>);

/// A filter's number as a probe writes it: every digit, so it picks what the param will.
fn lit(x: f64) -> String {
    format!("{x:?}")
}

impl State {
    /// Builds `src`, keeping the stages; with `anchors`, also each face's and edge's Forge text.
    fn run(&mut self, src: &str, anchors: bool, asks: Vec<String>) -> Result<forge_lang::Run, Fail> {
        let cache = self.cache.take().unwrap_or_default();
        let opts = forge_lang::Options {
            anchors,
            asks,
            checks: false,
            ..forge_lang::Options::default()
        };
        let got =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| forge_lang::run(src, opts, cache)));
        let Ok((run, cache)) = got else {
            return Err(Fail::Stop("kernel panic".into()));
        };
        self.cache = Some(cache);
        if let Some(e) = run.errors.first() {
            return Err(Fail::Stop(esc(&format!("{}: {}", e.kind(), e.text()))));
        }
        Ok(run)
    }

    /// The program so far, built: Stop with the first error where it does not build.
    pub fn check(&mut self, src: &str) -> Result<(), Fail> {
        self.run(src, false, vec![]).map(|_| ())
    }

    /// Twice the diagonal of the box round the part the program builds: further than anything
    /// through it reaches.
    pub fn reach(&mut self, src: &str) -> Result<f64, Fail> {
        let run = self.run(src, false, vec![])?;
        let Some(b) = run.body else { return Ok(1.0) };
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for v in b.vertex_ids() {
            let p = b.vertex(v).point;
            for (k, x) in [p.x, p.y, p.z].into_iter().enumerate() {
                lo[k] = lo[k].min(x);
                hi[k] = hi[k].max(x);
            }
        }
        let d: f64 = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f64>().sqrt();
        Ok(2.0 * d.max(1.0) + lo.iter().chain(&hi).map(|x| x.abs()).fold(0.0, f64::max))
    }

    /// The edges of the part the program builds.
    pub fn edges(&mut self, src: &str) -> Result<Vec<Edge>, Fail> {
        let run = self.run(src, true, vec![])?;
        self.src = src.into();
        self.placed = run
            .feats
            .iter()
            .filter(|f| f.top_level && matches!(f.kind, "pattern" | "hole"))
            .map(|f| f.id.clone())
            .collect();
        self.made = run
            .feats
            .iter()
            .filter(|f| f.top_level && f.kind != "sketch")
            .map(|f| f.id.clone())
            .collect();
        self.anchors = run.anchors;
        self.body = run.body;
        Ok(self.body.as_ref().map(dump).unwrap_or_default())
    }

    /// Each face's pattern copies and hole places (`P[k]`), on the part as `edges` last built it.
    fn places(&mut self) -> Result<HashMap<Name, Vec<String>>, Fail> {
        let mut out: HashMap<Name, Vec<String>> = HashMap::new();
        if self.placed.is_empty() {
            return Ok(out);
        }
        let src = self.src.clone();
        // a few places first, more where a pattern has them
        let (mut from, mut upto) = (0, 16);
        let mut left = self.placed.clone();
        while !left.is_empty() {
            let texts: Vec<String> = left
                .iter()
                .flat_map(|p| (from..upto).map(move |k| format!("{p}[{k}]")))
                .collect();
            let run = self.run(&src, false, texts)?;
            let mut more = vec![];
            for (t, a) in run.asked {
                if let Asked::Faces(fs, _) = a {
                    for f in fs {
                        out.entry(f).or_default().push(t.clone());
                    }
                    if let Some(p) = t.strip_suffix(&format!("[{}]", upto - 1)) {
                        more.push(p.to_string());
                    }
                }
            }
            left = more;
            (from, upto) = (upto, upto * 4);
        }
        // a copy of one feature: that feature's anchors carried onto it (`P[k].end`), which tell
        // apart faces alike within the copy
        let mut count: HashMap<String, usize> = HashMap::new();
        for t in out.values().flatten() {
            if let Some((p, k)) = t.strip_suffix(']').and_then(|t| t.split_once('[')) {
                let k: usize = k.parse().unwrap_or(0);
                let c = count.entry(p.to_string()).or_default();
                *c = (*c).max(k + 1);
            }
        }
        let anchors = self.anchors.clone().unwrap_or_default();
        let mut texts = vec![];
        for line in src.lines() {
            let Some((p, rest)) = line.split_once(" = pattern(of: ") else {
                continue;
            };
            // `of: s` or `of: [s, t]`
            let of = match rest.strip_prefix('[') {
                Some(r) => r.split_once(']').map(|x| x.0),
                None => rest.split_once(',').map(|x| x.0),
            };
            let Some(of) = of.filter(|_| count.contains_key(p)) else {
                continue;
            };
            let mut own: Vec<&str> = vec![];
            for s in of.split(", ") {
                own.extend(
                    anchors
                        .faces
                        .values()
                        .filter_map(|a| a.strip_prefix(s).filter(|r| r.starts_with('.'))),
                );
            }
            own.sort();
            own.dedup();
            for k in 1..count[p] {
                for a in &own {
                    texts.push(format!("{p}[{k}]{a}"));
                }
            }
        }
        if !texts.is_empty() {
            let run = self.run(&src, false, texts)?;
            for (t, a) in run.asked {
                if let Asked::Faces(fs, _) = a {
                    for f in fs {
                        out.entry(f).or_default().push(t.clone());
                    }
                }
            }
        }
        Ok(out)
    }

    /// What each text selects on the part as `edges` last built it.
    fn ask(&mut self, texts: &[String]) -> Result<Vec<Option<BTreeSet<Name>>>, Fail> {
        let src = self.src.clone();
        let run = self.run(&src, false, texts.to_vec())?;
        Ok(run
            .asked
            .into_iter()
            .map(|(_, a)| match a {
                Asked::Edges(es) => Some(es.into_iter().collect()),
                _ => None,
            })
            .collect())
    }

    /// A Forge selection of exactly these edges of the part as `edges` last built it: each by
    /// the two faces it lies between, or by a feature (`feats`) that made one of its faces,
    /// narrowed by its shape, radius or length where that names more edges than these; failing
    /// those, between the features (or anchors) whose faces meet in it (a blend's own edges),
    /// narrowed as well by the anchored faces at its ends (`touching: A.edges`) or the world axis
    /// a line runs along. The selection's text, and the params its numbers need (`(name, value)`,
    /// read in the feature's statement). Unresolved where no narrowing keeps out an edge not wanted.
    pub fn select(
        &mut self,
        want: &BTreeSet<Name>,
        feats: &[String],
    ) -> Result<(String, Vec<(String, f64)>), Fail> {
        let picks = self.pick(want, feats)?;
        let text: Vec<&str> = picks.iter().map(|p| p.1.as_str()).collect();
        Ok((text.join(" + "), picks.into_iter().flat_map(|p| p.2).collect()))
    }

    /// The part's top-level features other than sketches, as `edges` last built it.
    pub fn made(&self) -> &[String] {
        &self.made
    }

    /// As `select`, its numbers written out in the selection (no params): for tools that write
    /// programs the style rules do not hold (forge_lang::fuzz::options).
    pub fn select_literal(&mut self, want: &BTreeSet<Name>, feats: &[String]) -> Result<String, Fail> {
        let picks = self.pick(want, feats)?;
        let text: Vec<&str> = picks.iter().map(|p| p.0.as_str()).collect();
        Ok(text.join(" + "))
    }

    /// The pieces of a selection of exactly these edges: see `select`.
    fn pick(&mut self, want: &BTreeSet<Name>, feats: &[String]) -> Result<Vec<Cand>, Fail> {
        let unnamed =
            || Fail::Unresolved("an edge no Forge selection names apart from its neighbours".into());
        let (Some(anchors), Some(body)) = (self.anchors.clone(), self.body.clone()) else {
            return Err(unnamed());
        };
        let ids: HashMap<Name, forge_topo::EdgeId> =
            body.edge_ids().map(|e| (body.edge(e).name, e)).collect();
        // a pattern's copies (and a hole's places) are alike: each copy names its own
        let places = self.places()?;
        // (probe text, final text, its params): each a candidate piece of the union, broadest first
        let mut bases: Vec<String> = vec![];
        for n in want {
            if let Some(t) = anchors.edges.get(n)
                && !bases.contains(t)
            {
                bases.push(t.clone());
            }
        }
        let mut picks = vec![];
        let mut left: BTreeSet<Name> = want.clone();
        let mut k = 0;
        // first the anchors, copies and features; then, for the edges none of those names, where
        // the faces of the features (or anchors) on either side meet: an edge of a blend's, a
        // shell's or a cut's faces, which no anchor names on its own
        for round in 0..2 {
            let mut owners: HashMap<Name, Vec<String>> = HashMap::new();
            let mut cands: Vec<Cand> = vec![];
            if round == 0 {
                for b in &bases {
                    cands.push((b.clone(), b.clone(), vec![]));
                }
            } else {
                if left.is_empty() {
                    break;
                }
                // each face's features: those whose `faces(of: F)` holds it
                let texts: Vec<String> = feats.iter().map(|f| format!("faces(of: {f})")).collect();
                for (f, got) in feats.iter().zip(self.ask_faces(&texts)?) {
                    for n in got.into_iter().flatten() {
                        owners.entry(n).or_default().push(f.clone());
                    }
                }
            }
            let these: Vec<Name> = if round == 0 {
                want.iter().copied().collect()
            } else {
                left.iter().copied().collect()
            };
            for n in &these {
                let Some(&e) = ids.get(n) else {
                    return Err(unnamed());
                };
                let kind = select::curve_kind(&body.edge(e).curve);
                let r = select::radius(&body, Ent::Edge(e));
                let l = select::length(&body, e);
                let len = vec![
                    ("longer".to_string(), l * (1.0 - 1e-6)),
                    ("shorter".to_string(), l * (1.0 + 1e-6)),
                ];
                let mut filters: Vec<Vec<(String, f64)>> = vec![vec![]];
                if let Some(r) = r {
                    filters.push(vec![("r".into(), r)]);
                }
                filters.push(len.clone());
                if let Some(r) = r {
                    let mut f = vec![("r".to_string(), r)];
                    f.extend(len.clone());
                    filters.push(f);
                }
                let fs: Vec<Name> = select::faces_of_edge(&body, e)
                    .iter()
                    .map(|&f| body.face(f).name)
                    .collect();
                let mut heads: Vec<String> = vec![];
                if round == 0 {
                    heads.extend(
                        anchors
                            .edges
                            .get(n)
                            .map(|t| t.strip_suffix(')').unwrap_or(t).to_string()),
                    );
                    // the copy or place holding a face of it: its edges, its crease, or where it
                    // meets the face on the other side
                    let crease = select::crease(&body, e).map(|c| format!(", crease: \"{c}\""));
                    for (i, f) in fs.iter().enumerate() {
                        for p in places.get(f).into_iter().flatten() {
                            heads.push(format!("edges(of: {p}"));
                            if let Some(c) = &crease {
                                heads.push(format!("edges(of: {p}{c}"));
                            }
                            let other = fs.get(1 - i.min(1)).filter(|_| fs.len() == 2);
                            let mut sides: Vec<String> = other
                                .and_then(|g| anchors.faces.get(g))
                                .cloned()
                                .into_iter()
                                .collect();
                            sides.extend(other.and_then(|g| places.get(g)).into_iter().flatten().cloned());
                            for q in sides.iter().filter(|q| *q != p) {
                                heads.push(format!("edges(between: ({p}, {q})"));
                            }
                        }
                    }
                    heads.extend(feats.iter().map(|f| format!("edges(of: {f}")));
                } else if let [fa, fb] = fs[..] {
                    let sides = |f: Name| -> Vec<String> {
                        let mut v: Vec<String> = anchors.faces.get(&f).cloned().into_iter().collect();
                        v.extend(owners.get(&f).into_iter().flatten().cloned());
                        v
                    };
                    for a in sides(fa) {
                        for b in &sides(fb) {
                            let h = format!("edges(between: ({a}, {b})");
                            if !heads.contains(&h) {
                                heads.push(h);
                            }
                        }
                    }
                }
                // last: an edge alike in all else to others (a symmetric part's) told apart by the
                // anchored faces at its ends, or by the world axis a line runs along
                let mut more: Vec<String> = vec![String::new()];
                if round == 1 {
                    let x = body.edge(e);
                    let ends = [x.start, x.end];
                    let mut touch: Vec<String> = vec![];
                    for g in body.edge_ids() {
                        let y = body.edge(g);
                        if !ends.contains(&y.start) && !ends.contains(&y.end) {
                            continue;
                        }
                        for f in select::faces_of_edge(&body, g) {
                            if let Some(t) = anchors.faces.get(&body.face(f).name)
                                && !touch.contains(t)
                            {
                                touch.push(t.clone());
                            }
                        }
                    }
                    more.extend(touch.iter().map(|t| format!(", touching: {t}.edges")));
                    if let forge_geom::Curve::Line(l) = &x.curve {
                        let d = l.dir.get();
                        for (w, a) in [("X", d.x), ("Y", d.y), ("Z", d.z)] {
                            if a.abs() >= 1.0 - 1e-12 {
                                more.push(format!(", along: {w}"));
                            }
                        }
                    }
                }
                for h in &heads {
                    for (f, m) in filters.iter().flat_map(|f| more.iter().map(move |m| (f, m))) {
                        let p: String = f.iter().map(|(key, v)| format!(", {key}: {}", lit(*v))).collect();
                        let mut ps = vec![];
                        let t: String = f
                            .iter()
                            .map(|(key, v)| {
                                let name = format!("e{k}{key}");
                                ps.push((name.clone(), *v));
                                format!(", {key}: {name}")
                            })
                            .collect();
                        cands.push((
                            format!("{h}, shape: \"{kind}\"{p}{m})"),
                            format!("{h}, shape: \"{kind}\"{t}{m})"),
                            ps,
                        ));
                        k += 1;
                    }
                }
            }
            let probes: Vec<String> = cands.iter().map(|c| c.0.clone()).collect();
            let got = self.ask(&probes)?;
            for (c, s) in cands.into_iter().zip(&got) {
                let Some(s) = s else { continue };
                if !s.is_empty() && s.is_subset(want) && s.iter().any(|x| left.contains(x)) {
                    left.retain(|x| !s.contains(x));
                    picks.push(c);
                }
            }
        }
        if !left.is_empty() {
            return Err(unnamed());
        }
        // the union, asked as a whole: exactly the edges wanted
        let union: Vec<&str> = picks.iter().map(|p| p.0.as_str()).collect();
        let whole = self.ask(&[union.join(" + ")])?;
        if whole.first().cloned().flatten().as_ref() != Some(want) {
            return Err(Fail::Unresolved(
                "a selection that would not name its edges alone".into(),
            ));
        }
        Ok(picks)
    }

    /// The edges or faces (by name) a selection's text names on the part `src` builds.
    pub fn names(&mut self, src: &str, text: &str) -> Result<Vec<Name>, Fail> {
        let run = self.run(src, false, vec![text.to_string()])?;
        Ok(match run.asked.into_iter().next().map(|x| x.1) {
            Some(Asked::Edges(es)) => es,
            Some(Asked::Faces(fs, _)) => fs,
            _ => vec![],
        })
    }

    /// What each text selects as faces on the part as `edges` last built it.
    fn ask_faces(&mut self, texts: &[String]) -> Result<Vec<Option<BTreeSet<Name>>>, Fail> {
        let src = self.src.clone();
        let run = self.run(&src, false, texts.to_vec())?;
        Ok(run
            .asked
            .into_iter()
            .map(|(_, a)| match a {
                Asked::Faces(fs, _) => Some(fs.into_iter().collect()),
                _ => None,
            })
            .collect())
    }

    /// The face of this name on the part as `edges` last built it, where it is planar.
    pub fn plane(&self, face: Name) -> Option<Plane> {
        let b = self.body.as_ref()?;
        let f = select::all(b).into_iter().find_map(|e| match e {
            Ent::Face(f) if b.face(f).name == face => Some(f),
            _ => None,
        })?;
        let n = select::plane_normal(b, f)?;
        let at = *select::samples(b, Ent::Face(f)).first()?;
        Some(Plane {
            at: (at.x, at.y, at.z),
            n: (n.x, n.y, n.z),
        })
    }

    /// A Forge selection of exactly these faces of the part as `edges` last built it (a shell's
    /// open faces): each by its anchor (`E.end`, `E.side(S.l)`, a pattern copy's `P[k].end`), or
    /// by a feature (`feats`) that made it, narrowed by its shape and radius, numbers written
    /// out. Unresolved where no union of those names exactly these.
    pub fn select_faces(&mut self, want: &BTreeSet<Name>, feats: &[String]) -> Result<String, Fail> {
        let unnamed = || Fail::Unresolved("a face no Forge selection names apart from its neighbours".into());
        let (Some(anchors), Some(body)) = (self.anchors.clone(), self.body.clone()) else {
            return Err(unnamed());
        };
        let ids: HashMap<Name, forge_topo::FaceId> =
            body.face_ids().map(|f| (body.face(f).name, f)).collect();
        let places = self.places()?;
        let mut cands: Vec<String> = vec![];
        let add = |t: String, cands: &mut Vec<String>| {
            if !cands.contains(&t) {
                cands.push(t);
            }
        };
        for n in want {
            if let Some(t) = anchors.faces.get(n) {
                add(t.clone(), &mut cands);
            }
            for p in places.get(n).into_iter().flatten() {
                add(p.clone(), &mut cands);
            }
        }
        for n in want {
            let Some(&f) = ids.get(n) else {
                return Err(unnamed());
            };
            let kind = select::surface_kind(&body.face(f).surface);
            let r = select::radius(&body, Ent::Face(f));
            let mut heads: Vec<String> = feats.iter().map(|f| format!("faces(of: {f}")).collect();
            heads.extend(
                places
                    .get(n)
                    .into_iter()
                    .flatten()
                    .map(|p| format!("faces(of: {p}")),
            );
            for h in heads {
                add(format!("{h}, shape: \"{kind}\")"), &mut cands);
                if let Some(r) = r {
                    add(format!("{h}, shape: \"{kind}\", r: {})", lit(r)), &mut cands);
                }
            }
        }
        let got = self.ask_faces(&cands)?;
        let mut picks = vec![];
        let mut left: BTreeSet<Name> = want.clone();
        for (c, s) in cands.into_iter().zip(&got) {
            let Some(s) = s else { continue };
            if !s.is_empty() && s.is_subset(want) && s.iter().any(|x| left.contains(x)) {
                left.retain(|x| !s.contains(x));
                picks.push(c);
            }
        }
        if !left.is_empty() {
            return Err(unnamed());
        }
        let whole = self.ask_faces(&[picks.join(" + ")])?;
        if whole.first().cloned().flatten().as_ref() != Some(want) {
            return Err(Fail::Unresolved(
                "a selection that would not name its faces alone".into(),
            ));
        }
        Ok(picks.join(" + "))
    }
}

fn dump(b: &Body) -> Vec<Edge> {
    let mut out = vec![];
    for e in select::all(b) {
        let Ent::Edge(x) = e else { continue };
        let edge = b.edge(x);
        let tan = |t: f64| edge.curve.tangent(t).map(|u| (u.get().x, u.get().y, u.get().z));
        out.push(Edge {
            h: edge.name,
            c: select::crease(b, x).unwrap_or(""),
            v: [edge.start.0 as usize, edge.end.0 as usize],
            t: [tan(edge.range.lo), tan(edge.range.hi)],
            p: b.edge_points(x, 16).iter().map(|q| (q.x, q.y, q.z)).collect(),
            f: select::faces_of_edge(b, x)
                .into_iter()
                .map(|f| b.face(f).name)
                .collect(),
        });
    }
    out
}

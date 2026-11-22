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
    /// narrowed by its shape, radius or length where that names more edges than these. The
    /// selection's text, and the params its numbers need (`(name, value)`, read in the feature's
    /// statement). Unresolved where no narrowing keeps out an edge not wanted.
    pub fn select(
        &mut self,
        want: &BTreeSet<Name>,
        feats: &[String],
    ) -> Result<(String, Vec<(String, f64)>), Fail> {
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
        let mut cands: Vec<Cand> = vec![];
        let mut bases: Vec<String> = vec![];
        for n in want {
            if let Some(t) = anchors.edges.get(n)
                && !bases.contains(t)
            {
                bases.push(t.clone());
            }
        }
        for b in &bases {
            cands.push((b.clone(), b.clone(), vec![]));
        }
        let mut k = 0;
        for n in want {
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
            let mut heads: Vec<String> = anchors
                .edges
                .get(n)
                .map(|t| t.strip_suffix(')').unwrap_or(t).to_string())
                .into_iter()
                .collect();
            // the copy or place holding a face of it: its edges, its crease, or where it meets the
            // face on the other side
            let fs: Vec<Name> = select::faces_of_edge(&body, e)
                .iter()
                .map(|&f| body.face(f).name)
                .collect();
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
            for h in &heads {
                for f in &filters {
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
                        format!("{h}, shape: \"{kind}\"{p})"),
                        format!("{h}, shape: \"{kind}\"{t})"),
                        ps,
                    ));
                    k += 1;
                }
            }
        }
        let probes: Vec<String> = cands.iter().map(|c| c.0.clone()).collect();
        let got = self.ask(&probes)?;
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
        // the union, asked as a whole: exactly the edges wanted
        let union: Vec<&str> = picks.iter().map(|p| p.0.as_str()).collect();
        let whole = self.ask(&[union.join(" + ")])?;
        if whole.first().cloned().flatten().as_ref() != Some(want) {
            return Err(Fail::Unresolved(
                "a selection that would not name its edges alone".into(),
            ));
        }
        let text: Vec<&str> = picks.iter().map(|p| p.1.as_str()).collect();
        Ok((text.join(" + "), picks.into_iter().flat_map(|p| p.2).collect()))
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
        });
    }
    out
}

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
        self.anchors = run.anchors;
        self.body = run.body;
        Ok(self.body.as_ref().map(dump).unwrap_or_default())
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

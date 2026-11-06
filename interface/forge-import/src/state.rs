//! The part as built from the lines so far: its edges, by handle, for references resolved against
//! the geometry. One model per body, so a longer history rebuilds only its tail.

use crate::Fail;
use crate::py::V3;
use forge_agent::model::Model;
use forge_agent::select::{self, Ent};
use forge_geom::CurveGeom;

pub struct Edge {
    pub h: String,
    pub c: &'static str,
    pub v: [usize; 2],
    pub t: [Option<V3>; 2],
    /// 17 points evenly along it.
    pub p: Vec<V3>,
}

#[derive(Default)]
pub struct State {
    model: Option<Model>,
}

/// As a message travels in JSON from the kernel's process: quotes, backslashes, controls blanked.
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

impl State {
    pub fn edges(&mut self, lines: &[String]) -> Result<Vec<Edge>, Fail> {
        let src = lines.join("\n") + "\n";
        let model = &mut self.model;
        let got = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match model {
                Some(m) => m.update(&src),
                None => *model = Some(Model::new(&src)),
            }
            let m = model.as_ref().expect("just made");
            if let Some(f) = &m.error {
                return Err(Fail::Stop(esc(&f.message())));
            }
            Ok(m.body().map(dump).unwrap_or_default())
        }));
        got.unwrap_or_else(|_| {
            self.model = None;
            Err(Fail::Stop("kernel panic".into()))
        })
    }
}

fn dump(b: &forge_topo::Body) -> Vec<Edge> {
    let hs = select::handles(b);
    let mut out = vec![];
    for e in select::all(b) {
        let Ent::Edge(x) = e else { continue };
        let edge = b.edge(x);
        let tan = |t: f64| edge.curve.tangent(t).map(|u| (u.get().x, u.get().y, u.get().z));
        out.push(Edge {
            h: hs.get(&select::name(b, e)).cloned().unwrap_or_default(),
            c: select::crease(b, x).unwrap_or(""),
            v: [edge.start.0 as usize, edge.end.0 as usize],
            t: [tan(edge.range.lo), tan(edge.range.hi)],
            p: b.edge_points(x, 16).iter().map(|q| (q.x, q.y, q.z)).collect(),
        });
    }
    out
}

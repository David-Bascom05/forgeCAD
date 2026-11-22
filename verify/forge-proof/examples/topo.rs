//! A .fg part's edges and faces after its last statement, for importers that resolve references
//! against the geometry (tools/find-parts/recreate). Keeps one cache, so a longer source rebuilds
//! only its tail. stdin: `N\n` then N bytes of source, repeatedly. stdout: one JSON line per source.
//!
//! `sel N\n` then N bytes: a first line `edges NAME..` or `faces NAME..` (the `n` of the edges, or
//! the `name` of the faces, dumped for the same source), then the source. stdout: `{"sel":TEXT}`, a
//! Forge selection naming exactly those by lineage (forge_import::state), its numbers written out;
//! `{"unresolved":WHY}` where none does; `{"stop":WHY}` where the source does not build.
//!
//! `ask N\n` then N bytes: a first line, a selection's text, then the source. stdout:
//! `{"names":[NAME..]}`, the edges or faces it names on the part the source builds.
use forge_agent::select::{self, Ent};
use forge_geom::{CurveGeom, SurfaceGeom};
use forge_import::Fail;
use forge_import::state::State;
use forge_lang::fuzz::Built;
use forge_math::Point3;
use forge_topo::Name;
use std::collections::BTreeSet;
use std::io::{BufRead, Read, Write};

fn pts(p: &[Point3]) -> String {
    let t: Vec<String> = p.iter().map(|q| format!("[{},{},{}]", q.x, q.y, q.z)).collect();
    format!("[{}]", t.join(","))
}

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

fn dump(m: &Built, src: &str) -> String {
    let mut o = format!(
        "{{\"built\":{},\"lines\":{}",
        m.built(),
        forge_lang::fuzz::statements(src).len()
    );
    if let Some(f) = m.error() {
        o += &format!(
            ",\"at\":{},\"err\":\"{}\"",
            m.line_of(src, f),
            esc(&forge_lang::fuzz::message(f))
        );
    }
    let Some(b) = m.body() else {
        return o + "}";
    };
    let b = &b;
    let hs = select::handles(b);
    let h = |e: Ent| hs.get(&select::name(b, e)).cloned().unwrap_or_default();
    let mut edges = vec![];
    let mut faces = vec![];
    for e in select::all(b) {
        match e {
            Ent::Edge(x) => {
                let edge = b.edge(x);
                let fs: Vec<String> = select::faces_of_edge(b, x)
                    .into_iter()
                    .map(|f| format!("\"{}\"", h(Ent::Face(f))))
                    .collect();
                let tan = |t: f64| edge.curve.tangent(t).map(|u| u.get());
                let (t0, t1) = (tan(edge.range.lo), tan(edge.range.hi));
                let tt: Vec<String> = [t0, t1]
                    .iter()
                    .map(|t| t.map_or("null".into(), |v| format!("[{},{},{}]", v.x, v.y, v.z)))
                    .collect();
                // the cosine between the faces' normals at each end (1: they meet tangentially there)
                let level = |t: f64| -> String {
                    let p = edge.curve.eval(t);
                    let n: Vec<_> = edge
                        .coedges
                        .iter()
                        .filter_map(|&c| {
                            let f = b.coedge_face(c);
                            b.face_normal(f, b.face(f).surface.project(p))
                        })
                        .collect();
                    if let [n0, n1] = n[..] {
                        format!("{}", n0.dot(n1.get()))
                    } else {
                        "null".into()
                    }
                };
                let dd = [level(edge.range.lo), level(edge.range.hi)];
                edges.push(format!(
                    "{{\"n\":{},\"h\":\"{}\",\"f\":[{}],\"k\":\"{}\",\"c\":\"{}\",\"v\":[{},{}],\"t\":[{}],\"d\":[{}],\"p\":{}}}",
                    edge.name.0,
                    h(e),
                    fs.join(","),
                    select::curve_kind(&edge.curve),
                    select::crease(b, x).unwrap_or(""),
                    edge.start.0,
                    edge.end.0,
                    tt.join(","),
                    dd.join(","),
                    pts(&b.edge_points(x, 16))
                ));
            }
            Ent::Face(f) => {
                let n = select::plane_normal(b, f)
                    .map_or("null".into(), |v| format!("[{},{},{}]", v.x, v.y, v.z));
                faces.push(format!(
                    "{{\"name\":{},\"h\":\"{}\",\"k\":\"{}\",\"n\":{n},\"p\":{}}}",
                    b.face(f).name.0,
                    h(e),
                    select::surface_kind(&b.face(f).surface),
                    pts(&select::samples(b, e))
                ));
            }
            Ent::Vert(_) => {}
        }
    }
    format!(
        "{o},\"edges\":[{}],\"faces\":[{}]}}",
        edges.join(","),
        faces.join(",")
    )
}

/// A selection of the edges or faces a `sel` request names.
fn select(state: &mut State, req: &str) -> String {
    let (head, src) = req.split_once('\n').unwrap_or((req, ""));
    let mut words = head.split_whitespace();
    let what = words.next().unwrap_or("");
    let want: BTreeSet<Name> = words.filter_map(|w| w.parse().ok()).map(Name).collect();
    let got = state.edges(src).and_then(|_| {
        let feats = state.made().to_vec();
        if what == "faces" {
            state.select_faces(&want, &feats)
        } else {
            state.select_literal(&want, &feats)
        }
    });
    match got {
        Ok(t) => format!("{{\"sel\":\"{}\"}}", t.replace('\\', "\\\\").replace('"', "\\\"")),
        Err(Fail::Stop(w)) => format!("{{\"stop\":\"{}\"}}", esc(&w)),
        Err(f) => format!("{{\"unresolved\":\"{}\"}}", esc(f.why())),
    }
}

/// The names of the edges or faces an `ask` request's selection names.
fn names(state: &mut State, req: &str) -> String {
    let (text, src) = req.split_once('\n').unwrap_or((req, ""));
    match state.names(src, text) {
        Ok(ns) => {
            let t: Vec<String> = ns.iter().map(|n| n.0.to_string()).collect();
            format!("{{\"names\":[{}]}}", t.join(","))
        }
        Err(f) => format!("{{\"stop\":\"{}\"}}", esc(f.why())),
    }
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let stdin = std::io::stdin();
    let mut r = stdin.lock();
    let mut cache: Option<forge_lang::build::Cache> = None;
    let mut state = State::default();
    let mut line = String::new();
    loop {
        line.clear();
        if r.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let t = line.trim().to_string();
        let (verb, n) = match t.split_once(' ') {
            Some((v, n)) => (v, n.parse::<usize>().expect("a length")),
            None => ("", t.parse().expect("a length")),
        };
        let mut buf = vec![0; n];
        r.read_exact(&mut buf).expect("the source");
        let src = String::from_utf8_lossy(&buf).into_owned();
        if !verb.is_empty() {
            let ask = verb == "ask";
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if ask {
                    names(&mut state, &src)
                } else {
                    select(&mut state, &src)
                }
            }))
            .unwrap_or_else(|_| {
                state = State::default();
                "{\"stop\":\"kernel panic\"}".to_string()
            });
            let mut w = std::io::stdout().lock();
            let _ = writeln!(w, "{out}");
            let _ = w.flush();
            continue;
        }
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (m, c) = forge_lang::fuzz::build_cached(
                &src,
                forge_lang::fuzz::options(),
                cache.take().unwrap_or_default(),
            );
            cache = Some(c);
            dump(&m, &src)
        }))
        .unwrap_or_else(|p| {
            cache = None;
            let t = p.downcast_ref::<String>().cloned().unwrap_or_default();
            format!("{{\"panic\":\"{}\"}}", esc(&t))
        });
        let mut w = std::io::stdout().lock();
        let _ = writeln!(w, "{out}");
        let _ = w.flush();
    }
}

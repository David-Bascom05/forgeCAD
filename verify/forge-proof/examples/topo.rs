//! A part's edges and faces after its last line, for importers that resolve references against the
//! geometry (tools/find-parts/recreate). Keeps one model, so a longer source rebuilds only its tail.
//! stdin: `N\n` then N bytes of source, repeatedly. stdout: one JSON line per source.
use forge_agent::model::Model;
use forge_agent::select::{self, Ent};
use forge_geom::CurveGeom;
use forge_math::Point3;
use std::io::{BufRead, Read, Write};

fn pts(p: &[Point3]) -> String {
    let t: Vec<String> = p.iter().map(|q| format!("[{},{},{}]", q.x, q.y, q.z)).collect();
    format!("[{}]", t.join(","))
}

fn esc(s: &str) -> String {
    s.chars()
        .map(|c| if c == '"' || c == '\\' || (c as u32) < 32 { ' ' } else { c })
        .collect()
}

fn dump(m: &Model) -> String {
    let mut o = format!("{{\"built\":{},\"lines\":{}", m.built(), m.lines.len());
    if let Some(f) = &m.error {
        o += &format!(",\"at\":{},\"err\":\"{}\"", f.at, esc(&f.msg));
    }
    let Some(b) = m.body() else {
        return o + "}";
    };
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
                edges.push(format!(
                    "{{\"h\":\"{}\",\"f\":[{}],\"k\":\"{}\",\"c\":\"{}\",\"v\":[{},{}],\"t\":[{}],\"p\":{}}}",
                    h(e),
                    fs.join(","),
                    select::curve_kind(&edge.curve),
                    select::crease(b, x).unwrap_or(""),
                    edge.start.0,
                    edge.end.0,
                    tt.join(","),
                    pts(&b.edge_points(x, 16))
                ));
            }
            Ent::Face(f) => {
                let n = select::plane_normal(b, f)
                    .map_or("null".into(), |v| format!("[{},{},{}]", v.x, v.y, v.z));
                faces.push(format!(
                    "{{\"h\":\"{}\",\"k\":\"{}\",\"n\":{n},\"p\":{}}}",
                    h(e),
                    select::surface_kind(&b.face(f).surface),
                    pts(&select::samples(b, e))
                ));
            }
            Ent::Vert(_) => {}
        }
    }
    format!("{o},\"edges\":[{}],\"faces\":[{}]}}", edges.join(","), faces.join(","))
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let stdin = std::io::stdin();
    let mut r = stdin.lock();
    let mut m: Option<Model> = None;
    let mut line = String::new();
    loop {
        line.clear();
        if r.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let n: usize = line.trim().parse().expect("a length");
        let mut buf = vec![0; n];
        r.read_exact(&mut buf).expect("the source");
        let src = String::from_utf8_lossy(&buf).into_owned();
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match &mut m {
                Some(m) => m.update(&src),
                None => m = Some(Model::new(&src)),
            }
            dump(m.as_ref().expect("just made"))
        }))
        .unwrap_or_else(|p| {
            m = None;
            let t = p.downcast_ref::<String>().cloned().unwrap_or_default();
            format!("{{\"panic\":\"{}\"}}", esc(&t))
        });
        let mut w = std::io::stdout().lock();
        let _ = writeln!(w, "{out}");
        let _ = w.flush();
    }
}

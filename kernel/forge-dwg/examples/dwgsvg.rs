//! A file's model space as SVG, for looking at the scene. `dwgsvg FILE OUT.svg`
use forge_dwg::scene::{BoxFont, Options, Prim, Space, build};
use std::fmt::Write;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let d = forge_dwg::file::read(&std::fs::read(&a[1]).unwrap()).unwrap();
    let s = build(&d, Space::Model, &BoxFont, &Options::default());
    let Some((lo, hi)) = s.extents else {
        eprintln!("empty");
        return;
    };
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let scale = 1600.0 / w.max(h).max(1e-9);
    let mut out = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' style='background:#1e1e1e'>\n",
        (w * scale).ceil() + 20.0,
        (h * scale).ceil() + 20.0
    );
    let p = |q: &[f64; 2]| {
        format!(
            "{:.2},{:.2}",
            (q[0] - lo[0]) * scale + 10.0,
            (hi[1] - q[1]) * scale + 10.0
        )
    };
    for it in &s.items {
        let c = format!("rgb({},{},{})", it.rgb[0], it.rgb[1], it.rgb[2]);
        match &it.prim {
            Prim::Path { pts, closed } => {
                let tag = if *closed { "polygon" } else { "polyline" };
                let pts: Vec<String> = pts.iter().map(p).collect();
                let _ = writeln!(
                    out,
                    "<{tag} points='{}' fill='none' stroke='{c}' stroke-width='1'/>",
                    pts.join(" ")
                );
            }
            Prim::Fill { rings } => {
                let d: String = rings
                    .iter()
                    .map(|r| format!("M{}Z", r.iter().map(p).collect::<Vec<_>>().join("L")))
                    .collect();
                let _ = writeln!(out, "<path d='{d}' fill='{c}' fill-rule='evenodd'/>");
            }
        }
    }
    out.push_str("</svg>\n");
    std::fs::write(&a[2], out).unwrap();
    eprintln!(
        "{} items, {} layers, extents {lo:?} {hi:?}",
        s.items.len(),
        s.layers.len()
    );
}

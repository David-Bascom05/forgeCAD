//! A page as SVG, for a quick look in a browser: y flipped, millimetres.

use crate::page::{Align, Page};
use std::fmt::Write;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn svg(p: &Page) -> String {
    let mut o = String::new();
    let _ = write!(
        o,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">\n<rect width=\"{w}\" height=\"{h}\" fill=\"white\"/>\n<g transform=\"translate(0 {h}) scale(1 -1)\" fill=\"none\" stroke-linecap=\"round\">\n",
        w = p.w,
        h = p.h
    );
    for s in &p.strokes {
        let pts = s.geom.points(0.01);
        let mut d = String::new();
        for (k, q) in pts.iter().enumerate() {
            let _ = write!(d, "{}{:.4} {:.4}", if k == 0 { "M" } else { " L" }, q.x, q.y);
        }
        let c = s.pen.colour().map(|x| (x * 255.0) as u8);
        let dash = s.pen.dashes();
        let dash = if dash.is_empty() {
            String::new()
        } else {
            format!(
                " stroke-dasharray=\"{}\"",
                dash.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
            )
        };
        let _ = writeln!(
            o,
            "<path d=\"{d}\" stroke=\"rgb({},{},{})\" stroke-width=\"{}\"{dash}/>",
            c[0],
            c[1],
            c[2],
            s.pen.width()
        );
    }
    for (pen, poly) in &p.fills {
        let c = pen.colour().map(|x| (x * 255.0) as u8);
        let pts: Vec<String> = poly.iter().map(|q| format!("{:.4},{:.4}", q.x, q.y)).collect();
        let _ = writeln!(
            o,
            "<polygon points=\"{}\" fill=\"rgb({},{},{})\"/>",
            pts.join(" "),
            c[0],
            c[1],
            c[2]
        );
    }
    o.push_str("</g>\n");
    for t in &p.texts {
        let anchor = match t.align {
            Align::Left => "start",
            Align::Centre => "middle",
            Align::Right => "end",
        };
        let c = t.pen.colour().map(|x| (x * 255.0) as u8);
        let _ = writeln!(
            o,
            "<text x=\"{:.4}\" y=\"{:.4}\" font-family=\"Helvetica, Arial, sans-serif\" font-size=\"{:.4}\" text-anchor=\"{anchor}\" fill=\"rgb({},{},{})\" transform=\"rotate({:.4} {:.4} {:.4})\">{}</text>",
            t.at.x,
            p.h - t.at.y,
            t.h / 0.718,
            c[0],
            c[1],
            c[2],
            -t.angle,
            t.at.x,
            p.h - t.at.y,
            esc(&t.text)
        );
    }
    o.push_str("</svg>\n");
    o
}

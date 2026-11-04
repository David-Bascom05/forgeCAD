//! Pages as one vector PDF (1.4): lines and arcs as paths (conics as cubic Béziers, exact up to
//! the Bézier's own 2.7e-4 r for a quarter turn), text in the base font Helvetica, nothing
//! embedded.

use crate::geom2::Geom2;
use crate::page::{Align, Page, text_width};
use forge_math::dmath;
use std::fmt::Write;

const PT: f64 = 72.0 / 25.4;

fn n(x: f64) -> String {
    let s = format!("{x:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn path(o: &mut String, g: &Geom2) {
    match g {
        Geom2::Line { a, b } => {
            let _ = write!(o, "{} {} m {} {} l ", n(a.x), n(a.y), n(b.x), n(b.y));
        }
        Geom2::Conic { c, u, v, t0, t1 } => {
            let pieces = (((t1 - t0) / dmath::FRAC_PI_2).ceil() as usize).max(1);
            let at = |t: f64| {
                let (s, co) = dmath::sin_cos(t);
                (*c + *u * co + *v * s, *v * co - *u * s)
            };
            let (p0, _) = at(*t0);
            let _ = write!(o, "{} {} m ", n(p0.x), n(p0.y));
            for k in 0..pieces {
                let a = t0 + (t1 - t0) * k as f64 / pieces as f64;
                let b = t0 + (t1 - t0) * (k + 1) as f64 / pieces as f64;
                let h = 4.0 / 3.0 * dmath::sin((b - a) / 4.0) / dmath::cos((b - a) / 4.0);
                let ((pa, da), (pb, db)) = (at(a), at(b));
                let (c1, c2) = (pa + da * h, pb - db * h);
                let _ = write!(
                    o,
                    "{} {} {} {} {} {} c ",
                    n(c1.x),
                    n(c1.y),
                    n(c2.x),
                    n(c2.y),
                    n(pb.x),
                    n(pb.y)
                );
            }
        }
        Geom2::Poly(p) => {
            for (k, q) in p.iter().enumerate() {
                let _ = write!(o, "{} {} {} ", n(q.x), n(q.y), if k == 0 { "m" } else { "l" });
            }
        }
    }
}

/// The text as a PDF string for a font in /WinAnsiEncoding: ASCII as it is, Latin-1 and the
/// WinAnsi extras (dashes, quotes, bullet, ellipsis, euro) by their codes, Ø for ⌀, else `?`.
pub fn pdf_str(s: &str) -> String {
    let mut o = String::from("(");
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                o.push('\\');
                o.push(c);
            }
            ' '..='~' => o.push(c),
            _ => {
                let code = match c {
                    '\u{2300}' => 0xd8,
                    '\u{a0}'..='\u{ff}' => c as u32,
                    '\u{20ac}' => 0x80,
                    '\u{2026}' => 0x85,
                    '\u{2018}' => 0x91,
                    '\u{2019}' => 0x92,
                    '\u{201c}' => 0x93,
                    '\u{201d}' => 0x94,
                    '\u{2022}' => 0x95,
                    '\u{2013}' => 0x96,
                    '\u{2014}' => 0x97,
                    '\u{2122}' => 0x99,
                    _ => b'?' as u32,
                };
                let _ = write!(o, "\\{code:03o}");
            }
        }
    }
    o.push(')');
    o
}

fn content(p: &Page) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "{} 0 0 {} 0 0 cm 1 J 1 j", n(PT), n(PT));
    let mut strokes: Vec<_> = p.strokes.iter().collect();
    strokes.sort_by_key(|s| s.pen);
    let mut last = None;
    for s in strokes {
        if last != Some(s.pen) {
            let c = s.pen.colour();
            let dash: Vec<String> = s.pen.dashes().iter().map(|x| n(*x)).collect();
            let _ = writeln!(
                o,
                "{} w [{}] 0 d {} {} {} RG",
                n(s.pen.width()),
                dash.join(" "),
                n(c[0]),
                n(c[1]),
                n(c[2])
            );
            last = Some(s.pen);
        }
        path(&mut o, &s.geom);
        o.push_str("S\n");
    }
    for (pen, poly) in &p.fills {
        let c = pen.colour();
        let _ = write!(o, "{} {} {} rg ", n(c[0]), n(c[1]), n(c[2]));
        path(&mut o, &Geom2::Poly(poly.clone()));
        o.push_str("h f\n");
    }
    for t in &p.texts {
        let c = t.pen.colour();
        let size = t.h / 0.718;
        let w = text_width(&t.text, t.h);
        let shift = match t.align {
            Align::Left => 0.0,
            Align::Centre => -0.5 * w,
            Align::Right => -w,
        };
        let (s, co) = dmath::sin_cos(t.angle.to_radians());
        let at = t.at + forge_math::Vec2::new(co, s) * shift;
        let _ = writeln!(
            o,
            "BT {} {} {} rg /F1 {} Tf {} {} {} {} {} {} Tm {} Tj ET",
            n(c[0]),
            n(c[1]),
            n(c[2]),
            n(size),
            n(co),
            n(s),
            n(-s),
            n(co),
            n(at.x),
            n(at.y),
            pdf_str(&t.text)
        );
    }
    o
}

pub fn pdf(pages: &[Page]) -> Vec<u8> {
    let mut objs: Vec<String> = Vec::new();
    // 1 catalog, 2 pages, 3 font, then a page and its content per page
    objs.push("<< /Type /Catalog /Pages 2 0 R >>".into());
    let kids: Vec<String> = (0..pages.len()).map(|k| format!("{} 0 R", 4 + 2 * k)).collect();
    objs.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        pages.len()
    ));
    objs.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into());
    for (k, p) in pages.iter().enumerate() {
        objs.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            n(p.w * PT),
            n(p.h * PT),
            5 + 2 * k
        ));
        let c = content(p);
        objs.push(format!("<< /Length {} >>\nstream\n{c}endstream", c.len()));
    }
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (k, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{o}\nendobj\n", k + 1);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1);
    for off in offsets {
        let _ = writeln!(out, "{off:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objs.len() + 1
    );
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Pen, Stroke, Text};
    use forge_math::{Point2, Vec2};

    #[test]
    fn a_page_is_a_pdf() {
        let page = Page {
            w: 297.0,
            h: 210.0,
            strokes: vec![Stroke {
                pen: Pen::Visible,
                geom: Geom2::Conic {
                    c: Point2::new(100.0, 100.0),
                    u: Vec2::new(20.0, 0.0),
                    v: Vec2::new(0.0, 10.0),
                    t0: 0.0,
                    t1: dmath::TAU,
                },
            }],
            fills: vec![],
            texts: vec![Text {
                at: Point2::new(10.0, 10.0),
                h: 3.5,
                angle: 0.0,
                align: Align::Left,
                text: "\u{2300}6 (2x)".into(),
                pen: Pen::Thin,
            }],
        };
        let b = pdf(&[page]);
        let s = String::from_utf8(b).unwrap();
        assert!(s.starts_with("%PDF-1.4"));
        assert!(s.contains("(\\3306 \\(2x\\)) Tj"));
        // four Béziers for the ellipse
        assert_eq!(s.matches(" c ").count(), 4);
        let x = s.find("startxref\n").unwrap();
        let at: usize = s[x + 10..].lines().next().unwrap().parse().unwrap();
        assert!(s[at..].starts_with("xref"));
    }
}

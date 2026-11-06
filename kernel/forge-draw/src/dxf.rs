//! A page as an AutoCAD R2000 (AC1015) DXF in millimetres: a layer per line type (VISIBLE,
//! HIDDEN, CENTER, ..) with its line type; lines, arcs, circles and ellipses exactly, other curves
//! as polylines, text, arrowheads as solids. Dimensions are written exploded (lines and text),
//! as most exchanges do.

use crate::geom2::Geom2;
use crate::page::{Align, Page, Pen};
use forge_math::dmath;
use std::fmt::Write;

struct Dxf {
    out: String,
    next: u32,
}

impl Dxf {
    fn pair(&mut self, code: u16, v: &str) {
        let _ = write!(self.out, "{code:>3}\n{v}\n");
    }
    fn num(&mut self, code: u16, v: f64) {
        let _ = write!(self.out, "{code:>3}\n{v:?}\n");
    }
    fn hex(&mut self, code: u16, h: u32) {
        let _ = write!(self.out, "{code:>3}\n{h:X}\n");
    }
    fn handle(&mut self) -> u32 {
        self.next += 1;
        self.next - 1
    }
    fn table(&mut self, name: &str, h: u32, rows: impl FnOnce(&mut Dxf)) {
        let start = self.out.len();
        rows(self);
        let body = self.out.split_off(start);
        let n = body.matches(&format!("  0\n{name}\n")).count();
        self.pair(0, "TABLE");
        self.pair(2, name);
        self.hex(5, h);
        self.pair(330, "0");
        self.pair(100, "AcDbSymbolTable");
        self.pair(70, &n.to_string());
        if name == "DIMSTYLE" {
            self.pair(100, "AcDbDimStyleTable");
        }
        self.out.push_str(&body);
        self.pair(0, "ENDTAB");
    }
    fn head(&mut self, kind: &str, layer: &str, owner: u32) {
        let h = self.handle();
        self.pair(0, kind);
        self.hex(5, h);
        self.hex(330, owner);
        self.pair(100, "AcDbEntity");
        self.pair(8, layer);
    }
}

/// The line types the pens use: name, description, pattern (mm; negative a gap, 0 a dot).
pub(crate) fn line_types() -> Vec<(&'static str, &'static str, Vec<f64>)> {
    let pat = |p: Pen| -> Vec<f64> {
        p.dashes()
            .iter()
            .enumerate()
            .map(|(k, x)| if k % 2 == 0 { *x } else { -*x })
            .collect()
    };
    vec![
        ("HIDDEN", "Hidden __ __ __", pat(Pen::Hidden)),
        ("CENTER", "Center ____ _ ____", pat(Pen::Center)),
        ("PHANTOM", "Phantom ____ _ _ ____", pat(Pen::Phantom)),
    ]
}

pub(crate) fn line_type(p: Pen) -> &'static str {
    match p {
        Pen::Hidden => "HIDDEN",
        Pen::Center | Pen::Cut => "CENTER",
        Pen::Phantom => "PHANTOM",
        _ => "Continuous",
    }
}

/// AutoCAD colour index per pen.
pub(crate) fn colour(p: Pen) -> &'static str {
    match p {
        Pen::Visible | Pen::Border => "7",
        Pen::Hidden => "8",
        Pen::Tangent => "9",
        Pen::Center | Pen::Phantom => "1",
        Pen::Thin => "3",
        Pen::Hatch => "4",
        Pen::Cut => "1",
        Pen::Sick => "6",
    }
}

/// Lineweight in hundredths of a mm.
pub(crate) fn weight(p: Pen) -> String {
    ((p.width() * 100.0).round() as i64).to_string()
}

pub fn dxf(p: &Page) -> String {
    let mut w = Dxf {
        out: String::new(),
        next: 0x20,
    };
    let ms_rec = 0x1F;
    let ps_rec = 0x1E;
    w.pair(0, "SECTION");
    w.pair(2, "HEADER");
    w.pair(9, "$ACADVER");
    w.pair(1, "AC1015");
    w.pair(9, "$INSUNITS");
    w.pair(70, "4");
    w.pair(9, "$MEASUREMENT");
    w.pair(70, "1");
    w.pair(9, "$LTSCALE");
    w.num(40, 1.0);
    w.pair(9, "$EXTMIN");
    for (c, v) in [(10, 0.0), (20, 0.0), (30, 0.0)] {
        w.num(c, v);
    }
    w.pair(9, "$EXTMAX");
    for (c, v) in [(10, p.w), (20, p.h), (30, 0.0)] {
        w.num(c, v);
    }
    w.pair(9, "$HANDSEED");
    let seed_at = w.out.len();
    w.pair(5, "SEED");
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "TABLES");
    w.table("VPORT", 0x8, |w| {
        let h = w.handle();
        w.pair(0, "VPORT");
        w.hex(5, h);
        w.hex(330, 0x8);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbViewportTableRecord");
        w.pair(2, "*Active");
        w.pair(70, "0");
        for (c, v) in [(10, 0.0), (20, 0.0), (11, 1.0), (21, 1.0)] {
            w.num(c, v);
        }
        for (c, v) in [(12, p.w / 2.0), (22, p.h / 2.0), (40, p.h), (41, p.w / p.h)] {
            w.num(c, v);
        }
    });
    w.table("LTYPE", 0x5, |w| {
        for name in ["ByBlock", "ByLayer", "Continuous"] {
            let h = w.handle();
            w.pair(0, "LTYPE");
            w.hex(5, h);
            w.hex(330, 0x5);
            w.pair(100, "AcDbSymbolTableRecord");
            w.pair(100, "AcDbLinetypeTableRecord");
            w.pair(2, name);
            w.pair(70, "0");
            w.pair(3, if name == "Continuous" { "Solid line" } else { "" });
            w.pair(72, "65");
            w.pair(73, "0");
            w.num(40, 0.0);
        }
        for (name, desc, pat) in line_types() {
            let h = w.handle();
            w.pair(0, "LTYPE");
            w.hex(5, h);
            w.hex(330, 0x5);
            w.pair(100, "AcDbSymbolTableRecord");
            w.pair(100, "AcDbLinetypeTableRecord");
            w.pair(2, name);
            w.pair(70, "0");
            w.pair(3, desc);
            w.pair(72, "65");
            w.pair(73, &pat.len().to_string());
            w.num(40, pat.iter().map(|x| x.abs()).sum());
            for x in pat {
                w.num(49, x);
                w.pair(74, "0");
            }
        }
    });
    w.table("LAYER", 0x2, |w| {
        let mut layers: Vec<(&str, &str, &str, String)> = vec![("0", "7", "Continuous", "-3".into())];
        for pen in Pen::ALL {
            layers.push((pen.layer(), colour(pen), line_type(pen), weight(pen)));
        }
        layers.push(("TEXT", "3", "Continuous", "-3".into()));
        for (name, c, lt, lw) in layers {
            let h = w.handle();
            w.pair(0, "LAYER");
            w.hex(5, h);
            w.hex(330, 0x2);
            w.pair(100, "AcDbSymbolTableRecord");
            w.pair(100, "AcDbLayerTableRecord");
            w.pair(2, name);
            w.pair(70, "0");
            w.pair(62, c);
            w.pair(6, lt);
            w.pair(370, &lw);
            w.pair(390, "F");
        }
    });
    w.table("STYLE", 0x3, |w| {
        let h = w.handle();
        w.pair(0, "STYLE");
        w.hex(5, h);
        w.hex(330, 0x3);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbTextStyleTableRecord");
        w.pair(2, "Standard");
        w.pair(70, "0");
        w.num(40, 0.0);
        w.num(41, 1.0);
        w.num(50, 0.0);
        w.pair(71, "0");
        w.num(42, 2.5);
        w.pair(3, "arial.ttf");
        w.pair(4, "");
    });
    w.table("VIEW", 0x6, |_| {});
    w.table("UCS", 0x7, |_| {});
    w.table("APPID", 0x9, |w| {
        let h = w.handle();
        w.pair(0, "APPID");
        w.hex(5, h);
        w.hex(330, 0x9);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbRegAppTableRecord");
        w.pair(2, "ACAD");
        w.pair(70, "0");
    });
    w.table("DIMSTYLE", 0xA, |w| {
        let h = w.handle();
        w.pair(0, "DIMSTYLE");
        w.hex(105, h);
        w.hex(330, 0xA);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbDimStyleTableRecord");
        w.pair(2, "Standard");
        w.pair(70, "0");
    });
    w.table("BLOCK_RECORD", 0x1, |w| {
        for (name, h) in [("*Model_Space", ms_rec), ("*Paper_Space", ps_rec)] {
            w.pair(0, "BLOCK_RECORD");
            w.hex(5, h);
            w.hex(330, 0x1);
            w.pair(100, "AcDbSymbolTableRecord");
            w.pair(100, "AcDbBlockTableRecord");
            w.pair(2, name);
            w.pair(340, "0");
        }
    });
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "BLOCKS");
    for (name, rec, paper) in [("*Model_Space", ms_rec, false), ("*Paper_Space", ps_rec, true)] {
        let (b, e) = (w.handle(), w.handle());
        w.pair(0, "BLOCK");
        w.hex(5, b);
        w.hex(330, rec);
        w.pair(100, "AcDbEntity");
        if paper {
            w.pair(67, "1");
        }
        w.pair(8, "0");
        w.pair(100, "AcDbBlockBegin");
        w.pair(2, name);
        w.pair(70, "0");
        for c in [10, 20, 30] {
            w.num(c, 0.0);
        }
        w.pair(3, name);
        w.pair(1, "");
        w.pair(0, "ENDBLK");
        w.hex(5, e);
        w.hex(330, rec);
        w.pair(100, "AcDbEntity");
        if paper {
            w.pair(67, "1");
        }
        w.pair(8, "0");
        w.pair(100, "AcDbBlockEnd");
    }
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "ENTITIES");
    for s in &p.strokes {
        curve(&mut w, &s.geom, s.pen.layer(), ms_rec);
    }
    for (pen, poly) in &p.fills {
        // a fan of solids
        for k in 1..poly.len().saturating_sub(1) {
            w.head("SOLID", pen.layer(), ms_rec);
            w.pair(100, "AcDbTrace");
            let q = [poly[0], poly[k], poly[k + 1], poly[k + 1]];
            for (i, pt) in q.iter().enumerate() {
                w.num(10 + i as u16, pt.x);
                w.num(20 + i as u16, pt.y);
                w.num(30 + i as u16, 0.0);
            }
        }
    }
    for t in &p.texts {
        w.head("TEXT", if t.pen == Pen::Sick { "SICK" } else { "TEXT" }, ms_rec);
        w.pair(100, "AcDbText");
        for (k, v) in [(10, t.at.x), (20, t.at.y), (30, 0.0), (40, t.h)] {
            w.num(k, v);
        }
        w.pair(1, &dxf_text(&t.text));
        w.num(50, t.angle);
        w.pair(7, "Standard");
        let h = match t.align {
            Align::Left => "0",
            Align::Centre => "1",
            Align::Right => "2",
        };
        if h != "0" {
            w.pair(72, h);
            for (k, v) in [(11, t.at.x), (21, t.at.y), (31, 0.0)] {
                w.num(k, v);
            }
        }
        w.pair(100, "AcDbText");
    }
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "OBJECTS");
    let (root, group) = (0xC, 0xD);
    w.pair(0, "DICTIONARY");
    w.hex(5, root);
    w.pair(330, "0");
    w.pair(100, "AcDbDictionary");
    w.pair(281, "1");
    w.pair(3, "ACAD_GROUP");
    w.hex(350, group);
    w.pair(0, "DICTIONARY");
    w.hex(5, group);
    w.hex(330, root);
    w.pair(100, "AcDbDictionary");
    w.pair(281, "1");
    w.pair(0, "ENDSEC");
    w.pair(0, "EOF");

    let seed = format!("{:X}", w.next);
    let line = "  5\nSEED\n";
    let mut out = w.out;
    let at = seed_at + out[seed_at..].find(line).expect("the seed placeholder");
    out.replace_range(at..at + line.len(), &format!("  5\n{seed}\n"));
    out
}

/// AutoCAD's control codes for the drawing signs: %%c diameter, %%d degree, %%p plus-minus.
pub(crate) fn dxf_text(s: &str) -> String {
    s.replace(['\u{2300}', '\u{d8}'], "%%c")
        .replace('\u{b0}', "%%d")
        .replace('\u{b1}', "%%p")
}

fn curve(w: &mut Dxf, g: &Geom2, layer: &str, owner: u32) {
    match g {
        Geom2::Line { a, b } => {
            w.head("LINE", layer, owner);
            w.pair(100, "AcDbLine");
            for (k, v) in [(10, a.x), (20, a.y), (30, 0.0), (11, b.x), (21, b.y), (31, 0.0)] {
                w.num(k, v);
            }
        }
        Geom2::Conic { c, u, v, t0, t1 } => {
            if let Some((_, r)) = g.circle() {
                let whole = t1 - t0 >= dmath::TAU * (1.0 - 1e-12);
                w.head(if whole { "CIRCLE" } else { "ARC" }, layer, owner);
                w.pair(100, "AcDbCircle");
                for (k, x) in [(10, c.x), (20, c.y), (30, 0.0), (40, r)] {
                    w.num(k, x);
                }
                if !whole {
                    let alpha = dmath::atan2(u.y, u.x);
                    let (lo, hi) = if u.cross(*v) > 0.0 {
                        (alpha + t0, alpha + t1)
                    } else {
                        (alpha - t1, alpha - t0)
                    };
                    w.pair(100, "AcDbArc");
                    w.num(50, lo.to_degrees());
                    w.num(51, hi.to_degrees());
                }
                return;
            }
            // principal axes of the conjugate pair
            let (a, b, k) = (u.dot(*u), v.dot(*v), u.dot(*v));
            let tau = 0.5 * dmath::atan2(2.0 * k, a - b);
            let (st, ct) = dmath::sin_cos(tau);
            let (mut major, mut minor) = (*u * ct + *v * st, *v * ct - *u * st);
            let mut shift = tau;
            if minor.length() > major.length() {
                let m = major;
                major = minor;
                minor = m * -1.0;
                shift += dmath::FRAC_PI_2;
            }
            // the curve is c + cos(t - shift) major + sin(t - shift) minor
            let ccw = major.cross(minor) > 0.0;
            let (p0, p1) = if ccw {
                (t0 - shift, t1 - shift)
            } else {
                (shift - t1, shift - t0)
            };
            w.head("ELLIPSE", layer, owner);
            w.pair(100, "AcDbEllipse");
            for (kk, x) in [
                (10, c.x),
                (20, c.y),
                (30, 0.0),
                (11, major.x),
                (21, major.y),
                (31, 0.0),
            ] {
                w.num(kk, x);
            }
            for (kk, x) in [(210, 0.0), (220, 0.0), (230, 1.0)] {
                w.num(kk, x);
            }
            w.num(40, minor.length() / major.length());
            w.num(41, p0);
            w.num(42, p1);
        }
        Geom2::Poly(pts) => {
            w.head("LWPOLYLINE", layer, owner);
            w.pair(100, "AcDbPolyline");
            w.pair(90, &pts.len().to_string());
            w.pair(70, "0");
            for q in pts {
                w.num(10, q.x);
                w.num(20, q.y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Stroke, Text};
    use forge_math::{Point2, Vec2};

    #[test]
    fn layers_and_entities() {
        let ell = Geom2::Conic {
            c: Point2::new(50.0, 50.0),
            u: Vec2::new(10.0, 5.0),
            v: Vec2::new(-2.0, 6.0),
            t0: 0.3,
            t1: 2.0,
        };
        let page = Page {
            w: 297.0,
            h: 210.0,
            strokes: vec![
                Stroke {
                    pen: Pen::Hidden,
                    geom: Geom2::Line {
                        a: Point2::new(0.0, 0.0),
                        b: Point2::new(10.0, 0.0),
                    },
                },
                Stroke {
                    pen: Pen::Visible,
                    geom: ell.clone(),
                },
            ],
            fills: vec![],
            texts: vec![Text {
                at: Point2::new(5.0, 5.0),
                h: 3.5,
                angle: 0.0,
                align: Align::Centre,
                text: "\u{2300}6".into(),
                pen: Pen::Thin,
            }],
        };
        let s = dxf(&page);
        assert!(s.contains("\nHIDDEN\n"));
        assert!(s.contains("\nELLIPSE\n"));
        assert!(s.contains("%%c6"));
        // the ellipse written is the conic: check a point at the written parameters
        let get = |code: &str| -> Vec<f64> {
            let lines: Vec<&str> = s.lines().collect();
            let at = lines.iter().position(|l| *l == "ELLIPSE").unwrap();
            lines[at..]
                .windows(2)
                .filter(|w| w[0].trim() == code)
                .map(|w| w[1].parse().unwrap())
                .take(1)
                .collect()
        };
        let (mx, my, ratio, p0) = (get("11")[0], get("21")[0], get("40")[0], get("41")[0]);
        let major = Vec2::new(mx, my);
        let minor = major.perp() * ratio;
        let (s0, c0) = dmath::sin_cos(p0);
        let q = Point2::new(50.0, 50.0) + major * c0 + minor * s0;
        let want = [ell.at(0.3), ell.at(2.0)];
        assert!(want.iter().any(|w| w.distance(q) < 1e-9), "{q:?} vs {want:?}");
    }
}

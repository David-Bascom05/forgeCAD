//! Dimensions as AutoCAD stores them: the DIMENSION entity (its definition points, measurement
//! and text position) and the anonymous "*D" block that is its picture (dimension and extension
//! lines, arrowheads as solids, the text as MTEXT, the definition points as POINTs on DEFPOINTS),
//! sized by the current dimension style. Other programs draw a dimension from its block, so the
//! block is what they see.

use super::geom::{PI, V2, v2};
use crate::bits::R;
use crate::edit::History;
use crate::file::Drawing;
use crate::header::Val;
use crate::io::{Color, Href};
use crate::types::{self, Data, DimCommon};

/// The sizes a dimension is drawn with: the DIMSTYLE's (or the header's) times DIMSCALE.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub handle: u32,
    /// Arrowhead size, extension past the dimension line, offset from the origin, text height,
    /// text gap.
    pub asz: f64,
    pub exe: f64,
    pub exo: f64,
    pub txt: f64,
    pub gap: f64,
    /// Decimal places, and 1 for text above the line (0 centred in it).
    pub dec: i16,
    pub tad: i16,
    pub text_style: u32,
}

fn hd(d: &Drawing, k: &str, dflt: f64) -> f64 {
    match d.header.get(k) {
        Some(Val::D(x)) if x.is_finite() => *x,
        _ => dflt,
    }
}

impl Style {
    /// The current dimension style (DIMSTYLE), else the header's variables.
    pub fn current(d: &Drawing) -> Style {
        let h = d.current("DIMSTYLE").unwrap_or(0);
        let ts = d.current("TEXTSTYLE").unwrap_or(0);
        if let Some(Data::DimStyle(s)) = d.objects.get(&h).map(|o| &o.data) {
            let v = &s.vars;
            let k = if v.scale > 0.0 { v.scale } else { 1.0 };
            return Style {
                handle: h,
                asz: v.asz * k,
                exe: v.exe * k,
                exo: v.exo * k,
                txt: v.txt * k,
                gap: v.gap.abs() * k,
                dec: v.dec,
                tad: v.tad,
                text_style: if s.txsty.handle != 0 { s.txsty.handle } else { ts },
            };
        }
        let k = hd(d, "DIMSCALE", 1.0).max(1e-12);
        Style {
            handle: h,
            asz: hd(d, "DIMASZ", 2.5) * k,
            exe: hd(d, "DIMEXE", 1.25) * k,
            exo: hd(d, "DIMEXO", 0.625) * k,
            txt: hd(d, "DIMTXT", 2.5) * k,
            gap: hd(d, "DIMGAP", 0.625).abs() * k,
            dec: 4,
            tad: 1,
            text_style: ts,
        }
    }

    fn fmt(&self, v: f64) -> String {
        format!("{:.*}", self.dec.clamp(0, 8) as usize, v)
    }
}

/// What to dimension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// DIMLINEAR: the distance between two points along `angle` (0 horizontal, pi/2 vertical),
    /// its line through `at`.
    Linear { p1: V2, p2: V2, at: V2, angle: f64 },
    /// DIMALIGNED: the distance between two points, its line parallel to them through `at`.
    Aligned { p1: V2, p2: V2, at: V2 },
    /// DIMRADIUS of a circle or arc, its text at `at`.
    Radius { c: V2, r: f64, at: V2 },
    /// DIMDIAMETER alike.
    Diameter { c: V2, r: f64, at: V2 },
}

/// A piece of the dimension's block, and whether it is a definition point (on DEFPOINTS).
pub struct Made {
    pub entity: Data,
    pub pieces: Vec<(Data, bool)>,
    pub measurement: f64,
}

fn line(a: V2, b: V2) -> Data {
    Data::Line(types::Line {
        start: a.arr3(0.0),
        end: b.arr3(0.0),
        ..types::Line::default()
    })
}

/// A closed filled arrowhead: its tip, pointing along `dir`.
fn arrow(tip: V2, dir: V2, size: f64) -> Data {
    let u = dir.unit().unwrap_or(v2(1.0, 0.0));
    let base = tip - u * size;
    let s = u.perp() * (size / 6.0);
    let (b1, b2) = (base + s, base - s);
    Data::Solid(types::Solid {
        corners: [tip.arr(), b1.arr(), b2.arr(), b2.arr()],
        ..types::Solid::default()
    })
}

fn mtext(d: &Drawing, st: &Style, at: V2, angle: f64, s: &str) -> Data {
    let ctx = types::MTextContext {
        attachment: 5,
        insert: at.arr3(0.0),
        x_axis: [dcos(angle), dsin(angle), 0.0],
        ..types::MTextContext::default()
    };
    Data::MText(types::MText {
        insert: at.arr3(0.0),
        x_axis: [dcos(angle), dsin(angle), 0.0],
        text_height: st.txt,
        attachment: 5,
        text: d.s(s),
        style: Href::new(5, st.text_style),
        context: Some(ctx),
        ..types::MText::default()
    })
}

fn dcos(a: f64) -> f64 {
    forge_math::dmath::cos(a)
}
fn dsin(a: f64) -> f64 {
    forge_math::dmath::sin(a)
}

/// Text along a direction, turned to read from below or the right.
fn readable(a: f64) -> f64 {
    let mut a = super::geom::wrap(a);
    if a > PI * 0.5 + 1e-9 && a <= PI * 1.5 + 1e-9 {
        a -= PI;
    }
    super::geom::wrap(a)
}

/// A rough width of text: for leaving a gap in the line under centred text.
fn text_w(s: &str, h: f64) -> f64 {
    s.chars().count() as f64 * h * 0.8
}

fn common(st: &Style, text_mid: V2, measurement: f64) -> DimCommon {
    DimCommon {
        text_mid: text_mid.arr(),
        // the text where the dimension put it (not moved), its block this dimension's alone
        flags1: 1 | 2,
        text_rotation: 0.0,
        measurement,
        attachment: 5,
        ins_rotation: 0.0,
        dimstyle: Href::new(5, st.handle),
        // (the block is set when it exists)
        block: Href::new(5, 0),
        ..DimCommon::default()
    }
}

/// The dimension's entity and block pieces.
pub fn make(d: &Drawing, st: &Style, k: Kind) -> Option<Made> {
    match k {
        Kind::Linear { p1, p2, at, angle } => Some(linear(d, st, p1, p2, at, angle, false)),
        Kind::Aligned { p1, p2, at } => {
            let a = (p2 - p1).angle();
            (p1.dist(p2) > 0.0).then(|| linear(d, st, p1, p2, at, a, true))
        }
        Kind::Radius { c, r, at } => radial(d, st, c, r, at, false),
        Kind::Diameter { c, r, at } => radial(d, st, c, r, at, true),
    }
}

fn linear(d: &Drawing, st: &Style, p1: V2, p2: V2, at: V2, angle: f64, aligned: bool) -> Made {
    let u = V2::polar(angle, 1.0);
    let n = u.perp();
    // the dimension line's ends: the points projected onto it
    let q1 = at + u * (p1 - at).dot(u);
    let q2 = at + u * (p2 - at).dot(u);
    let measurement = (p2 - p1).dot(u).abs();
    let mut pieces = vec![];
    for (p, q) in [(p1, q1), (p2, q2)] {
        let v = q - p;
        if let Some(dir) = v.unit() {
            pieces.push((line(p + dir * st.exo, q + dir * st.exe), false));
        } else {
            pieces.push((line(q - n * st.exo, q + n * st.exe), false));
        }
    }
    let text = st.fmt(measurement);
    let ta = readable(angle);
    let mid = q1.mid(q2);
    // which side of the line is "up" for the text
    let up = V2::polar(ta, 1.0).perp();
    let text_mid = if st.tad != 0 {
        mid + up * (st.gap + st.txt * 0.5)
    } else {
        mid
    };
    let dl = q2 - q1;
    let len = dl.len();
    if st.tad == 0 && len > 0.0 {
        // the line broken round the text
        let half = text_w(&text, st.txt) * 0.5 + st.gap;
        let du = dl / len;
        if len * 0.5 > half {
            pieces.push((line(q1, mid - du * half), false));
            pieces.push((line(mid + du * half, q2), false));
        } else {
            pieces.push((line(q1, q2), false));
        }
    } else {
        pieces.push((line(q1, q2), false));
    }
    if len > 0.0 {
        let du = dl / len;
        // arrowheads inside when there is room for them, else outside pointing in
        if len >= 2.0 * st.asz {
            pieces.push((arrow(q1, -du, st.asz), false));
            pieces.push((arrow(q2, du, st.asz), false));
        } else {
            pieces.push((arrow(q1, du, st.asz), false));
            pieces.push((arrow(q2, -du, st.asz), false));
            pieces.push((line(q1 - du * st.asz * 2.0, q1), false));
            pieces.push((line(q2, q2 + du * st.asz * 2.0), false));
        }
    }
    pieces.push((mtext(d, st, text_mid, ta, &text), false));
    for p in [p1, p2, q2] {
        pieces.push((super::make::point(p), true));
    }
    let dc = common(st, text_mid, measurement);
    let entity = if aligned {
        Data::DimAligned(types::DimAligned {
            dim: dc,
            pt13: p1.arr3(0.0),
            pt14: p2.arr3(0.0),
            pt10: q2.arr3(0.0),
            ext_rotation: 0.0,
        })
    } else {
        Data::DimLinear(types::DimLinear {
            dim: dc,
            pt13: p1.arr3(0.0),
            pt14: p2.arr3(0.0),
            pt10: q2.arr3(0.0),
            ext_rotation: 0.0,
            rotation: super::geom::wrap(angle),
        })
    };
    Made {
        entity,
        pieces,
        measurement,
    }
}

fn radial(d: &Drawing, st: &Style, c: V2, r: f64, at: V2, dia: bool) -> Option<Made> {
    if r.is_nan() || r <= 0.0 {
        return None;
    }
    let u = (at - c).unit().unwrap_or(v2(1.0, 0.0));
    let q = c + u * r;
    let far = c - u * r;
    let measurement = if dia { 2.0 * r } else { r };
    let text = format!("{}{}", if dia { "%%c" } else { "R" }, st.fmt(measurement));
    let outside = at.dist(c) > r;
    let mut pieces = vec![];
    // the leader from the arc out to the text, or the line in from the centre (or across)
    let from = if dia { far } else { c };
    if outside {
        let land = at - u * (st.gap + text_w(&text, st.txt) * 0.5);
        pieces.push((line(q, land), false));
        pieces.push((arrow(q, -u, st.asz), false));
        if dia {
            pieces.push((line(from, q), false));
            pieces.push((arrow(far, u, st.asz), false));
        }
    } else {
        pieces.push((line(from, q), false));
        pieces.push((arrow(q, u, st.asz), false));
        if dia {
            pieces.push((arrow(far, -u, st.asz), false));
        }
    }
    let ta = readable(u.angle());
    pieces.push((mtext(d, st, at, ta, &text), false));
    pieces.push((super::make::point(q), true));
    pieces.push((super::make::point(if dia { far } else { c }), true));
    let dc = common(st, at, measurement);
    let entity = if dia {
        Data::DimDiameter(types::DimDiameter {
            dim: dc,
            pt15: q.arr3(0.0),
            pt10: far.arr3(0.0),
            leader_len: 0.0,
        })
    } else {
        Data::DimRadius(types::DimRadius {
            dim: dc,
            pt10: c.arr3(0.0),
            pt15: q.arr3(0.0),
            leader_len: 0.0,
        })
    };
    Some(Made {
        entity,
        pieces,
        measurement,
    })
}

/// Puts a made dimension into `space` on `layer`: its "*D" block (pieces on layer 0 by block,
/// definition points on DEFPOINTS), then the DIMENSION referring to it. Its handle.
pub fn add(d: &mut Drawing, hist: &mut History, space: u32, layer: u32, m: Made) -> R<u32> {
    let n = d
        .objects
        .values()
        .filter_map(|o| match &o.data {
            Data::Block(b) => b.name.text().strip_prefix("*D").and_then(|k| k.parse::<u32>().ok()),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let block = d.add_block(hist, &format!("*D{}", n + 1), true)?;
    let zero = d.layer("0").unwrap_or(layer);
    let defpoints = match d.layer("DEFPOINTS") {
        Some(h) => h,
        None => {
            let h = d.add_layer(hist, "Defpoints", 7)?;
            // never plotted
            d.modify(hist, h, |o| {
                if let Data::Layer(l) = &mut o.data {
                    l.flags &= !0x10;
                }
            })?;
            h
        }
    };
    for (data, defpt) in m.pieces {
        let h = d.add_entity(hist, block, if defpt { defpoints } else { zero }, data)?;
        d.modify(hist, h, |o| {
            if let Some(e) = o.ent.as_mut() {
                e.color = Color::index(0);
                e.lineweight = if defpt { 29 } else { 30 };
                e.ltype_flags = 1;
            }
        })?;
    }
    let mut ent = m.entity;
    let dc = match &mut ent {
        Data::DimLinear(x) => &mut x.dim,
        Data::DimAligned(x) => &mut x.dim,
        Data::DimRadius(x) => &mut x.dim,
        Data::DimDiameter(x) => &mut x.dim,
        _ => return crate::bits::bad("not a dimension"),
    };
    dc.block = Href::new(5, block);
    d.add_entity(hist, space, layer, ent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st() -> Style {
        Style {
            handle: 0,
            asz: 2.5,
            exe: 1.25,
            exo: 0.625,
            txt: 2.5,
            gap: 0.625,
            dec: 2,
            tad: 1,
            text_style: 0,
        }
    }

    #[test]
    fn a_horizontal_dimension() {
        let d = Drawing::default();
        let m = make(
            &d,
            &st(),
            Kind::Linear {
                p1: v2(0.0, 0.0),
                p2: v2(30.0, 5.0),
                at: v2(10.0, 20.0),
                angle: 0.0,
            },
        )
        .unwrap();
        assert!((m.measurement - 30.0).abs() < 1e-12);
        let Data::DimLinear(x) = &m.entity else { panic!() };
        assert_eq!(x.pt10, [30.0, 20.0, 0.0]);
        // two extension lines, the line, two arrows, the text, three points
        assert_eq!(m.pieces.len(), 9);
        let texts: Vec<String> = m
            .pieces
            .iter()
            .filter_map(|(p, _)| match p {
                Data::MText(t) => Some(t.text.text()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["30.00"]);
        assert_eq!(m.pieces.iter().filter(|p| p.1).count(), 3);
    }

    #[test]
    fn a_diameter() {
        let d = Drawing::default();
        let m = make(
            &d,
            &st(),
            Kind::Diameter {
                c: v2(0.0, 0.0),
                r: 5.0,
                at: v2(20.0, 0.0),
            },
        )
        .unwrap();
        assert!((m.measurement - 10.0).abs() < 1e-12);
        let Data::DimDiameter(x) = &m.entity else { panic!() };
        assert_eq!(x.pt15, [5.0, 0.0, 0.0]);
        assert_eq!(x.pt10, [-5.0, 0.0, 0.0]);
    }
}
